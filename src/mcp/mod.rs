//! `stealthlingo mcp`: a Model Context Protocol server on stdin/stdout that
//! lets AI agents look words up, read the study list and add words to it.

pub mod protocol;
pub mod server;
pub mod tools;

use anyhow::Result;

use crate::commands::Context;

pub use server::serve;

pub fn run(ctx: &mut Context) -> Result<()> {
    serve(ctx, std::io::stdin().lock(), std::io::stdout().lock())
}
