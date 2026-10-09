use anyhow::Result;

use super::study::{start_session, SessionRequest};
use super::Context;

/// Flashcard review of words whose review time has arrived. No new words.
pub fn run(ctx: &mut Context, count: Option<usize>, minutes: Option<u32>) -> Result<()> {
    start_session(ctx, &SessionRequest::review(count, minutes))
}
