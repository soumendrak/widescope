use crate::models::layout::{TimelineBlock, TimelineLayout, TimelineRow};
use crate::models::trace::Trace;
use crate::utils::format_duration;
use std::collections::HashMap;

pub fn compute_timeline_layout(trace: &Trace) -> TimelineLayout {
    if trace.spans.is_empty() {
        return TimelineLayout {
            blocks: vec![],
            rows: vec![],
            trace_duration_ns: 0,
            trace_duration_display: "0ns".to_string(),
        };
    }

    let trace_start = trace
        .spans
        .iter()
        .map(|s| s.start_time_ns)
        .min()
        .unwrap_or(0);
    let trace_end = trace.spans.iter().map(|s| s.end_time_ns).max().unwrap_or(0);
    let trace_duration = if trace_end > trace_start {
        trace_end - trace_start
    } else {
        1
    };

    let mut service_groups: HashMap<String, Vec<usize>> = HashMap::new();
    let mut service_order: Vec<String> = Vec::new();

    for (idx, span) in trace.spans.iter().enumerate() {
        let entry = service_groups
            .entry(span.service_name.clone())
            .or_insert_with(|| {
                service_order.push(span.service_name.clone());
                Vec::new()
            });
        entry.push(idx);
    }

    service_order.sort_by_key(|svc| {
        service_groups[svc]
            .iter()
            .map(|&i| trace.spans[i].start_time_ns)
            .min()
            .unwrap_or(u64::MAX)
    });

    let mut rows: Vec<TimelineRow> = Vec::new();
    let mut blocks: Vec<TimelineBlock> = Vec::new();
    let mut global_row_index = 0u32;

    for svc in &service_order {
        let span_indices = &service_groups[svc];

        let mut spans_in_group: Vec<&crate::models::span::Span> =
            span_indices.iter().map(|&i| &trace.spans[i]).collect();
        spans_in_group.sort_by_key(|s| s.start_time_ns);

        let mut lanes: Vec<u64> = Vec::new();

        for span in &spans_in_group {
            let mut placed = false;
            for (lane_idx, lane_end) in lanes.iter_mut().enumerate() {
                if span.start_time_ns >= *lane_end {
                    *lane_end = span.end_time_ns;

                    let row_index = global_row_index + lane_idx as u32;
                    blocks.push(make_block(span, trace_start, trace_duration, row_index));
                    placed = true;
                    break;
                }
            }
            if !placed {
                let lane_idx = lanes.len();
                lanes.push(span.end_time_ns);
                let row_index = global_row_index + lane_idx as u32;
                blocks.push(make_block(span, trace_start, trace_duration, row_index));
            }
        }

        let lane_count = lanes.len().max(1) as u32;
        for lane_idx in 0..lane_count {
            rows.push(TimelineRow {
                service_name: svc.clone(),
                row_index: global_row_index + lane_idx,
                lane_index: lane_idx,
            });
        }
        global_row_index += lane_count;
    }

    TimelineLayout {
        blocks,
        rows,
        trace_duration_ns: trace_duration,
        trace_duration_display: format_duration(trace_duration),
    }
}

fn make_block(
    span: &crate::models::span::Span,
    trace_start: u64,
    trace_duration: u64,
    row_index: u32,
) -> TimelineBlock {
    let x_start = (span.start_time_ns - trace_start) as f64 / trace_duration as f64;
    let x_end = (span.end_time_ns - trace_start) as f64 / trace_duration as f64;
    let x_end = if x_end <= x_start {
        x_start + 1e-9
    } else {
        x_end
    };

    TimelineBlock {
        span_id: span.span_id.clone(),
        label: format!("{}: {}", span.service_name, span.operation_name),
        service_name: span.service_name.clone(),
        x_start,
        x_end,
        row_index,
        is_error: span.status.is_error(),
        is_llm: span.llm.is_some(),
        duration_ns: span.duration_ns,
        duration_display: format_duration(span.duration_ns),
    }
}
