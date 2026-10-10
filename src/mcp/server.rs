//! The stdio request loop: one JSON-RPC message per line in, one per line out.

use std::io::{BufRead, Write};

use anyhow::{Context as _, Result};
use serde_json::{json, Value};

use super::protocol::{self, Incoming, RpcError, METHOD_NOT_FOUND};
use super::tools;
use crate::commands::Context;

const INSTRUCTIONS: &str = "StealthLingo is the user's English vocabulary trainer. \
Use it to save words the user may not know so they come up in practice.\n\
- Call get_study_status and list_words before suggesting words. If many saved words \
have not been started yet (new_not_started), add few words or none.\n\
- Words reported as `dismissed` were removed by the user: do not suggest them again.\n\
- Give each word a short `reason` saying where it came up and why it is worth learning. \
Never put definitions or translations in the reason.\n\
- Definitions come only from Wiktionary. When telling the user what a word means, use \
what lookup_word or add_words returned.";

/// Serves MCP requests from `input` until it closes. Only protocol messages
/// are written to `output`.
pub fn serve(ctx: &mut Context, input: impl BufRead, mut output: impl Write) -> Result<()> {
    let mut version = protocol::SUPPORTED_VERSIONS[0];
    for line in input.lines() {
        let line = line.context("could not read from standard input")?;
        if line.trim().is_empty() {
            continue;
        }
        let reply = match protocol::parse(&line) {
            Err(reply) => Some(reply),
            Ok(Incoming::Notification { .. } | Incoming::Other) => None,
            Ok(Incoming::Request { id, method, params }) => {
                Some(match handle(ctx, &mut version, &method, &params) {
                    Ok(result) => protocol::result(id, result),
                    Err(err) => protocol::error(id, err.code, &err.message),
                })
            }
        };
        if let Some(reply) = reply {
            serde_json::to_writer(&mut output, &reply)?;
            output.write_all(b"\n")?;
            output.flush()?;
        }
    }
    Ok(())
}

fn handle(
    ctx: &mut Context,
    version: &mut &'static str,
    method: &str,
    params: &Value,
) -> Result<Value, RpcError> {
    match method {
        "initialize" => {
            *version = protocol::negotiate(params.get("protocolVersion").and_then(Value::as_str));
            Ok(json!({
                "protocolVersion": *version,
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": {
                    "name": "stealthlingo",
                    "title": "StealthLingo",
                    "version": env!("CARGO_PKG_VERSION"),
                },
                "instructions": INSTRUCTIONS,
            }))
        }
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools::definitions(version) })),
        "tools/call" => {
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| RpcError::invalid_params("missing tool \"name\""))?;
            let arguments = match params.get("arguments") {
                None | Some(Value::Null) => json!({}),
                Some(args) => args.clone(),
            };
            match ctx.reload_config() {
                Ok(warnings) => {
                    for warning in warnings {
                        eprintln!("Warning: {warning}");
                    }
                }
                Err(err) => eprintln!("Warning: {err:#}; keeping the previous settings"),
            }
            tools::call(ctx, name, &arguments, version)
        }
        _ => Err(RpcError::new(
            METHOD_NOT_FOUND,
            format!("unknown method \"{method}\""),
        )),
    }
}
