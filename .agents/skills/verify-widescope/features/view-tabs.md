# View tabs

## Sub-features

`viewTabs()` in `ui/src/lib/views.ts` picks the tabs, and the set depends on whether the trace has LLM spans. The OTLP sample gets Waterfall, Flame, Conversation, Graph and Diff. More views sit behind the "More lenses" button and are not driven yet.

## How to get to it (user POV)

Load a trace, then click a tab in the "View mode" tab list.

## Driving it with drive.mjs

```bash
node $S/drive.mjs http://127.0.0.1:$P test-fixtures/otlp/sample_llm_pipeline.json $E --expect-spans 7 --tabs
node $S/drive.mjs http://127.0.0.1:$P test-fixtures/otlp/upload-samples/01-simple-request.json $E --expect-spans 3 --tabs
```

The first trace has LLM spans and gets the Conversation tab. The second has none and gets Timeline. For each tab the summary records `selected` and `rendered`, where `rendered` means that view's own root element is visible in `#view-panel` (`VIEW_ROOT` in `drive.mjs`). A tab with no `VIEW_ROOT` entry fails, so add one when you add a tab. Screenshots are `<run>-3-tab-<name>.png`.

## Gotchas

At 1440x900 a tab's `innerText` is only its glyph, so `drive.mjs` names tabs by the `.vt-label` text.
