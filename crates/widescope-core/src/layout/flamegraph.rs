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

    let mut roots: Vec<&crate::models::span::Span> = trace
        .root_span_ids
        .iter()
        .filter_map(|id| trace.get_span(id))
        .collect();
    roots.sort_by_key(|s| s.start_time_ns);

    let mut nodes: Vec<FlameNode> = Vec::with_capacity(trace.spans.len());
    let mut max_depth = 0u32;

    for root in &roots {
        visit_span(
            root,
            0,
            trace_start,
            trace_duration,
            trace,
            &mut nodes,
            &mut max_depth,
        );
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
        safety_category: span.top_safety_category(),
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
        visit_span(
            child,
            depth + 1,
            trace_start,
            trace_duration,
            trace,
            nodes,
            max_depth,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::span::{Span, SpanKind, SpanStatus};
    use crate::models::trace::InputFormat;
    use std::collections::HashMap;

    fn span(id: &str, parent: Option<&str>, start: u64, dur: u64) -> Span {
        Span {
            trace_id: "t".into(),
            span_id: id.into(),
            parent_span_id: parent.map(String::from),
            operation_name: id.into(),
            service_name: "svc".into(),
            span_kind: SpanKind::Internal,
            start_time_ns: start,
            end_time_ns: start + dur,
            duration_ns: dur,
            self_time_ns: dur,
            status: SpanStatus::Ok,
            attributes: HashMap::new(),
            events: vec![],
            llm: None,
            safety: vec![],
        }
    }

    fn build(spans: Vec<Span>) -> Trace {
        crate::trace_builder::build_trace(spans, InputFormat::Unknown, vec![]).unwrap()
    }

    #[test]
    fn root_spans_full_width_children_nested() {
        // root [0..100], two children [0..40] and [40..100], grandchild under c1.
        let trace = build(vec![
            span("root", None, 0, 100),
            span("c1", Some("root"), 0, 40),
            span("c2", Some("root"), 40, 60),
            span("g1", Some("c1"), 0, 40),
        ]);

        let layout = compute_flamegraph_layout(&trace);
        assert_eq!(layout.nodes.len(), 4);
        assert_eq!(layout.max_depth, 2);

        let node = |id: &str| layout.nodes.iter().find(|n| n.span_id == id).unwrap();

        // Root fills the whole width.
        assert!((node("root").x - 0.0).abs() < 1e-9);
        assert!((node("root").width - 1.0).abs() < 1e-9);
        assert_eq!(node("root").depth, 0);
        assert_eq!(node("g1").depth, 2);

        // Siblings tile without overlap: c1 ends where c2 starts.
        let c1 = node("c1");
        let c2 = node("c2");
        assert!(c1.x + c1.width <= c2.x + 1e-9, "siblings must not overlap");
        // A child never exceeds its parent's span on the x axis.
        assert!(c2.x + c2.width <= node("root").x + node("root").width + 1e-9);
    }

    #[test]
    fn empty_trace_yields_empty_layout() {
        let layout = compute_flamegraph_layout(&Trace {
            trace_id: "t".into(),
            spans: vec![],
            span_index: HashMap::new(),
            children_index: HashMap::new(),
            resources: HashMap::new(),
            root_span_ids: vec![],
            total_duration_ns: 0,
            span_count: 0,
            service_count: 0,
            has_errors: false,
            detected_format: InputFormat::Unknown,
            warnings: vec![],
        });
        assert!(layout.nodes.is_empty());
        assert_eq!(layout.max_depth, 0);
    }
}
