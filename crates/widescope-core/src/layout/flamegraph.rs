use crate::models::layout::{FlameGraphLayout, FlameNode};
use crate::models::trace::Trace;
use crate::utils::format_duration;

pub fn compute_flamegraph_layout(trace: &Trace) -> FlameGraphLayout {
    if trace.spans.is_empty() {
        return FlameGraphLayout {
            nodes: vec![],
            max_depth: 0,
            trace_duration_ns: 0,
            trace_duration_display: "0ns".to_string(),
        };
    }

    let trace_start = trace.spans.iter().map(|s| s.start_time_ns).min().unwrap_or(0);
    let trace_end = trace.spans.iter().map(|s| s.end_time_ns).max().unwrap_or(0);
    let trace_duration = if trace_end > trace_start { trace_end - trace_start } else { 1 };

    let mut roots: Vec<&crate::models::span::Span> = trace
        .root_span_ids
        .iter()
        .filter_map(|id| trace.get_span(id))
        .collect();
    roots.sort_by_key(|s| s.start_time_ns);

    let mut nodes: Vec<FlameNode> = Vec::with_capacity(trace.spans.len());
    let mut max_depth = 0u32;

    for root in &roots {
        visit_span(root, 0, trace_start, trace_duration, trace, &mut nodes, &mut max_depth);
    }

    FlameGraphLayout {
        nodes,
        max_depth,
        trace_duration_ns: trace_duration,
        trace_duration_display: format_duration(trace_duration),
    }
}

fn visit_span(
    span: &crate::models::span::Span,
    depth: u32,
    trace_start: u64,
    trace_duration: u64,
    trace: &Trace,
    nodes: &mut Vec<FlameNode>,
    max_depth: &mut u32,
) {
    let x = (span.start_time_ns - trace_start) as f64 / trace_duration as f64;
    let width = if trace_duration > 0 {
        span.duration_ns as f64 / trace_duration as f64
    } else {
        0.0
    };

    let width = if width == 0.0 { 1e-9 } else { width };

    if depth > *max_depth {
        *max_depth = depth;
    }

    nodes.push(FlameNode {
        span_id: span.span_id.clone(),
        label: format!("{}: {}", span.service_name, span.operation_name),
        x,
        width,
        depth,
        color_key: span.service_name.clone(),
        is_error: span.status.is_error(),
        is_llm: span.llm.is_some(),
        duration_ns: span.duration_ns,
        self_time_ns: span.self_time_ns,
        duration_display: format_duration(span.duration_ns),
        self_time_display: format_duration(span.self_time_ns),
    });

    let mut children: Vec<&crate::models::span::Span> = trace
        .get_children(&span.span_id)
        .iter()
        .filter_map(|id| trace.get_span(id))
        .collect();
    children.sort_by_key(|s| s.start_time_ns);

    for child in children {
        visit_span(child, depth + 1, trace_start, trace_duration, trace, nodes, max_depth);
    }
}
