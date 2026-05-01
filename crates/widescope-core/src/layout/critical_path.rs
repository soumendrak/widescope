use crate::models::trace::Trace;
use serde::Serialize;

#[derive(Serialize)]
pub struct CriticalPath {
    pub span_ids: Vec<String>,
    pub total_self_time_ns: u64,
    pub total_self_time_display: String,
    pub bottleneck_span_id: Option<String>,
}

pub fn compute_critical_path(trace: &Trace) -> CriticalPath {
    use crate::utils::format_duration;

    // Build a map from span to its heaviest child by self-time
    let mut next_on_path: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    let mut max_self_time: std::collections::HashMap<&str, u64> = std::collections::HashMap::new();

    for span in &trace.spans {
        max_self_time.insert(span.span_id.as_str(), span.self_time_ns);
    }

    // For each span, find which child has the highest self_time
    for span in &trace.spans {
        let children = trace.get_children(&span.span_id);
        let mut best_child: Option<&str> = None;
        let mut best_self = 0u64;

        for child_id in children {
            if let Some(child) = trace.get_span(child_id) {
                if child.self_time_ns > best_self {
                    best_self = child.self_time_ns;
                    best_child = Some(child.span_id.as_str());
                }
            }
        }

        if let Some(child_id) = best_child {
            next_on_path.insert(span.span_id.as_str(), child_id);
        }
    }

    // Find the root with the highest cumulative self_time along its path
    let mut best_total: u64 = 0;
    let mut best_path: Vec<String> = Vec::new();
    let mut best_bottleneck: Option<String> = None;

    for root_id in &trace.root_span_ids {
        let mut path: Vec<String> = Vec::new();
        let mut total = 0u64;
        let mut bottleneck_self = 0u64;
        let mut bottleneck_id: Option<String> = None;
        let mut current: &str = root_id.as_str();

        loop {
            let self_t = max_self_time.get(current).copied().unwrap_or(0);
            total += self_t;
            if self_t > bottleneck_self {
                bottleneck_self = self_t;
                bottleneck_id = Some(current.to_string());
            }
            path.push(current.to_string());

            match next_on_path.get(current) {
                Some(next) => current = next,
                None => break,
            }
        }

        if total > best_total {
            best_total = total;
            best_path = path;
            best_bottleneck = bottleneck_id;
        }
    }

    CriticalPath {
        span_ids: best_path,
        total_self_time_ns: best_total,
        total_self_time_display: format_duration(best_total),
        bottleneck_span_id: best_bottleneck,
    }
}
