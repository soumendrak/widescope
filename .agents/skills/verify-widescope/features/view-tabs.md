# View tabs

## Sub-features

`viewTabs()` in `ui/src/lib/views.ts` picks the tabs, and the set depends on whether the trace has LLM spans. The OTLP sample gets Waterfall, Flame, Conversation, Graph and Diff. More views sit behind the "More lenses" button and are not driven yet.

## How to get to it (user POV)

Load a trace, then click a tab in the "View mode" tab list.

## Driving it with drive.mjs

```bash
node $S/drive.mjs http://127.0.0.1:$P test-fixtures/otlp/sample_llm_pipeline.json $E --expect-spans 7 --tabs
```

The summary lists each tab with `selected` and `rendered`. Screenshots are `<file>-3-tab-<name>.png`.

## Gotchas

At 1440x900 a tab's `innerText` is only its glyph, so `drive.mjs` names tabs by the `.vt-label` text.
