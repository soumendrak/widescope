# Open a trace file

## Sub-features

- OTLP JSON, Jaeger JSON and OpenInference JSON, detected by shape in `crates/widescope-core/src/parsers/mod.rs`.
- A `.zip` holding one trace JSON is unpacked in the browser (`readZipFile` in `ui/src/lib/input.ts`) and loads like the JSON.
- A file the parser rejects shows the error banner instead of a trace.

## How to get to it (user POV)

Open `/editor/`, click "Open file" and pick a `.json` or `.zip` file.

## Driving it with drive.mjs

```bash
node $S/drive.mjs http://127.0.0.1:$P test-fixtures/otlp/sample_llm_pipeline.json $E --expect-spans 7 --expect-format 'OTLP JSON'
node $S/drive.mjs http://127.0.0.1:$P test-fixtures/jaeger/sample_llm_pipeline.json $E --expect-spans 7 --expect-format 'Jaeger JSON'
node $S/drive.mjs http://127.0.0.1:$P test-fixtures/openinference/sample_llm_pipeline.json $E --expect-spans 7 --expect-format OpenInference
python3 -c 'import sys, zipfile; zipfile.ZipFile(sys.argv[2], "w", zipfile.ZIP_DEFLATED).write(sys.argv[1], "trace.json")' test-fixtures/otlp/sample_llm_pipeline.json /tmp/sample-otlp.zip
node $S/drive.mjs http://127.0.0.1:$P /tmp/sample-otlp.zip $E --expect-spans 7 --expect-format 'OTLP JSON'
printf '{"hello":"world"}' > /tmp/not-a-trace.json
node $S/drive.mjs http://127.0.0.1:$P /tmp/not-a-trace.json $E --expect-error "Unrecognized trace format"
```

## Gotchas

- `--expect-format` compares the stats-bar badge, which shows `FORMAT_LABELS` from `ui/src/components/FilterBar.svelte`.
- The file picker only offers `.json` and `.zip` (`input.accept` in `ui/src/lib/input.ts`). Playwright's file chooser ignores that filter, so drive.mjs reads the picker's `accept` and fails a file a user couldn't pick.
- The files are capped at 20 MB in the UI.
