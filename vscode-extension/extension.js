// WideScope VS Code extension — opens a trace .json in the hosted WideScope
// viewer inside a WebView, and re-pushes it when the file changes on disk.
//
// ponytail: loads the editor from the live CDN (widescope.soumendrak.com) per
// the issue's "load from CDN" option — no dist bundling, no build step. Swap
// EDITOR_URL for a bundled local copy only if offline use is ever required.
const vscode = require('vscode');

const EDITOR_ORIGIN = 'https://widescope.soumendrak.com';
const EDITOR_URL = `${EDITOR_ORIGIN}/editor/?embed=1`;

function activate(context) {
  context.subscriptions.push(
    vscode.commands.registerCommand('widescope.viewTrace', async (uri) => {
      const target = uri || vscode.window.activeTextEditor?.document.uri;
      if (!target) {
        vscode.window.showErrorMessage('WideScope: no trace file selected.');
        return;
      }
      openViewer(context, target);
    })
  );
}

async function openViewer(context, uri) {
  const panel = vscode.window.createWebviewPanel(
    'widescopeTrace',
    `WideScope — ${basename(uri)}`,
    vscode.ViewColumn.Active,
    { enableScripts: true, retainContextWhenHidden: true }
  );
  panel.webview.html = pageHtml();

  const push = async () => {
    const bytes = await vscode.workspace.fs.readFile(uri);
    panel.webview.postMessage({ text: Buffer.from(bytes).toString('utf8') });
  };

  // Push once the iframe signals it's ready, and again on every file save.
  panel.webview.onDidReceiveMessage((m) => {
    if (m && m.type === 'ready') void push();
  }, undefined, context.subscriptions);

  // Watch the whole dir with a plain '*' and match by path, so filenames with
  // glob metachars (trace[1].json) still fire. Atomic saves land as create, not
  // change, so watch both.
  const dir = vscode.Uri.joinPath(uri, '..');
  const watcher = vscode.workspace.createFileSystemWatcher(
    new vscode.RelativePattern(dir, '*')
  );
  const onChange = (changed) => { if (changed.fsPath === uri.fsPath) void push(); };
  watcher.onDidChange(onChange);
  watcher.onDidCreate(onChange);
  panel.onDidDispose(() => watcher.dispose());
}

function basename(uri) {
  return uri.path.split('/').pop() || 'trace.json';
}

function pageHtml() {
  // Bridge: forward trace text from the extension host into the editor iframe,
  // and tell the host once the iframe has loaded so it can send the first push.
  return `<!DOCTYPE html>
<html>
<head><meta charset="utf-8">
<style>html,body,iframe{margin:0;padding:0;border:0;width:100%;height:100vh}</style>
</head>
<body>
<iframe id="ws" src="${EDITOR_URL}" allow="clipboard-read; clipboard-write"></iframe>
<script>
  const vscode = acquireVsCodeApi();
  const frame = document.getElementById('ws');
  let pending = null;
  function send(text) {
    frame.contentWindow.postMessage({ type: 'widescope:load', text }, '${EDITOR_ORIGIN}');
  }
  let editorReady = false;
  window.addEventListener('message', (e) => {
    // From the extension host: the trace text to display.
    if (e.data && typeof e.data.text === 'string') {
      pending = e.data.text;
      if (editorReady) send(pending);
    }
    // From the editor iframe: its message listener is now attached.
    if (e.data && e.data.type === 'widescope:ready') {
      editorReady = true;
      if (pending !== null) send(pending);
    }
  });
  frame.addEventListener('load', () => vscode.postMessage({ type: 'ready' }));
</script>
</body>
</html>`;
}

module.exports = { activate, deactivate() {} };
