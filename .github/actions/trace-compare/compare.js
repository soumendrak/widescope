#!/usr/bin/env node
// Compare two trace JSON files (OTLP / Jaeger / OpenInference) and emit a
// markdown diff table. Used by the WideScope GitHub Action (action.yml).
//
// ponytail: standalone token/span extraction, NOT the full convention resolver
// in widescope-core (that's WASM). Covers the common OTel GenAI / OpenInference
// / Jaeger token conventions; switch to the `widescope` CLI (issue #15) for the
// authoritative metrics (incl. cost) once it exists.

const fs = require("fs");

const TOKEN_KEYS = {
  input: ["gen_ai.usage.input_tokens", "llm.token_count.prompt", "input_tokens", "prompt_tokens"],
  output: ["gen_ai.usage.output_tokens", "llm.token_count.completion", "output_tokens", "completion_tokens"],
  total: ["gen_ai.usage.total_tokens", "llm.token_count.total", "total_tokens"],
};

// Normalise any of the three trace shapes into a list of {attrs} where attrs is
// a flat {key: number|string} map.
function extractSpans(doc) {
  const spans = [];
  if (Array.isArray(doc.resourceSpans)) {
    // OTLP JSON
    for (const rs of doc.resourceSpans) {
      for (const ss of rs.scopeSpans || []) {
        for (const sp of ss.spans || []) {
          spans.push({ attrs: otlpAttrs(sp.attributes) });
        }
      }
    }
  } else if (Array.isArray(doc.data)) {
    // Jaeger JSON
    for (const t of doc.data) {
      for (const sp of t.spans || []) {
        spans.push({ attrs: kvAttrs(sp.tags) });
      }
    }
  } else if (Array.isArray(doc.spans)) {
    // OpenInference JSON (attributes already a map)
    for (const sp of doc.spans) spans.push({ attrs: sp.attributes || {} });
  } else {
    throw new Error("unrecognized trace format (expected resourceSpans, data, or spans)");
  }
  return spans;
}

function otlpAttrs(list) {
  const out = {};
  for (const a of list || []) {
    const v = a.value || {};
    out[a.key] = v.intValue ?? v.doubleValue ?? v.stringValue ?? v.boolValue;
  }
  return out;
}

function kvAttrs(list) {
  const out = {};
  for (const a of list || []) out[a.key] = a.value;
  return out;
}

function pick(attrs, keys) {
  for (const k of keys) {
    if (attrs[k] != null) {
      const n = Number(attrs[k]);
      if (!Number.isNaN(n)) return n;
    }
  }
  return 0;
}

function metrics(doc) {
  const spans = extractSpans(doc);
  const m = { spans: spans.length, input_tokens: 0, output_tokens: 0, total_tokens: 0 };
  for (const s of spans) {
    m.input_tokens += pick(s.attrs, TOKEN_KEYS.input);
    m.output_tokens += pick(s.attrs, TOKEN_KEYS.output);
    const t = pick(s.attrs, TOKEN_KEYS.total);
    m.total_tokens += t || pick(s.attrs, TOKEN_KEYS.input) + pick(s.attrs, TOKEN_KEYS.output);
  }
  return m;
}

function load(path) {
  return metrics(JSON.parse(fs.readFileSync(path, "utf8")));
}

function fmt(n) {
  return n.toLocaleString("en-US");
}

function delta(base, cand) {
  const d = cand - base;
  if (d === 0) return "—";
  const pct = base === 0 ? "" : ` (${d > 0 ? "+" : ""}${((d / base) * 100).toFixed(1)}%)`;
  return `${d > 0 ? "🔺 +" : "🔻 "}${fmt(d)}${pct}`;
}

const ROWS = [
  ["Spans", "spans"],
  ["Input tokens", "input_tokens"],
  ["Output tokens", "output_tokens"],
  ["Total tokens", "total_tokens"],
];

function table(base, cand) {
  const lines = [
    "| Metric | Baseline | Candidate | Δ |",
    "| --- | ---: | ---: | --- |",
  ];
  for (const [label, key] of ROWS) {
    lines.push(`| ${label} | ${fmt(base[key])} | ${fmt(cand[key])} | ${delta(base[key], cand[key])} |`);
  }
  return lines.join("\n");
}

// Returns array of violation strings (empty = within budget).
function checkBudget(cand, budgetJson) {
  if (!budgetJson || !budgetJson.trim()) return [];
  let budget;
  try {
    budget = JSON.parse(budgetJson);
  } catch (e) {
    throw new Error(`invalid budget JSON: ${e.message}`);
  }
  const map = {
    max_spans: "spans",
    max_input_tokens: "input_tokens",
    max_output_tokens: "output_tokens",
    max_total_tokens: "total_tokens",
  };
  const violations = [];
  for (const [bk, mk] of Object.entries(map)) {
    if (budget[bk] != null && cand[mk] > budget[bk]) {
      violations.push(`**${mk}** ${fmt(cand[mk])} exceeds budget ${fmt(budget[bk])}`);
    }
  }
  return violations;
}

function main() {
  const [, , baselinePath, candidatePath, budgetJson] = process.argv;
  if (!baselinePath || !candidatePath) {
    console.error("usage: compare.js <baseline.json> <candidate.json> [budget-json]");
    process.exit(2);
  }
  const base = load(baselinePath);
  const cand = load(candidatePath);
  const violations = checkBudget(cand, budgetJson);

  let body = "<!-- widescope-trace-compare -->\n";
  body += "### 🔭 WideScope trace comparison\n\n";
  body += table(base, cand) + "\n";
  if (violations.length) {
    body += "\n> ❌ **Budget violated**\n";
    for (const v of violations) body += `> - ${v}\n`;
  } else if (budgetJson && budgetJson.trim()) {
    body += "\n> ✅ Within budget\n";
  }

  const out = process.env.GITHUB_OUTPUT;
  if (out) {
    fs.appendFileSync(out, `budget_exceeded=${violations.length ? "true" : "false"}\n`);
    const delim = "WIDESCOPE_BODY_EOF";
    fs.appendFileSync(out, `body<<${delim}\n${body}\n${delim}\n`);
  }
  process.stdout.write(body + "\n");
}

if (require.main === module) main();

module.exports = { metrics, checkBudget, table, delta };
