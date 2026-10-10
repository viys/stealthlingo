use std::io::{Read, Write};
use std::net::TcpListener;

use chrono::Utc;
use serde_json::{json, Value};
use stealthlingo::commands::Context;
use stealthlingo::config::Paths;
use stealthlingo::dictionary::{self, wiktionary, Endpoints};
use stealthlingo::mcp::serve;
use stealthlingo::storage::Database;

const WIKT_REST: &str = include_str!("fixtures/wiktionary_ephemeral.json");

fn context(dir: &tempfile::TempDir) -> Context {
    let mut ctx = Context::load_from(Paths::at(dir.path().to_path_buf())).unwrap();
    ctx.endpoints = Endpoints {
        rest_base: "http://127.0.0.1:9/definition".to_string(),
        action_api: "http://127.0.0.1:9/api.php".to_string(),
    };
    ctx.config.http_timeout_secs = 2;
    ctx.save_config().unwrap();
    ctx
}

/// Caches copies of the "ephemeral" entry under these headwords, unsaved.
fn cache(ctx: &Context, words: &[&str]) {
    let entry = wiktionary::parse_definitions("ephemeral", WIKT_REST).unwrap();
    for word in words {
        let mut entry = entry.clone();
        entry.word = word.to_string();
        ctx.db
            .cache_word(word, &entry, WIKT_REST, dictionary::SOURCE, Utc::now())
            .unwrap();
    }
}

fn rpc(ctx: &mut Context, messages: &[Value]) -> Vec<Value> {
    let input: String = messages.iter().map(|m| format!("{m}\n")).collect();
    let mut output = Vec::new();
    serve(ctx, input.as_bytes(), &mut output).unwrap();
    String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn request(id: i64, method: &str, params: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
}

/// Calls a tool and returns the JSON-RPC response.
fn call_raw(ctx: &mut Context, tool: &str, arguments: Value) -> Value {
    let replies = rpc(
        ctx,
        &[request(
            1,
            "tools/call",
            json!({ "name": tool, "arguments": arguments }),
        )],
    );
    assert_eq!(replies.len(), 1);
    replies[0].clone()
}

/// Calls a tool that must succeed at the protocol level; returns its result.
fn call(ctx: &mut Context, tool: &str, arguments: Value) -> Value {
    let reply = call_raw(ctx, tool, arguments);
    assert!(reply.get("error").is_none(), "{reply}");
    reply["result"].clone()
}

fn add(ctx: &mut Context, words: Value) -> Vec<Value> {
    let result = call(ctx, "add_words", json!({ "words": words }));
    result["structuredContent"]["results"]
        .as_array()
        .unwrap()
        .clone()
}

fn statuses(results: &[Value]) -> Vec<&str> {
    results
        .iter()
        .map(|r| r["status"].as_str().unwrap())
        .collect()
}

fn set_limit(ctx: &mut Context, limit: u32) {
    ctx.config.mcp_daily_add_limit = limit;
    ctx.save_config().unwrap();
}

/// Answers every HTTP request with 404 Not Found. Returns the base URL.
fn not_found_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut request = Vec::new();
            let mut buf = [0; 4096];
            while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                match stream.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => request.extend_from_slice(&buf[..n]),
                }
            }
            let _ = write!(
                stream,
                "HTTP/1.1 404 X\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
        }
    });
    base
}

