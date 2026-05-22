import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

const WIDESCOPE_CDN = 'https://widescope.soumendrak.com';
const VIEW_TYPE = 'widescope.traceView';

function getNonce(): string {
  let text = '';
  const possible = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
  for (let i = 0; i < 64; i++) {
    text += possible.charAt(Math.floor(Math.random() * possible.length));
  }
  return text;
}

async function openTraceInPanel(traceJson: string, fileName: string): Promise<void> {
  const panel = vscode.window.createWebviewPanel(
    VIEW_TYPE,
    `WideScope: ${fileName}`,
    vscode.ViewColumn.Active,
    {
      enableScripts: true,
      retainContextWhenHidden: true,
      localResourceRoots: [],
    }
  );

  panel.iconPath = vscode.Uri.joinPath(panel.webview.asWebviewUri(vscode.Uri.file(path.join(__dirname, '..'))), 'icons', 'icon.png').fsPath;

  const nonce = getNonce();

  // Escape trace JSON for safe embedding in HTML
  const escapedJson = traceJson
    .replace(/\\/g, '\\\\')
    .replace(/`/g, '\\`')
    .replace(/\$/g, '\\$');

  panel.webview.html = getWebviewContent(escapedJson, fileName, nonce);
}

function getWebviewContent(traceJson: string, fileName: string, nonce: string): string {
  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <meta http-equiv="Content-Security-Policy" content="default-src 'self' https://widescope.soumendrak.com; script-src 'self' 'wasm-unsafe-eval' 'unsafe-eval' https://widescope.soumendrak.com; style-src 'self' 'unsafe-inline' https://widescope.soumendrak.com; connect-src 'self' https:; img-src 'self' data: blob:; font-src 'self';">
  <title>WideScope — ${fileName}</title>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body, html { height: 100%; background: #0f172a; color: #f1f5f9; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; }
    .loading { display: flex; align-items: center; justify-content: center; height: 100%; flex-direction: column; gap: 1rem; }
    .loading-logo { font-size: 3rem; }
    .loading-text { font-size: 1rem; opacity: 0.7; }
    .error { color: #f87171; padding: 2rem; text-align: center; }
    iframe { border: none; width: 100%; height: 100%; }
  </style>
</head>
<body>
  <div id="loading" class="loading">
    <div class="loading-logo">🔭</div>
    <div class="loading-text">Loading WideScope…</div>
  </div>
  <iframe id="widescope-frame" style="display:none" allow="clipboard-write"></iframe>
  <script nonce="${nonce}">
    const traceJson = \`${traceJson}\`;
    const fileName = ${JSON.stringify(fileName)};
    const cdnUrl = ${JSON.stringify(WIDESCOPE_CDN)};

    // Try loading from CDN with the trace passed via URL fragment
    const iframe = document.getElementById('widescope-frame');
    const loading = document.getElementById('loading');

    // Construct the URL with the trace embedded as a compressed fragment
    // Fallback: pass trace as a window message to the iframe
    const targetUrl = cdnUrl + '?embed=1';

    iframe.src = targetUrl;
    iframe.onload = () => {
      try {
        // Post the trace to the WideScope iframe
        iframe.contentWindow.postMessage({
          type: 'widescope:load-trace',
          trace: traceJson,
          fileName: fileName
        }, cdnUrl);
        loading.style.display = 'none';
        iframe.style.display = 'block';
      } catch (e) {
        // If CDN unavailable, show raw JSON viewer
        showFallback(traceJson);
      }
    };

    iframe.onerror = () => {
      showFallback(traceJson);
    };

    function showFallback(json) {
      loading.innerHTML = '<div class="loading-logo">🔭</div><div class="loading-text" style="color:#fbbf24">WideScope CDN unavailable. Below is the raw trace:</div><pre style="max-height:calc(100vh-160px);overflow:auto;padding:1rem;font-size:0.75rem;line-height:1.4;white-space:pre-wrap;word-break:break-all;background:#1e293b;border-radius:8px;margin:0 2rem">' + json.replace(/</g,'&lt;').replace(/>/g,'&gt;') + '</pre>';
    }

    // Listen for messages from the iframe
    window.addEventListener('message', (event) => {
      if (event.origin !== new URL(cdnUrl).origin) return;

      if (event.data.type === 'widescope:ready') {
        // WideScope is loaded and ready
        loading.style.display = 'none';
        iframe.style.display = 'block';
      }
    });
  </script>
</body>
</html>`;
}

export function activate(context: vscode.ExtensionContext): void {
  console.log('[WideScope] Extension activated');

  // Command: View trace file from explorer right-click
  const viewTraceCmd = vscode.commands.registerCommand(
    'widescope.viewTrace',
    async (uri: vscode.Uri) => {
      if (!uri) {
        vscode.window.showErrorMessage('WideScope: No file selected.');
        return;
      }

      try {
        const raw = await fs.promises.readFile(uri.fsPath, 'utf-8');
        // Quick validation: check it's valid JSON
        JSON.parse(raw);
        const fileName = path.basename(uri.fsPath);
        await openTraceInPanel(raw, fileName);
      } catch (err) {
        if (err instanceof SyntaxError) {
          vscode.window.showErrorMessage(`WideScope: File is not valid JSON — ${err.message}`);
        } else {
          vscode.window.showErrorMessage(`WideScope: Failed to read file — ${String(err)}`);
        }
      }
    }
  );

  // Command: View trace from active editor
  const viewFromEditorCmd = vscode.commands.registerCommand(
    'widescope.viewFromEditor',
    async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) {
        vscode.window.showErrorMessage('WideScope: No active editor.');
        return;
      }

      const document = editor.document;
      if (document.languageId !== 'json') {
        vscode.window.showErrorMessage('WideScope: Active file must be JSON.');
        return;
      }

      try {
        JSON.parse(document.getText());
        const fileName = path.basename(document.fileName);
        await openTraceInPanel(document.getText(), fileName);
      } catch (err) {
        if (err instanceof SyntaxError) {
          vscode.window.showErrorMessage(`WideScope: Invalid JSON — ${err.message}`);
        } else {
          vscode.window.showErrorMessage(`WideScope: Error — ${String(err)}`);
        }
      }
    }
  );

  context.subscriptions.push(viewTraceCmd, viewFromEditorCmd);
}

export function deactivate(): void {
  console.log('[WideScope] Extension deactivated');
}
