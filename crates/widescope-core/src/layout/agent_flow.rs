use crate::models::llm::LlmOperationType;
use crate::models::trace::Trace;
use crate::utils::format_duration;
use serde::Serialize;
use std::collections::HashMap;

/// A node in the agent flow DAG — one agentic span (agent step, tool call,
/// chain step, retrieval, or LLM call).
#[derive(Serialize)]
pub struct AgentFlowNode {
    pub span_id: String,
    pub label: String,
    /// "agent" | "tool" | "chain" | "retrieval" | "llm"
    pub kind: String,
    pub tool_name: Option<String>,
    pub status: String,
    pub duration_ns: u64,
    pub duration_display: String,
    /// Depth among flow-node ancestors (0 = top-level). Drives the vertical layer.
    pub layer: usize,
    /// Global sequence rank by start time. Drives ordering within a layer.
    pub order: usize,
    /// Iteration index for repeated tool/chain calls under the same parent (loops).
    pub iteration: Option<usize>,
    /// Tool-call arguments (tool nodes only) — shown inline in the timeline view.
    pub arguments: Option<String>,
    /// Tool-call result (tool nodes only) — shown inline in the timeline view.
    pub result: Option<String>,
    /// First output-message content for llm/agent nodes — the agent's "thought".
    pub detail: Option<String>,
}

/// A directed edge: an agent spawning a child step ("spawn"), or sequential
/// control flow between sibling steps ("sequence").
#[derive(Serialize)]
pub struct AgentFlowEdge {
    pub source: String,
    pub target: String,
    /// "spawn" | "sequence"
    pub kind: String,
}

#[derive(Serialize)]
pub struct AgentFlow {
    pub nodes: Vec<AgentFlowNode>,
    pub edges: Vec<AgentFlowEdge>,
}

fn flow_kind(op: &LlmOperationType) -> Option<&'static str> {
    match op {
        LlmOperationType::AgentStep => Some("agent"),
        LlmOperationType::ToolCall => Some("tool"),
        LlmOperationType::ChainStep => Some("chain"),
        LlmOperationType::Retrieval => Some("retrieval"),
        LlmOperationType::ChatCompletion | LlmOperationType::TextCompletion => Some("llm"),
        _ => None,
    }
}

