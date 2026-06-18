# WideScope for VS Code

View LLM/agent **trace JSON** files in the [WideScope](https://widescope.soumendrak.com)
viewer without leaving your editor.

- Right-click any `.json` file in the Explorer → **WideScope: View trace file**
- Or run the command from the Command Palette with a `.json` open
- The panel reloads automatically when the file changes on disk

## How it works

The panel embeds the hosted WideScope editor (`/editor/?embed=1`) and pushes the
file's contents in via `postMessage` — nothing is uploaded; the trace stays in
the WebView. Requires an internet connection (it loads the viewer from the CDN).

## Develop / run

```sh
cd vscode-extension
# Open this folder in VS Code and press F5 to launch an Extension Development Host.
```

## Package / publish

```sh
npm i -g @vscode/vsce
vsce package        # -> vscode-widescope-0.1.0.vsix
vsce publish        # needs a publisher + PAT
```

## Not yet done (MVP scope)

- **Compare two traces** — WideScope already has a diff view; a "Compare" command
  would open two files into it. Add when requested.
- **Offline mode** — bundle `ui/dist/` into the extension instead of the CDN.
