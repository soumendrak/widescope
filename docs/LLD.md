# WideScope — Low Level Design

> **Version:** 0.1 (MVP) | **Date:** 2026-04-06 | **Status:** Draft | **Parent:** HLD v1

---

## Table of Contents

> **Reading guide:** This document uses two levels of specification:
> - **Normative** — schemas, algorithms, invariant rules, error codes, and type contracts. These are binding on implementation.
> - **UX suggestion** *(italicized or explicitly labeled)* — banner wording, keyboard shortcut phrasing, warning text, and UI layout details. These illustrate intent but may be freely adapted by implementers.

**Core LLD (implementation-critical)**

1. [Repository Structure](#1-repository-structure)
2. [Build Pipeline](#2-build-pipeline)
3. [Data Models (Rust)](#3-data-models-rust)
4. [Parsing Pipeline](#4-parsing-pipeline)
5. [Conventions Resolver](#5-conventions-resolver)
6. [WASM–JS Boundary Contract](#6-wasmjs-boundary-contract)
7. [Svelte Shell Architecture](#7-svelte-shell-architecture)
8. [Rendering Layer](#8-rendering-layer)
9. [Input Handling](#9-input-handling)
10. [Security and CSP](#10-security-and-csp)
11. [Error Handling Strategy](#11-error-handling-strategy)

**Non-Functional Requirements & Testing**

12. [Testing Strategy](#12-testing-strategy)
13. [Performance Budgets and Degradation](#13-performance-budgets-and-degradation)
14. [Accessibility](#14-accessibility)

**Roadmap (not part of MVP LLD — included for planning context)**

15. [Phased Delivery](#15-phased-delivery)

---

## 1. Repository Structure

```
widescope/
├── crates/
│   └── widescope-core/           # Rust WASM library crate
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs            # WASM entry, #[wasm_bindgen] exports
│           ├── models/
│           │   ├── mod.rs
│           │   ├── span.rs       # Canonical Span, SpanKind, Status
│           │   ├── trace.rs      # Trace (collection of Spans)
│           │   ├── resource.rs   # Resource attributes
│           │   ├── llm.rs        # LlmSpanAttributes canonical model
│           │   └── layout.rs     # FlameNode, TimelineRow, computed layouts
│           ├── parsers/
│           │   ├── mod.rs        # Format detection + dispatch
│           │   ├── otlp_json.rs  # OTLP JSON parser
│           │   ├── jaeger.rs     # Jaeger JSON parser (v1)
│           │   └── openinference.rs  # OpenInference parser (v1)
│           ├── conventions/
│           │   ├── mod.rs
│           │   ├── registry.rs   # Load & merge conventions JSON
│           │   └── resolver.rs   # Raw attributes → LlmSpanAttributes
│           ├── analytics/              # v1 — not part of MVP
│           │   ├── mod.rs
│           │   ├── critical_path.rs
│           │   ├── cost.rs
│           │   └── stats.rs
│           ├── layout/
│           │   ├── mod.rs
│           │   ├── flamegraph.rs
│           │   └── timeline.rs
│           └── errors.rs
├── ui/                           # Svelte SPA shell
│   ├── package.json
│   ├── svelte.config.js
│   ├── vite.config.ts
│   ├── src/
│   │   ├── main.ts
│   │   ├── App.svelte
│   │   ├── lib/
│   │   │   ├── wasm.ts           # WASM loader & typed wrapper
│   │   │   ├── types.ts          # TS mirrors of Rust layout structs
│   │   │   ├── input.ts          # Drag-drop, paste, file-picker
│   │   │   ├── theme.ts          # Light/dark theme store
│   │   │   └── sample.ts         # Embedded sample trace
│   │   ├── components/
│   │   │   ├── Toolbar.svelte
│   │   │   ├── FlameGraph.svelte     # Canvas-based
│   │   │   ├── Timeline.svelte       # SVG swimlane
│   │   │   ├── SpanDetail.svelte     # Right sidebar
│   │   │   ├── LlmPanel.svelte      # v1 — LLM attribute view
│   │   │   ├── DagView.svelte        # v1 — Service DAG
│   │   │   ├── DropZone.svelte
│   │   │   ├── JsonPasteModal.svelte
│   │   │   └── ErrorBanner.svelte
│   │   └── stores/
│   │       ├── trace.ts
│   │       └── selection.ts
│   ├── static/
│   │   └── index.html
│   └── tests/
├── conventions/                  # Community-maintained mappings
│   ├── opentelemetry.json
│   ├── openinference.json
│   ├── langchain.json
│   └── README.md
├── test-fixtures/
│   ├── otlp/
│   ├── jaeger/
│   └── openinference/
├── docs/
│   ├── HLD.md
│   └── LLD.md
├── .github/workflows/
│   ├── ci.yml
│   └── release.yml
├── Cargo.toml                    # Workspace root
├── rust-toolchain.toml
└── README.md
```

**Key constraint:** The production artifact is a static folder — `index.html`, `app.js`, `widescope_core_bg.wasm`. No `node_modules` in dist. No runtime fetch — conventions and sample trace are bundled into JS at build time (see Sections 5.4 and 7.4). Single-file HTML export (v1) inlines WASM as base64 + all JS/CSS.

---

## 2. Build Pipeline

### 2.1 Rust → WASM

| Tool | Purpose |
|---|---|
| `wasm-pack build --target web` | Compile to `.wasm` + JS glue |
| `wasm-opt -Oz` | Post-build size optimization |
| `wasm-bindgen` | Generate TS type declarations |

**Target:** `wasm32-unknown-unknown`

```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
```

**WASM binary budget:** ≤ 500 KB gzipped at MVP.

### 2.2 Svelte → JS

Vite as bundler with `vite-plugin-wasm` and `vite-plugin-top-level-await`. **JS bundle budget:** ≤ 100 KB gzipped (includes build-time-bundled conventions and sample trace).

### 2.3 CI (GitHub Actions)

```
cargo check/clippy → cargo test → wasm-pack build → wasm-opt → vite build → Playwright E2E → Deploy
```

---

## 3. Data Models (Rust)

### 3.1 Core Span

```rust
// crates/widescope-core/src/models/span.rs
pub type Timestamp = u64; // Nanosecond Unix
pub type Duration = u64;  // Nanoseconds

pub struct Span {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub operation_name: String,
    pub service_name: String,
    pub span_kind: SpanKind,
    pub start_time_ns: Timestamp,
    pub end_time_ns: Timestamp,
    pub duration_ns: Duration,
    pub self_time_ns: Duration,          // Computed during build_trace (Section 4.8)
    pub status: SpanStatus,
    pub attributes: HashMap<String, AttributeValue>,
    pub events: Vec<SpanEvent>,
    pub llm: Option<LlmSpanAttributes>,
}

pub enum SpanKind { Internal, Server, Client, Producer, Consumer }
pub enum SpanStatus { Unset, Ok, Error { message: String } }

pub enum AttributeValue {
    String(String), Int(i64), Float(f64), Bool(bool),
    StringArray(Vec<String>), IntArray(Vec<i64>),
    FloatArray(Vec<f64>), BoolArray(Vec<bool>),
}

pub struct SpanEvent {
    pub name: String,
    pub timestamp_ns: Timestamp,
    pub attributes: HashMap<String, AttributeValue>,
}
```

### 3.2 LLM Canonical Model

```rust
// crates/widescope-core/src/models/llm.rs
pub struct LlmSpanAttributes {
    pub operation_type: LlmOperationType,
    pub model_name: Option<String>,
    pub model_provider: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub estimated_cost_usd: Option<f64>,
    pub input_messages: Vec<LlmMessage>,
    pub output_messages: Vec<LlmMessage>,
    pub tool_calls: Vec<ToolCall>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub max_tokens: Option<u64>,
    pub embedding_dimensions: Option<u64>,
    pub embedding_count: Option<u64>,
    pub retrieved_documents: Vec<RetrievedDocument>,
}

pub enum LlmOperationType {
    ChatCompletion, TextCompletion, Embedding, Rerank,
    ToolCall, AgentStep, ChainStep, Retrieval, Unknown(String),
}

pub struct LlmMessage { pub role: String, pub content: Option<String> }
pub struct ToolCall { pub name: String, pub arguments: Option<String>, pub result: Option<String> }
pub struct RetrievedDocument { pub id: Option<String>, pub score: Option<f64>, pub content_snippet: Option<String> }
```

### 3.3 Trace Model

```rust
// crates/widescope-core/src/models/trace.rs
pub struct Trace {
    pub trace_id: String,
    pub spans: Vec<Span>,
    pub resources: HashMap<String, Resource>,
    pub root_span_ids: Vec<String>,   // Multiple roots possible (Section 4.6); first = primary root
    pub total_duration_ns: u64,
    pub span_count: usize,
    pub service_count: usize,
    pub has_errors: bool,
    pub detected_format: InputFormat,
}

pub enum InputFormat { OtlpJson, JaegerJson, OpenInferenceJson, Unknown }
```

### 3.4 Layout Models (WASM → JS)

These contain coordinates, raw numeric values, and display metadata. **Section 6.2 is the authoritative contract** — the TypeScript interfaces there define the exact JSON shape. These Rust structs must serialize to match that contract exactly.

```rust
// crates/widescope-core/src/models/layout.rs

pub struct FlameNode {
    pub span_id: String,
    pub label: String,              // "service: operation"
    pub x: f64,                     // Normalized [0,1]
    pub width: f64,                 // Normalized [0,1]
    pub depth: u32,
    pub color_key: String,          // Service name for color
    pub is_error: bool,
    pub is_llm: bool,
    pub duration_ns: u64,           // Raw (normative)
    pub self_time_ns: u64,          // Raw (normative)
    pub duration_display: String,   // Convenience: "12.3ms"
    pub self_time_display: String,  // Convenience
}

pub struct FlameGraphLayout {
    pub nodes: Vec<FlameNode>,
    pub max_depth: u32,
    pub trace_duration_ns: u64,           // Raw (normative)
    pub trace_duration_display: String,   // Convenience
}

pub struct TimelineBlock {
    pub span_id: String,
    pub label: String,
    pub service_name: String,
    pub x_start: f64,
    pub x_end: f64,
    pub row_index: u32,
    pub is_error: bool,
    pub is_llm: bool,
    pub duration_ns: u64,           // Raw (normative)
    pub duration_display: String,   // Convenience
}

pub struct TimelineLayout {
    pub blocks: Vec<TimelineBlock>,
    pub rows: Vec<TimelineRow>,
    pub trace_duration_ns: u64,           // Raw (normative)
    pub trace_duration_display: String,   // Convenience
}

pub struct TimelineRow {
    pub service_name: String,
    pub row_index: u32,
    pub lane_index: u32,          // 0-indexed within service group (see Section 8.2)
}

pub struct SpanDetail {
    pub span_id: String,
    pub trace_id: String,
    pub parent_span_id: Option<String>,
    pub operation_name: String,
    pub service_name: String,
    pub span_kind: String,
    #[serde(serialize_with = "serialize_u64_as_string")]
    pub start_time_ns: u64,         // Serialized as JSON string for JS precision safety
    pub start_time_display: String,
    pub duration_ns: u64,
    pub duration_display: String,
    pub self_time_ns: u64,
    pub self_time_display: String,
    pub status: String,
    pub error_message: Option<String>,
    pub attributes: Vec<(String, String)>,
    pub events: Vec<EventDetail>,
    pub llm: Option<LlmDetail>,
    pub children_ids: Vec<String>,
}
```

All structs derive `Serialize, Deserialize` via serde. `LlmDetail`, `EventDetail`, `MessageDetail`, `ToolCallDetail` are flattened display-ready versions of their model counterparts.

> **Contract enforcement:** Absolute nanosecond timestamps (`start_time_ns`, `timestamp_ns` in events) are `u64` in Rust but must serialize as JSON strings using a custom serde serializer (`serialize_u64_as_string`). This prevents JS `number` precision loss (see Section 6.2 note). Duration fields remain as JSON numbers.

---

## 4. Parsing Pipeline

### 4.1 Architecture

```
raw bytes → detect_format() → dispatch to parser → Vec<Span> → conventions::resolve() → Trace
```

### 4.2 Format Detection

Heuristic on top-level JSON keys, evaluated in order:

| Condition | Format |
|---|---|
| Key `resourceSpans` exists | `OtlpJson` |
| Key `data` + `data[0].traceID` | `JaegerJson` |
| Key `spans` + first span has `context.trace_id` | `OpenInferenceJson` |
| None match | `WideError::UnrecognizedFormat` |

First validates JSON via `serde_json::from_str::<Value>`. Invalid JSON → `WideError::InvalidJson`.

### 4.3 OTLP JSON Parser (MVP)

**OTLP field → Canonical Span mapping:**

| OTLP JSON field | `Span` field |
|---|---|
| `traceId` (hex) | `trace_id` |
| `spanId` (hex) | `span_id` |
| `parentSpanId` | `parent_span_id` (None if empty) |
| `name` | `operation_name` |
| `kind` (int 0-4) | `span_kind` |
| `startTimeUnixNano` (string) | `start_time_ns` (u64) |
| `endTimeUnixNano` (string) | `end_time_ns` (u64) |
| computed | `duration_ns = end - start` |
| `status.code` | `status` enum |
| `attributes[]` | `attributes` HashMap |
| `events[]` | `events` Vec |
| Resource `service.name` | `service_name` |

**OTLP AnyValue → AttributeValue:**

| OTLP `AnyValue` variant | `AttributeValue` | Notes |
|---|---|---|
| `stringValue` | `String` | |
| `intValue` (JSON string) | `Int` (i64) | OTLP encodes as string to avoid JSON precision loss |
| `doubleValue` | `Float` (f64) | |
| `boolValue` | `Bool` | |
| `arrayValue.values[]` | Appropriate `*Array` variant | Homogeneous arrays only; mixed → `StringArray` of JSON reprs |
| `bytesValue` | **Unsupported at MVP** | Stored as `String` with base64 representation |
| `kvlistValue` | **Unsupported at MVP** | Stored as `String` with JSON representation |
| Nested `arrayValue` | **Unsupported at MVP** | Flattened to `StringArray` of JSON reprs |

**Error handling:** Missing required fields → skip span + warning. At least one valid span required.

### 4.4 Jaeger JSON Parser (v1)

Key differences: timestamps in **microseconds** (×1000), `tags` array instead of `attributes`, `processID` links to `processes` map for service name, `references` array for parent links.

### 4.5 OpenInference JSON Parser (v1)

Spans carry `context.trace_id`, `context.span_id`, `parent_id`. Attributes use OpenInference conventions (`llm.model_name`, `llm.token_count.prompt`, etc.). Convention resolution fills `LlmSpanAttributes`.

### 4.6 Trace Invariants and Malformed Input

The parser must handle real-world traces that violate ideal assumptions. Each case has a **deterministic** behavior with explicit rationale.

#### Duplicate `span_id`

**Rule:** Keep the first occurrence encountered during parsing. Discard subsequent duplicates. Log warning with code `DUPLICATE_SPAN_ID`.

**Rationale:** "First wins" is chosen over "most complete wins" because:
- Completeness comparison requires defining a scoring function over arbitrary attribute sets, which is itself ambiguous.
- Input order within a single JSON file is deterministic for a given exporter — the same file always produces the same result.
- If input order varies across exporters, the user should deduplicate before export. This is not a problem the viewer should silently resolve with heuristics.

#### Timestamp inversion (`end_time_ns < start_time_ns`)

**Rule:** Normalize for layout by swapping start/end. Set `duration_ns = end_time_ns - start_time_ns` (after swap). **Preserve the original raw values** in the warning context so the user can debug the source exporter.

```
warning: {
  code: "TIMESTAMP_INVERTED",
  message: "Span 'abc123' has end < start (swapped for display)",
  count: 1,
  context: { span_id: "abc123", original_start_ns: 1700000002000, original_end_ns: 1700000001000 }
}
```

**Rationale:** Rejecting the span entirely would lose data. Swapping is the least destructive normalization. Preserving originals in the warning lets the user identify exporter bugs without the viewer hiding them.

#### Zero-duration spans (`start_time_ns == end_time_ns`)

**Rule:** Accept as valid. `duration_ns = 0`. Layout assigns a minimum visual width (1px equivalent in normalized coordinates). No warning.

#### Cyclic parent references

**Rule:** Detect during tree construction. Sever the **specific back-edge** that closes the cycle.

**Algorithm detail:**

```
function build_tree(spans):
    // Phase 1: Build adjacency from parent_span_id
    children_map: HashMap<span_id, Vec<span_id>>
    for span in spans:
        if span.parent_span_id is Some(pid) and pid in span_index:
            children_map[pid].push(span.span_id)

    // Phase 2: DFS from all roots, detect back-edges
    visited_global = {}       // all visited spans across all DFS trees
    for root in roots:
        path_set = {}         // spans on the CURRENT DFS path (ancestor chain)
        dfs(root, path_set, visited_global)

    // Phase 3: Handle rootless cycles
    // If any spans remain unvisited, they belong to fully cyclic components
    // (every node has a parent within the cycle, so no natural root exists).
    // For each unvisited span: sever its parent link, making it a root,
    // then DFS from it to detect and sever any remaining back-edges.
    //
    // **Iteration order (normative):** `all_span_ids` is iterated in **parse order**
    // (the order spans were encountered in the input file). This ensures:
    //   - deterministic severing across runs on the same input
    //   - minimal surprise (the "first" span in the file becomes the root)
    //   - no dependency on span_id lexical ordering or timestamp values
    for span_id in all_span_ids:  // parse order
        if span_id not in visited_global:
            // Sever this span's parent link to break into the cycle
            sever_parent(span_id)   // sets parent_span_id = None, logs CYCLE_SEVERED
            roots.push(span_id)
            path_set = {}
            dfs(span_id, path_set, visited_global)

    function dfs(span_id, path_set, visited_global):
        if span_id in path_set:
            // CYCLE: span_id is already an ancestor on this path.
            // Sever: remove span_id from its parent's children list.
            // Set span_id.parent_span_id = None (becomes orphan root).
            // Log warning with code CYCLE_SEVERED.
            return
        if span_id in visited_global:
            return  // already processed in another tree
        path_set.add(span_id)
        visited_global.add(span_id)
        for child_id in children_map[span_id]:
            dfs(child_id, path_set, visited_global)
        path_set.remove(span_id)
```

**Why Phase 3 is needed:** Consider spans A→B→A (mutual parents). Neither is a root. Without Phase 3, root-based DFS never visits them, and the cycle survives into layout — causing infinite recursion or missing spans.

**Post-severing state:**
- The severed span's `parent_span_id` is set to `None`. It becomes an orphan root.
- The original `parent_span_id` is preserved in the warning context for debugging.
- `children_ids` in `SpanDetail` reflects the post-severing tree (the severed span is removed from its former parent's children list).

#### Multiple `trace_id` values

**Rule:** Group by `trace_id`. Use the group with the most spans. Log warning listing discarded trace IDs.

#### Multiple disconnected root spans

**Rule:** Accept all roots. Layout renders them all at **depth 0** using their absolute time coordinates. The first root (earliest `start_time_ns`) is `root_span_ids[0]`.

**Overlap rendering (normative):** Because flame graph coordinates are absolute (`x = (start_time_ns - trace_start) / trace_duration`), two root spans that overlap in time will produce overlapping rectangles at depth 0. This is **accepted behavior** — it mirrors how Jaeger renders multi-root traces. The later-rendered root paints on top (painter's order = `root_span_ids` order = `start_time_ns` ascending). Users can click either root; hit-test returns the **topmost** (last-painted) span at that coordinate.

> **Why not stack roots into separate lanes?** Stacking would require the flame graph to become a multi-lane layout, which conflicts with the depth-based coordinate model and adds significant complexity. For the rare multi-root case, visual overlap at depth 0 with distinct subtrees below is an acceptable and conventional trade-off.

#### Orphan parent references (`parent_span_id` not in span set)

**Rule:** Treat as root. This handles cross-trace parent refs and truncated exports.

#### Cross-trace parent (`parent_span_id` in a different `trace_id` group)

**Rule:** Treat as orphan root within the selected trace group.

#### Missing `service_name`

**Rule:** Default to `"unknown_service"`.

#### Empty input (zero parseable spans)

**Rule:** Return `WideError::NoValidSpans`.

### 4.7 Trace Assembly

```rust
pub fn build_trace(spans: Vec<Span>, format: InputFormat) -> Result<Trace, WideError> {
    // 1. Deduplicate by span_id (first wins)
    // 2. Group by trace_id; pick most populated if multiple
    // 3. Validate timestamps: swap if end < start
    // 4. Build parent→children index; detect cycles via DFS visited-set
    // 5. Identify roots: spans with no parent or parent not in set
    //    Primary root = earliest start_time_ns among roots
    // 6. total_duration_ns = max(end) - min(start) across all spans
    // 7. Compute self_time per span (interval-union algorithm, see below)
    // 8. Collect unique services → resources map
    // 9. Return Trace
}
```

### 4.8 Self-Time Algorithm

Self-time is **not** `duration - sum(child durations)`. That formula double-subtracts when child spans overlap in time. The correct definition:

> **self_time(span)** = span.duration − duration_of(**union** of direct child intervals, each clipped to parent bounds)

**Algorithm:**

```
function compute_self_time(span, children):
    if children is empty:
        return span.duration_ns

    // Clip each child to parent bounds
    intervals = []
    for child in children:
        clipped_start = max(child.start_time_ns, span.start_time_ns)
        clipped_end   = min(child.end_time_ns, span.end_time_ns)
        if clipped_start < clipped_end:
            intervals.push((clipped_start, clipped_end))

    // Merge overlapping intervals
    intervals.sort_by(|a, b| a.0.cmp(&b.0))
    merged = [intervals[0]]
    for interval in intervals[1..]:
        if interval.0 <= merged.last().1:
            merged.last().1 = max(merged.last().1, interval.1)
        else:
            merged.push(interval)

    // Union duration
    child_occupied = sum(end - start for (start, end) in merged)
    self_time = span.duration_ns - child_occupied
    return max(self_time, 0)  // clamp for safety against clock skew
```

**Why this matters:** In async LLM pipelines, a parent span often has multiple child HTTP calls that overlap (parallel tool calls, concurrent retrievals). The naive formula would report negative self-time or artificially low values. The interval-union approach is correct for both sequential and parallel children.

**Complexity:** O(c log c) per span where c = number of direct children. Negligible even for 5,000 spans.

---

## 5. Conventions Resolver

### 5.1 Purpose

Maps raw span attributes to the canonical `LlmSpanAttributes` struct. This is the **single point of change** when a new AI framework is added.

### 5.2 Conventions JSON Schema

```json
{
  "name": "OpenTelemetry Semantic Conventions for GenAI",
  "version": "1.28.0",
  "detect": {
    "attribute_prefix": "gen_ai.",
    "any_key_present": ["gen_ai.system", "gen_ai.request.model"]
  },
  "mappings": {
    "operation_type": {
      "attribute": "gen_ai.operation.name",
      "values": {
        "chat": "ChatCompletion",
        "text_completion": "TextCompletion",
        "embeddings": "Embedding"
      },
      "default": "Unknown"
    },
    "model_name": { "attribute": "gen_ai.request.model" },
    "model_provider": { "attribute": "gen_ai.system" },
    "input_tokens": { "attribute": "gen_ai.usage.input_tokens", "type": "int" },
    "output_tokens": { "attribute": "gen_ai.usage.output_tokens", "type": "int" },
    "total_tokens": { "attribute": "gen_ai.usage.total_tokens", "type": "int" },
    "temperature": { "attribute": "gen_ai.request.temperature", "type": "float" },
    "top_p": { "attribute": "gen_ai.request.top_p", "type": "float" },
    "max_tokens": { "attribute": "gen_ai.request.max_tokens", "type": "int" },
    "input_messages": {
      "source": "events",
      "event_name": "gen_ai.content.prompt",
      "content_attribute": "gen_ai.prompt"
    },
    "output_messages": {
      "source": "events",
      "event_name": "gen_ai.content.completion",
      "content_attribute": "gen_ai.completion"
    }
  }
}
```

### 5.3 Resolution Algorithm

```
for each span in trace.spans:
    for each convention in loaded_conventions (ordered by priority):
        if convention.detect matches span.attributes:
            span.llm = resolve(span, convention.mappings)
            break  // first match wins
```

**Priority (MVP):** Built-in order: `opentelemetry.json` > `openinference.json` > `langchain.json`. First matching convention wins. User-supplied overrides are deferred to v1 (see Section 5.4).

### 5.4 Conventions Loading

> **Key constraint:** `connect-src: 'none'` blocks **all** `fetch()` / `XMLHttpRequest` calls, including same-origin. Conventions therefore **cannot** be loaded via runtime fetch in any CSP-enforced deployment. They must be embedded at build time.

**MVP mechanism:** Vite imports conventions JSON as static string constants at build time using `?raw` imports:

```typescript
// ui/src/lib/conventions-bundle.ts  (generated or hand-maintained)
import otelRaw from '../../../conventions/opentelemetry.json?raw';
import openinferenceRaw from '../../../conventions/openinference.json?raw';

export const BUNDLED_CONVENTIONS: string[] = [otelRaw, openinferenceRaw];
```

The loader in `wasm.ts` passes these directly to `wasm.init()`:

```typescript
export async function loadWasm(): Promise<void> {
  await init();
  const merged = '[' + BUNDLED_CONVENTIONS.join(',') + ']';
  wasmInit(merged);
}
```

No runtime fetch. No CSP conflict. Works identically across all deployment modes.

**Per-deployment behavior:**

| Deployment mode | Conventions source | User override |
|---|---|---|
| **GitHub Pages / hosted** | Bundled into JS at build time | Not supported at MVP. User must fork and rebuild. |
| **Local `file://`** | Same bundled JS | Same — no override mechanism. |
| **Single-file HTML export (v1)** | Inlined in the JS bundle within the HTML | No override possible. |

**User override (v1):** When user-configurable conventions are added in v1, they will **not** use `fetch()`. Instead, the user loads a `custom.json` file through the same file picker / drag-drop mechanism used for traces. The app detects it is a conventions file (top-level `"name"` + `"mappings"` keys), merges it into the priority list, and stores it in a Svelte store for the duration of the session. This respects `connect-src: 'none'`.

**Failure behavior:** Since conventions are embedded at build time, total loading failure is not possible at runtime — the strings are always present in the JS bundle. If a conventions JSON is malformed (e.g., corrupted during a manual edit of `conventions/opentelemetry.json` before build), `wasm.init()` logs a `CONVENTION_ERROR` warning and proceeds with remaining valid conventions. If **all** conventions are invalid, `wasm.init()` accepts an empty array — spans render without LLM-specific attributes, and a warning is surfaced in `TraceSummary.warnings`.

---

## 6. WASM–JS Boundary Contract

### 6.1 Exported Functions

All data crosses the boundary as JSON strings (via `serde_json`). This avoids complex `wasm-bindgen` type marshalling and keeps the boundary debuggable.

```rust
// crates/widescope-core/src/lib.rs
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn init(conventions_json: &str) -> Result<String, JsValue>;
// Returns: InitResult JSON (always Ok; convention parse errors are warnings, not fatal)

#[wasm_bindgen]
pub fn parse_trace(raw_input: &str) -> Result<String, JsValue>;
// Returns: TraceSummary JSON

#[wasm_bindgen]
pub fn compute_flamegraph() -> Result<String, JsValue>;
// Returns: FlameGraphLayout JSON

#[wasm_bindgen]
pub fn compute_timeline() -> Result<String, JsValue>;
// Returns: TimelineLayout JSON

#[wasm_bindgen]
pub fn get_span_detail(span_id: &str) -> Result<String, JsValue>;
// Returns: SpanDetail JSON

#[wasm_bindgen]
pub fn search_spans(query: &str) -> Result<String, JsValue>;
// Returns: JSON array of matching span_ids (v1 — not part of MVP)
```

### 6.2 Exact Response Schemas

Every WASM→JS response has a concrete schema. These are the **normative contracts** between Rust and TypeScript — both sides must match exactly.

> **Design principle: raw values are normative, display strings are convenience.**
> Duration, token count, and cost fields are returned as **raw numeric values** (the canonical contract). Rust also provides pre-formatted `*_display` strings as a convenience to avoid reimplementing formatting in JS, but JS is free to ignore them and format from raw values. If the formatting logic ever needs to change (locale, precision, tooltip detail), only the Rust display helpers change — the raw contract is stable.
>
> **Timestamp precision constraint:** Absolute nanosecond Unix timestamps (e.g., `1700000001000000000`) exceed JavaScript's `Number.MAX_SAFE_INTEGER` (`2^53 - 1 ≈ 9.0e15`). Therefore:
> - **Absolute timestamps** (`start_time_ns`, `timestamp_ns`) are serialized as **JSON strings** and typed as `string` in TypeScript. JS must not parse these to `number` for arithmetic — use the `_display` convenience string or `BigInt(value)` if comparison is needed.
> - **Durations** (`duration_ns`, `self_time_ns`, `total_duration_ns`) remain as `number`. The maximum trace duration that fits safely is ~104 days in nanoseconds — well within any realistic trace.
> - **Normalized coordinates** (`x`, `width`, `x_start`, `x_end`) remain as `number` (always in `[0, 1]`).

**`init` → `InitResult`:**

```typescript
interface InitResult {
  conventions_loaded: number;         // Count of successfully parsed convention files
  warnings: ParseWarning[];           // Convention parse failures (non-fatal)
}
```

`init()` **always returns `Ok`**. Convention parse errors are returned as warnings inside `InitResult`, not thrown as `Err`. The WASM module initializes with whatever valid conventions were parsed (possibly zero). JS stores `InitResult.warnings` and merges them into the next `TraceSummary.warnings` display.

**`parse_trace` → `TraceSummary`:**

```typescript
// ui/src/lib/types.ts
interface TraceSummary {
  trace_id: string;
  span_count: number;
  service_count: number;
  detected_format: 'OtlpJson' | 'JaegerJson' | 'OpenInferenceJson';
  has_errors: boolean;
  total_duration_ns: number;          // Raw: nanoseconds (normative)
  total_duration_display: string;     // Convenience: "4.23s"
  root_operation: string | null;
  root_service: string | null;
  warnings: ParseWarning[];
}

interface ParseWarning {
  code: string;
  message: string;
  count: number;
  context: Record<string, string | number> | null;
  // Structured debug details for warnings that carry extra info:
  //   TIMESTAMP_INVERTED  → { span_id, original_start_ns: string, original_end_ns: string }
  //   CYCLE_SEVERED       → { child_span_id, severed_parent_span_id }
  //   DUPLICATE_SPAN_ID   → { span_id, kept: "first", discarded_index: number }
  //   TRACE_ID_DISCARDED  → { discarded_trace_id, discarded_span_count: number }
  //   LARGE_TRACE         → { span_count: number }
  //   CONVENTION_ERROR    → { file_name, parse_error }
  // For grouped warnings (count > 1), context reflects the first occurrence.
  // Null when no additional context is meaningful.
}
```

**`compute_flamegraph` → `FlameGraphLayout`:**

```typescript
interface FlameGraphLayout {
  nodes: FlameNode[];
  max_depth: number;                  // 0-indexed
  trace_duration_ns: number;          // Raw (normative)
  trace_duration_display: string;     // Convenience
}

interface FlameNode {
  span_id: string;
  label: string;                      // "service: operation"
  x: number;                          // Normalized [0, 1]
  width: number;                      // Normalized [0, 1]
  depth: number;                      // 0 = root
  color_key: string;                  // Service name
  is_error: boolean;
  is_llm: boolean;
  duration_ns: number;                // Raw (normative)
  self_time_ns: number;               // Raw (normative)
  duration_display: string;           // Convenience: "12.3ms"
  self_time_display: string;          // Convenience: "4.1ms"
}
```

**`compute_timeline` → `TimelineLayout`:**

```typescript
interface TimelineLayout {
  blocks: TimelineBlock[];
  rows: TimelineRow[];
  trace_duration_ns: number;          // Raw (normative)
  trace_duration_display: string;     // Convenience
}

interface TimelineBlock {
  span_id: string;
  label: string;
  service_name: string;
  x_start: number;                    // Normalized [0, 1]
  x_end: number;                      // Normalized [0, 1]
  row_index: number;
  is_error: boolean;
  is_llm: boolean;
  duration_ns: number;                // Raw (normative)
  duration_display: string;           // Convenience
}

interface TimelineRow {
  service_name: string;
  row_index: number;
}
```

**`get_span_detail` → `SpanDetail`:**

```typescript
interface SpanDetail {
  span_id: string;
  trace_id: string;
  parent_span_id: string | null;
  operation_name: string;
  service_name: string;
  span_kind: 'Internal' | 'Server' | 'Client' | 'Producer' | 'Consumer';
  start_time_ns: string;              // Raw (normative) — string for precision (see note above)
  start_time_display: string;         // Convenience
  duration_ns: number;                // Raw (normative)
  duration_display: string;           // Convenience
  self_time_ns: number;               // Raw (normative)
  self_time_display: string;          // Convenience
  status: 'Unset' | 'Ok' | 'Error';
  error_message: string | null;
  attributes: [string, string][];     // Sorted key-value pairs
  events: EventDetail[];
  llm: LlmDetail | null;
  children_ids: string[];
}

interface EventDetail {
  name: string;
  timestamp_ns: string;               // Raw (normative) — string for precision
  timestamp_display: string;          // Convenience
  attributes: [string, string][];
}

interface LlmDetail {
  operation_type: string;
  model_name: string | null;
  model_provider: string | null;
  input_tokens: number | null;
  output_tokens: number | null;
  total_tokens: number | null;
  estimated_cost_usd: number | null;  // Raw numeric USD (normative). JS formats for display.
  temperature: number | null;
  input_messages: { role: string; content: string | null }[];
  output_messages: { role: string; content: string | null }[];
  tool_calls: { name: string; arguments: string | null; result: string | null }[];
}
```

**`search_spans` → `string[]` (v1):**

```json
["span_id_1", "span_id_2", "span_id_3"]
```

**Search specification (v1 — normative):**

| Aspect | Behavior |
|---|---|
| **Fields searched** | `operation_name`, `service_name`, `span_id`, attribute keys and string values |
| **Matching** | Case-insensitive substring match. No regex, no fuzzy matching at MVP/v1. |
| **Empty query** | Returns empty array (no results, not all spans) |
| **Result ordering** | Spans ordered by `start_time_ns` ascending |
| **Result cap** | No cap — all matching span IDs returned |
| **UI: navigation** | JS stores results in `searchResults` store. Toolbar shows "N matches". Arrow buttons or `↑`/`↓` cycle through results, each setting `selectedSpanId` and auto-panning the flame graph. |
| **UI: highlighting** | Matching spans in flame graph get a highlight border. Non-matching spans are dimmed (reduced opacity). |
| **UI: empty result** | *(UX suggestion)* "No spans match 'query'" message in toolbar area |

### 6.3 State Management

> **Design constraint:** The WASM module holds exactly **one trace at a time**. This is a deliberate MVP limitation, not a neutral implementation detail.

**Rationale:** Single-trace-per-page simplifies the WASM state model, avoids handle management in the JS↔WASM boundary, and matches the primary use case (inspect one file). Multi-trace comparison (e.g., side-by-side diff) is explicitly out of scope for MVP and v1.

**Consequences:**
- `parse_trace` replaces any previously stored trace. Layout data from the old trace is invalidated.
- JS must discard all cached layout data when a new trace is loaded.
- Testing: each WASM test must call `parse_trace` to set up state; there is no isolation between tests unless `parse_trace` is called again.

**Lifecycle semantics:**

| Scenario | Behavior |
|---|---|
| **Double-parse (user loads a new file while previous is "loading")** | WASM is single-threaded. `parse_trace` is synchronous and blocking. The JS call stack prevents a second invocation until the first returns. Therefore, double-parse is **not possible** within a single tab — the second file read callback fires only after the first `handleRawInput` completes. No mutex or guard is needed. |
| **Partial failure (parse succeeds, layout fails)** | **All-or-nothing render (MVP decision).** If `parse_trace` succeeds but `compute_flamegraph()` throws, the JS handler catches the layout error and sets `status: 'error'` — the UI shows the error banner, not a partial view. Rationale: showing a toolbar summary without a flame graph is confusing and untestable for MVP. The `Trace` remains in the `RefCell` but is harmless — it is replaced on the next `parse_trace`. The user can retry by re-dropping the same file. *(v1 consideration: if timeline is added, parse success + flame graph success + timeline failure could show the flame graph alone with a warning. This requires a richer `status` model — e.g., `'partial'` — deferred to v1.)* |
| **Consistency across calls** | Within a single `handleRawInput` call, `parse_trace`, `compute_flamegraph`, `compute_timeline` all execute synchronously on the same JS microtask. No interleaving is possible. They are guaranteed to reference the same trace. `get_span_detail` (called later on click) reads from the same `RefCell` — safe as long as no new `parse_trace` has occurred. JS must invalidate cached `SpanDetail` when `traceState.status` changes. |
| **`get_span_detail` after new trace loaded** | If the user clicks a span from the old flame graph layout while a new trace is already loaded (race between render and click), `get_span_detail` may return `SpanNotFound` because the `span_id` no longer exists in the new trace. The JS shell handles this gracefully by clearing the selection. |

**Future migration path (v2+):** If multi-trace is needed, replace the `RefCell<Option<Trace>>` with a handle-based API: `parse_trace` returns a `trace_handle: u32`, and all subsequent calls take the handle as a parameter. This is a backward-incompatible API change, scoped to a major version.

```rust
thread_local! {
    static TRACE: RefCell<Option<Trace>> = RefCell::new(None);
    static CONVENTIONS: RefCell<Vec<Convention>> = RefCell::new(Vec::new());
}
```

### 6.4 TypeScript Wrapper

```typescript
// ui/src/lib/wasm.ts
import init, {
  init as wasmInit, parse_trace, compute_flamegraph,
  get_span_detail,
  // v1: compute_timeline, search_spans
} from '../../crates/widescope-core/pkg/widescope_core';
import { BUNDLED_CONVENTIONS } from './conventions-bundle';

let initWarnings: ParseWarning[] = [];

export async function loadWasm(): Promise<void> {
  await init();
  const merged = '[' + BUNDLED_CONVENTIONS.join(',') + ']';
  const result: InitResult = JSON.parse(wasmInit(merged));
  initWarnings = result.warnings;  // Stored for merging into first TraceSummary display
}

export function getInitWarnings(): ParseWarning[] { return initWarnings; }

export function parseTrace(raw: string): TraceSummary {
  return JSON.parse(parse_trace(raw));
}

export function getFlameGraphLayout(): FlameGraphLayout {
  return JSON.parse(compute_flamegraph());
}
// ... analogous wrappers
```

### 6.5 Error Protocol

All WASM functions use the single `WideError` enum (Section 11.2). On error, `JsValue` contains a JSON string with this schema:

```json
{
  "error_type": "WideError",
  "code": "INVALID_JSON",
  "message": "Expected ',' or '}' at line 42 column 7",
  "context": { "line": 42, "column": 7 }
}
```

```typescript
// ui/src/lib/types.ts
interface WasmError {
  error_type: string;
  code: string;
  message: string;
  context: Record<string, unknown> | null;
}
```

| Code | `WideError` variant | Severity |
|---|---|---|
| `INVALID_JSON` | `InvalidJson` | Fatal |
| `UNRECOGNIZED_FORMAT` | `UnrecognizedFormat` | Fatal |
| `NO_VALID_SPANS` | `NoValidSpans` | Fatal |
| `SPAN_MISSING_REQUIRED` | (included in `TraceSummary.warnings`) | Warning |
| `NO_TRACE_LOADED` | `NoTraceLoaded` | Fatal |
| `SPAN_NOT_FOUND` | `SpanNotFound` | Fatal |
| `CONVENTION_ERROR` | `ConventionError` | Warning |
| `INTERNAL_ERROR` | (JS-side fallback when WASM error is not parseable) | Fatal |

---

## 7. Svelte Shell Architecture

### 7.1 Component Tree

```
App.svelte
├── Toolbar.svelte
│   ├── File Open button
│   ├── Format badge ("OTLP JSON")
│   ├── Trace summary ("1,234 spans · 3 services · 4.2s")
│   ├── View toggle: Flame Graph | Timeline  (v1 — MVP shows flame only)
│   ├── Search input                         (v1)
│   └── Theme toggle                         (v1)
├── DropZone.svelte          (full-page overlay on drag)
├── JsonPasteModal.svelte    (v1 — on Ctrl+V or button)
├── FlameGraph.svelte        (active view in MVP)
│   └── <canvas>
├── Timeline.svelte          (v1 — conditional: active view)
│   └── <svg>
├── SpanDetail.svelte        (right sidebar on selection)
│   └── LlmPanel.svelte     (nested, if span.llm present)
└── ErrorBanner.svelte       (top banner on errors)
```

### 7.2 Stores

```typescript
// stores/trace.ts
interface TraceState {
  status: 'empty' | 'loading' | 'loaded' | 'error';
  summary: TraceSummary | null;
  flameLayout: FlameGraphLayout | null;
  timelineLayout: TimelineLayout | null;
  error: WasmError | null;
  isSampleTrace: boolean;             // true when auto-loaded sample; suppresses warning banner
}
export const traceState = writable<TraceState>({ status: 'empty', ... });

// stores/selection.ts
export const selectedSpanId = writable<string | null>(null);
export const hoveredSpanId = writable<string | null>(null);
export const activeView = writable<'flame' | 'timeline'>('flame'); // v1: 'timeline' option added when timeline ships
export const searchQuery = writable<string>('');   // v1
export const searchResults = writable<string[]>([]); // v1
```

### 7.3 Data Flow

```
1. User drops file / opens file / pastes JSON
2. input.ts reads as text (FileReader.readAsText)
3. traceState.status = 'loading'
4. wasm.parseTrace(rawText)
   ├─ success → summary + flameLayout [→ + timelineLayout in v1] → status = 'loaded'
   └─ error   → wasmError → status = 'error'
5. Reactive: FlameGraph re-renders [+ Timeline in v1]
6. Click span → selectedSpanId.set(spanId)
7. Reactive: SpanDetail calls wasm.getSpanDetail(spanId)
```

### 7.4 First-Run Experience

When `status === 'empty'`, the app auto-loads a bundled sample trace — an OTLP export of a realistic LLM pipeline (~15-20 spans, 3 services: gateway, rag-retriever, llm-service) so the Conference Explorer persona sees a rendered trace immediately.

> **CSP compliance:** The sample trace is imported at build time as a string constant (like conventions), not fetched at runtime. This avoids conflicting with `connect-src: 'none'`.

```typescript
// ui/src/lib/sample.ts
import sampleRaw from '../../../test-fixtures/otlp/sample_llm_pipeline.json?raw';
export const SAMPLE_TRACE: string = sampleRaw;
```

*(UX suggestion)* A visible badge — e.g., "Sample trace · Load your own file" — should distinguish sample data from user-loaded traces.

**Sample trace warning behavior (normative):**
- The sample trace is a curated fixture and **should not produce warnings**. If it does, that's a build/test bug.
- The JS shell tracks whether the current trace is sample-loaded via a `isSampleTrace: boolean` field in `traceState`.
- If `isSampleTrace` is true, the warning banner is **suppressed** — even if the bundled trace somehow generates warnings (defensive). This avoids confusing first-time users with diagnostic noise they didn't cause.
- When the user loads their own file, `isSampleTrace` is set to `false` and warnings display normally.

---

## 8. Rendering Layer

### 8.1 Flame Graph (Canvas)

**Coordinate system:**
- WASM returns normalized `x ∈ [0,1]`, `width ∈ [0,1]`.
- JS maps: `px_x = x * canvasWidth`, `px_w = width * canvasWidth`.
- Each depth level = `ROW_HEIGHT = 24px`. Canvas height = `(max_depth + 1) * 24`.

**Render loop (per frame):**

```
1. Clear canvas
2. For each FlameNode:
   a. Compute pixel rect from normalized coords + zoom/pan
   b. Cull if outside viewport
   c. Fill with service color
   d. If is_error: red overlay
   e. If is_llm: ⚡ icon at left edge
   f. If width > 40px: draw truncated label
   g. Highlight border if hovered
   h. Selection border (thicker) if selected
3. Draw time axis
```

**Interactions:**

| Event | Action |
|---|---|
| `mousemove` | Hit-test → `hoveredSpanId` |
| `click` | Hit-test → `selectedSpanId` |
| `wheel` + Ctrl | Zoom [1x–100x] |
| `wheel` | Vertical scroll |
| `mousedown` + drag | Horizontal pan |
| `dblclick` | Zoom-to-fit clicked span subtree |
| `0` key or toolbar button | **Reset zoom** — zoom = 1×, pan = 0 (fit entire trace in viewport) |
| `F` key or toolbar button | **Fit selection** — if a span is selected, zoom-to-fit that span's subtree; otherwise same as reset zoom |

**Hit-test optimization:** Spatial index with 64 x-axis buckets. Each node inserted into overlapping bucket(s). On `mousemove`, scan only the relevant bucket + check depth. Reduces O(n) → O(n/64) average for 5,000 spans.

**Color palette:** 12 visually distinct, accessible colors assigned to services in order of first appearance. Error spans get red overlay. LLM spans get icon overlay (same base color for service grouping).

### 8.2 Timeline Swimlane (SVG)

**Structure:** One **service group** per distinct `service_name`. Within each group, spans are packed into **lanes** to avoid temporal overlap.

**Lane packing algorithm (normative):**

```
for each service_group ordered by first span start_time:
    lanes = []
    spans_in_group.sort_by(start_time_ns)
    for span in spans_in_group:
        placed = false
        for lane in lanes:
            if span.start_time_ns >= lane.last_end_time_ns:
                lane.push(span)
                lane.last_end_time_ns = span.end_time_ns
                placed = true
                break
        if not placed:
            lanes.push(new Lane with [span])
    assign row_index sequentially across all service groups and their lanes
```

This ensures overlapping spans within a service render on separate sub-rows instead of occluding each other. The `row_index` in `TimelineBlock` maps to the actual visual row (including sub-rows).

**Dimensions:** Lane height `28px`, service group header `20px`, gap between groups `8px`.

```svg
<svg>
  <g class="time-axis">...</g>
  <g class="service-groups">
    <g class="service-group" data-service="gateway">
      <text class="group-label">gateway</text>
      <g class="lane" data-lane="0">
        <rect class="span-block" data-span-id="abc" ... />
      </g>
      <g class="lane" data-lane="1">
        <rect class="span-block" data-span-id="def" ... />
      </g>
    </g>
  </g>
</svg>
```

SVG elements get `role="button"`, `aria-label`, `tabindex` for accessibility. Keyboard: Tab through spans, Enter to select.

**Layout contract update:** `TimelineRow` now represents a lane within a service group:

```typescript
interface TimelineRow {
  service_name: string;
  row_index: number;
  lane_index: number;            // 0-indexed within the service group
}
```

### 8.3 Span Detail Sidebar

Default width `400px`, min `300px`, max `600px`. Resizable via drag handle on left edge. Pushes main view left (not overlay). On viewports narrower than `768px`, sidebar becomes a bottom sheet (full width, max 50% height).

**Sections:**
1. **Header:** operation, service badge, duration, status icon
2. **Timing:** start, end, duration, self-time, % of trace
3. **Attributes:** sorted key-value table, filterable, long values expand-on-click
4. **Events:** chronological, expandable
5. **LLM Panel** (conditional): see 8.4
6. **Children:** clickable links to child spans

### 8.4 LLM Panel (v1)

Nested inside SpanDetail, visible for LLM spans.

**Sections:**
1. **Model info:** provider badge, model name, operation type
2. **Token usage:** input/output/total with horizontal bar chart
3. **Cost:** estimated USD
4. **Messages:** collapsible input/output messages, syntax-highlighted JSON
5. **Tool calls:** collapsible with arguments and results
6. **Retrieved documents** (RAG): list with relevance scores

---

## 9. Input Handling

### 9.1 File Picker

```typescript
function openFilePicker(): void {
  const input = document.createElement('input');
  input.type = 'file';
  input.accept = '.json';
  input.onchange = (e) => {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (file) handleFile(file);
  };
  input.click();
}
```

### 9.2 Drag and Drop

`DropZone.svelte` listens on document body. Shows overlay on `dragenter`/`dragover`, hides on `dragleave`/`drop`. On `drop`, reads the first file.

### 9.3 Paste

Global `keydown` listener for Ctrl/Cmd+V when no input element is focused → opens `JsonPasteModal` with `<textarea>`.

### 9.4 Common Handler

```typescript
const MAX_FILE_SIZE = 20 * 1024 * 1024; // 20 MB (see Section 13.3 memory amplification note)

async function handleFile(file: File): Promise<void> {
  if (file.size > MAX_FILE_SIZE) { showError('File too large (20 MB max)'); return; }
  handleRawInput(await file.text());
}

function handleRawInput(text: string): void {
  traceState.set({ status: 'loading', summary: null, flameLayout: null, timelineLayout: null, error: null });
  try {
    const summary = parseTrace(text);
    const flameLayout = getFlameGraphLayout();
    const timelineLayout = null; // v1: getTimelineLayout() when timeline is added
    traceState.set({ status: 'loaded', summary, flameLayout, timelineLayout, error: null });
  } catch (err) {
    const wasmError = safeParseWasmError(err);
    traceState.set({ status: 'error', summary: null, flameLayout: null, timelineLayout: null, error: wasmError });
  }
}

/** Safely extract a WasmError from a thrown WASM value.
 *  WASM errors may arrive as: (a) a string containing JSON, (b) an Error
 *  whose .message is JSON, or (c) something unexpected. Never assume shape. */
function safeParseWasmError(err: unknown): WasmError {
  let raw: string | undefined;
  if (typeof err === 'string') raw = err;
  else if (err instanceof Error) raw = err.message;

  if (raw) {
    try { return JSON.parse(raw) as WasmError; } catch { /* fall through */ }
  }

  return {
    error_type: 'Unknown',
    code: 'INTERNAL_ERROR',
    message: String(err),
    context: null,
  };
}
```

> **Note on `.jsonl` support:** JSONL (newline-delimited JSON) is **deferred to v2**. The MVP file picker accepts `.json` only. JSONL requires line-by-line parsing semantics that are not yet specified.

---

## 10. Security and CSP

### 10.1 Content Security Policy

```html
<meta http-equiv="Content-Security-Policy"
      content="default-src 'self';
               script-src 'self' 'wasm-unsafe-eval';
               style-src 'self' 'unsafe-inline';
               img-src 'self' data: blob:;
               connect-src 'none';
               font-src 'self';
               object-src 'none';
               base-uri 'self';
               form-action 'none';">
```

| Directive | Value | Reason |
|---|---|---|
| `connect-src` | `'none'` | Blocks **all** `fetch()`, `XMLHttpRequest`, and WebSocket calls (same-origin included). This is the primary privacy enforcement layer. |
| `script-src` | `'self' 'wasm-unsafe-eval'` | WASM instantiation; no `eval()` or inline scripts |
| `style-src` | `'self' 'unsafe-inline'` | Svelte scoped styles compile to inline `<style>` |
| `img-src` | `'self' data: blob:` | PNG export via `Canvas.toBlob` |
| `form-action` | `'none'` | No form submissions |

### 10.2 WASM Sandbox

Browser-enforced guarantees (not app-level):
- Cannot access DOM, network, filesystem
- Cannot access memory outside linear memory

### 10.3 Privacy Guarantees and Caveats

The app is **designed** not to make outbound network requests. Enforcement depends on deployment mode:

| Mode | Enforcement mechanism | Strength |
|---|---|---|
| **Hosted (GitHub Pages, self-hosted)** | CSP `connect-src: 'none'` meta tag + WASM sandbox | **Strong.** Browser enforces CSP. Two independent layers prevent exfiltration. |
| **Local `file://`** | WASM sandbox only | **Moderate.** Browsers do not consistently enforce CSP meta tags for `file://` origins. The WASM module still cannot make network calls (no JS bridge is provided), but the JS shell is not CSP-constrained. |
| **Single-file HTML export** | CSP meta tag + WASM sandbox | **Strong** if opened via a local HTTP server. **Moderate** if opened as `file://` (same caveat as above). |

**What we can claim:**
- "The application is designed to make zero outbound network requests."
- "In hosted deployments, this is enforced by Content Security Policy (`connect-src: 'none'`) and the WASM sandbox."
- "The WASM module physically cannot make network calls — it has no network API bindings."

**What we should not claim:** "It is impossible for any data to leave the browser under any circumstances." This overstates what `file://` mode can guarantee.

### 10.4 Data Lifecycle

1. File read into JS string → passed to WASM → parsed in linear memory
2. Layout returned as JSON → rendered to Canvas/SVG
3. On page close or new trace → previous data GC'd (JS) / dropped (WASM `RefCell` replaced)

**No `localStorage`, `IndexedDB`, cookies, or persistent storage.** Each session is ephemeral. No trace data persists after the tab is closed or a new trace is loaded.

---

## 11. Error Handling Strategy

### 11.1 Error Categories

| Category | Severity | User-facing behavior |
|---|---|---|
| Invalid JSON syntax | Fatal | Red banner with line/column info |
| Unrecognized format | Fatal | Banner suggesting supported formats |
| No valid spans | Fatal | Banner listing what was found and why spans were rejected |
| Individual span parse failure | Warning | Yellow banner: "3 of 1,237 spans could not be parsed" — rest renders normally |
| Convention resolution failure | Silent | Span renders without LLM panel |
| Render error (canvas) | Fatal | Banner suggesting refresh |

### 11.2 Rust Error Types

```rust
// crates/widescope-core/src/errors.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WideError {
    #[error("Invalid JSON: {message}")]
    InvalidJson { message: String, line: Option<usize>, column: Option<usize> },

    #[error("Unrecognized trace format")]
    UnrecognizedFormat,

    #[error("No valid spans found in input")]
    NoValidSpans { attempted: usize, failures: Vec<String> },

    #[error("No trace loaded")]
    NoTraceLoaded,

    #[error("Span not found: {span_id}")]
    SpanNotFound { span_id: String },

    #[error("Convention error: {message}")]
    ConventionError { message: String },
}
```

All variants implement `Into<JsValue>` via JSON serialization matching the error protocol in Section 6.5.

### 11.3 Warning Model

Warnings are **non-fatal observations** that do not prevent the trace from rendering. They are distinct from errors in transport, lifecycle, and UI treatment.

**Design principle:** Errors are **thrown** (Rust `Err` → JS exception). Warnings are **returned in-band** (embedded in successful response payloads). They never share a transport path.

**Warning sources and lifecycle:**

| Source | When generated | Where accumulated | How surfaced |
|---|---|---|---|
| Skipped spans (missing required fields) | `parse_trace` | `TraceSummary.warnings` | Yellow banner on load |
| Duplicate `span_id` discarded | `parse_trace` | `TraceSummary.warnings` | Yellow banner on load |
| Timestamp inversion corrected | `parse_trace` | `TraceSummary.warnings` | Yellow banner on load |
| Cycle severed | `parse_trace` | `TraceSummary.warnings` | Yellow banner on load |
| Convention resolution failure for a span | `parse_trace` (during convention pass) | `TraceSummary.warnings` | Yellow banner on load |
| Convention file malformed at init | `init()` | Returned in `InitResult.warnings` (always `Ok`, never thrown) — WASM initializes with remaining valid conventions | Yellow banner on init |
| Large trace (>10,000 spans) | `parse_trace` | `TraceSummary.warnings` | Yellow banner on load |

**Accumulation rules:**
- Warnings are accumulated **only during `parse_trace`** (and `init()` for convention errors). Layout functions (`compute_flamegraph`, `compute_timeline`, `get_span_detail`) do not generate warnings.
- Warnings are stored in the `Trace` struct alongside spans. `TraceSummary.warnings` is a snapshot taken at parse time.
- Warnings are **not** globally accumulated across calls. Each `parse_trace` call produces a fresh warning list.
- Grouped warnings use `count` to avoid flooding: e.g., one `ParseWarning { code: "SPAN_MISSING_REQUIRED", message: "12 spans skipped: missing spanId", count: 12 }` instead of 12 individual warnings.

**UI contract:** The JS shell renders warnings from `TraceSummary.warnings` in a dismissible yellow banner below the toolbar. The banner is shown once per trace load. Convention init warnings (from `init()`) are shown in the same banner if present. The exact wording is a **UX suggestion**, not a normative contract — implementers may rephrase.

**Banner overflow behavior (normative):**

| Condition | Behavior |
|---|---|
| **1–3 warnings** | All shown inline in the banner, one line each |
| **4+ warnings** | Banner shows a collapsed summary: *"4 warnings during trace load"* with an **expand/collapse toggle**. Expanded view shows all warnings as a scrollable list, max height `200px`. |
| **Dismiss** | Clicking the dismiss button (×) hides the banner for the current trace. It does not reappear until a new trace is loaded. |
| **Warning with `context`** | Each warning line can optionally show a "details" toggle that reveals the `context` key-value pairs inline. Collapsed by default. |
| **Init + parse warnings combined** | Init warnings (from `InitResult`) are prepended to the list with a `[init]` prefix to distinguish their source. |

---

## 12. Testing Strategy

### 12.1 Rust Unit Tests

Inline `#[cfg(test)]` modules in each source file. Fixture-driven from `test-fixtures/`.

| Module | Coverage |
|---|---|
| `parsers::otlp_json` | Valid OTLP, missing fields, malformed JSON, empty `resourceSpans`, zero spans |
| `parsers::jaeger` | Jaeger export, μs→ns conversion, process resolution |
| `parsers::openinference` | Phoenix export, attribute mapping |
| `parsers::detect_format` | Each format, ambiguous cases, non-JSON |
| `conventions::resolver` | GenAI attributes, OpenInference, missing attributes, user override precedence |
| `models::trace::build_trace` | Root detection, self-time, multi-root handling |
| `models::trace::build_trace` (invariants) | Duplicate span_id first-wins, timestamp inversion swap+warning, cycle detection+severing, multi-trace-id selection, orphan parent → root |
| `models::trace::self_time` | Overlapping children (interval union), sequential children, zero-duration spans, children extending outside parent bounds |
| `parsers::otlp_json` (AnyValue) | `bytesValue` → base64 string, `kvlistValue` → JSON string, nested arrays → flattened `StringArray` |
| `layout::flamegraph` | x/width computation, depth assignment, sorting, multi-root sequential rendering |
| `layout::timeline` | Row assignment, block positioning |

### 12.2 WASM Integration Tests

`wasm-pack test --headless --chrome` — tests the full pipeline through `#[wasm_bindgen]`:

```rust
#[wasm_bindgen_test]
fn test_parse_otlp_and_get_flamegraph() {
    let raw = include_str!("../../../test-fixtures/otlp/llm_chat_completion.json");
    init("[]").unwrap();
    let summary_json = parse_trace(raw).unwrap();
    let summary: TraceSummary = serde_json::from_str(&summary_json).unwrap();
    assert!(summary.span_count > 0);

    let flame_json = compute_flamegraph().unwrap();
    let flame: FlameGraphLayout = serde_json::from_str(&flame_json).unwrap();
    assert!(!flame.nodes.is_empty());
}
```

### 12.3 Svelte Component Tests

Vitest + `@testing-library/svelte`:

| Test | Coverage |
|---|---|
| `Toolbar.test.ts` | Format badge updates, search input events |
| `FlameGraph.test.ts` | Canvas renders, click dispatches selection |
| `SpanDetail.test.ts` | Correct attributes for mock span |
| `DropZone.test.ts` | Overlay on dragenter, handler on drop |

### 12.4 End-to-End Tests (Playwright)

| Scenario | Steps |
|---|---|
| Sample auto-load | Open page → flame graph has spans |
| File upload | Upload fixture → correct span count in toolbar |
| Span selection | Click flame bar → sidebar with correct operation |
| Error display | Upload invalid JSON → error banner |
| Paste JSON | Ctrl+V → paste in modal → trace loads |
| Theme toggle | Click button → `data-theme="dark"` |

### 12.5 Contract Snapshot Tests

To prevent drift between Rust serialization and the TypeScript interfaces in Section 6.2, the following **snapshot tests** are run in CI:

```rust
// crates/widescope-core/tests/contract_snapshots.rs
use insta::assert_json_snapshot;

#[test]
fn trace_summary_shape() {
    let raw = include_str!("../../../test-fixtures/otlp/llm_chat_completion.json");
    init("[]").unwrap();
    let json: serde_json::Value = serde_json::from_str(&parse_trace(raw).unwrap()).unwrap();
    assert_json_snapshot!("trace_summary", json);
}

#[test]
fn flamegraph_layout_shape() {
    // ... parse_trace first ...
    let json: serde_json::Value = serde_json::from_str(&compute_flamegraph().unwrap()).unwrap();
    assert_json_snapshot!("flamegraph_layout", json);
}

#[test]
fn span_detail_shape() {
    // ... parse_trace first ...
    let json: serde_json::Value = serde_json::from_str(&get_span_detail("some_span_id").unwrap()).unwrap();
    assert_json_snapshot!("span_detail", json);
}

#[test]
fn init_result_shape() {
    let json: serde_json::Value = serde_json::from_str(&init("[]").unwrap()).unwrap();
    assert_json_snapshot!("init_result", json);
}
```

**How this prevents drift:**
- `insta` snapshots capture the exact JSON key names, types, and nesting structure.
- If a Rust struct field is added, removed, or renamed, the snapshot diff surfaces it in CI.
- The developer then updates the snapshot **and** the TS interface in Section 6.2 together.
- Snapshots are committed to `crates/widescope-core/tests/snapshots/`.

**v1 extension:** Generate TS types from a JSON Schema or `ts-rs` crate to eliminate manual sync entirely.

### 12.6 Invariant and Degradation Tests

Tests that specifically cover the new trace invariants (Section 4.6) and degradation behaviors (Section 13.4):

**WASM integration (Rust):**

| Test | Fixture | Assertion |
|---|---|---|
| Duplicate span_id | OTLP with 2 spans sharing `span_id` | `span_count` = N-1, `warnings` contains `DUPLICATE_SPAN_ID` |
| Timestamp inversion | Span with `end < start` | Warning contains `TIMESTAMP_INVERTED` + original timestamps in context |
| Cycle A→B→A | Two spans with mutual parent refs | One becomes orphan root, `warnings` contains `CYCLE_SEVERED`, `children_ids` reflects severed tree |
| Multi-trace-id | File with spans from 2 trace IDs | Largest group selected, warning lists discarded trace ID |
| Multi-root | 3 spans, none with parent | All rendered at depth 0, `root_span_ids` has 3 entries, earliest = `root_span_ids[0]` |
| Unparseable WASM error | Force non-JSON error from WASM (e.g., panic) | `safeParseWasmError` returns `INTERNAL_ERROR` fallback |
| Large trace warning | >10,000 span fixture | `warnings` contains `LARGE_TRACE` warning |
| Convention init failure | Pass malformed JSON to `init()` | `InitResult.warnings` contains `CONVENTION_ERROR`, `conventions_loaded` = 0, subsequent `parse_trace` still works (no LLM attributes) |
| Empty conventions | `init("[]")` | `parse_trace` succeeds, `span.llm` is null for all spans |

**E2E (Playwright):**

| Scenario | Steps |
|---|---|
| Warning banner on bad spans | Upload fixture with 3 invalid spans → yellow banner with count |
| Warning dismissal | Click dismiss on warning banner → banner hides, trace still rendered |
| Large trace warning | Upload >10K span fixture → warning banner shown before render |

### 12.6 Performance Benchmarks

`criterion` crate:

| Benchmark | Target |
|---|---|
| Parse 5,000-span OTLP JSON | < 100ms |
| Flame graph layout (5,000 spans) | < 50ms |
| Timeline layout (5,000 spans) | < 30ms |
| Convention resolution (5,000 spans) | < 20ms |
| **Total: file drop → rendered** | **< 200ms** |

---

## 13. Performance Budgets and Degradation

> **Note:** These budgets are **aspirational targets**, not pass/fail MVP gates. They guide implementation decisions and profiling priorities. Missing a target does not block release — but significant misses (>2×) should trigger investigation and a documented fallback decision.

### 13.1 Load Time

| Metric | Target |
|---|---|
| WASM (gzipped) | ≤ 500 KB |
| JS bundle (gzipped) | ≤ 100 KB |
| CSS (gzipped) | ≤ 10 KB |
| **Total transfer** | **≤ 650 KB** |
| TTI (3G) | ≤ 3s |
| TTI (Wi-Fi) | ≤ 1s |

> **JS bundle note:** The previous 60 KB target was unrealistic given build-time-bundled conventions and sample trace. 100 KB is more realistic with Vite tree-shaking and gzip.

### 13.2 Runtime

| Metric | Target |
|---|---|
| Parse + layout (1,000 spans) | < 50ms |
| Parse + layout (5,000 spans) | < 200ms |
| Flame graph frame | < 16ms (60fps) |
| Hit-test on mousemove | < 2ms |
| Span detail retrieval | < 1ms |

### 13.3 Memory

| Metric | Target |
|---|---|
| WASM linear memory (1,000 spans) | < 10 MB |
| WASM linear memory (5,000 spans) | < 50 MB |
| JS heap (layout data) | < 20 MB |
| Canvas memory | 1 buffer, viewport-sized |

**Memory amplification note:** A raw JSON trace file undergoes several expansions during processing. For a 10 MB input file, expect:

| Stage | Estimated memory | Reason |
|---|---|---|
| JS string (input text) | ~10 MB | UTF-16 internal representation may be larger |
| WASM copy (input bytes) | ~10 MB | Passed across boundary as string |
| `serde_json::Value` (transient) | ~20-30 MB | JSON AST with per-value allocations |
| Canonical `Vec<Span>` | ~15-25 MB | Depends on attribute count per span |
| Layout JSON (returned to JS) | ~5-10 MB | FlameGraphLayout serialized |
| JS parsed layout objects | ~5-10 MB | Parsed JSON in JS heap |
| **Peak transient total** | **~60-90 MB** | During parse; drops after GC and WASM dealloc |
| **Steady state** | **~30-50 MB** | Canonical spans + layout objects |

This means the `MAX_FILE_SIZE` of 50 MB could cause peak memory of ~300-450 MB, which may stress low-memory devices. Consider lowering to **20 MB** for MVP, or adding a memory warning for files > 20 MB.

### 13.4 Known Tensions and Degradation Strategies

The budgets above contain potential conflicts. This section documents each tension and its explicit fallback.

**Tension 1: JSON serialization at every WASM↔JS boundary call.**

Serializing a 5,000-node `FlameGraphLayout` to JSON and parsing it in JS is not free. Estimated overhead: ~10-20ms for serialization + parsing on a mid-range laptop.

- **Mitigation (MVP):** Accept the cost. It fits within the 200ms budget alongside parse + layout.
- **Fallback (v2):** If profiling shows serialization is a bottleneck, switch `FlameGraphLayout` and `TimelineLayout` to a flat `Float64Array` / `Uint32Array` binary encoding passed via shared WASM memory. This avoids JSON entirely but requires manual (de)serialization code on both sides. Only justified if JSON overhead exceeds 50ms.

**Tension 2: Hidden ARIA tree with thousands of nodes.**

A 5,000-element ARIA tree is not free in DOM memory or layout cost, even if hidden with `display: none`.

- **Mitigation (MVP):** Do **not** build the ARIA tree at MVP. MVP accessibility is limited to the SVG timeline (natively accessible) and keyboard shortcuts for basic flame graph navigation.
- **Fallback (v1):** Build the ARIA tree only for the **visible viewport** of the flame graph (virtualized). When the user zooms/pans, update the ARIA subtree to reflect only visible nodes. Cap at 200 ARIA elements.
- **Threshold:** If span count > 500, skip the full ARIA tree entirely and rely on keyboard nav + screen reader announcements for the selected span only.

**Tension 3: Canvas hit-testing at 5,000 spans.**

The 64-bucket spatial index (Section 8.1) may still be slow if many spans are concentrated in a narrow time range.

- **Mitigation:** Throttle `mousemove` hit-testing to once per `requestAnimationFrame` (max 60 checks/sec, not once per mouse event).
- **Fallback:** If a single bucket contains > 200 nodes, switch to a k-d tree for that region. This is a v2 optimization — MVP accepts occasional >1ms hit-tests.

**Tension 4: Large trace causes overall slowdown (>10,000 spans).**

Beyond the design target of 5,000 spans, all budgets may be exceeded.

- **Guard:** After `parse_trace`, if `span_count > 10,000`, show a warning: "Large trace (N spans). Rendering may be slow." Let the user proceed.
- **Future (v2):** Implement span sampling — display top-N spans by duration or error status, with an option to expand.

---

## 14. Accessibility

### 14.1 Flame Graph (Canvas)

Canvas is inherently inaccessible. Approach is phased to balance performance (see Section 13.4, Tension 2):

**MVP — keyboard navigation (normative):**

The flame graph maintains a **keyboard cursor** (`focusedSpanId`) independent of mouse hover. It is driven entirely by keyboard input and does not require any hidden DOM.

| Key | Action | Edge behavior |
|---|---|---|
| `↑` | Move to parent span | At root: no-op |
| `↓` | Move to first child (by `start_time_ns`) | Leaf node: no-op |
| `←` | Move to previous sibling (same parent, ordered by `start_time_ns`) | First sibling: wrap to last sibling. At root level with multiple roots: move to previous root. |
| `→` | Move to next sibling | Last sibling: wrap to first sibling. At root level: move to next root. |
| `Enter` | Set `selectedSpanId = focusedSpanId` (opens SpanDetail sidebar) | — |
| `Escape` | Clear selection, keep focus cursor | — |
| `Home` | Move to first root span | — |
| `End` | Move to last root span | — |

**Focus tracking and viewport:**
- The keyboard cursor is always a valid `span_id` from the current `FlameGraphLayout`.
- On each cursor move, the canvas **auto-scrolls/auto-pans** to ensure the focused node is visible. If the node is outside the current zoom viewport, the viewport pans minimally to bring the node into view. No zoom change.
- **Focus indicator:** 2px high-contrast ring (white with dark outline for visibility on any service color) drawn on canvas around the focused node's rect.

**Screen reader announcements:**
- Driven by **keyboard focus changes only** (not mouse hover). When `focusedSpanId` changes, an `aria-live="polite"` region is updated with: *`"{service}: {operation}, {duration_display}, {N} children"`*. *(UX suggestion — exact wording may vary.)*
- When `selectedSpanId` changes (Enter pressed), a second announcement: *"Span detail opened for {operation}"*.

**v1 — virtualized ARIA tree:**
- `<div role="tree">` alongside canvas, populated only with nodes visible in the current zoom viewport (capped at 200 elements). Updated on zoom/pan via `IntersectionObserver`-like logic against the normalized coordinate viewport.
- **Threshold:** If `span_count > 500`, the ARIA tree is not built. Only `aria-live` announcements and keyboard nav are available.

### 14.2 Timeline (SVG)

Natively accessible:
- Each `<rect>` has `role="button"`, `aria-label`, `tabindex="0"`.
- Tab order = chronological within each row.
- CSS focus: `rect:focus { outline: 2px solid var(--focus-color); }`.

### 14.3 General

- Visible focus indicators on all interactive elements.
- Color is never the sole information channel (errors have icons + text).
- Minimum contrast 4.5:1 (WCAG AA).
- Respects `prefers-reduced-motion` — disables animations.
- Respects `prefers-color-scheme` — auto light/dark.

### 14.4 Mobile / Touch Stance

**Desktop-first (normative).** WideScope is designed for desktop browsers with mouse + keyboard. Touch and mobile are explicitly **not targeted** for MVP or v1.

**Rationale:** Flame graph exploration requires precise hover, multi-level zoom/pan, and keyboard navigation — interactions that map poorly to touch. The primary persona (developer/ML engineer) uses a desktop or laptop.

**What works incidentally on mobile:**
- File picker and drag-drop (via mobile browser file API)
- Span detail sidebar (responsive bottom sheet at < 768px)
- Error/warning banners

**What does not work on mobile:**
- Canvas zoom/pan (no pinch-to-zoom mapping)
- Hover-based hit-test and tooltips
- Keyboard navigation

**Future (v2+):** If mobile demand emerges, add touch gesture mapping (pinch-to-zoom, tap-to-select, long-press for detail). This is a significant UX effort and is out of scope until validated by user feedback.

---

## 15. Phased Delivery

### Phase 1: MVP

**Goal:** Drop an OTLP JSON file → see flame graph → click spans to inspect. Smallest loop that proves value.

| Component | Deliverable |
|---|---|
| Rust: OTLP JSON parser | `resourceSpans` → canonical `Span` |
| Rust: Trace assembly | Build `Trace`, self-time computation, all invariants (Section 4.6) |
| Rust: Flame graph layout | `FlameGraphLayout` |
| Rust: Span detail | `SpanDetail` by `span_id` |
| Rust: OTel GenAI conventions | `gen_ai.*` → `LlmSpanAttributes` |
| WASM boundary | `init`, `parse_trace`, `compute_flamegraph`, `get_span_detail` |
| Svelte: Toolbar | File open, format badge, summary |
| Svelte: FlameGraph | Canvas render, zoom, pan, click, hover, keyboard nav |
| Svelte: SpanDetail | Sidebar with attributes, events, children |
| Svelte: DropZone | Drag-and-drop file loading |
| Svelte: ErrorBanner | Parse error + warning display |
| Sample trace | Bundled OTLP JSON (build-time), auto-loaded on first visit |
| CSP | Full `connect-src: 'none'` policy |
| CI | cargo test + wasm-pack build + vite build + Playwright |
| Hosting | GitHub Pages |

**Deferred from MVP to reduce delivery risk:**

| Component | Rationale for deferral |
|---|---|
| Timeline view | Second visualization mode — not needed to prove core value |
| Search (`search_spans`) | Under-specified; better to add once core UX is validated |
| JsonPasteModal | File upload and drag-drop cover the primary input path |
| Theme toggle | Cosmetic; can ship with a single theme initially |
| LLM Panel (v1) | Conventions resolver lands in MVP, but the dedicated LLM panel UI is v1 |

### Phase 2: v1

**Goal:** Multi-format support, timeline view, search, LLM-specific views, static report export.

| Component | Deliverable |
|---|---|
| Svelte: Timeline | SVG swimlane with lane packing (Section 8.2) |
| Rust: Timeline layout | `TimelineLayout` + `compute_timeline` WASM export |
| Rust: Search | `search_spans` WASM export |
| Svelte: Search | Search input in toolbar, result highlighting in flame graph |
| Svelte: JsonPasteModal | Paste raw JSON |
| Svelte: Theme toggle | Light/dark theme |
| Rust: Jaeger parser | Jaeger UI export JSON |
| Rust: OpenInference parser | Arize Phoenix export |
| Rust: OpenInference conventions | `llm.*` → `LlmSpanAttributes` |
| Rust: LangChain conventions | LangChain attribute mappings |
| Rust: Critical path analysis | Identify bottleneck spans |
| Rust: Cost attribution | Token cost by model pricing table |
| Svelte: LlmPanel | Messages, tokens, cost, tool calls |
| Svelte: DagView | Service dependency DAG (SVG) |
| Static report | Self-contained HTML export (inline WASM base64) |
| Conventions registry | Community contribution workflow + CI validation |
| Accessibility | ARIA tree for flame graph |

### Phase 3: v2

**Goal:** Binary format support, image export, CLI integration.

| Component | Deliverable |
|---|---|
| Rust: OTLP Protobuf parser | `prost`-based protobuf decoding |
| Rust: URL-referenced file | Fetch trace from URL (requires CSP exception for that mode) |
| PNG export | `Canvas.toBlob` → download |
| CLI HTML report | Wasmtime wrapper for headless trace → HTML |
| Performance | Virtualized rendering for 10,000+ span traces |

---

## Appendix A: Dependency Inventory

### Rust (Cargo.toml)

| Crate | Version | Purpose |
|---|---|---|
| `wasm-bindgen` | 0.2.x | WASM↔JS interop |
| `serde` | 1.x | Serialization framework |
| `serde_json` | 1.x | JSON parsing |
| `thiserror` | 2.x | Error derive macros |
| `js-sys` | 0.3.x | JS type bindings (for `Date`, etc.) |
| `web-sys` | 0.3.x | Web API bindings (only if needed) |
| `wasm-bindgen-test` | 0.3.x | WASM test harness |
| `criterion` | 0.5.x | Benchmarks (dev only) |

No `prost` at MVP — added in v2 for protobuf support.

### JavaScript (package.json)

| Package | Purpose |
|---|---|
| `svelte` | UI framework (compile-time, no runtime) |
| `vite` | Bundler + dev server |
| `vite-plugin-wasm` | WASM import support |
| `vite-plugin-top-level-await` | Top-level await for WASM init |
| `@testing-library/svelte` | Component test utilities |
| `vitest` | Unit test runner |
| `playwright` | E2E test runner |

### Build Tools

| Tool | Purpose |
|---|---|
| `wasm-pack` | Rust → WASM compilation |
| `wasm-opt` (via `binaryen`) | WASM binary optimization |
| `cargo-watch` | Dev rebuild on save |

---

## Appendix B: Flame Graph Layout Algorithm

The layout algorithm converts a tree of spans into a list of `FlameNode` with normalized coordinates.

```
Input:  Trace (with parent→children index, root_span_ids: Vec<String>)
Output: Vec<FlameNode>

function layout(trace):
    // Supports multiple roots (see Section 4.6)
    roots = trace.root_span_ids
                 .map(|id| trace.spans[id])
                 .sort_by(|a, b| a.start_time_ns.cmp(&b.start_time_ns))

    trace_start = min(span.start_time_ns for span in trace.spans)
    trace_end   = max(span.end_time_ns for span in trace.spans)
    trace_duration = trace_end - trace_start

    nodes = []
    for root in roots:
        visit(root, depth=0, trace_start, trace_duration, nodes)
    return nodes

function visit(span, depth, trace_start, trace_duration, nodes):
    x     = (span.start_time_ns - trace_start) / trace_duration
    width = span.duration_ns / trace_duration

    nodes.push(FlameNode {
        span_id: span.span_id,
        label: format!("{}: {}", span.service_name, span.operation_name),
        x, width, depth,
        color_key: span.service_name,
        is_error: span.status is Error,
        is_llm: span.llm.is_some(),
        duration_ns: span.duration_ns,                    // Raw (normative)
        self_time_ns: span.self_time_ns,                  // Raw (normative)
        duration_display: format_duration(span.duration_ns),   // Convenience
        self_time_display: format_duration(span.self_time_ns), // Convenience
    })

    children = get_children(span.span_id)
    children.sort_by(|a, b| a.start_time_ns.cmp(&b.start_time_ns))
    for child in children:
        visit(child, depth + 1, trace_start, trace_duration, nodes)
```

**Edge cases:**
- Orphan spans (parent not in trace): treated as roots at depth 0, positioned by their own timestamps.
- Overlapping children: rendered overlapping (no artificial stacking). The user can zoom to distinguish.
- Zero-duration spans: rendered with `min_width = 1px` equivalent in normalized coords.

---

## Appendix C: Duration Formatting

```rust
fn format_duration(ns: u64) -> String {
    match ns {
        0..=999                    => format!("{}ns", ns),
        1_000..=999_999            => format!("{:.1}μs", ns as f64 / 1_000.0),
        1_000_000..=999_999_999    => format!("{:.1}ms", ns as f64 / 1_000_000.0),
        _                          => format!("{:.2}s",  ns as f64 / 1_000_000_000.0),
    }
}
```
