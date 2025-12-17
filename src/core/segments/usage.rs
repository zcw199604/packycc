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

        let context_limit = get_context_limit(&input.model.display_name);
        let context_used_token = parse_transcript_usage(&input.transcript_path);
        let context_used_rate = (context_used_token as f64 / context_limit as f64) * 100.0;
        let tokens_display = if context_used_token >= 1000 {
            format!("{:.1}k", context_used_token as f64 / 1000.0)
        } else {
            context_used_token.to_string()
        };

        // 生成进度条
        let bar_width = 10;
        let filled = ((context_used_rate / 100.0) * bar_width as f64).round() as usize;
        let filled = filled.min(bar_width); // 确保不超过总宽度
        let empty = bar_width - filled;
        let progress_bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));

        format!("◔ {:.1}% [{}] {} tokens", context_used_rate, progress_bar, tokens_display)
    }

    fn enabled(&self) -> bool {
        self.enabled
    }
}

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

    for line in lines.iter().rev() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Ok(entry) = serde_json::from_str::<TranscriptEntry>(line) {
            if entry.r#type.as_deref() == Some("assistant") {
                if let Some(message) = &entry.message {
                    if let Some(usage) = &message.usage {
                        return usage.input_tokens
                            + usage.cache_creation_input_tokens
                            + usage.cache_read_input_tokens;
                    }
                }
            }
        }
    }

    0
}