pub fn compute_agent_flow(trace: &Trace) -> AgentFlow {
    // Collect the agentic spans, sorted by start time so order/sequence is stable.
    let mut flow_spans: Vec<(&str, &str)> = trace
        .spans
        .iter()
        .filter_map(|s| {
            let kind = flow_kind(&s.llm.as_ref()?.operation_type)?;
            Some((s.span_id.as_str(), kind))
        })
        .collect();
    flow_spans.sort_by_key(|(id, _)| trace.get_span(id).map(|s| s.start_time_ns).unwrap_or(0));

    let is_flow: HashMap<&str, &str> = flow_spans.iter().copied().collect();
    let order_of: HashMap<&str, usize> = flow_spans
        .iter()
        .enumerate()
        .map(|(i, (id, _))| (*id, i))
        .collect();

    // Nearest flow-node ancestor, walking up the parent chain.
    let flow_parent = |span_id: &str| -> Option<String> {
        let mut cur = trace.get_span(span_id)?.parent_span_id.clone();
        while let Some(pid) = cur {
            if is_flow.contains_key(pid.as_str()) {
                return Some(pid);
            }
            cur = trace.get_span(&pid)?.parent_span_id.clone();
        }
        None
    };

    // Layer = number of flow-node ancestors. ponytail: O(depth) walk per node,
    // re-walked per call — traces are small, no cache needed.
    let layer_of = |span_id: &str| -> usize {
        let mut n = 0;
        let mut parent = flow_parent(span_id);
        while let Some(pid) = parent {
            n += 1;
            parent = flow_parent(&pid);
        }
        n
    };

    // Iteration index: per (parent, tool_name), repeated calls are loop iterations.
    let mut iter_counter: HashMap<(String, String), usize> = HashMap::new();

    let mut nodes = Vec::with_capacity(flow_spans.len());
    let mut edges = Vec::new();

    for (span_id, kind) in &flow_spans {
        let span = match trace.get_span(span_id) {
            Some(s) => s,
            None => continue,
        };
        let llm = span.llm.as_ref();
        let first_tool = llm.and_then(|l| l.tool_calls.first());
        let tool_name = first_tool
            .map(|t| t.name.clone())
            .filter(|_| *kind == "tool");
        let arguments = first_tool.and_then(|t| t.arguments.clone());
        let result = first_tool.and_then(|t| t.result.clone());
        // The agent's "thought" — the model's first output message, if any.
        let detail = llm
            .and_then(|l| l.output_messages.first())
            .and_then(|m| m.content.clone());

        let parent = flow_parent(span_id);

        // Iteration grouping keyed by parent + a stable name.
        let group_name = tool_name
            .clone()
            .unwrap_or_else(|| span.operation_name.clone());
        let parent_key = parent.clone().unwrap_or_default();
        let counter = iter_counter.entry((parent_key, group_name)).or_insert(0);
        let iteration = Some(*counter);
        *counter += 1;

        nodes.push(AgentFlowNode {
            span_id: span_id.to_string(),
            label: tool_name
                .clone()
                .unwrap_or_else(|| span.operation_name.clone()),
            kind: kind.to_string(),
            tool_name,
            status: span.status.as_str().to_string(),
            duration_ns: span.duration_ns,
            duration_display: format_duration(span.duration_ns),
            layer: layer_of(span_id),
            order: *order_of.get(span_id).unwrap_or(&0),
            iteration,
            arguments,
            result,
            detail,
        });

        if let Some(p) = parent {
            edges.push(AgentFlowEdge {
                source: p,
                target: span_id.to_string(),
                kind: "spawn".to_string(),
            });
        }
    }

    // Sequence edges: connect consecutive siblings (same flow-parent) in start order.
    let mut last_sibling: HashMap<String, String> = HashMap::new();
    for (span_id, _) in &flow_spans {
        let parent_key = flow_parent(span_id).unwrap_or_default();
        if let Some(prev) = last_sibling.get(&parent_key) {
            edges.push(AgentFlowEdge {
                source: prev.clone(),
                target: span_id.to_string(),
                kind: "sequence".to_string(),
            });
        }
        last_sibling.insert(parent_key, span_id.to_string());
    }

    AgentFlow { nodes, edges }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::llm::{LlmOperationType, LlmSpanAttributes, ToolCall};
    use crate::models::span::{Span, SpanKind, SpanStatus};
    use std::collections::HashMap;

    fn llm(op: LlmOperationType, tool: Option<&str>) -> LlmSpanAttributes {
        LlmSpanAttributes {
            operation_type: op,
            model_name: None,
            model_provider: None,
            input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            estimated_cost_usd: None,
            input_messages: vec![],
            output_messages: vec![],
            tool_calls: tool
                .map(|n| {
                    vec![ToolCall {
                        name: n.to_string(),
                        arguments: Some(format!("{{\"q\":\"{n}\"}}")),
                        result: Some("ok".to_string()),
                    }]
                })
                .unwrap_or_default(),
            temperature: None,
            top_p: None,
            max_tokens: None,
            embedding_dimensions: None,
            embedding_count: None,
            retrieved_documents: vec![],
            eval_scores: vec![],
        }
    }

    fn span(
        id: &str,
        parent: Option<&str>,
        start: u64,
        llm_attr: Option<LlmSpanAttributes>,
    ) -> Span {
        Span {
            trace_id: "t".into(),
            span_id: id.into(),
            parent_span_id: parent.map(String::from),
            operation_name: id.into(),
            service_name: "svc".into(),
            span_kind: SpanKind::Internal,
            start_time_ns: start,
            end_time_ns: start + 10,
            duration_ns: 10,
            self_time_ns: 10,
            status: SpanStatus::Ok,
            attributes: HashMap::new(),
            events: vec![],
            llm: llm_attr,
            safety: vec![],
        }
    }

    fn build(spans: Vec<Span>) -> Trace {
        crate::trace_builder::build_trace(spans, crate::models::trace::InputFormat::Unknown, vec![])
            .unwrap()
    }

    #[test]
    fn detects_agent_tool_loop() {
        // agent -> tool(search) x2 (loop) + a non-agentic span (ignored).
        let spans = vec![
            span("a", None, 0, Some(llm(LlmOperationType::AgentStep, None))),
            span(
                "t1",
                Some("a"),
                1,
                Some(llm(LlmOperationType::ToolCall, Some("search"))),
            ),
            span(
                "t2",
                Some("a"),
                2,
                Some(llm(LlmOperationType::ToolCall, Some("search"))),
            ),
            span("plain", Some("a"), 3, None),
        ];
        let flow = compute_agent_flow(&build(spans));

        assert_eq!(flow.nodes.len(), 3, "only agentic spans become nodes");
        let agent = flow.nodes.iter().find(|n| n.span_id == "a").unwrap();
        assert_eq!(agent.kind, "agent");
        assert_eq!(agent.layer, 0);

        let t2 = flow.nodes.iter().find(|n| n.span_id == "t2").unwrap();
        assert_eq!(t2.layer, 1, "tool is one layer below its agent");
        assert_eq!(t2.iteration, Some(1), "second search call is iteration 1");
        assert_eq!(
            t2.arguments.as_deref(),
            Some("{\"q\":\"search\"}"),
            "tool args inline"
        );
        assert_eq!(t2.result.as_deref(), Some("ok"), "tool result inline");

        // spawn edges a->t1, a->t2 ; sequence edge t1->t2.
        assert!(flow
            .edges
            .iter()
            .any(|e| e.source == "a" && e.target == "t1" && e.kind == "spawn"));
        assert!(flow
            .edges
            .iter()
            .any(|e| e.source == "t1" && e.target == "t2" && e.kind == "sequence"));
    }
}
