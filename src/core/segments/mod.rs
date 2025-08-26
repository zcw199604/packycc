pub mod directory;
pub mod git;
pub mod info_share;
pub mod model;
pub mod quota;
pub mod usage;

use crate::config::InputData;

pub trait Segment {
    fn render(&self, input: &InputData) -> String;
    fn enabled(&self) -> bool;
}

// Re-export all segment types
pub use directory::DirectorySegment;
pub use git::GitSegment;
pub use info_share::InfoShareSegment;
pub use model::ModelSegment;
pub use quota::QuotaSegment;
pub use usage::UsageSegment;
