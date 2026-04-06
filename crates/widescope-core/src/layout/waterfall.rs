use crate::models::layout::{WaterfallLayout, WaterfallRow};
use crate::models::trace::Trace;
use crate::utils::format_duration;

pub fn compute_waterfall_layout(trace: &Trace) -> WaterfallLayout {
    if trace.spans.is_empty() {
        return WaterfallLayout {
            rows: vec![],
            trace_duration_ns: 0,
            trace_duration_display: "0ns".to_string(),
            max_depth: 0,
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

    let mut rows: Vec<WaterfallRow> = Vec::with_capacity(trace.spans.len());
    let mut max_depth = 0u32;

    for root in &roots {
        visit_span(root, 0, trace_start, trace_duration, trace, &mut rows, &mut max_depth);
    }

    WaterfallLayout {
        rows,
        trace_duration_ns: trace_duration,
        trace_duration_display: format_duration(trace_duration),
        max_depth,
    }
}

fn visit_span(
    span: &crate::models::span::Span,
    depth: u32,
    trace_start: u64,
    trace_duration: u64,
    trace: &Trace,
    rows: &mut Vec<WaterfallRow>,
    max_depth: &mut u32,
) {
    let x_start = (span.start_time_ns - trace_start) as f64 / trace_duration as f64;
    let x_end = if trace_duration > 0 {
        (span.end_time_ns - trace_start) as f64 / trace_duration as f64
    } else {
        x_start + 1e-9
    };
    let x_end = if x_end <= x_start { x_start + 1e-9 } else { x_end };

    if depth > *max_depth {
        *max_depth = depth;
    }

    let children = trace.get_children(&span.span_id);
    let has_children = !children.is_empty();

    let (input_tokens, output_tokens, total_tokens, estimated_cost_usd) =
        if let Some(llm) = &span.llm {
            (llm.input_tokens, llm.output_tokens, llm.total_tokens, llm.estimated_cost_usd)
        } else {
            (None, None, None, None)
        };

    rows.push(WaterfallRow {
        span_id: span.span_id.clone(),
        operation_name: span.operation_name.clone(),
        service_name: span.service_name.clone(),
        span_kind: span.span_kind.as_str().to_string(),
        depth,
        x_start,
        x_end,
        color_key: span.service_name.clone(),
        is_error: span.status.is_error(),
        is_llm: span.llm.is_some(),
        has_children,
        duration_ns: span.duration_ns,
        duration_display: format_duration(span.duration_ns),
        self_time_ns: span.self_time_ns,
        self_time_display: format_duration(span.self_time_ns),
        input_tokens,
        output_tokens,
        total_tokens,
        estimated_cost_usd,
    });

    let mut child_spans: Vec<&crate::models::span::Span> = children
        .iter()
        .filter_map(|id| trace.get_span(id))
        .collect();
    child_spans.sort_by_key(|s| s.start_time_ns);

    for child in child_spans {
        visit_span(child, depth + 1, trace_start, trace_duration, trace, rows, max_depth);
    }
}
