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

        match cost.total_cost_usd {
            Some(usd) => format!("${:.4}", usd),
            None => String::new(),
        }
    }

    fn enabled(&self) -> bool {
        self.enabled
    }
}
