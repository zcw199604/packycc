use super::Segment;
use crate::config::{InputData, TranscriptEntry};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Token 使用信息
struct TokenUsage {
    input_tokens: u32,
    cache_read_tokens: u32,
    cache_write_tokens: u32,
    output_tokens: u32,
}

impl TokenUsage {
    /// 计算总输入 token（用于上下文窗口计算）
    fn total_input(&self) -> u32 {
        self.input_tokens + self.cache_read_tokens + self.cache_write_tokens
    }
}

/// 模型定价（每百万 token）
struct ModelPricing {
    input: f64,
    output: f64,
    cache_read: f64,
    cache_write: f64,
}

/// 根据模型名称获取上下文上限
fn get_context_limit(model_name: &str) -> u32 {
    if model_name.contains("1M") {
        1_000_000
    } else {
        200_000
    }
}

/// 根据模型名称和 token 使用量获取定价
fn get_model_pricing(model_name: &str, total_input_tokens: u32) -> ModelPricing {
    if model_name.contains("opus") || model_name.contains("Opus") {
        // Opus 4.5
        ModelPricing {
            input: 5.0,
            output: 25.0,
            cache_read: 0.5,
            cache_write: 6.25,
        }
    } else if model_name.contains("haiku") || model_name.contains("Haiku") {
        // Haiku
        ModelPricing {
            input: 0.25,
            output: 1.25,
            cache_read: 0.03,
            cache_write: 0.30,
        }
    } else if model_name.contains("1M") && total_input_tokens > 200_000 {
        // Sonnet 4.5 1M (>200K 价格翻倍)
        ModelPricing {
            input: 6.0,
            output: 22.5,
            cache_read: 0.60,
            cache_write: 7.50,
        }
    } else {
        // Sonnet 4.5 默认 (≤200K)
        ModelPricing {
            input: 3.0,
            output: 15.0,
            cache_read: 0.30,
            cache_write: 3.75,
        }
    }
}

/// 计算总费用
fn calculate_cost(usage: &TokenUsage, pricing: &ModelPricing) -> f64 {
    let million = 1_000_000.0;
    (usage.input_tokens as f64 / million) * pricing.input
        + (usage.cache_read_tokens as f64 / million) * pricing.cache_read
        + (usage.cache_write_tokens as f64 / million) * pricing.cache_write
        + (usage.output_tokens as f64 / million) * pricing.output
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

        let usage = parse_transcript_usage(&input.transcript_path);
        let context_limit = get_context_limit(&input.model.display_name);
        let context_used_token = usage.total_input();
        let context_used_rate = (context_used_token as f64 / context_limit as f64) * 100.0;

        // 格式化 token 显示
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

        // 计算费用
        let pricing = get_model_pricing(&input.model.display_name, context_used_token);
        let cost = calculate_cost(&usage, &pricing);

        format!(
            "◔ {:.1}% [{}] {} tokens · ${:.2}",
            context_used_rate, progress_bar, tokens_display, cost
        )
    }

    fn enabled(&self) -> bool {
        self.enabled
    }
}

/// 解析 transcript 文件获取 token 使用信息
/// 累加所有 assistant 消息的 token（代表整个会话的实际费用）
fn parse_transcript_usage<P: AsRef<Path>>(transcript_path: P) -> TokenUsage {
    let file = match fs::File::open(&transcript_path) {
        Ok(file) => file,
        Err(_) => {
            return TokenUsage {
                input_tokens: 0,
                cache_read_tokens: 0,
                cache_write_tokens: 0,
                output_tokens: 0,
            }
        }
    };

    let reader = BufReader::new(file);
    let lines: Vec<String> = reader
        .lines()
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_default();

    // 累加所有 token
    let mut total_input_tokens: u32 = 0;
    let mut total_cache_read_tokens: u32 = 0;
    let mut total_cache_write_tokens: u32 = 0;
    let mut total_output_tokens: u32 = 0;

    for line in lines.iter() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Ok(entry) = serde_json::from_str::<TranscriptEntry>(line) {
            if entry.r#type.as_deref() == Some("assistant") {
                if let Some(message) = &entry.message {
                    if let Some(usage) = &message.usage {
                        total_input_tokens += usage.input_tokens;
                        total_cache_read_tokens += usage.cache_read_input_tokens;
                        total_cache_write_tokens += usage.cache_creation_input_tokens;
                        total_output_tokens += usage.output_tokens;
                    }
                }
            }
        }
    }

    TokenUsage {
        input_tokens: total_input_tokens,
        cache_read_tokens: total_cache_read_tokens,
        cache_write_tokens: total_cache_write_tokens,
        output_tokens: total_output_tokens,
    }
}
