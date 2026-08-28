# widescope-cli

Headless CLI over `widescope-core` for CI/scripted trace analysis. The core's
WASM bindings compile and run natively (only JS interop is wasm-only), so the
CLI calls the same analysis functions the browser uses — no duplicated logic.

```
widescope analyze trace.json [--format text|json]   # parse + summarize
widescope compare baseline.json candidate.json       # metric deltas
widescope check trace.json --budget duration=30s ... # exit 1 on breach
```

`check` budgets: `duration` (`30s`/`500ms`/`100us`/bare ns), `errors`, `spans`.
Repeat `--budget` for multiple gates. Conventions + pricing are baked in from
`conventions/` at build time, matching the UI's bundle.

Skipped from the issue: `serve` (local HTTP/WebSocket UI host) — heavy and not
needed for CI gating, which is the stated use case. Add it if interactive
headless viewing is actually wanted.
