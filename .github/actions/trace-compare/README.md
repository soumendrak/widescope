# WideScope trace compare action

Compares two LLM trace JSON files (OTLP, Jaeger, or OpenInference) and posts the
token/span diff as a PR comment. Optionally fails the check when a budget is
exceeded. No build step — runs on Node already present on GitHub runners.

## Usage

```yaml
# .github/workflows/trace-eval.yml
on: pull_request
permissions:
  pull-requests: write   # to post the comment
jobs:
  trace-compare:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      # ... your eval step produces baseline.json + candidate.json ...
      - uses: ./.github/actions/trace-compare
        with:
          baseline: baseline.json
          candidate: candidate.json
          budget: '{"max_total_tokens": 100000, "max_spans": 500}'
          fail-on-budget: "true"
```

## Inputs

| Input | Default | Description |
| --- | --- | --- |
| `baseline` | – | Path to baseline trace JSON (required) |
| `candidate` | – | Path to candidate trace JSON (required) |
| `budget` | `""` | JSON limits: `max_spans`, `max_input_tokens`, `max_output_tokens`, `max_total_tokens` |
| `fail-on-budget` | `true` | Fail the check on any budget violation |
| `comment` | `true` | Post/update a PR comment with the diff table |
| `github-token` | `${{ github.token }}` | Token for posting the comment |

Output `budget-exceeded` is `"true"`/`"false"`.

## Notes

Token/span extraction is standalone (covers the common OTel GenAI,
OpenInference, and Jaeger token conventions) — it is **not** the full convention
resolver in `widescope-core`, and cost-based budgets are not computed yet. Once
the `widescope` CLI ([#15](https://github.com/soumendrak/widescope/issues/15))
lands, swap `compare.js` for `widescope compare --format markdown` for
authoritative metrics including cost.

Run the self-check with `node compare.test.js`.
