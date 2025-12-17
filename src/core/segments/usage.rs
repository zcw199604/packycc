use super::Segment;
use crate::config::{InputData, TranscriptEntry};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// 根据模型名称获取上下文上限
fn get_context_limit(model_name: &str) -> u32 {
    if model_name.contains("1M") {
        1_000_000
    } else {
        200_000
    }
}

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

        let context_used_token = parse_transcript_usage(&input.transcript_path);
        let context_limit = get_context_limit(&input.model.display_name);
        let context_used_rate = (context_used_token as f64 / context_limit as f64) * 100.0;

        // 格式化 token 显示（当前/总量）
        let current_display = format_token_count(context_used_token);
        let limit_display = format_token_count(context_limit);

        // 生成进度条（灰色底 + 浅绿色进度）
        let bar_width = 10;
        let filled = ((context_used_rate / 100.0) * bar_width as f64).round() as usize;
        let filled = filled.min(bar_width);
        let empty = bar_width - filled;
        // 浅绿色进度 \x1b[92m，灰色底 \x1b[90m
        let progress_bar = format!(
            "\x1b[92m{}\x1b[90m{}\x1b[0m",
            "▓".repeat(filled),
            "░".repeat(empty)
        );

        // 淡紫色的百分比和上下文大小 \x1b[95m
        format!(
            "{} \x1b[95m{:.1}% ({}/{})\x1b[0m",
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

/// 解析 transcript 文件获取最后一条记录的上下文 token
fn parse_transcript_usage<P: AsRef<Path>>(transcript_path: P) -> u32 {
    let file = match fs::File::open(&transcript_path) {
        Ok(file) => file,
        Err(_) => return 0,
    };

    let reader = BufReader::new(file);
    let lines: Vec<String> = reader
        .lines()
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_default();

    let mut last_context_tokens: u32 = 0;

    for line in lines.iter() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Ok(entry) = serde_json::from_str::<TranscriptEntry>(line) {
            if entry.r#type.as_deref() == Some("assistant") {
                if let Some(message) = &entry.message {
                    if let Some(usage) = &message.usage {
                        last_context_tokens = usage.input_tokens
                            + usage.cache_read_input_tokens
                            + usage.cache_creation_input_tokens;
                    }
                }
            }
        }
    }

    last_context_tokens
}