#[test]
fn initialize_negotiates_the_protocol_version() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir);
    let replies = rpc(
        &mut ctx,
        &[
            request(1, "initialize", json!({ "protocolVersion": "2025-03-26" })),
            json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
            request(2, "initialize", json!({ "protocolVersion": "1999-01-01" })),
            request(3, "ping", json!({})),
        ],
    );
    assert_eq!(replies.len(), 3, "notifications get no reply: {replies:?}");
    assert_eq!(replies[0]["id"], 1);
    assert_eq!(replies[0]["result"]["protocolVersion"], "2025-03-26");
    assert_eq!(replies[0]["result"]["serverInfo"]["name"], "stealthlingo");
    assert!(replies[0]["result"]["capabilities"]["tools"].is_object());
    let instructions = replies[0]["result"]["instructions"].as_str().unwrap();
    assert!(instructions.contains("dismissed"), "{instructions}");
    assert_eq!(replies[1]["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(replies[2]["result"], json!({}));
}

#[test]
fn tools_are_listed_with_output_schemas_only_for_newer_clients() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir);
    let replies = rpc(
        &mut ctx,
        &[
            request(1, "initialize", json!({ "protocolVersion": "2025-06-18" })),
            request(2, "tools/list", json!({})),
            request(3, "initialize", json!({ "protocolVersion": "2025-03-26" })),
            request(4, "tools/list", json!({})),
        ],
    );
    let tools = replies[1]["result"]["tools"].as_array().unwrap();
    let names: Vec<_> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        ["get_study_status", "list_words", "lookup_word", "add_words"]
    );
    assert!(tools.iter().all(|t| t["outputSchema"].is_object()));
    let add = &tools[3];
    assert_eq!(add["annotations"]["readOnlyHint"], false);
    assert_eq!(add["annotations"]["destructiveHint"], false);
    assert_eq!(add["annotations"]["idempotentHint"], true);
    assert_eq!(tools[0]["annotations"]["readOnlyHint"], true);

    let old = replies[3]["result"]["tools"].as_array().unwrap();
    assert_eq!(old.len(), 4);
    assert!(old.iter().all(|t| t.get("outputSchema").is_none()));
}

#[test]
fn protocol_errors_use_json_rpc_codes() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir);
    let input = format!(
        "{}\n{{not json\n\n{}\n{}\n",
        request(1, "resources/list", json!({})),
        request(
            2,
            "tools/call",
            json!({ "name": "remove_word", "arguments": {} })
        ),
        json!({ "jsonrpc": "2.0", "method": "notifications/cancelled" }),
    );
    let mut output = Vec::new();
    serve(&mut ctx, input.as_bytes(), &mut output).unwrap();
    let replies: Vec<Value> = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(replies.len(), 3, "{replies:?}");
    assert_eq!(replies[0]["error"]["code"], -32601);
    assert_eq!(replies[1]["error"]["code"], -32700);
    assert_eq!(replies[1]["id"], Value::Null);
    assert_eq!(replies[2]["error"]["code"], -32602);
}

#[test]
fn agents_add_words_with_a_reason_but_never_a_note() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir);
    cache(&ctx, &["ephemeral"]);

    let rejected = call_raw(
        &mut ctx,
        "add_words",
        json!({ "words": [{ "word": "ephemeral", "note": "短暂的" }] }),
    );
    assert_eq!(rejected["error"]["code"], -32602, "{rejected}");
    assert_eq!(ctx.db.collection_size().unwrap(), 0);

    let results = add(
        &mut ctx,
        json!([{ "word": "Ephemeral", "reason": "Second paragraph of the article" }]),
    );
    assert_eq!(statuses(&results), ["added"]);
    assert_eq!(results[0]["headword"], "ephemeral");
    assert!(results[0]["summary"].as_str().unwrap().len() > 3);

    let words = ctx.db.list_words(None).unwrap();
    assert_eq!(words.len(), 1);
    assert!(words[0].added_by_agent);
    assert_eq!(
        words[0].added_reason.as_deref(),
        Some("Second paragraph of the article")
    );
    assert_eq!(words[0].note, None);
    let study = ctx.db.new_items(10, false).unwrap();
    assert_eq!(study[0].note, None, "reasons never appear while practising");

    let again = add(
        &mut ctx,
        json!([{ "word": "ephemeral", "reason": "other" }]),
    );
    assert_eq!(statuses(&again), ["already_saved"]);
    let words = ctx.db.list_words(None).unwrap();
    assert_eq!(
        words[0].added_reason.as_deref(),
        Some("Second paragraph of the article")
    );
}

