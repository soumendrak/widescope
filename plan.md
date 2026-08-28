# WideScope UX Overhaul Plan

*2026-08-28. Built from three inputs: driving the current app in a browser (landing, editor, all five views, both themes), a file-by-file map of `ui/`, and pattern research on Langfuse, Honeycomb, Jaeger/Tempo, and Braintrust/Phoenix.*

## Verdict: rebuild the shell, keep the engine

No from-scratch rewrite. The hard 40% — the Rust/WASM parsers, the canvas flame graph, the virtualized waterfall, share links — works and is worth keeping. What's failing is everything around it: layout, hierarchy, theming, polish. That all lives in two god components (`App.svelte`, 1,579 lines; `Toolbar.svelte`, 1,010 lines) that would be rewritten during any cleanup anyway.

**Rebuild the UI shell around one design system and a workspace-first layout.** All of the visible change, a fraction of a rewrite's cost.

## 1. Diagnosis

Each finding verified live in the running app or pinned to file:line.

| # | Severity | Finding |
|---|----------|---------|
| 1 | Critical | **The JSON editor never yields the stage.** After loading a trace, the raw JSON textarea still owns the top half of the screen; the waterfall is pushed below the fold; selecting a span opens the inspector as an overlay *inside* the already-halved viz area. First run stacks a hero panel *and* the empty textarea. This one hierarchy inversion is most of the "UX feels wrong." |
| 2 | Critical | **Split identity.** Landing is dark-only; editor defaults light — crossing `/` → `/editor/` is a white flash into a different-feeling product. The two pages share zero tokens; even the brand blue differs (`#3B82F6` vs `#2563eb`). |
| 3 | High | **The span inspector is an afterthought.** The best content (model, tokens, cost, prompt/completion, RAG docs) sits in a cramped overlay covering the waterfall it describes. With nothing selected, `SpanDetail.svelte` renders nothing at all — layout shift, no affordance. |
| 4 | High | **Graph view is effectively invisible** in light mode: near-white labels, hairline gray edges on white. Canvas/SVG colors are partly hardcoded hex that ignores the theme (`FlameGraph.svelte` has ~6 more). |
| 5 | High | **Landing content doesn't exist without scroll-triggered JS.** Sections 01–03 are blank until an `IntersectionObserver` fires; this exact fragility already blanked the page in Cloudflare prod when CSP blocked the inline script. |
| 6 | High | **Accessibility debt**: orphaned `role="row"`, `role="tree"` on an opaque canvas, div-buttons ignoring Space, dialogs without focus traps, tabs without tabpanels, global `Cmd+V` hijack that can dead-end in "Clipboard access was blocked." (Punch list in §6.) |
| 7 | Medium | **Emoji as iconography** (🎯 🌙 ⏳ ⚠️ 🔗): inconsistent across OSes, verbose for screen readers, reads unfinished. ~10 inline SVGs replace them all. |
| 8 | Medium | **Silent filters, missing empty states.** No "N of M spans" readout; a filter that empties the view shows a blank panel. Only DiffView has a real empty state; loading is a single ⏳. |
| 9 | Medium | **God components, no shared primitives.** ~60% of the UI codebase is CSS; button/chip/panel/dialog styles are re-declared per component with different values. |
| 10 | Medium | **Loose ends**: number keys 1–5 use a different view order than the animation's `VIEW_ORDER` (wrong slide directions); `100vh` without `dvh`; dead `site/` folder (1,124 lines); `setupGlobalPasteListener` exported but never imported. |

## 2. Prior art — what the leaders converged on

