use crate::errors::WideError;
use crate::models::resource::Resource;
use crate::models::span::Span;
use crate::models::trace::{InputFormat, ParseWarning, Trace};
use std::collections::{HashMap, HashSet};

pub fn build_trace(
    raw_spans: Vec<Span>,
    format: InputFormat,
    mut extra_warnings: Vec<ParseWarning>,
) -> Result<Trace, WideError> {
    if raw_spans.is_empty() {
        return Err(WideError::NoValidSpans {
            attempted: 0,
            failures: vec![],
        });
    }

    let mut warnings: Vec<ParseWarning> = std::mem::take(&mut extra_warnings);

    // 1. Deduplicate by span_id (first wins)
    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut dup_count = 0usize;
    let mut first_dup: Option<String> = None;
    let mut spans: Vec<Span> = Vec::with_capacity(raw_spans.len());

    for (idx, span) in raw_spans.into_iter().enumerate() {
        if seen_ids.contains(&span.span_id) {
            dup_count += 1;
            if first_dup.is_none() {
                first_dup = Some(span.span_id.clone());
            }
        } else {
            seen_ids.insert(span.span_id.clone());
            spans.push(span);
        }
        let _ = idx;
    }

    if dup_count > 0 {
        warnings.push(
            ParseWarning::new(
                "DUPLICATE_SPAN_ID",
                format!("{} duplicate span_id(s) discarded (first wins)", dup_count),
            )
            .with_count(dup_count)
            .with_context(serde_json::json!({
                "span_id": first_dup.unwrap_or_default(),
                "kept": "first",
            })),
        );
    }

    // 2. Group by trace_id; pick most populated
    let mut trace_groups: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, span) in spans.iter().enumerate() {
        trace_groups
            .entry(span.trace_id.clone())
            .or_default()
            .push(i);
    }

    let (primary_trace_id, primary_indices) = if trace_groups.len() == 1 {
        trace_groups.into_iter().next().unwrap()
    } else {
        let mut best_id = String::new();
        let mut best_count = 0;
        let mut all_ids: Vec<(String, usize)> = trace_groups
            .iter()
            .map(|(id, idxs)| (id.clone(), idxs.len()))
            .collect();
        all_ids.sort_by_key(|b| std::cmp::Reverse(b.1));

        for (id, count) in &all_ids {
            if *count > best_count {
                best_count = *count;
                best_id = id.clone();
            }
        }

        for (id, count) in &all_ids {
            if id != &best_id {
                warnings.push(
                    ParseWarning::new(
                        "TRACE_ID_DISCARDED",
                        format!(
                            "Discarded trace_id '{}' ({} spans); using '{}' ({} spans)",
                            id, count, best_id, best_count
                        ),
                    )
                    .with_context(serde_json::json!({
                        "discarded_trace_id": id,
                        "discarded_span_count": count,
                    })),
                );
            }
        }

        let indices = trace_groups.remove(&best_id).unwrap();
        (best_id, indices)
    };

    let spans: Vec<Span> = primary_indices
        .into_iter()
        .map(|i| spans[i].clone())
        .collect();

    // 3. Validate/normalize timestamps
    let spans: Vec<Span> = spans
        .into_iter()
        .map(|mut span| {
            if span.end_time_ns < span.start_time_ns {
                warnings.push(
                    ParseWarning::new(
                        "TIMESTAMP_INVERTED",
                        format!(
                            "Span '{}' has end < start (swapped for display)",
                            span.span_id
                        ),
                    )
                    .with_context(serde_json::json!({
                        "span_id": span.span_id,
                        "original_start_ns": span.start_time_ns.to_string(),
                        "original_end_ns": span.end_time_ns.to_string(),
                    })),
                );
                std::mem::swap(&mut span.start_time_ns, &mut span.end_time_ns);
                span.duration_ns = span.end_time_ns - span.start_time_ns;
            }
            span
        })
        .collect();

    // 4. Build span index
    let span_index: HashMap<String, usize> = spans
        .iter()
        .enumerate()
        .map(|(i, s)| (s.span_id.clone(), i))
        .collect();

    // 5. Build parent→children index; detect and sever cycles
    let mut children_map: HashMap<String, Vec<String>> = HashMap::new();
    for span in &spans {
        if let Some(pid) = &span.parent_span_id {
            if span_index.contains_key(pid) {
                children_map
                    .entry(pid.clone())
                    .or_default()
                    .push(span.span_id.clone());
            }
        }
    }

    let (children_map, mut cycle_warnings) =
        detect_and_sever_cycles(children_map, &span_index, &spans);
    warnings.append(&mut cycle_warnings);

    // 6. Identify roots
    let in_children: HashSet<String> = children_map
        .values()
        .flat_map(|v| v.iter().cloned())
        .collect();

    let mut root_span_ids: Vec<String> = spans
        .iter()
        .filter(|s| {
            s.parent_span_id.is_none()
                || !span_index.contains_key(s.parent_span_id.as_ref().unwrap())
        })
        .map(|s| s.span_id.clone())
        .collect();

    // Also include orphaned roots from cycle severing
    for span in &spans {
        if !in_children.contains(&span.span_id) && !root_span_ids.contains(&span.span_id) {
            root_span_ids.push(span.span_id.clone());
        }
    }

    // Sort roots by start_time_ns ascending
    root_span_ids.sort_by_key(|id| {
        span_index
            .get(id)
            .map(|&i| spans[i].start_time_ns)
            .unwrap_or(u64::MAX)
    });
    root_span_ids.dedup();

    // 7. Compute total_duration_ns
    let min_start = spans.iter().map(|s| s.start_time_ns).min().unwrap_or(0);
    let max_end = spans.iter().map(|s| s.end_time_ns).max().unwrap_or(0);
    let total_duration_ns = max_end.saturating_sub(min_start);

    // 8. Compute self_time per span
    let mut spans = spans;
    for i in 0..spans.len() {
        let span_id = spans[i].span_id.clone();
        let span_start = spans[i].start_time_ns;
        let span_end = spans[i].end_time_ns;
        let span_dur = spans[i].duration_ns;

        let child_ids = children_map.get(&span_id).cloned().unwrap_or_default();
        let child_intervals: Vec<(u64, u64)> = child_ids
            .iter()
            .filter_map(|cid| {
                span_index.get(cid).map(|&ci| {
                    let cs = spans[ci].start_time_ns.max(span_start);
                    let ce = spans[ci].end_time_ns.min(span_end);
                    (cs, ce)
                })
            })
            .filter(|(cs, ce)| cs < ce)
            .collect();

        spans[i].self_time_ns = compute_self_time(span_dur, &child_intervals);
    }

    // 9. Collect unique services → resources map
    let mut resources: HashMap<String, Resource> = HashMap::new();
    for span in &spans {
        resources
            .entry(span.service_name.clone())
            .or_insert_with(|| Resource {
                service_name: span.service_name.clone(),
                attributes: std::collections::HashMap::new(),
            });
    }

    let has_errors = spans.iter().any(|s| s.status.is_error());
    let span_count = spans.len();
    let service_count = resources.len();

    // Large trace warning
    if span_count > 10_000 {
        warnings.push(
            ParseWarning::new(
                "LARGE_TRACE",
                format!("Large trace ({} spans). Rendering may be slow.", span_count),
            )
            .with_context(serde_json::json!({ "span_count": span_count })),
        );
    }

    Ok(Trace {
        trace_id: primary_trace_id,
        span_index,
        children_index: children_map,
        spans,
        resources,
        root_span_ids,
        total_duration_ns,
        span_count,
        service_count,
        has_errors,
        detected_format: format,
        warnings,
    })
}

