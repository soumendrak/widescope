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
    Compare { baseline: PathBuf, candidate: PathBuf },
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
                Format::Text => print_summary(&summary),
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Compare { baseline, candidate } => {
            let base = parse(&baseline)?;
            let cand = parse(&candidate)?;
            print_comparison(&base, &cand);
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
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("reading {}: {e}", path.display()))?;
    let json = core::parse_trace(&raw).map_err(|e| e.to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

fn print_summary(s: &Value) {
    let g = |k: &str| s.get(k).cloned().unwrap_or(Value::Null);
    println!("Trace ID:    {}", str_of(&g("trace_id")));
    println!("Format:      {}", str_of(&g("detected_format")));
    println!("Spans:       {}", g("span_count"));
    println!("Services:    {}", g("service_count"));
    println!("LLM spans:   {}", g("llm_span_count"));
    println!("Errors:      {}", g("error_count"));
    println!("Duration:    {}", str_of(&g("total_duration_display")));
    println!("Latency p50: {}", str_of(&g("latency_p50_display")));
    println!("Latency p95: {}", str_of(&g("latency_p95_display")));
    if let Some(op) = g("root_operation").as_str() {
        println!("Root:        {} ({})", op, str_of(&g("root_service")));
    }
    let warnings = g("warnings");
    if let Some(w) = warnings.as_array() {
        if !w.is_empty() {
            println!("Warnings:    {}", w.len());
        }
    }
}

fn print_comparison(base: &Value, cand: &Value) {
    println!("{:<14} {:>16} {:>16} {:>12}", "Metric", "Baseline", "Candidate", "Delta");
    for (label, key) in [
        ("Duration (ns)", "total_duration_ns"),
        ("Spans", "span_count"),
        ("Errors", "error_count"),
        ("LLM spans", "llm_span_count"),
    ] {
        let b = num(base, key);
        let c = num(cand, key);
        let delta = c - b;
        let pct = if b != 0.0 { delta / b * 100.0 } else { 0.0 };
        println!(
            "{label:<14} {b:>16} {c:>16} {:>+11.1}%",
            pct
        );
    }
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
    Ok(if failed { ExitCode::FAILURE } else { ExitCode::SUCCESS })
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
