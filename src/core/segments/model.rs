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
        match display_name {
            // Opus 系列
            name if name.contains("opus-4-5") || name.contains("opus-4.5") => "Opus 4.5".to_string(),
            name if name.contains("claude-4-1-opus") => "Opus 4.1".to_string(),
            name if name.contains("claude-4-opus") || name.contains("opus-4") => "Opus 4".to_string(),
            // Sonnet 系列
            name if name.contains("sonnet-4-5") || name.contains("sonnet-4.5") => "Sonnet 4.5".to_string(),
            name if name.contains("claude-4-sonnet") || name.contains("sonnet-4") => "Sonnet 4".to_string(),
            name if name.contains("claude-3-7-sonnet") => "Sonnet 3.7".to_string(),
            name if name.contains("claude-3-5-sonnet") => "Sonnet 3.5".to_string(),
            name if name.contains("claude-3-sonnet") => "Sonnet 3".to_string(),
            // Haiku 系列
            name if name.contains("haiku") => "Haiku".to_string(),
            // 默认
            _ => display_name.to_string(),
        }
    }
}
