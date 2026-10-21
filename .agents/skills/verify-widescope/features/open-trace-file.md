# Open a trace file

## Sub-features

- OTLP JSON, Jaeger JSON and OpenInference JSON, detected by shape in `crates/widescope-core/src/parsers/mod.rs`.
- A file the parser rejects shows the error banner instead of a trace.

## How to get to it (user POV)

Open `/editor/`, click "Open file" and pick a `.json` or `.zip` file.

## Driving it with drive.mjs

```bash
node $S/drive.mjs http://127.0.0.1:$P test-fixtures/otlp/sample_llm_pipeline.json $E --expect-spans 7 --expect-format 'OTLP JSON'
node $S/drive.mjs http://127.0.0.1:$P test-fixtures/jaeger/sample_llm_pipeline.json $E --expect-spans 7 --expect-format 'Jaeger JSON'
node $S/drive.mjs http://127.0.0.1:$P test-fixtures/openinference/sample_llm_pipeline.json $E --expect-spans 7 --expect-format OpenInference
printf '{"hello":"world"}' > /tmp/not-a-trace.json
node $S/drive.mjs http://127.0.0.1:$P /tmp/not-a-trace.json $E --expect-error "Unrecognized trace format"
```

## Gotchas

- `--expect-format` compares the stats-bar badge, which shows `FORMAT_LABELS` from `ui/src/components/FilterBar.svelte`.

- The file picker only offers `.json` and `.zip` (`ui/src/lib/input.ts`). Playwright's file chooser ignores that filter, so a drive can load an extension a user can't pick. Check `input.accept` when you add a format.
- The files are capped at 20 MB in the UI.
