# WideScope VS Code Extension

Inspect LLM and AI agent traces directly in VS Code without leaving your editor.

## Features

- **Right-click a `.json` trace file** → "View Trace in WideScope"
- **Active JSON editor** → "View Current Trace in WideScope"
- Opens a WebView panel with the full WideScope UI embedded
- Loads OTLP, Jaeger, or OpenInference trace JSON instantly
- All data stays local — no upload, no telemetry

## Usage

1. Open a trace JSON file in VS Code (or right-click one in the Explorer)
2. Run the "WideScope: View Trace in WideScope" command
3. The trace opens in a new editor tab with flame graph, timeline, waterfall, and LLM detail views

## Requirements

- VS Code 1.85.0 or higher
- Internet connection (loads WideScope UI from CDN on first use)

## Development

```bash
cd extensions/vscode
npm install
npm run compile
# Press F5 in VS Code to launch Extension Development Host
```

## Publishing

```bash
npm install -g @vscode/vsce
vsce package
vsce publish
```

## License

MIT — see [LICENSE](../../LICENSE)
