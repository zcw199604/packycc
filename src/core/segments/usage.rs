use super::Segment;
use crate::config::InputData;

/// 上下文使用率 segment，显示当前上下文使用情况
pub struct UsageSegment {
    enabled: bool,
}

impl UsageSegment {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

impl Segment for UsageSegment {
    fn render(&self, input: &InputData) -> String {
        if !self.enabled {
            return String::new();
        }

        // 直接从 context_window 获取数据
        let context_window = match &input.context_window {
            Some(cw) => cw,
            None => return String::new(),
        };

        let context_limit = context_window.context_window_size.unwrap_or(200_000);
        let context_used = match &context_window.current_usage {
            Some(usage) => {
                usage.input_tokens
                    + usage.cache_creation_input_tokens
                    + usage.cache_read_input_tokens
            }
            None => return String::new(),
        };

        let context_used_rate = (context_used as f64 / context_limit as f64) * 100.0;

        // 格式化 token 显示（当前/总量）
        let current_display = format_token_count(context_used);
        let limit_display = format_token_count(context_limit);

        // 生成进度条（薰衣草色进度 + 深灰色底）
        let bar_width = 10;
        let filled = ((context_used_rate / 100.0) * bar_width as f64).round() as usize;
        let filled = filled.min(bar_width);
        let empty = bar_width - filled;
        // 薰衣草色进度 \x1b[38;5;147m，深灰色底 \x1b[90m
        let progress_bar = format!(
            "\x1b[38;5;147m{}\x1b[90m{}\x1b[0m",
            "▓".repeat(filled),
            "░".repeat(empty)
        );

        // 薰衣草色的百分比和上下文大小 \x1b[38;5;147m
        format!(
            "{} \x1b[38;5;147m{:.1}% ({}/{})\x1b[0m",
            progress_bar, context_used_rate, current_display, limit_display
        )
    }

    fn enabled(&self) -> bool {
        self.enabled
    }
}

/// 格式化 token 数量显示
fn format_token_count(tokens: u32) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1000 {
        format!("{:.1}K", tokens as f64 / 1000.0)
    } else {
        tokens.to_string()
    }
}
