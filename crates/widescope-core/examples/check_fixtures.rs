//! Smoke-check every trace fixture through the full core pipeline.
//!
//! Usage: cargo run -p widescope-core --example check_fixtures -- [dir]
//!
//! ponytail: catch_unwind instead of a real harness — on non-wasm targets any
//! `JsValue` conversion (i.e. every error path) panics, so a panic here means
//! "the core rejected or choked on this file", which is exactly what we want to
//! see per fixture instead of aborting the whole run.
use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use widescope_core::*;

fn step<T>(name: &str, failures: &mut Vec<String>, f: impl FnOnce() -> T) -> Option<T> {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(v) => Some(v),
        Err(_) => {
            failures.push(name.to_string());
            None
        }
    }
}

fn main() {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("test-fixtures"));

    let conventions = ["opentelemetry", "openinference", "langchain"]
        .iter()
        .map(|n| fs::read_to_string(format!("conventions/{n}.json")).expect("conventions"))
        .collect::<Vec<_>>()
        .join(",");
    let init_report = init(&format!("[{conventions}]")).expect("init conventions");
    let pricing_report =
        init_pricing(&fs::read_to_string("conventions/pricing.json").expect("pricing"))
            .expect("init pricing");
    println!("init: {init_report}\npricing: {pricing_report}\n");

    std::panic::set_hook(Box::new(|_| {}));

    let mut fixtures = Vec::new();
    collect_json_files(&root, &mut fixtures);
    fixtures.sort();

    let mut bad = 0usize;
    for path in &fixtures {
        let raw = fs::read_to_string(path).expect("read fixture");
        let mut failures: Vec<String> = Vec::new();

        let summary = step("parse_trace", &mut failures, || parse_trace(&raw))
            .and_then(|r| r.ok())
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());

        let Some(summary) = summary else {
            eprintln!("{}\n  FAILED: parse_trace\n", path.display());
            bad += 1;
            continue;
        };

        step("flamegraph", &mut failures, compute_flamegraph);
        step("timeline", &mut failures, compute_timeline);
        step("waterfall", &mut failures, compute_waterfall);
        step("service_graph", &mut failures, get_service_graph);
        step("critical_path", &mut failures, get_critical_path);
        step("cost_breakdown", &mut failures, get_cost_breakdown);
        step("search", &mut failures, || search_spans("a"));
        step("filter", &mut failures, || {
            filter_spans(r#"{"has_error":true}"#)
        });

        // span detail for every span the flamegraph knows about
        if let Some(Ok(flame)) = step("flamegraph_json", &mut failures, compute_flamegraph) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&flame) {
                let ids: Vec<String> = v["nodes"]
                    .as_array()
                    .map(|n| {
                        n.iter()
                            .filter_map(|x| x["span_id"].as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                let total = ids.len();
                let broken = ids
                    .iter()
                    .filter(|id| {
                        catch_unwind(AssertUnwindSafe(|| get_span_detail(id).is_err()))
                            .unwrap_or(true)
                    })
                    .count();
                if broken > 0 {
                    failures.push(format!("span_detail {broken}/{total}"));
                }
            }
        }

        step("share_roundtrip", &mut failures, || {
            let blob = compress_share(&raw);
            assert_eq!(decompress_share(&blob).unwrap(), raw);
        });

        let warnings = summary["warnings"].as_array().map(Vec::len).unwrap_or(0);
        println!(
            "{}\n  format={} spans={} services={} llm_spans={} errors={} dur={} warnings={}",
            path.display(),
            summary["detected_format"].as_str().unwrap_or("?"),
            summary["span_count"],
            summary["service_count"],
            summary["llm_span_count"],
            summary["error_count"],
            summary["total_duration_display"].as_str().unwrap_or("?"),
            warnings,
        );
        if warnings > 0 {
            for w in summary["warnings"].as_array().unwrap() {
                println!(
                    "  warn[{}]: {}",
                    w["code"].as_str().unwrap_or("?"),
                    w["message"].as_str().unwrap_or("?")
                );
            }
        }
        if !failures.is_empty() {
            println!("  FAILED: {}", failures.join(", "));
            bad += 1;
        }
        println!();
    }

    let _ = std::panic::take_hook();
    println!("{} fixtures, {} with failures", fixtures.len(), bad);
    if bad > 0 {
        std::process::exit(1);
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
