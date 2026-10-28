use crate::config::{Config, InputData};
use crate::core::segments::{
    CostSegment, DirectorySegment, GitSegment, ModelSegment, QuotaSegment, Segment, UsageSegment,
};

pub struct StatusLineGenerator {
    config: Config,
}

impl StatusLineGenerator {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// 按 Claude Code 设置的 `COLUMNS` 换行；拿不到宽度时保持单行
    pub fn generate(&self, input: &InputData) -> String {
        let width = std::env::var("COLUMNS")
            .ok()
            .and_then(|v| v.trim().parse().ok())
            .filter(|w: &usize| *w > 0);
        self.generate_with_width(input, width)
    }

    pub fn generate_with_width(&self, input: &InputData, width: Option<usize>) -> String {
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

        if self.config.segments.quota {
            let content = QuotaSegment::new(true).render(input);
            if !content.is_empty() {
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

        join_lines(&segments, width)
    }
}

const SEPARATOR: &str = "\x1b[37m | \x1b[0m";
const SEPARATOR_WIDTH: usize = 3;

/// 在段边界处换行，单个段超宽时独占一行而不截断
fn join_lines(segments: &[String], width: Option<usize>) -> String {
    let Some(width) = width else {
        return segments.join(SEPARATOR);
    };

    let mut lines: Vec<String> = Vec::new();
    let mut line_width = 0;
    for segment in segments {
        let w = display_width(segment);
        match lines.last_mut() {
            Some(line) if line_width + SEPARATOR_WIDTH + w <= width => {
                line.push_str(SEPARATOR);
                line.push_str(segment);
                line_width += SEPARATOR_WIDTH + w;
            }
            _ => {
                lines.push(segment.clone());
                line_width = w;
            }
        }
    }
    lines.join("\n")
}

pub(crate) fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // CSI 序列：ESC [ ... 以 @~ 范围内的字符结束
            if chars.next() == Some('[') {
                for c in chars.by_ref() {
                    if ('@'..='~').contains(&c) {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// 终端显示宽度：中日韩文字与 emoji 占 2 列
fn display_width(s: &str) -> usize {
    strip_ansi(s)
        .chars()
        .map(|c| match c as u32 {
            0x1100..=0x115F
            | 0x2E80..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE30..=0xFE4F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x1F300..=0x1F64F
            | 0x1F900..=0x1F9FF
            | 0x20000..=0x3FFFD => 2,
            _ => 1,
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_width_ignores_ansi_and_counts_wide_chars() {
        assert_eq!(display_width("\x1b[1;33m💰$1.23\x1b[0m"), 7);
        assert_eq!(display_width("○ 项目"), 6);
    }

    #[test]
    fn wraps_only_at_segment_boundaries() {
        let segs: Vec<String> = ["aaaa", "bbbb", "cccc"].map(String::from).to_vec();
        let plain = |s: String| strip_ansi(&s);
        assert_eq!(plain(join_lines(&segs, None)), "aaaa | bbbb | cccc");
        assert_eq!(plain(join_lines(&segs, Some(18))), "aaaa | bbbb | cccc");
        assert_eq!(plain(join_lines(&segs, Some(17))), "aaaa | bbbb\ncccc");
        assert_eq!(plain(join_lines(&segs, Some(3))), "aaaa\nbbbb\ncccc");
    }
}
