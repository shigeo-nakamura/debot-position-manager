mod position_manager;
use std::fmt;

pub use position_manager::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub enum PositionType {
    #[default]
    Long,
    Short,
}

impl fmt::Display for PositionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PositionType::Long => write!(f, "Long"),
            PositionType::Short => write!(f, "Short"),
        }
    }
}

impl PositionType {
    pub fn opposite(&self) -> PositionType {
        match self {
            PositionType::Long => PositionType::Short,
            PositionType::Short => PositionType::Long,
        }
    }
}
