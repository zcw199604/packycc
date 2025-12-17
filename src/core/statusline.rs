use crate::config::{Config, InputData};
use crate::core::segments::{CostSegment, DirectorySegment, GitSegment, ModelSegment, Segment, UsageSegment};

pub struct StatusLineGenerator {
    config: Config,
}

impl StatusLineGenerator {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn generate(&self, input: &InputData) -> String {
        let mut segments = Vec::new();

        // Assemble segments with proper colors
        if self.config.segments.model {
            let model_segment = ModelSegment::new(true);
            let content = model_segment.render(input);
            segments.push(format!("\x1b[1;36m{}\x1b[0m", content));
        }

        if self.config.segments.directory {
            let dir_segment = DirectorySegment::new(true);
            let content = dir_segment.render(input);
            // Extract directory name without icon
            let dir_name = content.trim_start_matches('○').trim_start();
            segments.push(format!("\x1b[1;33m○\x1b[0m \x1b[1;32m{}\x1b[0m", dir_name));
        }

        if self.config.segments.git {
            let git_segment = GitSegment::new(true);
            let git_output = git_segment.render(input);
            if !git_output.is_empty() {
                segments.push(format!("\x1b[1;34m{}\x1b[0m", git_output));
            }
        }

        if self.config.segments.usage {
            let usage_segment = UsageSegment::new(true);
            let content = usage_segment.render(input);
            if !content.is_empty() {
                // 颜色已在 UsageSegment 内部设置
                segments.push(content);
            }
        }

        if self.config.segments.cost {
            let cost_segment = CostSegment::new(true);
            let content = cost_segment.render(input);
            if !content.is_empty() {
                segments.push(format!("\x1b[1;33m💰{}\x1b[0m", content));
            }
        }

        // Join segments with white separator
        segments.join("\x1b[37m | \x1b[0m")
    }
}