#[test]
fn bad_words_are_reported_one_by_one() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir);
    cache(&ctx, &["brisk"]);
    let long = "a".repeat(65);
    let result = call(
        &mut ctx,
        "add_words",
        json!({ "words": [
            { "word": "  " },
            { "word": "two\nlines" },
            { "word": long },
            { "word": "brisk", "reason": "x".repeat(201) },
            { "word": "zzyzx" },
        ]}),
    );
    let results = result["structuredContent"]["results"].as_array().unwrap();
    assert_eq!(
        statuses(results),
        ["invalid", "invalid", "invalid", "invalid", "network_error"]
    );
    assert_eq!(result["isError"], true, "nothing could be added");
    assert_eq!(ctx.db.collection_size().unwrap(), 0);

    let too_many: Vec<Value> = (0..21)
        .map(|i| json!({ "word": format!("w{i}") }))
        .collect();
    let reply = call_raw(&mut ctx, "add_words", json!({ "words": too_many }));
    assert_eq!(reply["error"]["code"], -32602);
    assert!(reply["error"]["message"].as_str().unwrap().contains("20"));
}

#[test]
fn words_missing_from_wiktionary_are_not_added() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir);
    let base = not_found_server();
    ctx.endpoints = Endpoints {
        rest_base: format!("{base}/definition"),
        action_api: format!("{base}/api.php"),
    };
    let results = add(&mut ctx, json!([{ "word": "qwertyuiop" }]));
    assert_eq!(statuses(&results), ["not_found"]);
    assert_eq!(ctx.db.collection_size().unwrap(), 0);

    let lookup = call(&mut ctx, "lookup_word", json!({ "word": "qwertyuiop" }));
    assert_eq!(lookup["isError"], true);
}

#[test]
fn words_the_user_removed_are_not_added_back_by_agents() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir);
    cache(&ctx, &["ephemeral", "brisk"]);
    assert_eq!(
        statuses(&add(&mut ctx, json!([{ "word": "ephemeral" }]))),
        ["added"]
    );

    assert!(ctx
        .db
        .remove_from_collection("ephemeral", Utc::now())
        .unwrap());
    let results = add(&mut ctx, json!([{ "word": "ephemeral" }]));
    assert_eq!(statuses(&results), ["dismissed"]);
    assert_eq!(ctx.db.collection_size().unwrap(), 0);

    // Removing by link archives the word; agents cannot restore it either.
    let brisk = ctx.db.find_cached("brisk").unwrap().unwrap();
    ctx.db
        .add_to_collection(brisk.id, None, Utc::now())
        .unwrap();
    ctx.db.archive_from_collection("brisk", Utc::now()).unwrap();
    assert_eq!(
        statuses(&add(&mut ctx, json!([{ "word": "brisk" }]))),
        ["dismissed"]
    );

    // The user saving it again lifts the block.
    let ephemeral = ctx.db.find_cached("ephemeral").unwrap().unwrap();
    ctx.db
        .add_to_collection(ephemeral.id, None, Utc::now())
        .unwrap();
    ctx.db
        .remove_from_collection("ephemeral", Utc::now())
        .unwrap();
    assert_eq!(
        statuses(&add(&mut ctx, json!([{ "word": "ephemeral" }]))),
        ["dismissed"]
    );
    ctx.db
        .add_to_collection(ephemeral.id, None, Utc::now())
        .unwrap();
    assert_eq!(
        statuses(&add(&mut ctx, json!([{ "word": "ephemeral" }]))),
        ["already_saved"]
    );
}

#[test]
fn the_daily_limit_counts_removed_words_and_is_read_on_every_call() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir);
    cache(&ctx, &["alpha", "beta", "gamma"]);
    set_limit(&mut ctx, 1);
    let results = add(
        &mut ctx,
        json!([{ "word": "alpha" }, { "word": "beta" }, { "word": "zzyzx" }]),
    );
    assert_eq!(
        statuses(&results),
        ["added", "limit_reached", "limit_reached"],
        "no lookup once the limit is reached"
    );

    ctx.db.remove_from_collection("alpha", Utc::now()).unwrap();
    assert_eq!(
        statuses(&add(&mut ctx, json!([{ "word": "gamma" }]))),
        ["limit_reached"]
    );

    set_limit(&mut ctx, 0);
    let status = call(&mut ctx, "get_study_status", json!({}));
    assert_eq!(status["structuredContent"]["agent_adds_left"], 0);
    assert_eq!(status["structuredContent"]["agent_added_today"], 1);

    set_limit(&mut ctx, 5);
    let result = call(
        &mut ctx,
        "add_words",
        json!({ "words": [{ "word": "gamma" }] }),
    );
    assert_eq!(result["structuredContent"]["agent_adds_left"], 3);
    assert_eq!(result["isError"], false);
}

