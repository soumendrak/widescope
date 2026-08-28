//! Headless CLI over `widescope-core`. The core's `#[wasm_bindgen]` functions
//! compile and run natively (only JS interop is wasm-only), so we call them
//! directly and read their JSON results — no separate native API needed.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde_json::Value;
use widescope_core as core;

// Same convention/pricing bundles the browser UI loads (see conventions-bundle.ts).
const OTEL: &str = include_str!("../../../conventions/opentelemetry.json");
const OPENINFERENCE: &str = include_str!("../../../conventions/openinference.json");
const LANGCHAIN: &str = include_str!("../../../conventions/langchain.json");
const PRICING: &str = include_str!("../../../conventions/pricing.json");

#[derive(Parser)]
#[command(name = "widescope", about = "Headless WideScope trace analysis")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse + analyze a trace, print a summary.
    Analyze {
        trace: PathBuf,
        /// Output format: text (default) or json.
        #[arg(long, default_value = "text")]
        format: Format,
    },
    /// Diff two traces and report metric regressions.
    Compare {
        baseline: PathBuf,
        candidate: PathBuf,
    },
    /// Check a trace against budgets; non-zero exit on breach.
    /// Budgets: duration=30s, errors=0, cost=0.50 (repeatable).
    Check {
        trace: PathBuf,
        #[arg(long = "budget", value_name = "KEY=VALUE")]
        budgets: Vec<String>,
    },
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum Format {
    Text,
    Json,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli.command) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cmd: Command) -> Result<ExitCode, String> {
    init_core()?;
    match cmd {
        Command::Analyze { trace, format } => {
            let summary = parse(&trace)?;
            match format {
                Format::Json => println!("{}", serde_json::to_string_pretty(&summary).unwrap()),
                Format::Text => print!("{}", format_summary(&summary)),
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Compare {
            baseline,
            candidate,
        } => {
            let base = parse(&baseline)?;
            let cand = parse(&candidate)?;
            print!("{}", format_comparison(&base, &cand));
            Ok(ExitCode::SUCCESS)
        }
        Command::Check { trace, budgets } => {
            let summary = parse(&trace)?;
            check_budgets(&summary, &budgets)
        }
    }
}

/// Load conventions + pricing into core's thread-local state (mirrors the UI's
/// `loadWasm`). Pricing failure is non-fatal, matching the UI.
fn init_core() -> Result<(), String> {
    let merged = format!("[{OTEL},{OPENINFERENCE},{LANGCHAIN}]");
    core::init(&merged).map_err(|e| e.to_string())?;
    let _ = core::init_pricing(PRICING);
    Ok(())
}

fn parse(path: &PathBuf) -> Result<Value, String> {
    let raw =
        std::fs::read_to_string(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    let json = core::parse_trace(&raw).map_err(|e| e.to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

fn format_summary(s: &Value) -> String {
    use std::fmt::Write as _;
    let g = |k: &str| s.get(k).cloned().unwrap_or(Value::Null);
    let mut out = String::new();
    let _ = writeln!(out, "Trace ID:    {}", str_of(&g("trace_id")));
    let _ = writeln!(out, "Format:      {}", str_of(&g("detected_format")));
    let _ = writeln!(out, "Spans:       {}", g("span_count"));
    let _ = writeln!(out, "Services:    {}", g("service_count"));
    let _ = writeln!(out, "LLM spans:   {}", g("llm_span_count"));
    let _ = writeln!(out, "Errors:      {}", g("error_count"));
    let _ = writeln!(out, "Duration:    {}", str_of(&g("total_duration_display")));
    let _ = writeln!(out, "Latency p50: {}", str_of(&g("latency_p50_display")));
    let _ = writeln!(out, "Latency p95: {}", str_of(&g("latency_p95_display")));
    if let Some(op) = g("root_operation").as_str() {
        let _ = writeln!(out, "Root:        {} ({})", op, str_of(&g("root_service")));
    }
    if let Some(w) = g("warnings").as_array() {
        if !w.is_empty() {
            let _ = writeln!(out, "Warnings:    {}", w.len());
        }
    }
    out
}

fn format_comparison(base: &Value, cand: &Value) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{:<14} {:>16} {:>16} {:>12}",
        "Metric", "Baseline", "Candidate", "Delta"
    );
    for (label, key) in [
        ("Duration (ns)", "total_duration_ns"),
        ("Spans", "span_count"),
        ("Errors", "error_count"),
        ("LLM spans", "llm_span_count"),
    ] {
        let b = num(base, key);
        let c = num(cand, key);
        // A zero baseline has no meaningful percentage to report.
        let pct = if b != 0.0 { (c - b) / b * 100.0 } else { 0.0 };
        let _ = writeln!(out, "{label:<14} {b:>16} {c:>16} {pct:>+11.1}%");
    }
    out
}

/// Returns FAILURE if any budget is breached.
fn check_budgets(s: &Value, budgets: &[String]) -> Result<ExitCode, String> {
    let mut failed = false;
    for budget in budgets {
        let (key, val) = budget
            .split_once('=')
            .ok_or_else(|| format!("budget must be KEY=VALUE: {budget}"))?;
        let (actual, limit, unit) = match key {
            "duration" => (
                num(s, "total_duration_ns"),
                parse_duration_ns(val)? as f64,
                "ns",
            ),
            "errors" => (num(s, "error_count"), parse_f64(val)?, ""),
            "spans" => (num(s, "span_count"), parse_f64(val)?, ""),
            other => return Err(format!("unknown budget key: {other}")),
        };
        let ok = actual <= limit;
        let mark = if ok { "PASS" } else { "FAIL" };
        println!("[{mark}] {key}: {actual}{unit} <= {limit}{unit}");
        failed |= !ok;
    }
    Ok(if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

fn num(v: &Value, key: &str) -> f64 {
    v.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}

fn str_of(v: &Value) -> String {
    v.as_str().unwrap_or("-").to_string()
}

fn parse_f64(s: &str) -> Result<f64, String> {
    s.trim().parse().map_err(|_| format!("not a number: {s}"))
}

/// Parse "30s" / "500ms" / "100us" / "1000" (bare = ns) into nanoseconds.
fn parse_duration_ns(s: &str) -> Result<u64, String> {
    let s = s.trim();
    let (num, mult) = if let Some(n) = s.strip_suffix("ms") {
        (n, 1_000_000.0)
    } else if let Some(n) = s.strip_suffix("us").or_else(|| s.strip_suffix("µs")) {
        (n, 1_000.0)
    } else if let Some(n) = s.strip_suffix('s') {
        (n, 1_000_000_000.0)
    } else {
        (s, 1.0)
    };
    num.trim()
        .parse::<f64>()
        .map(|v| (v * mult) as u64)
        .map_err(|_| format!("invalid duration: {s}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_parsing() {
        assert_eq!(parse_duration_ns("30s").unwrap(), 30_000_000_000);
        assert_eq!(parse_duration_ns("500ms").unwrap(), 500_000_000);
        assert_eq!(parse_duration_ns("100us").unwrap(), 100_000);
        assert_eq!(parse_duration_ns("1000").unwrap(), 1000);
        assert!(parse_duration_ns("abc").is_err());
    }
}

/// The CLI surface: every subcommand, both output formats, and the exit codes
/// CI depends on.
#[cfg(test)]
mod cli_tests {
    use super::*;
    use std::io::Write;

    const OTLP: &str = "../../test-fixtures/otlp/sample_llm_pipeline.json";
    const JAEGER: &str = "../../test-fixtures/jaeger/sample_llm_pipeline.json";

    fn fixture(rel: &str) -> PathBuf {
        // Cargo runs tests from the crate root; the fixtures live at the repo root.
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
    }

    /// Write a throwaway file and hand back its path, for the I/O error paths.
    fn temp_file(name: &str, contents: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("widescope-cli-{name}"));
        let mut file = std::fs::File::create(&path).expect("temp file");
        file.write_all(contents.as_bytes())
            .expect("write temp file");
        path
    }

    fn is_success(code: ExitCode) -> bool {
        format!("{code:?}") == format!("{:?}", ExitCode::SUCCESS)
    }

    #[test]
    fn analyze_prints_a_summary_in_both_formats() {
        for format in [Format::Text, Format::Json] {
            let code = run(Command::Analyze {
                trace: fixture(OTLP),
                format,
            })
            .expect("analyze should succeed");
            assert!(is_success(code));
        }
    }

    #[test]
    fn the_text_summary_names_every_headline_metric() {
        init_core().unwrap();
        let summary = parse(&fixture(OTLP)).unwrap();
        let text = format_summary(&summary);
        for expected in [
            "Trace ID:",
            "Format:",
            "Spans:",
            "Services:",
            "LLM spans:",
            "Errors:",
            "Duration:",
            "Latency p50:",
            "Latency p95:",
            "Root:",
        ] {
            assert!(text.contains(expected), "missing {expected} in:\n{text}");
        }
        assert!(text.contains("OtlpJson"));
        assert!(text.contains("Spans:       7"));
    }

    #[test]
    fn a_summary_missing_optional_fields_still_renders() {
        // No root operation and no warnings: both blocks are skipped.
        let bare = serde_json::json!({ "span_count": 1 });
        let text = format_summary(&bare);
        assert!(text.contains("Trace ID:    -"));
        assert!(!text.contains("Root:"));
        assert!(!text.contains("Warnings:"));

        let warned = serde_json::json!({
            "trace_id": "t",
            "root_operation": "GET /",
            "root_service": "api",
            "warnings": [{ "code": "X" }],
        });
        let text = format_summary(&warned);
        assert!(text.contains("Root:        GET / (api)"));
        assert!(text.contains("Warnings:    1"));
    }

    #[test]
    fn compare_reports_a_delta_per_metric() {
        let code = run(Command::Compare {
            baseline: fixture(OTLP),
            candidate: fixture(JAEGER),
        })
        .expect("compare should succeed");
        assert!(is_success(code));

        let base = serde_json::json!({"total_duration_ns": 100, "span_count": 4});
        let cand = serde_json::json!({"total_duration_ns": 150, "span_count": 2});
        let table = format_comparison(&base, &cand);
        assert!(table.contains("Duration (ns)"));
        assert!(table.contains("+50.0%"), "{table}");
        assert!(table.contains("-50.0%"), "{table}");
        // A zero baseline reports 0% rather than infinity or NaN.
        let table = format_comparison(
            &serde_json::json!({}),
            &serde_json::json!({"span_count": 9}),
        );
        assert!(table.contains("+0.0%"), "{table}");
    }

    #[test]
    fn check_passes_when_every_budget_holds() {
        let code = run(Command::Check {
            trace: fixture(OTLP),
            budgets: vec!["duration=30s".into(), "errors=0".into(), "spans=100".into()],
        })
        .expect("check should run");
        assert!(is_success(code));
    }

    #[test]
    fn check_fails_the_build_when_a_budget_is_breached() {
        let code = run(Command::Check {
            trace: fixture(OTLP),
            budgets: vec!["duration=1ms".into()],
        })
        .expect("check should run");
        assert!(!is_success(code), "a breached budget must exit non-zero");
    }

    #[test]
    fn check_rejects_malformed_budget_arguments() {
        init_core().unwrap();
        let summary = parse(&fixture(OTLP)).unwrap();

        let err = check_budgets(&summary, &["duration".into()]).unwrap_err();
        assert!(err.contains("KEY=VALUE"), "{err}");

        let err = check_budgets(&summary, &["latency=5s".into()]).unwrap_err();
        assert!(err.contains("unknown budget key"), "{err}");

        let err = check_budgets(&summary, &["errors=many".into()]).unwrap_err();
        assert!(err.contains("not a number"), "{err}");

        let err = check_budgets(&summary, &["duration=soon".into()]).unwrap_err();
        assert!(err.contains("invalid duration"), "{err}");
    }

    #[test]
    fn a_missing_file_is_reported_with_its_path() {
        let err = run(Command::Analyze {
            trace: PathBuf::from("/definitely/not/here.json"),
            format: Format::Text,
        })
        .unwrap_err();
        assert!(err.contains("/definitely/not/here.json"), "{err}");
    }

    #[test]
    fn an_unparseable_trace_is_reported_rather_than_panicking() {
        let path = temp_file("invalid.json", "{ not json");
        let err = run(Command::Analyze {
            trace: path.clone(),
            format: Format::Text,
        })
        .unwrap_err();
        assert!(!err.is_empty());
        let _ = std::fs::remove_file(path);

        let path = temp_file("unknown-format.json", r#"{"hello":"world"}"#);
        assert!(run(Command::Analyze {
            trace: path.clone(),
            format: Format::Text
        })
        .is_err());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn numeric_helpers_default_rather_than_fail() {
        let v = serde_json::json!({"n": 5, "s": "text"});
        assert_eq!(num(&v, "n"), 5.0);
        assert_eq!(num(&v, "missing"), 0.0);
        assert_eq!(num(&v, "s"), 0.0);
        assert_eq!(str_of(&v["s"]), "text");
        assert_eq!(str_of(&v["n"]), "-");
        assert_eq!(parse_f64(" 2.5 ").unwrap(), 2.5);
        assert!(parse_f64("two").is_err());
    }

    #[test]
    fn durations_accept_every_documented_unit() {
        assert_eq!(parse_duration_ns("30s").unwrap(), 30_000_000_000);
        assert_eq!(parse_duration_ns("500ms").unwrap(), 500_000_000);
        assert_eq!(parse_duration_ns("100us").unwrap(), 100_000);
        assert_eq!(parse_duration_ns("100µs").unwrap(), 100_000);
        assert_eq!(parse_duration_ns(" 1000 ").unwrap(), 1000);
        assert!(parse_duration_ns("abc").is_err());
        assert!(parse_duration_ns("").is_err());
    }
}
