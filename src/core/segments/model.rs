use super::Segment;
use crate::config::InputData;

pub struct ModelSegment {
    enabled: bool,
}

impl ModelSegment {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

impl Segment for ModelSegment {
    fn render(&self, input: &InputData) -> String {
        if !self.enabled {
            return String::new();
        }

        format!("● {}", self.format_model_name(&input.model.display_name))
    }

    fn enabled(&self) -> bool {
        self.enabled
    }
}

impl ModelSegment {
    fn format_model_name(&self, display_name: &str) -> String {
        // 首先处理 (1M context) -> 1M
        let name = display_name.replace("(1M context)", "1M").trim().to_string();

        match name.as_str() {
            // Opus 系列
            n if n.contains("opus-4-5") || n.contains("opus-4.5") => "Opus 4.5".to_string(),
            n if n.contains("claude-4-1-opus") => "Opus 4.1".to_string(),
            n if n.contains("claude-4-opus") || n.contains("opus-4") => "Opus 4".to_string(),
            // Sonnet 系列
            n if n.contains("sonnet-4-5") || n.contains("sonnet-4.5") => {
                // 如果包含 1M，保留
                if name.contains("1M") {
                    "Sonnet 4.5 1M".to_string()
                } else {
                    "Sonnet 4.5".to_string()
                }
            }
            n if n.contains("claude-4-sonnet") || n.contains("sonnet-4") => "Sonnet 4".to_string(),
            n if n.contains("claude-3-7-sonnet") => "Sonnet 3.7".to_string(),
            n if n.contains("claude-3-5-sonnet") => "Sonnet 3.5".to_string(),
            n if n.contains("claude-3-sonnet") => "Sonnet 3".to_string(),
            // Haiku 系列
            n if n.contains("haiku") => "Haiku".to_string(),
            // 默认
            _ => name,
        }
    }
}
