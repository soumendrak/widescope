use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use widescope_core::{compute_flamegraph, compute_timeline, compute_waterfall, parse_trace};

fn main() {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("test-fixtures"));

    let mut fixtures = Vec::new();
    collect_json_files(&root, &mut fixtures);
    fixtures.sort();

    if fixtures.is_empty() {
        eprintln!("No JSON fixtures found under {}", root.display());
        std::process::exit(1);
    }

    println!("fixture,size_mb,parse_ms,flame_ms,timeline_ms,waterfall_ms,spans");

    for path in fixtures {
        let raw = match fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(err) => {
                eprintln!("{}: failed to read fixture: {err}", path.display());
                continue;
            }
        };

        let parse_started = Instant::now();
        let summary_json = match parse_trace(&raw) {
            Ok(json) => json,
            Err(err) => {
                eprintln!("{}: failed to parse fixture: {err:?}", path.display());
                continue;
            }
        };
        let parse_ms = parse_started.elapsed().as_secs_f64() * 1000.0;

        let flame_started = Instant::now();
        let _ = compute_flamegraph();
        let flame_ms = flame_started.elapsed().as_secs_f64() * 1000.0;

        let timeline_started = Instant::now();
        let _ = compute_timeline();
        let timeline_ms = timeline_started.elapsed().as_secs_f64() * 1000.0;

        let waterfall_started = Instant::now();
        let _ = compute_waterfall();
        let waterfall_ms = waterfall_started.elapsed().as_secs_f64() * 1000.0;

        let span_count = serde_json::from_str::<serde_json::Value>(&summary_json)
            .ok()
            .and_then(|value| value.get("span_count").and_then(|count| count.as_u64()))
            .unwrap_or(0);

        println!(
            "{},{:.2},{:.2},{:.2},{:.2},{:.2},{}",
            path.display(),
            raw.len() as f64 / 1024.0 / 1024.0,
            parse_ms,
            flame_ms,
            timeline_ms,
            waterfall_ms,
            span_count
        );
    }
}

fn collect_json_files(path: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "json") {
            out.push(path);
        }
    }
}