#[test]
fn status_and_word_list_describe_the_study_list_without_notes() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir);
    cache(&ctx, &["ephemeral", "brisk"]);
    let brisk = ctx.db.find_cached("brisk").unwrap().unwrap();
    ctx.db
        .add_to_collection(brisk.id, Some("private note"), Utc::now())
        .unwrap();
    add(
        &mut ctx,
        json!([{ "word": "ephemeral", "reason": "article" }]),
    );

    let status = call(&mut ctx, "get_study_status", json!({}));
    let data = &status["structuredContent"];
    assert_eq!(data["saved_words"], 2);
    assert_eq!(data["new_not_started"], 2);
    assert_eq!(data["daily_goal"], 10);
    assert_eq!(data["words_practised_today"], 0);
    assert_eq!(data["accuracy_today"], Value::Null);
    assert_eq!(data["agent_adds_left"], 29);

    let list = call(&mut ctx, "list_words", json!({}));
    let text = list.to_string();
    assert!(!text.contains("private note"), "{text}");
    let words = list["structuredContent"]["words"].as_array().unwrap();
    assert_eq!(words.len(), 2);
    assert_eq!(words[0]["word"], "brisk");
    assert_eq!(words[0]["added_by"], "user");
    assert_eq!(words[1]["added_by"], "agent");
    assert_eq!(words[1]["due_at"], Value::Null);

    let filtered = call(
        &mut ctx,
        "list_words",
        json!({ "query": "EPH", "status": "new", "limit": 1 }),
    );
    assert_eq!(filtered["structuredContent"]["total"], 1);
    let learning = call(&mut ctx, "list_words", json!({ "status": "learning" }));
    assert_eq!(learning["structuredContent"]["total"], 0);
    let bad = call_raw(&mut ctx, "list_words", json!({ "status": "known" }));
    assert_eq!(bad["error"]["code"], -32602);
}

#[test]
fn lookups_return_wiktionary_data_without_saving() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir);
    cache(&ctx, &["ephemeral"]);
    let result = call(&mut ctx, "lookup_word", json!({ "word": "ephemeral" }));
    assert_eq!(result["isError"], false);
    let entry = &result["structuredContent"];
    assert_eq!(entry["word"], "ephemeral");
    assert_eq!(entry["saved"], false);
    assert!(!entry["meanings"].as_array().unwrap().is_empty());
    assert!(result["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("ephemeral"));
    assert_eq!(ctx.db.collection_size().unwrap(), 0);

    let offline = call(&mut ctx, "lookup_word", json!({ "word": "zzyzx" }));
    assert_eq!(offline["isError"], true);
}

#[test]
fn backups_keep_who_added_a_word_and_what_was_removed() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir);
    cache(&ctx, &["ephemeral", "brisk"]);
    add(
        &mut ctx,
        json!([{ "word": "ephemeral", "reason": "article" }, { "word": "brisk" }]),
    );
    ctx.db.remove_from_collection("brisk", Utc::now()).unwrap();
    let backup = ctx.db.export_backup(Utc::now()).unwrap();

    let mut restored = Database::open_in_memory().unwrap();
    restored.import_backup(&backup).unwrap();
    let words = restored.list_words(None).unwrap();
    assert_eq!(words.len(), 1);
    assert!(words[0].added_by_agent);
    assert_eq!(words[0].added_reason.as_deref(), Some("article"));
    assert!(restored.is_dismissed("brisk").unwrap());
    assert!(!restored.is_dismissed("ephemeral").unwrap());
}
