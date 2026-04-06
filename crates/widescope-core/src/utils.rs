pub fn format_duration(ns: u64) -> String {
    match ns {
        0..=999 => format!("{}ns", ns),
        1_000..=999_999 => format!("{:.1}μs", ns as f64 / 1_000.0),
        1_000_000..=999_999_999 => format!("{:.1}ms", ns as f64 / 1_000_000.0),
        _ => format!("{:.2}s", ns as f64 / 1_000_000_000.0),
    }
}

pub fn format_timestamp_display(ns: u64) -> String {
    let secs = ns / 1_000_000_000;
    let ms = (ns % 1_000_000_000) / 1_000_000;
    format!("{}.{:03}s", secs, ms)
}
