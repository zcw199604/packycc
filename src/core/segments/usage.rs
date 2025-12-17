use super::Segment;
use crate::config::{InputData, TranscriptEntry};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Token 使用信息
struct TokenUsage {
    // 用于费用计算（累加值）
    total_input_tokens: u32,
    total_cache_read_tokens: u32,
    total_cache_write_tokens: u32,
    total_output_tokens: u32,
    // 用于上下文使用率（最后一条记录）
    last_context_tokens: u32,
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
    (usage.total_input_tokens as f64 / million) * pricing.input
        + (usage.total_cache_read_tokens as f64 / million) * pricing.cache_read
        + (usage.total_cache_write_tokens as f64 / million) * pricing.cache_write
        + (usage.total_output_tokens as f64 / million) * pricing.output
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
        // 使用最后一条记录的上下文 token 计算使用率
        let context_used_token = usage.last_context_tokens;
        let context_used_rate = (context_used_token as f64 / context_limit as f64) * 100.0;

        // 格式化 token 显示（当前/总量）
        let current_display = format_token_count(context_used_token);
        let limit_display = format_token_count(context_limit);

        // 生成进度条 🟩🟩⬜⬜⬜⬜⬜⬜⬜⬜
        let bar_width = 10;
        let filled = ((context_used_rate / 100.0) * bar_width as f64).round() as usize;
        let filled = filled.min(bar_width); // 确保不超过总宽度
        let empty = bar_width - filled;
        let progress_bar = format!("{}{}", "🟩".repeat(filled), "⬜".repeat(empty));

        // 计算费用（使用累加的 token）
        let pricing = get_model_pricing(&input.model.display_name, context_used_token);
        let cost = calculate_cost(&usage, &pricing);

        format!(
            "{} {:.1}% ({}/{}) | ${:.2}",
            progress_bar, context_used_rate, current_display, limit_display, cost
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

/// 解析 transcript 文件获取 token 使用信息
/// - 累加所有 token 用于费用计算
/// - 最后一条记录的上下文 token 用于使用率计算
fn parse_transcript_usage<P: AsRef<Path>>(transcript_path: P) -> TokenUsage {
    let file = match fs::File::open(&transcript_path) {
        Ok(file) => file,
        Err(_) => {
            return TokenUsage {
                total_input_tokens: 0,
                total_cache_read_tokens: 0,
                total_cache_write_tokens: 0,
                total_output_tokens: 0,
                last_context_tokens: 0,
            }
        }
    };

    let reader = BufReader::new(file);
    let lines: Vec<String> = reader
        .lines()
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_default();

    // 累加所有 token（用于费用计算）
    let mut total_input_tokens: u32 = 0;
    let mut total_cache_read_tokens: u32 = 0;
    let mut total_cache_write_tokens: u32 = 0;
    let mut total_output_tokens: u32 = 0;

    // 最后一条记录的上下文 token（用于使用率计算）
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
                        // 累加用于费用计算
                        total_input_tokens += usage.input_tokens;
                        total_cache_read_tokens += usage.cache_read_input_tokens;
                        total_cache_write_tokens += usage.cache_creation_input_tokens;
                        total_output_tokens += usage.output_tokens;

                        // 更新最后一条的上下文 token
                        last_context_tokens = usage.input_tokens
                            + usage.cache_read_input_tokens
                            + usage.cache_creation_input_tokens;
                    }
                }
            }
        }
    }

    TokenUsage {
        total_input_tokens,
        total_cache_read_tokens,
        total_cache_write_tokens,
        total_output_tokens,
        last_context_tokens,
    }
}
