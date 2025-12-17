use super::Segment;
use crate::config::InputData;

/// 会话费用 segment，显示当前会话的总费用
pub struct CostSegment {
    enabled: bool,
}

impl CostSegment {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

impl Segment for CostSegment {
    fn render(&self, input: &InputData) -> String {
        if !self.enabled {
            return String::new();
        }

        let cost = match &input.cost {
            Some(c) => c,
            None => return String::new(),
        };

        let mut parts = Vec::new();

        // 费用
        if let Some(usd) = cost.total_cost_usd {
            parts.push(format!("${:.4}", usd));
        }

        // 代码行数变更
        let lines_added = cost.total_lines_added.unwrap_or(0);
        let lines_removed = cost.total_lines_removed.unwrap_or(0);
        if lines_added > 0 || lines_removed > 0 {
            parts.push(format!("+{}/-{}", lines_added, lines_removed));
        }

        parts.join(" ")
    }

    fn enabled(&self) -> bool {
        self.enabled
    }
}
