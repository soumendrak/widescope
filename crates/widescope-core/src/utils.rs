pub fn format_duration(ns: u64) -> String {
    match ns {
        0..=999 => format!("{}ns", ns),
        1_000..=999_999 => format!("{:.1}μs", ns as f64 / 1_000.0),
        1_000_000..=999_999_999 => format!("{:.1}ms", ns as f64 / 1_000_000.0),
        _ => format!("{:.2}s", ns as f64 / 1_000_000_000.0),
    }
}

/// Render an epoch-nanosecond instant as a readable UTC wall clock.
///
/// The old form printed raw epoch seconds (`1713300000.160s`), which no reader
/// can date at a glance. Formatted here rather than in JS so every surface —
/// span detail, events, exports — agrees on one representation.
///
/// Args:
///     ns: Nanoseconds since the Unix epoch.
///
/// Returns:
///     `YYYY-MM-DD HH:MM:SS.mmm UTC`.
pub fn format_timestamp_display(ns: u64) -> String {
    let secs = (ns / 1_000_000_000) as i64;
    let ms = (ns % 1_000_000_000) / 1_000_000;
    let days = secs.div_euclid(86_400);
    let tod = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03} UTC",
        y,
        m,
        d,
        tod / 3600,
        (tod % 3600) / 60,
        tod % 60,
        ms
    )
}

/// Days since 1970-01-01 to a civil (year, month, day).
///
/// Howard Hinnant's `civil_from_days`; a date crate would be a dependency for
/// fourteen lines of arithmetic.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_epoch_ns_as_utc_wall_clock() {
        assert_eq!(
            format_timestamp_display(0),
            "1970-01-01 00:00:00.000 UTC"
        );
        // 2024-04-12T05:20:00.160Z — a leap year, past the Feb/Mar boundary.
        assert_eq!(
            format_timestamp_display(1_712_899_200_160_000_000),
            "2024-04-12 05:20:00.160 UTC"
        );
        // 2000-02-29: the century leap year the naive formula gets wrong.
        assert_eq!(
            format_timestamp_display(951_782_400_000_000_000),
            "2000-02-29 00:00:00.000 UTC"
        );
    }
}
