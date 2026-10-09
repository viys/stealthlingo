pub mod scheduling;
pub mod session;
pub mod spelling;

pub use scheduling::{schedule, Grade, ScheduleState, Status};

/// Kind of practice recorded in `practice_attempts.mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PracticeMode {
    Memory,
    Spelling,
    ListeningSpelling,
}

impl PracticeMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::Spelling => "spelling",
            Self::ListeningSpelling => "listening_spelling",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "memory" => Some(Self::Memory),
            "spelling" => Some(Self::Spelling),
            "listening_spelling" => Some(Self::ListeningSpelling),
            _ => None,
        }
    }
}
