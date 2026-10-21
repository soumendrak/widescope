---
name: verify-widescope
description: >
  Proves a WideScope change works before review. Runs the standards gate, serves
  the production build of the trace viewer, opens a trace file through the real
  "Open file" button in headless Chromium and checks what the viewer shows. Use
  after changing crates/, ui/, test-fixtures/ or conventions/, and before opening
  or updating a pull request.
---

# Verify WideScope

Run everything from the repository root. The scripts need Linux (`setsid`, `ss`), `curl` and `jq`.

## 1. Toolchain and standards

Run `just doctor` and fix every `FAIL` line. Then `git fetch origin`, bring the branch up to date with `origin/main` and run `just check`. It must end with `STANDARDS OK`. Never add a comment, a coverage skip or `@ts-ignore` to get it green.

## 2. Launch

```bash
P=9183
S=.agents/skills/verify-widescope/scripts
E=/tmp/verify-widescope/$P
$S/launch.sh $P
```

`launch.sh` builds the WASM package and `ui/dist`, then serves the build with `vite preview` on `127.0.0.1:$P`. It prints `ready:` only after `/editor/` answers and the listener belongs to the process group it started. If the preview fails to start, launch.sh kills the group only when its leader is still this port's `vite preview`. It refuses a port that is already taken and touches nothing. Logs and evidence go to `$E`. A new launch moves the previous launch's screenshots and summaries to `$E/previous-<time>/`, so `$E` only holds evidence from the current build. To test a new edit, stop and launch again: the preview serves the build, not the source.

## 3. Doctor

```bash
$S/doctor.sh $P
```

Read-only. It checks that the port belongs to the process group in `$E/server.pid`, that `/editor/` answers, and that the WASM in `ui/dist` is byte-identical to `crates/widescope-core/pkg`. Drive only after `DOCTOR OK`. Run it again whenever a drive looks wrong.

## 4. Drive

```bash
node $S/drive.mjs http://127.0.0.1:$P test-fixtures/otlp/sample_llm_pipeline.json $E --expect-spans 7 --expect-format 'OTLP JSON' --expect-tabs Waterfall,Flame,Conversation,Graph,Diff
```

The script opens `/editor/`, clicks the "Open file" button, picks the file in the file chooser and waits for either the stats bar or the `role="alert"` error banner. It names the run after the file's path, so files with the same name don't overwrite each other. It writes `<run>-drive.json` and screenshots to `$E`, prints the summary and exits 1 when any expectation fails.

- `--expect-spans N` fails unless the stats bar shows N spans.
- `--expect-format LABEL` fails unless the format badge reads LABEL (`OTLP JSON`, `Jaeger JSON` or `OpenInference`).
- `--expect-error TEXT` fails unless the error banner contains TEXT and no trace is loaded. Without it, any banner fails the run.
- `--tabs` clicks every view tab and fails if a tab is not selected afterwards or its view's root element is not visible. `--expect-tabs A,B,...` does the same and also fails unless the tab bar shows exactly that list.

[features/README.md](features/README.md) lists the journeys. Run every journey for the area you touched.

## 5. Evidence

- Drive the real path: the built app, the real file chooser, a real file on disk. No `?sample=1`, no store setters.
- Keep the before and after screenshots and the `-drive.json` summary. Put the summary and the key screenshots in the PR body.
- If a fixture lacks what a journey needs, report the journey as failed and why. Do not weaken it.

## 6. Stop

```bash
$S/stop.sh $P
```

It kills the process group recorded in `$E/server.pid` only if that group still owns the port, and it keeps the evidence in `$E`. Never `pkill node` or `pkill vite`: other agents share the machine.
