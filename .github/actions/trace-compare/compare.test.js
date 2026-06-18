// Run: node compare.test.js  (no framework; exits non-zero on failure)
const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { metrics, checkBudget, delta } = require("./compare.js");

const otlp = {
  resourceSpans: [{ scopeSpans: [{ spans: [
    { attributes: [
      { key: "gen_ai.usage.input_tokens", value: { intValue: "512" } },
      { key: "gen_ai.usage.output_tokens", value: { intValue: "256" } },
      { key: "gen_ai.usage.total_tokens", value: { intValue: "768" } },
    ] },
    { attributes: [{ key: "noise", value: { stringValue: "x" } }] },
  ] }] }],
};
let m = metrics(otlp);
assert.strictEqual(m.spans, 2);
assert.strictEqual(m.total_tokens, 768);

// Jaeger tags + total derived from input+output when total absent.
const jaeger = { data: [{ spans: [{ tags: [
  { key: "input_tokens", value: 10 },
  { key: "output_tokens", value: 5 },
] }] }] };
assert.strictEqual(metrics(jaeger).total_tokens, 15);

// OpenInference attribute map.
const oi = { spans: [{ attributes: { "llm.token_count.prompt": 7, "llm.token_count.completion": 3 } }] };
assert.strictEqual(metrics(oi).total_tokens, 10);

// Budget violations.
const cand = { spans: 600, input_tokens: 0, output_tokens: 0, total_tokens: 200000 };
assert.strictEqual(checkBudget(cand, "").length, 0);
assert.strictEqual(checkBudget(cand, '{"max_total_tokens": 100000, "max_spans": 500}').length, 2);
assert.strictEqual(checkBudget(cand, '{"max_total_tokens": 300000}').length, 0);
assert.throws(() => checkBudget(cand, "{bad"), /invalid budget JSON/);

assert.strictEqual(delta(100, 100), "—");
assert.ok(delta(100, 150).includes("+50"));

// Real fixtures parse without throwing.
const root = path.resolve(__dirname, "../../../test-fixtures");
for (const f of ["otlp/sample_llm_pipeline.json", "jaeger/sample_llm_pipeline.json", "openinference/sample_llm_pipeline.json"]) {
  const r = metrics(JSON.parse(fs.readFileSync(path.join(root, f), "utf8")));
  assert.ok(r.spans > 0, `${f} should have spans`);
}

console.log("ok");
