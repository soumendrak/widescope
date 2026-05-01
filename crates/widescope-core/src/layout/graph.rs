use crate::models::trace::Trace;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
pub struct ServiceGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Serialize)]
pub struct GraphNode {
    pub service: String,
    pub span_count: usize,
    pub error_count: usize,
    pub llm_count: usize,
    pub total_duration_ns: u64,
    pub total_duration_display: String,
}

#[derive(Serialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub call_count: usize,
    pub total_duration_ns: u64,
    pub total_duration_display: String,
}

pub fn compute_service_graph(trace: &Trace) -> ServiceGraph {
    let mut svc_spans: HashMap<&str, usize> = HashMap::new();
    let mut svc_errors: HashMap<&str, usize> = HashMap::new();
    let mut svc_llm: HashMap<&str, usize> = HashMap::new();
    let mut svc_dur: HashMap<&str, u64> = HashMap::new();

    for span in &trace.spans {
        let svc = span.service_name.as_str();
        *svc_spans.entry(svc).or_default() += 1;
        *svc_dur.entry(svc).or_default() += span.duration_ns;
        if span.status.is_error() {
            *svc_errors.entry(svc).or_default() += 1;
        }
        if span.llm.is_some() {
            *svc_llm.entry(svc).or_default() += 1;
        }
    }

    let mut edge_map: HashMap<(&str, &str), (usize, u64)> = HashMap::new();

    for span in &trace.spans {
        let parent_svc = span
            .parent_span_id
            .as_deref()
            .and_then(|pid| trace.get_span(pid))
            .map(|p| p.service_name.as_str());

        if let Some(p_svc) = parent_svc {
            let c_svc = span.service_name.as_str();
            if p_svc != c_svc {
                let entry = edge_map.entry((p_svc, c_svc)).or_default();
                entry.0 += 1;
                entry.1 += span.duration_ns;
            }
        }
    }

    use crate::utils::format_duration;

    let nodes: Vec<GraphNode> = svc_spans
        .iter()
        .map(|(&svc, &count)| GraphNode {
            service: svc.to_string(),
            span_count: count,
            error_count: *svc_errors.get(svc).unwrap_or(&0),
            llm_count: *svc_llm.get(svc).unwrap_or(&0),
            total_duration_ns: *svc_dur.get(svc).unwrap_or(&0),
            total_duration_display: format_duration(*svc_dur.get(svc).unwrap_or(&0)),
        })
        .collect();

    let edges: Vec<GraphEdge> = edge_map
        .iter()
        .map(|((src, tgt), (count, dur))| GraphEdge {
            source: src.to_string(),
            target: tgt.to_string(),
            call_count: *count,
            total_duration_ns: *dur,
            total_duration_display: format_duration(*dur),
        })
        .collect();

    ServiceGraph { nodes, edges }
}
