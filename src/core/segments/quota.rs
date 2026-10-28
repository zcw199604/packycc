use super::Segment;
use crate::config::{InputData, RateLimits, RateWindow};
use chrono::{DateTime, Datelike, Local, TimeZone};

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

struct WindowSpec {
    label: &'static str,
    length: i64,
    /// 窗口开始不足此时长时样本太少，不推算用尽时间
    min_elapsed: i64,
    /// 剩余额度按此时长均分（5h 按小时，7d 按天）
    budget_unit: i64,
    budget_label: &'static str,
    /// 时间显示为当天时刻（5h），否则为剩余时长/星期几（7d）
    same_day: bool,
}

const FIVE_HOUR: WindowSpec = WindowSpec {
    label: "5h",
    length: 5 * 3600,
    min_elapsed: 15 * 60,
    budget_unit: 3600,
    budget_label: "h",
    same_day: true,
};

const SEVEN_DAY: WindowSpec = WindowSpec {
    label: "7d",
    length: 7 * 86_400,
    min_elapsed: 12 * 3600,
    budget_unit: 86_400,
    budget_label: "天",
    same_day: false,
};

const WEEKDAYS: [&str; 7] = ["周一", "周二", "周三", "周四", "周五", "周六", "周日"];

/// 例如 `5h 30% (14:12) ≈15%/h · 7d 78% (2d4h) ≈10%/天 ⚠周四 20:43`
/// 5h 显示本地重置时刻，7d 显示剩余时长；按平均速度会提前用尽时追加 ⚠ 时刻
pub fn render_quota<Tz: TimeZone>(limits: &RateLimits, now: DateTime<Tz>) -> String
where
    Tz::Offset: std::fmt::Display,
{
    [
        (&FIVE_HOUR, &limits.five_hour),
        (&SEVEN_DAY, &limits.seven_day),
    ]
    .into_iter()
    .filter_map(|(spec, window)| render_window(spec, window.as_ref()?, &now))
    .collect::<Vec<_>>()
    .join("\x1b[90m · \x1b[0m")
}

fn render_window<Tz: TimeZone>(
    spec: &WindowSpec,
    window: &RateWindow,
    now: &DateTime<Tz>,
) -> Option<String>
where
    Tz::Offset: std::fmt::Display,
{
    let used = window.used_percentage?;
    let pct = used.round() as i64;
    let local = |ts: i64| now.timezone().timestamp_opt(ts, 0).single();
    // 已过重置时间但数据尚未刷新：不显示负数倒计时和推算
    let remaining = window
        .resets_at
        .map(|ts| ts - now.timestamp())
        .filter(|secs| *secs > 0);

    let reset = match (remaining, window.resets_at) {
        (Some(_), Some(ts)) if spec.same_day => local(ts)
            .map(|t| format!(" ({})", t.format("%H:%M")))
            .unwrap_or_default(),
        (Some(secs), _) => format!(" ({})", format_remaining(secs)),
        _ => String::new(),
    };
    let mut out = colorize(spec.label, pct, &format!("{}%{}", pct, reset));

    let Some(remaining) = remaining else {
        return Some(out);
    };
    if let Some(budget) = budget_per_unit(spec, used, remaining) {
        out.push_str(&format!(
            " \x1b[90m≈{}%/{}\x1b[0m",
            budget, spec.budget_label
        ));
    }
    if let Some(eta) =
        exhausted_in(spec, used, remaining).and_then(|secs| local(now.timestamp() + secs))
    {
        let when = if spec.same_day {
            eta.format("%H:%M").to_string()
        } else {
            let day = WEEKDAYS[eta.weekday().num_days_from_monday() as usize];
            format!("{} {}", day, eta.format("%H:%M"))
        };
        out.push_str(&format!(" \x1b[38;5;203m⚠{}\x1b[0m", when));
    }
    Some(out)
}

/// 剩余额度均分到重置前的每小时/每天；不足一个单位时与已用占比重复，不显示
fn budget_per_unit(spec: &WindowSpec, used: f64, remaining: i64) -> Option<i64> {
    let units = remaining as f64 / spec.budget_unit as f64;
    let left = 100.0 - used;
    if units < 1.0 || left <= 0.0 {
        return None;
    }
    Some((left / units).round() as i64)
}

/// 按窗口开始以来的平均速度，到 100% 还需多少秒；只在早于重置时返回
fn exhausted_in(spec: &WindowSpec, used: f64, remaining: i64) -> Option<i64> {
    let elapsed = spec.length - remaining;
    if elapsed < spec.min_elapsed || used <= 0.0 || used >= 100.0 {
        return None;
    }
    let secs = ((100.0 - used) * elapsed as f64 / used) as i64;
    (secs < remaining).then_some(secs)
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
        assert_eq!(strip(&out), "5h 30% (14:12) ≈32%/h · 7d 41% (2d4h) ≈27%/天");
    }

    #[test]
    fn warns_with_local_time_when_average_pace_exhausts_before_reset() {
        let now = now();
        let ts = now.timestamp();
        let out = render_quota(
            &limits(
                Some((72.0, ts + 2 * 3600 + 12 * 60)),
                Some((78.0, ts + 2 * 86_400 + 4 * 3600)),
            ),
            now,
        );
        assert_eq!(
            strip(&out),
            "5h 72% (14:12) ≈13%/h ⚠13:05 · 7d 78% (2d4h) ≈10%/天 ⚠周四 20:43"
        );
        assert!(out.contains("\x1b[38;5;203m⚠周四 20:43"), "{out}");
    }

    #[test]
    fn no_warning_early_in_window_or_once_exhausted() {
        let now = now();
        let ts = now.timestamp();
        // 窗口刚开始：5h 才过 10 分钟、7d 才过 1 小时，速度再快也不推算
        let early = render_quota(
            &limits(
                Some((20.0, ts + 5 * 3600 - 600)),
                Some((10.0, ts + 7 * 86_400 - 3600)),
            ),
            now,
        );
        assert_eq!(
            strip(&early),
            "5h 20% (16:50) ≈17%/h · 7d 10% (6d23h) ≈13%/天"
        );

        // 已用满：不再显示可用额度和用尽时刻
        let full = render_quota(&limits(Some((100.0, ts + 2 * 3600)), None), now);
        assert_eq!(strip(&full), "5h 100% (14:00)");
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