fn compute_self_time(duration_ns: u64, child_intervals: &[(u64, u64)]) -> u64 {
    if child_intervals.is_empty() {
        return duration_ns;
    }

    let mut intervals = child_intervals.to_vec();
    intervals.sort_by_key(|i| i.0);

    let mut merged: Vec<(u64, u64)> = vec![intervals[0]];
    for &(start, end) in &intervals[1..] {
        let last = merged.last_mut().unwrap();
        if start <= last.1 {
            last.1 = last.1.max(end);
        } else {
            merged.push((start, end));
        }
    }

    let child_occupied: u64 = merged.iter().map(|(s, e)| e - s).sum();
    duration_ns.saturating_sub(child_occupied)
}

fn detect_and_sever_cycles(
    mut children_map: HashMap<String, Vec<String>>,
    span_index: &HashMap<String, usize>,
    spans: &[Span],
) -> (HashMap<String, Vec<String>>, Vec<ParseWarning>) {
    let mut warnings: Vec<ParseWarning> = Vec::new();
    let mut visited_global: HashSet<String> = HashSet::new();

    // Build parent map for cycle severing
    let mut parent_map: HashMap<String, String> = HashMap::new();
    for (parent_id, children) in &children_map {
        for child_id in children {
            parent_map.insert(child_id.clone(), parent_id.clone());
        }
    }

    // Phase 1: DFS from all natural roots
    let natural_roots: Vec<String> = spans
        .iter()
        .filter(|s| {
            s.parent_span_id.is_none()
                || !span_index.contains_key(s.parent_span_id.as_ref().unwrap())
        })
        .map(|s| s.span_id.clone())
        .collect();

    for root_id in &natural_roots {
        let mut path_set: HashSet<String> = HashSet::new();
        dfs(
            root_id,
            &mut path_set,
            &mut visited_global,
            &mut children_map,
            &mut parent_map,
            &mut warnings,
        );
    }

    // Phase 2: Handle rootless cycles — iterate in parse order
    let unvisited: Vec<String> = spans
        .iter()
        .map(|s| s.span_id.clone())
        .filter(|id| !visited_global.contains(id))
        .collect();

    for span_id in unvisited {
        if visited_global.contains(&span_id) {
            continue;
        }
        let severed_parent = parent_map.remove(&span_id).unwrap_or_default();
        if let Some(old_parent_children) = children_map.get_mut(&severed_parent) {
            old_parent_children.retain(|c| c != &span_id);
        }
        warnings.push(
            ParseWarning::new(
                "CYCLE_SEVERED",
                format!(
                    "Cycle detected: span '{}' severed from parent '{}' (rootless cycle)",
                    span_id, severed_parent
                ),
            )
            .with_context(serde_json::json!({
                "child_span_id": span_id,
                "severed_parent_span_id": severed_parent,
            })),
        );

        let mut path_set: HashSet<String> = HashSet::new();
        dfs(
            &span_id,
            &mut path_set,
            &mut visited_global,
            &mut children_map,
            &mut parent_map,
            &mut warnings,
        );
    }

    (children_map, warnings)
}

fn dfs(
    span_id: &str,
    path_set: &mut HashSet<String>,
    visited_global: &mut HashSet<String>,
    children_map: &mut HashMap<String, Vec<String>>,
    parent_map: &mut HashMap<String, String>,
    warnings: &mut Vec<ParseWarning>,
) {
    if path_set.contains(span_id) {
        let parent = parent_map.get(span_id).cloned().unwrap_or_default();
        if let Some(parent_children) = children_map.get_mut(&parent) {
            parent_children.retain(|c| c != span_id);
        }
        warnings.push(
            ParseWarning::new(
                "CYCLE_SEVERED",
                format!("Cycle detected: back-edge to span '{}' severed", span_id),
            )
            .with_context(serde_json::json!({
                "child_span_id": span_id,
                "severed_parent_span_id": parent,
            })),
        );
        return;
    }
    if visited_global.contains(span_id) {
        return;
    }

    path_set.insert(span_id.to_string());
    visited_global.insert(span_id.to_string());

    let children = children_map.get(span_id).cloned().unwrap_or_default();

    for child_id in children {
        dfs(
            &child_id,
            path_set,
            visited_global,
            children_map,
            parent_map,
            warnings,
        );
    }

    path_set.remove(span_id);
}