| Pattern | Proven by | Status in WideScope |
|---------|-----------|---------------------|
| Master–detail split: viz + persistent inspector pane | Langfuse, Jaeger, Tempo, Phoenix | Missing |
| Dark mode as the default | Every observability tool | Inverted |
| Demo data instead of an empty first run | Langfuse; measured SaaS onboarding lift | Partial — sample exists but hides behind hero + textarea |
| Critical path in the waterfall, on by default | Jaeger ([PR #1582](https://github.com/jaegertracing/jaeger-ui/pull/1582)) | Partial — opt-in flame toggle only |
| Tree/timeline as toggled lenses on one trace | Langfuse ([trace view](https://langfuse.com/changelog/2025-03-19-new-trace-view)) | Have — 5 views; needs coherence, not more views |
| Conversation view for LLM spans | Langfuse sessions, Braintrust threads | Missing — **the differentiator**; prompt/completion parsing already exists |
| ⌘K command palette | Linear/Vercel/GitHub — table stakes | Partial — ⌘K only focuses search |
| Landing: visible content, quickstart, demo link up top | Langfuse, PostHog | Fragile |

Fair note: in-trace search with operators, share links, and budgets are already ahead of some hosted tools. Deliberately skipped: Honeycomb's BubbleUp (needs many traces + aggregates — wrong shape for a single-trace viewer today).

## 3. Target: the workspace-first layout

One inversion drives everything: **the trace is the hero; JSON is source code.**

```
Today                                Proposed
┌──────────────────────────────┐     ┌──────────────────────────────┐
│ toolbar + stats + filters    │     │ slim bar · trace ▾ · ⌘K ·    │
├──────────────────────────────┤     │ views · share                │
│                              │     ├────────────────────┬─────────┤
│ JSON editor (~50%, always    │     │                    │inspector│
│ open)                        │     │ visualization      │rail     │
├───────────────────┬──────────┤     │ stage              │(persist-│
│ waterfall         │inspector │     │                    │ ent)    │
│ (squeezed)        │(overlay) │     ├────────────────────┴─────────┤
└───────────────────┴──────────┘     │ ▸ source · trace.json (drawer)│
                                     └──────────────────────────────┘
```

**Layout principles**

- First run = the workspace itself, empty, with three actions in the stage: sample / open / paste. No hero panel stacked on a textarea.
- Editor becomes a collapsible bottom drawer — open on first run, auto-collapsed the moment a trace parses.
- Inspector always present, resizable, with a "select a span" hint state; stats (spans, cost, p50/p95) move to its summary header.
- One top bar: brand, trace switcher, ⌘K, view tabs, share. Filters live with the views they filter, with an always-visible "N of M spans" count.
- Dark by default, light as the toggle.

**System principles**

- One `tokens.css` — color, space, type, radius, motion — imported by the landing *and* the app. The theme schism becomes structurally impossible.
- Shared primitives: Button, Chip, Badge, Panel, EmptyState, Dialog (with focus trap).
- A ~10-glyph inline SVG icon set replaces every emoji.
- Canvas reads all colors from tokens via one `getComputedStyle` helper — no hardcoded hex in renderers.

## 4. Roadmap — five phases, shipped in order

Sequenced so every phase ships something visible, and the design system lands *before* the layout rebuild so the new shell is assembled from primitives instead of accreting a third god component.

### Phase 0 — Triage (a weekend) — shipped

- [x] **Auto-collapse the editor on successful parse** — the single highest-leverage change in this plan; the collapse mechanic already exists behind the Submit button.
- [x] Inspector empty state ("select a span to inspect") instead of vanishing.
- [x] Fix Graph view contrast (tokens, not hardcoded hex).
- [x] Add the "N of M spans" filter readout.
- [x] Fix the 1–5 shortcut vs `VIEW_ORDER` mismatch; stop intercepting `Cmd+V` when focus is in a text field.
- [x] Delete `site/` and the unused `setupGlobalPasteListener`.
- [x] **Validated** (§5): editor collapses to 88px on `?sample=1` and stays open while typing; stage 757px sits *beside* the 420px rail with no overlap; `4` → graph; kind:producer → "0 of 7 spans" in red.

*Why first: real relief this week, and it de-risks the bigger phases.*

### Phase 1 — Design system (1–2 weeks) — shipped

- [x] Extract `tokens.css`: one palette (dark default + light), spacing scale, type scale, radii, easings — consumed by both entry points.
- [x] Build shared primitives + the SVG icon set; migrate components onto them, deleting duplicated CSS as you go.
- [x] Split `App.svelte` into WasmBoot / Workspace / EditorDrawer / WelcomeState / keyboard router; split `Toolbar.svelte` into TopBar / FilterBar.
- [x] **Validated** (§5): `--color-accent` resolves `#3b82f6` on both `/` and `/editor/` (landing's `--blue` alias now maps to it, its private palette deleted); scales resolve on both; no emoji left in chrome; tabs pair with `#view-panel` and render in `VIEW_ORDER`; dialog traps focus and restores it to the opener; Phase 0's 10 regressions all still pass. App.svelte 1579→489 lines, Toolbar 1010→129.

### Phase 2 — Workspace shell (1–2 weeks) — shipped

- [x] Ship the three-zone layout: slim bar, full-height stage, persistent inspector rail, source drawer.
- [x] New first-run: empty workspace with sample / open / paste actions in the stage.
- [x] Dark default; theme toggle preserved; every empty, loading, and error state designed (skeletons for view switches).
- [x] Fold the a11y punch list (§6) into the new shell as it's built — not as a later pass.
- [x] **Validated** (§5): stage went from 34% to **67%** of an 800px viewport (collapsed drawer 308px → 45px); zones stack with no overlap and no horizontal scroll at 1280 and 768, rail beside the stage at 1280 and below it at 768; first run = load actions in the stage, drawer open, no hero; empty/loading/error all designed and non-blank; **axe clean** across all five views, empty state, and with a dialog open; skip link is the first tab stop and reveals; Space activates rows; theme resolves correctly across all five OS/stored combinations; console clean.

*This is the phase where WideScope stops looking like a JSON tool with charts and starts looking like an observability product.*

### Phase 3 — Signature features (ongoing, pick by impact) — 3 of 4 shipped

- [x] **Conversation view**: LLM spans rendered as a chat transcript with token/cost per turn — the LLM-native lens no generic trace viewer has. Lead the landing page with this.
- [x] **Critical path on by default** in the waterfall, Jaeger-style.
- [x] **Real ⌘K palette**: search spans, switch views, load sample, toggle theme, export — one surface.
- [ ] Diff view: from a comparison table toward aligned side-by-side waterfalls. *(Deferred — deliberately descoped; the other three carried the phase.)*
- [x] **Validated** (§5): transcript order matches waterfall order exactly (`embeddings, context.rerank, chat, tool_call.search_web`) with per-turn tokens (`512 → 256 tok`) and cost (`$0.003840`); critical path marks 3 of 7 bars at first paint and its state is shared between waterfall and flame; ⌘K opens focused and executes span-jump, view switch, theme, and critical-path toggle by keyboard alone; slot 3 shows Timeline on a non-LLM trace and a stale `conversation` view falls back without blanking; axe clean on both new surfaces; 11/11 regressions pass, console clean.

*Slot 3 swaps rather than adding a tab: Conversation on LLM traces, Timeline otherwise — five lenses either way, per the no-sixth-view guardrail. `lib/views.ts` is the single source for the tab list, the 1–5 order, and the slide direction.*

### Phase 4 — Landing rebuild (a few days) — shipped

- [x] Content visible with JS disabled; animations are progressive enhancement only.
- [x] Script in an external file so Cloudflare's CSP can never blank the page again.
- [x] Keep the "page is a trace" concept — it's genuinely distinctive — rebuilt on shared tokens, with a light theme.
- [x] Langfuse-pattern structure: "try the sample trace" is now the **primary** hero CTA, GitHub stars and a three-step quickstart section are in. **Screenshot captured** — driving a real Chromium from a script (rather than the MCP browser, which never persisted a file) produced a shot of the current UI at `?sample=1` with a span selected; it now backs `docs/assets/screenshot.webp`, the README and `og:image` asset, which had been showing the pre-overhaul UI. It was deliberately *not* swapped into the hero: the same phase mandates keeping the animated "page is a trace" scope, and a static image would delete it. Swapping remains a one-line change if the call goes the other way.
- [x] **Validated** (§5): all 6 sections readable with **JS disabled** and under a **script-blocking CSP** — the prod failure reproduced via an injected `script-src 'none'` header, not assumed; with JS on, 27/27 reveals still animate on scroll and the ruler runs to 2,400 ms; **zero inline `<script>`** in either built page; the hero sample link lands on `/editor/?sample=1` with 7 spans parsed; landing **axe clean** (was 3 serious violations); theme set on `/` carries into `/editor/`.

*Why last: the landing's job is to show off the product. Rebuild it once there's a new product to show.*

### Follow-up — the Submit button was redundant

Live parse is unconditional (150 ms debounce, no size guard, no toggle), so by
the time Submit was clickable the trace had already parsed and rendered.
`submitEditor` ran the same `applyEditorValue`; its only unique effect was
collapse + focus the stage, which the disclosure caret already did. A leftover
from the submit-driven model live parse replaced.

- [x] Delete the Submit button; keep ⌘⏎ (renamed `dismissEditor` — flush the
      pending parse, collapse, hand focus to the trace) with the shortcut shown
      on the drawer strip.
- [x] Delete `onEditorKeyDown`, dead in `App.svelte` since the Phase 1 split.
- [x] Collapse `.editor-btn` / `.editor-btn--ghost` into one style — Submit was
      the only primary button, so the pair had nothing left to distinguish.
- [x] **Validated** (§5): typing alone yields a parsed trace ("1 of 1 spans")
      with the drawer still open; ⌘⏎ collapses, unmounts the textarea and puts
      focus in the stage; the caret reopens it; 12/12 regressions, axe clean,
      console clean.

## 5. Validation — a Playwright gate after every phase

No phase is done when the code is written. It is done when a clean build,
driven in a real browser, proves the phase's claims. Every phase ends with the
same loop:

1. `npx vite build` in `ui/` — a phase that does not compile is not reviewable.
2. `npx vite preview --port 4173`, then drive `/` and `/editor/?sample=1`.
3. Run that phase's **Validate** assertions plus the standing regression set below.
4. Fix what fails, rebuild, re-assert. Repeat until green — then tick the boxes.

**Clear the service worker first.** `vite-plugin-pwa` precaches the build, so a
second `vite preview` run happily serves the *previous* build's HTML and CSS —
the gate then validates code you are not looking at. This cost a false failure
in Phase 1 (the landing reported the old palette from cache). Before asserting:

```js
for (const r of await navigator.serviceWorker.getRegistrations()) await r.unregister();
for (const k of await caches.keys()) await caches.delete(k);
```

then reload. Treat an assertion that contradicts the source as a cache hit until
proven otherwise.

**Assert on the DOM, not on screenshots.** Screenshots are for judging taste;
they are useless as a gate and, in this setup, the MCP screenshot files did not
reliably land on disk where they could be read back. Every check above is
phrased as something `browser_evaluate` can return: a computed style, a measured
rect, a class, an attribute, an element count. Prefer measuring two rects and
comparing them (`stage.right <= rail.left`) over eyeballing an image.

**Standing regression set** — Phase 0's assertions become permanent, and each
phase appends its own. Re-run all of them every phase; the layout rebuild in
Phase 2 is exactly the kind of change that silently undoes Phase 0.

- Editor auto-collapses on load actions and stays open while typing. (Since Phase 2 collapsing unmounts the textarea and leaves a slim strip — assert on `.editor-panel--collapsed` plus the absence of `.editor-input`, not on a textarea height.)
- Inspector rail sits beside the stage, never over it, selected or not.
- `1`–`5` follow `VIEW_ORDER`; typing in a field never triggers a shortcut.
- The "N of M spans" readout tracks the active filter and flags the empty case.
- Graph edges and labels stay legible in both themes.
- Console is free of errors on load, view switch, and span select.

*Why this is in the plan and not left to habit: every finding in §1 was a
regression someone shipped believing it worked.*

## 5b. Final end-to-end pass — 2026-08-28

Every claim above re-driven against a clean `vite build` + `vite preview`, in one
scripted Chromium run (`63/63`), covering the standing regression set, each
phase's own assertions, axe on all five views plus the empty state, an open
dialog, and the landing in both themes. Six defects surfaced and were fixed:

| Defect | Fix |
|--------|-----|
| Landing `.chip` was `rgba(13,22,38,.5)` — a hardcoded dark fill that composited to mud in light theme (contrast 1.66, 11 axe violations); `.chip b` then failed at 3.86 | Chips read `--bg1` / `--ice`; landing is axe-clean in **both** themes |
| Booting `?sample=1` focused the stage, so the skip link was never the first tab stop | Focus follows a click, never a page load (`loadEditorText(text, moveFocus)`) |
| Two emoji left in the chrome: `↗` fullscreen, `📝` note | Both on the SVG icon set (`expand` / `collapse`, new `note` glyph) |
| Span start rendered as raw epoch seconds (`1713300000.160s`) | `format_timestamp_display` emits `YYYY-MM-DD HH:MM:SS.mmm UTC` (unit-tested, incl. the 2000 leap year) |
| A 26-level trace lost its span names entirely — 16 px/level of indent inside a fixed 320 px column | Indent flattens past 10 levels |
| A 200 KB attribute value rendered in full into the inspector | Attribute values scroll inside a 14 em box |

Two parser-level silences found by the new fixtures were also closed: OTLP and
OpenInference swapped inverted timestamps *during parsing*, which made
`TIMESTAMP_INVERTED` unreachable for every supported format, and a span pointing
at a parent absent from the file became an extra root with no warning at all —
a truncated export was indistinguishable from a complete one. Both now warn.

Coverage came from ten new fixtures in `test-fixtures/domains/` (Kubernetes,
banking, IoT, CI/CD, FHIR, video CDN, mobile RUM, legal-doc RAG, an edge-case
file, and a 2,000-span stress trace) driven through the whole core pipeline by
`cargo run -p widescope-core --example check_fixtures`.

## 5c. Merge with `main` — 2026-08-28

The branch was cut from a tree ~40 commits behind `main`, which had meanwhile
shipped four more lenses (Agent flow, Trends, Matrix, Dashboard), live SSE
streaming, session grouping, recent traces in IndexedDB, span notes in share
links, a PWA install button, ZIP upload, and a VS Code host bridge. Resolving
that was integration, not conflict-picking:

- **Views**: the tab bar keeps five lenses; the four later ones moved to a `⋯`
  overflow menu, shortcuts 6-9, and the palette. `lib/views.ts` grew
  `SECONDARY_VIEWS` and stays the single source for tabs, shortcuts and slide
  direction.
- **Toolbar**: live badge and pause, session-grouped trace switcher, install
  button and notes-carrying share links were ported into `TopBar`; the recent-
  trace chips found a better home in the first-run `WelcomeState`.
- **Landing**: `main`'s newer scroll-pinned hero, 3D tilt and scroll-to-top were
  kept, re-gated behind `.js-motion` so a blocked script still leaves the page
  readable, and its script merged with the theme toggle this branch added.
- **Service graph**: `main`'s redesign won outright; the contrast fix this
  branch made to the old one was moot.

Four a11y regressions the merge introduced were caught by the gate and fixed:
an overflow button inside `role="tablist"`, `role="img"` on the service graph
and agent flow SVGs (both have focusable children), and a decorative scroll cue
below 4.5:1. Gate after the merge: **72/72**, plus the repo's own Playwright
suite (4/4), vitest (30/30), `cargo test` (60), clippy clean, and 22/22
fixtures through the core pipeline.

## 6. Guardrails

**What not to do**

- No framework migration. Svelte 5 + Vite is right; adopt runes syntax opportunistically in files you touch, never as a project.
- No component library dependency. The hand-rolled aesthetic is part of the brand; it needs consolidation, not replacement.
- No backend, ever. Local-first is the moat.
- Don't chase full mobile parity. A readable read-only mobile view (waterfall + inspector as a sheet) is enough.
- Don't add a sixth **tab**. Five lenses already compete; the conversation view replaces the
  weakest surface for LLM traces rather than piling on. Lenses that arrived later (Agent flow,
  Trends, Matrix, Dashboard) live behind the `⋯` overflow beside the tabs, on shortcuts 6-9,
  and in ⌘K — reachable without lengthening the strip.

**Accessibility punch list**

- Waterfall: proper `treegrid` semantics, or drop the orphaned `role="row"`.
- Flame graph: keep canvas, add a visually-hidden list of visible spans; keep and extend the `aria-live` region.
- Dialogs: focus trap + `aria-modal` + focus restoration, built once into the shared Dialog.
- Tabs: `aria-controls` ↔ `role="tabpanel"` pairing.
- Real `<button>`s with accessible names for every glyph control; Space works everywhere Enter does.
- A `<main>` landmark and skip link in the editor; `dvh` alongside `vh`.

## Sources

- [Langfuse — new trace view](https://langfuse.com/changelog/2025-03-19-new-trace-view) · [sessions](https://langfuse.com/docs/observability/features/sessions)
- [Honeycomb — platform](https://www.honeycomb.io/platform)
- [Jaeger UI — critical path, PR #1582](https://github.com/jaegertracing/jaeger-ui/pull/1582)
- [Grafana Tempo — visualize traces](https://grafana.com/docs/tempo/latest/visualize-traces/)
- [Braintrust — LLM tracing tools 2026](https://www.braintrust.dev/articles/best-llm-tracing-tools-2026)
- [Arize Phoenix — LLM tracing](https://arize.com/blog/llm-tracing-and-observability-with-arize-phoenix/)
