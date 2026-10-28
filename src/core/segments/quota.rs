use super::Segment;
use crate::config::{InputData, RateLimits, RateWindow};
use chrono::{DateTime, Local, TimeZone};

/// 订阅限额 segment，显示 5h / 7d 窗口的使用占比和重置时间
pub struct QuotaSegment {
    enabled: bool,
}

impl QuotaSegment {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

impl Segment for QuotaSegment {
    fn render(&self, input: &InputData) -> String {
        if !self.enabled {
            return String::new();
        }
        match &input.rate_limits {
            Some(limits) => render_quota(limits, Local::now()),
            None => String::new(),
        }
    }

    fn enabled(&self) -> bool {
        self.enabled
    }
}

/// 5h 显示本地重置时刻（14:12），7d 显示剩余时长（2d4h）
pub fn render_quota<Tz: TimeZone>(limits: &RateLimits, now: DateTime<Tz>) -> String
where
    Tz::Offset: std::fmt::Display,
{
    let mut parts = Vec::new();

    if let Some((pct, resets_at)) = window(&limits.five_hour) {
        let reset = resets_at
            .and_then(|ts| now.timezone().timestamp_opt(ts, 0).single())
            .map(|t| format!(" ({})", t.format("%H:%M")))
            .unwrap_or_default();
        parts.push(colorize("5h", pct, &format!("{}%{}", pct, reset)));
    }

    if let Some((pct, resets_at)) = window(&limits.seven_day) {
        let reset = resets_at
            .map(|ts| ts - now.timestamp())
            .filter(|secs| *secs > 0)
            .map(|secs| format!(" ({})", format_remaining(secs)))
            .unwrap_or_default();
        parts.push(colorize("7d", pct, &format!("{}%{}", pct, reset)));
    }

    parts.join("\x1b[90m · \x1b[0m")
}

fn window(window: &Option<RateWindow>) -> Option<(i64, Option<i64>)> {
    let window = window.as_ref()?;
    let pct = window.used_percentage?.round() as i64;
    Some((pct, window.resets_at))
}

/// 标签暗灰；数值绿色 <70%，黄色 70%~89%，红色 ≥90%
fn colorize(label: &str, pct: i64, value: &str) -> String {
    let color = match pct {
        p if p >= 90 => 203,
        p if p >= 70 => 221,
        _ => 114,
    };
    format!(
        "\x1b[90m{}\x1b[0m \x1b[38;5;{}m{}\x1b[0m",
        label, color, value
    )
}

fn format_remaining(secs: i64) -> String {
    let days = secs / 86_400;
    let hours = secs % 86_400 / 3600;
    let minutes = (secs % 3600 / 60).max(if secs < 3600 { 1 } else { 0 });
    if days > 0 {
        format!("{}d{}h", days, hours)
    } else if hours > 0 {
        format!("{}h{}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::FixedOffset;

    fn limits(five: Option<(f64, i64)>, seven: Option<(f64, i64)>) -> RateLimits {
        let w = |v: Option<(f64, i64)>| {
            v.map(|(pct, ts)| RateWindow {
                used_percentage: Some(pct),
                resets_at: Some(ts),
            })
        };
        RateLimits {
            five_hour: w(five),
            seven_day: w(seven),
        }
    }

    fn strip(s: &str) -> String {
        crate::core::statusline::strip_ansi(s)
    }

    // 2026-10-28 12:00:00 +08:00
    fn now() -> DateTime<FixedOffset> {
        FixedOffset::east_opt(8 * 3600)
            .unwrap()
            .timestamp_opt(1_793_160_000, 0)
            .unwrap()
    }

    #[test]
    fn five_hour_shows_local_reset_clock_and_seven_day_shows_remaining() {
        let now = now();
        let ts = now.timestamp();
        let out = render_quota(
            &limits(
                Some((29.6, ts + 2 * 3600 + 12 * 60)),
                Some((41.2, ts + 2 * 86_400 + 4 * 3600 + 30)),
            ),
            now,
        );
        assert_eq!(strip(&out), "5h 30% (14:12) · 7d 41% (2d4h)");
    }

    #[test]
    fn remaining_formats_by_magnitude() {
        assert_eq!(format_remaining(2 * 86_400 + 4 * 3600), "2d4h");
        assert_eq!(format_remaining(3 * 3600 + 12 * 60), "3h12m");
        assert_eq!(format_remaining(45 * 60 + 59), "45m");
        assert_eq!(format_remaining(20), "1m");
    }

    #[test]
    fn color_switches_at_70_and_90_percent() {
        let value_color = |pct| colorize("5h", pct, "x").replace("\x1b[90m5h\x1b[0m ", "");
        assert_eq!(value_color(69), "\x1b[38;5;114mx\x1b[0m");
        assert_eq!(value_color(70), "\x1b[38;5;221mx\x1b[0m");
        assert_eq!(value_color(89), "\x1b[38;5;221mx\x1b[0m");
        assert_eq!(value_color(90), "\x1b[38;5;203mx\x1b[0m");
    }

    #[test]
    fn missing_windows_and_fields_are_hidden() {
        let now = now();
        let ts = now.timestamp();
        assert_eq!(render_quota(&RateLimits::default(), now), "");
        assert_eq!(
            strip(&render_quota(&limits(None, Some((10.0, ts + 3600))), now)),
            "7d 10% (1h0m)"
        );

        let no_reset = RateLimits {
            five_hour: Some(RateWindow {
                used_percentage: Some(5.0),
                resets_at: None,
            }),
            seven_day: Some(RateWindow {
                used_percentage: None,
                resets_at: Some(ts + 3600),
            }),
        };
        assert_eq!(strip(&render_quota(&no_reset, now)), "5h 5%");

        // 已过重置时间但数据尚未刷新：不显示负数倒计时
        assert_eq!(
            strip(&render_quota(&limits(None, Some((10.0, ts - 5))), now)),
            "7d 10%"
        );
    }
}
