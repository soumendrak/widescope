// WideScope desktop shell — a thin Tauri wrapper around the existing
// Svelte + WASM UI. The web UI already parses traces in WASM; the only thing a
// desktop build adds over the browser is reading files off local disk (open
// dialog + file associations). So this shell does exactly that: read a trace
// file in Rust, hand its text to the webview, and let the unchanged WASM
// pipeline render it.
//
// ponytail: no widescope-core dep and no native parse command — WASM parses
// fine inside the webview. Add native parsing / memmap (issue #34 Phase 3)
// only when a real multi-GB trace actually hits the browser memory ceiling.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::Path;
use std::sync::Mutex;

use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::DialogExt;

/// Trace file contents read before the webview is ready to receive events.
/// The frontend drains this on mount via the `drain_pending` command.
#[derive(Default)]
struct Pending(Mutex<Vec<String>>);

/// Event the frontend listens on for traces opened while the app is running.
const OPEN_TRACE_EVENT: &str = "open-trace";

fn read_trace(path: &Path) -> Option<String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(e) => {
            eprintln!("widescope: failed to read {}: {e}", path.display());
            None
        }
    }
}

/// Read a trace file and deliver it to the UI two ways: emit to an
/// already-loaded webview, and stash it so a not-yet-ready webview picks it up
/// on mount. The frontend de-dupes naturally — a launch file is only ever in
/// one of the two channels at mount time.
fn deliver_file(app: &tauri::AppHandle, path: &Path) {
    let Some(text) = read_trace(path) else { return };
    app.state::<Pending>().0.lock().unwrap().push(text.clone());
    let _ = app.emit(OPEN_TRACE_EVENT, text);
}

/// Return and clear any trace files queued before the UI was ready.
#[tauri::command]
fn drain_pending(state: tauri::State<Pending>) -> Vec<String> {
    std::mem::take(&mut *state.0.lock().unwrap())
}

fn open_file_dialog(app: &tauri::AppHandle) {
    let handle = app.clone();
    app.dialog()
        .file()
        .add_filter("Trace files", &["json", "trace"])
        .pick_file(move |picked| {
            if let Some(path) = picked.and_then(|fp| fp.into_path().ok()) {
                deliver_file(&handle, &path);
            }
        });
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Pending::default())
        .invoke_handler(tauri::generate_handler![drain_pending])
        .setup(|app| {
            // Native menu: File → Open Trace (⌘/Ctrl+O), plus a standard Quit.
            let open = MenuItemBuilder::new("Open Trace…")
                .id("open")
                .accelerator("CmdOrCtrl+O")
                .build(app)?;
            let file = SubmenuBuilder::new(app, "File")
                .item(&open)
                .separator()
                .quit()
                .build()?;
            let menu = MenuBuilder::new(app).item(&file).build()?;
            app.set_menu(menu)?;

            // Launch arg: `widescope trace.json`, or Windows/Linux "Open with".
            // (macOS delivers Finder double-clicks via RunEvent::Opened below.)
            if let Some(arg) = std::env::args().nth(1) {
                let path = std::path::PathBuf::from(arg);
                if path.is_file() {
                    if let Some(text) = read_trace(&path) {
                        app.state::<Pending>().0.lock().unwrap().push(text);
                    }
                }
            }
            Ok(())
        })
        .on_menu_event(|app, event| {
            if event.id() == "open" {
                open_file_dialog(app);
            }
        })
        .build(tauri::generate_context!())
        .expect("failed to build WideScope desktop app")
        .run(|_app, _event| {
            // macOS/iOS file association: Finder "Open with WideScope" on a
            // running instance arrives here as file:// URLs. `RunEvent::Opened`
            // only exists on Apple targets, so the arm is cfg-gated.
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            if let tauri::RunEvent::Opened { urls } = _event {
                for url in urls {
                    if let Ok(path) = url.to_file_path() {
                        deliver_file(_app, &path);
                    }
                }
            }
        });
}
