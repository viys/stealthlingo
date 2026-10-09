use anyhow::Result;

use super::study::start_session;
use super::Context;
use crate::cli::StudyMode;

/// Flashcard review of words whose review time has arrived. No new words.
pub fn run(ctx: &mut Context, count: Option<usize>, minutes: Option<u32>) -> Result<()> {
    start_session(ctx, StudyMode::Memory, true, minutes, count)
}
