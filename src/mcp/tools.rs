//! The tools offered to agents: definitions, argument checks and results.

use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};

use super::protocol::{supports_structured_output, RpcError};
use crate::commands::{cached_or_fetch, store_fetched, Context};
use crate::dictionary::{clean_word, normalize_headword};
use crate::error::DictionaryError;
use crate::learning::Status;
use crate::storage::{AgentAddOutcome, CachedWord};
use crate::time::{local_day_start, to_db};

pub const MAX_WORDS_PER_CALL: usize = 20;
pub const MAX_WORD_CHARS: usize = 64;
pub const MAX_REASON_CHARS: usize = 200;
/// Total network time for one `add_words` call, well below the roughly one
/// minute after which common MCP clients give up on a tool call.
pub const ADD_BUDGET: Duration = Duration::from_secs(30);
const DEFAULT_LIST_LIMIT: usize = 100;
const MAX_LIST_LIMIT: usize = 1000;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NoArgs {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListArgs {
    query: Option<String>,
    status: Option<String>,
    limit: Option<usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LookupArgs {
    word: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AddArgs {
    words: Vec<WordRequest>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WordRequest {
    word: String,
    reason: Option<String>,
}

/// A tool's answer: a human-readable summary plus machine-readable data.
struct ToolOutput {
    text: String,
    structured: Option<Value>,
    is_error: bool,
}

impl ToolOutput {
    fn ok(text: String, structured: Value) -> Self {
        Self {
            text,
            structured: Some(structured),
            is_error: false,
        }
    }

    fn failure(text: String) -> Self {
        Self {
            text,
            structured: None,
            is_error: true,
        }
    }

    fn into_result(self, version: &str) -> Value {
        let text = match &self.structured {
            Some(data) => format!(
                "{}\n\n{}",
                self.text,
                serde_json::to_string_pretty(data).unwrap_or_default()
            ),
            None => self.text,
        };
        let mut result = json!({
            "content": [{ "type": "text", "text": text }],
            "isError": self.is_error,
        });
        if let Some(data) = self.structured {
            if supports_structured_output(version) {
                result["structuredContent"] = data;
            }
        }
        result
    }
}

enum ToolError {
    /// Bad arguments: a protocol error, the call is not attempted.
    Params(RpcError),
    /// The tool ran but failed; reported to the agent as a tool result.
    Failed(anyhow::Error),
}

impl From<RpcError> for ToolError {
    fn from(err: RpcError) -> Self {
        Self::Params(err)
    }
}

impl From<anyhow::Error> for ToolError {
    fn from(err: anyhow::Error) -> Self {
        Self::Failed(err)
    }
}

fn parse_args<T: DeserializeOwned>(args: &Value) -> Result<T, RpcError> {
    serde_json::from_value(args.clone())
        .map_err(|e| RpcError::invalid_params(format!("invalid arguments: {e}")))
}

pub fn definitions(version: &str) -> Vec<Value> {
    let structured = supports_structured_output(version);
    let tools = [
        (
            "get_study_status",
            "Study status",
            "How much the user is studying: saved words, reviews due, saved words not \
             started yet, the daily goal and today's progress, and how many more words \
             agents may add today. Check this before suggesting words.",
            json!({ "type": "object", "properties": {}, "additionalProperties": false }),
            status_schema(),
            json!({ "readOnlyHint": true, "openWorldHint": false }),
        ),
        (
            "list_words",
            "List saved words",
            "Words on the user's study list with their learning status, next review time, \
             lapses (times forgotten after being learned) and a short Wiktionary definition. \
             Use it to avoid suggesting saved words and to judge the user's level.",
            json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Only words containing this text" },
                    "status": { "type": "string", "enum": ["new", "learning", "review", "mastered"] },
                    "limit": { "type": "integer", "minimum": 1, "maximum": MAX_LIST_LIMIT,
                               "default": DEFAULT_LIST_LIMIT },
                },
                "additionalProperties": false,
            }),
            list_schema(),
            json!({ "readOnlyHint": true, "openWorldHint": false }),
        ),
        (
            "lookup_word",
            "Look up a word",
            "The English Wiktionary entry for a word: parts of speech, definitions, examples, \
             IPA, whether a recording exists, source and license. Does not save the word.",
            json!({
                "type": "object",
                "properties": { "word": { "type": "string", "maxLength": MAX_WORD_CHARS } },
                "required": ["word"],
                "additionalProperties": false,
            }),
            lookup_schema(),
            json!({ "readOnlyHint": true, "openWorldHint": true }),
        ),
        (
            "add_words",
            "Add words to the study list",
            "Saves suggested English words or phrases to the user's study list, up to 20 per \
             call. Definitions always come from Wiktionary; `reason` says where the word came \
             up and why it is worth learning, never what it means. Returns one result per \
             word: added, already_saved, dismissed (the user removed it; do not suggest it \
             again), not_found, network_error, deferred (call again), limit_reached or invalid.",
            json!({
                "type": "object",
                "properties": {
                    "words": {
                        "type": "array",
                        "minItems": 1,
                        "maxItems": MAX_WORDS_PER_CALL,
                        "items": {
                            "type": "object",
                            "properties": {
                                "word": { "type": "string", "maxLength": MAX_WORD_CHARS },
                                "reason": { "type": "string", "maxLength": MAX_REASON_CHARS },
                            },
                            "required": ["word"],
                            "additionalProperties": false,
                        },
                    },
                },
                "required": ["words"],
                "additionalProperties": false,
            }),
            add_schema(),
            json!({ "readOnlyHint": false, "destructiveHint": false,
                    "idempotentHint": true, "openWorldHint": true }),
        ),
    ];
    tools
        .into_iter()
        .map(
            |(name, title, description, input, output, mut annotations)| {
                annotations["title"] = json!(title);
                let mut tool = json!({
                    "name": name,
                    "description": description,
                    "inputSchema": input,
                    "annotations": annotations,
                });
                if structured {
                    tool["title"] = json!(title);
                    tool["outputSchema"] = output;
                }
                tool
            },
        )
        .collect()
}

fn status_schema() -> Value {
    let count = json!({ "type": "integer" });
    json!({
        "type": "object",
        "properties": {
            "saved_words": count, "due_now": count, "new_not_started": count,
            "daily_new_limit": count, "new_started_today": count, "daily_goal": count,
            "words_practised_today": count, "answers_today": count,
            "accuracy_today": { "type": ["number", "null"] },
            "agent_added_today": count, "agent_add_limit": count, "agent_adds_left": count,
        },
        "required": ["saved_words", "due_now", "new_not_started", "daily_new_limit",
                     "daily_goal", "words_practised_today", "agent_adds_left"],
    })
}

fn list_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "total": { "type": "integer" },
            "returned": { "type": "integer" },
            "words": { "type": "array", "items": {
                "type": "object",
                "properties": {
                    "word": { "type": "string" },
                    "status": { "type": "string" },
                    "due_at": { "type": ["string", "null"] },
                    "has_audio": { "type": "boolean" },
                    "lapses": { "type": "integer" },
                    "summary": { "type": "string" },
                    "added_by": { "type": "string", "enum": ["user", "agent"] },
                },
                "required": ["word", "status", "has_audio", "lapses", "summary", "added_by"],
            }},
        },
        "required": ["total", "returned", "words"],
    })
}

fn lookup_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "word": { "type": "string" },
            "saved": { "type": "boolean" },
            "has_audio": { "type": "boolean" },
            "pronunciations": { "type": "array" },
            "meanings": { "type": "array" },
            "source_urls": { "type": "array", "items": { "type": "string" } },
            "license": { "type": ["string", "null"] },
        },
        "required": ["word", "saved", "has_audio", "meanings"],
    })
}

fn add_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "results": { "type": "array", "items": {
                "type": "object",
                "properties": {
                    "word": { "type": "string" },
                    "status": { "type": "string", "enum": ["added", "already_saved", "dismissed",
                        "not_found", "network_error", "deferred", "limit_reached", "invalid"] },
                    "headword": { "type": "string" },
                    "summary": { "type": "string" },
                    "has_audio": { "type": "boolean" },
                    "message": { "type": "string" },
                },
                "required": ["word", "status"],
            }},
            "added": { "type": "integer" },
            "agent_adds_left": { "type": "integer" },
        },
        "required": ["results", "added", "agent_adds_left"],
    })
}

pub fn call(ctx: &Context, name: &str, args: &Value, version: &str) -> Result<Value, RpcError> {
    let outcome = match name {
        "get_study_status" => parse_args::<NoArgs>(args)
            .map_err(ToolError::from)
            .and_then(|_| study_status(ctx)),
        "list_words" => list_words(ctx, args),
        "lookup_word" => lookup_word(ctx, args),
        "add_words" => add_words(ctx, args),
        _ => return Err(RpcError::invalid_params(format!("unknown tool \"{name}\""))),
    };
    let output = match outcome {
        Ok(output) => output,
        Err(ToolError::Params(err)) => return Err(err),
        Err(ToolError::Failed(err)) => ToolOutput::failure(format!("{err:#}")),
    };
    Ok(output.into_result(version))
}

fn agent_adds_left(ctx: &Context, day_start: DateTime<Utc>) -> anyhow::Result<i64> {
    let added = ctx.db.count_agent_adds_since(day_start)?;
    Ok((i64::from(ctx.config.mcp_daily_add_limit) - added).max(0))
}

fn study_status(ctx: &Context) -> Result<ToolOutput, ToolError> {
    let now = Utc::now();
    let day_start = local_day_start(now);
    let stats = ctx.db.stats(now, day_start)?;
    let not_started = ctx.db.count_new_ready()?;
    let agent_added = ctx.db.count_agent_adds_since(day_start)?;
    let left = agent_adds_left(ctx, day_start)?;
    let accuracy = (stats.attempts_today > 0)
        .then(|| stats.correct_today as f64 / stats.attempts_today as f64);
    let config = &ctx.config;
    let text = format!(
        "{} saved words, {} due for review, {} not started yet (up to {} new words a day). \
         Today: {} of {} words practised. Agents may add {} more words today (limit {}).",
        stats.total_words,
        stats.due_now,
        not_started,
        config.daily_new_limit,
        stats.words_today,
        config.daily_goal,
        left,
        config.mcp_daily_add_limit,
    );
    Ok(ToolOutput::ok(
        text,
        json!({
            "saved_words": stats.total_words,
            "due_now": stats.due_now,
            "new_not_started": not_started,
            "daily_new_limit": config.daily_new_limit,
            "new_started_today": stats.new_words_today,
            "daily_goal": config.daily_goal,
            "words_practised_today": stats.words_today,
            "answers_today": stats.attempts_today,
            "accuracy_today": accuracy,
            "agent_added_today": agent_added,
            "agent_add_limit": config.mcp_daily_add_limit,
            "agent_adds_left": left,
        }),
    ))
}

fn list_words(ctx: &Context, args: &Value) -> Result<ToolOutput, ToolError> {
    let args: ListArgs = parse_args(args)?;
    let status = args
        .status
        .map(|s| {
            Status::parse(&s.trim().to_lowercase()).ok_or_else(|| {
                RpcError::invalid_params(format!(
                    "unknown status \"{s}\"; use new, learning, review or mastered"
                ))
            })
        })
        .transpose()?;
    let limit = args.limit.unwrap_or(DEFAULT_LIST_LIMIT);
    if !(1..=MAX_LIST_LIMIT).contains(&limit) {
        return Err(
            RpcError::invalid_params(format!("limit must be from 1 to {MAX_LIST_LIMIT}")).into(),
        );
    }
    let query = args
        .query
        .map(|q| q.trim().to_lowercase())
        .filter(|q| !q.is_empty());
    // Matches the word only: personal notes stay private.
    let words: Vec<_> = ctx
        .db
        .list_words(None)?
        .into_iter()
        .filter(|w| status.is_none_or(|s| s == w.status))
        .filter(|w| {
            query
                .as_deref()
                .is_none_or(|q| w.display_word.to_lowercase().contains(q))
        })
        .collect();
    let listed: Vec<Value> = words
        .iter()
        .take(limit)
        .map(|w| {
            json!({
                "word": w.display_word,
                "status": w.status.as_str(),
                "due_at": (w.status != Status::New).then(|| to_db(w.due_at)),
                "has_audio": w.has_audio,
                "lapses": w.lapses,
                "summary": w.summary,
                "added_by": if w.added_by_agent { "agent" } else { "user" },
            })
        })
        .collect();
    let text = if listed.len() < words.len() {
        format!("Showing {} of {} saved words.", listed.len(), words.len())
    } else {
        format!("{} saved words.", words.len())
    };
    Ok(ToolOutput::ok(
        text,
        json!({ "total": words.len(), "returned": listed.len(), "words": listed }),
    ))
}

fn lookup_word(ctx: &Context, args: &Value) -> Result<ToolOutput, ToolError> {
    let args: LookupArgs = parse_args(args)?;
    let word = check_word(&args.word).map_err(RpcError::invalid_params)?;
    let cached = match cached_or_fetch(ctx, &word) {
        Ok((cached, _)) => cached,
        Err(err) => return Ok(ToolOutput::failure(format!("{err:#}"))),
    };
    let entry = &cached.entry;
    let mut pronunciations: Vec<Value> = entry
        .phonetics
        .iter()
        .filter_map(|p| {
            p.text
                .as_ref()
                .map(|ipa| json!({ "ipa": ipa, "accent": p.accent }))
        })
        .collect();
    if pronunciations.is_empty() {
        if let Some(ipa) = &entry.phonetic {
            pronunciations.push(json!({ "ipa": ipa, "accent": null }));
        }
    }
    let meanings: Vec<Value> = entry
        .meanings
        .iter()
        .map(|m| {
            json!({
                "part_of_speech": m.part_of_speech,
                "definitions": m.definitions.iter()
                    .map(|d| json!({ "definition": d.definition, "example": d.example }))
                    .collect::<Vec<_>>(),
                "synonyms": m.synonyms,
                "antonyms": m.antonyms,
            })
        })
        .collect();
    let saved = if cached.in_collection {
        "in the user's word list"
    } else {
        "not saved"
    };
    Ok(ToolOutput::ok(
        format!("{} — {} ({saved})", entry.word, entry.short_summary()),
        json!({
            "word": entry.word,
            "saved": cached.in_collection,
            "has_audio": entry.has_audio(),
            "pronunciations": pronunciations,
            "meanings": meanings,
            "source_urls": entry.source_urls,
            "license": entry.license,
        }),
    ))
}

/// The word as it will be looked up, or why it cannot be.
fn check_word(word: &str) -> Result<String, String> {
    if word.chars().any(char::is_control) {
        return Err("the word contains line breaks or control characters".to_string());
    }
    let word = clean_word(word);
    if word.is_empty() {
        return Err("the word is empty".to_string());
    }
    if word.chars().count() > MAX_WORD_CHARS {
        return Err(format!(
            "the word is longer than {MAX_WORD_CHARS} characters"
        ));
    }
    Ok(word)
}

fn check_reason(reason: Option<&str>) -> Result<Option<&str>, String> {
    let Some(reason) = reason.map(str::trim).filter(|r| !r.is_empty()) else {
        return Ok(None);
    };
    if reason.chars().any(char::is_control) {
        return Err("the reason must be one line without control characters".to_string());
    }
    if reason.chars().count() > MAX_REASON_CHARS {
        return Err(format!(
            "the reason is longer than {MAX_REASON_CHARS} characters"
        ));
    }
    Ok(Some(reason))
}

struct WordResult {
    word: String,
    status: &'static str,
    entry: Option<CachedWord>,
    message: Option<String>,
}

impl WordResult {
    fn new(word: &str, status: &'static str) -> Self {
        Self {
            word: word.to_string(),
            status,
            entry: None,
            message: None,
        }
    }

    fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    fn to_json(&self) -> Value {
        let mut value = json!({ "word": self.word, "status": self.status });
        if let Some(cached) = &self.entry {
            value["headword"] = json!(cached.entry.word);
            value["summary"] = json!(cached.entry.short_summary());
            value["has_audio"] = json!(cached.entry.has_audio());
        }
        if let Some(message) = &self.message {
            value["message"] = json!(message);
        }
        value
    }
}

fn add_words(ctx: &Context, args: &Value) -> Result<ToolOutput, ToolError> {
    let args: AddArgs = parse_args(args)?;
    if args.words.is_empty() {
        return Err(RpcError::invalid_params("\"words\" must list at least one word").into());
    }
    if args.words.len() > MAX_WORDS_PER_CALL {
        return Err(RpcError::invalid_params(format!(
            "at most {MAX_WORDS_PER_CALL} words per call (got {}); split the list into several calls",
            args.words.len()
        ))
        .into());
    }
    let started = Instant::now();
    let day_start = local_day_start(Utc::now());
    let mut results = Vec::new();
    let mut failure = None;
    for request in &args.words {
        match add_one(ctx, request, day_start, started) {
            Ok(result) => results.push(result),
            Err(err) => {
                failure = Some(format!(
                    "Stopped at \"{}\": {err:#}. Results for earlier words are final.",
                    request.word
                ));
                break;
            }
        }
    }

    let left = agent_adds_left(ctx, day_start)?;
    let added = results.iter().filter(|r| r.status == "added").count();
    let mut counts: Vec<(&str, usize)> = Vec::new();
    for result in &results {
        match counts.iter_mut().find(|(s, _)| *s == result.status) {
            Some((_, n)) => *n += 1,
            None => counts.push((result.status, 1)),
        }
    }
    let mut text = counts
        .iter()
        .map(|(status, n)| format!("{n} {}", status.replace('_', " ")))
        .collect::<Vec<_>>()
        .join(" · ");
    text.push_str(&format!(". Agents may add {left} more words today."));
    for result in &results {
        text.push_str(&format!("\n- {}: {}", result.word, result.status));
        if let Some(cached) = &result.entry {
            text.push_str(&format!(
                " ({} — {})",
                cached.entry.word,
                cached.entry.short_summary()
            ));
        }
        if let Some(message) = &result.message {
            text.push_str(&format!(". {message}"));
        }
    }
    let nothing_done = results.iter().all(|r| {
        matches!(
            r.status,
            "not_found" | "network_error" | "deferred" | "invalid"
        )
    });
    if let Some(failure) = &failure {
        text = format!("{failure}\n{text}");
    }
    let mut output = ToolOutput::ok(
        text,
        json!({
            "results": results.iter().map(WordResult::to_json).collect::<Vec<_>>(),
            "added": added,
            "agent_adds_left": left,
        }),
    );
    output.is_error = failure.is_some() || nothing_done;
    Ok(output)
}

fn add_one(
    ctx: &Context,
    request: &WordRequest,
    day_start: DateTime<Utc>,
    started: Instant,
) -> anyhow::Result<WordResult> {
    let typed = request.word.as_str();
    let word = match check_word(typed) {
        Ok(word) => word,
        Err(message) => return Ok(WordResult::new(typed, "invalid").with_message(message)),
    };
    let reason = match check_reason(request.reason.as_deref()) {
        Ok(reason) => reason,
        Err(message) => return Ok(WordResult::new(typed, "invalid").with_message(message)),
    };
    let limit = ctx.config.mcp_daily_add_limit;
    let cached = match ctx.db.find_cached(&normalize_headword(&word))? {
        Some(cached) => cached,
        None => {
            if agent_adds_left(ctx, day_start)? == 0 {
                return Ok(limit_reached(typed, limit));
            }
            let left = ADD_BUDGET.saturating_sub(started.elapsed());
            if left < Duration::from_secs(1) {
                return Ok(WordResult::new(typed, "deferred").with_message(
                    "Not looked up: this call ran out of time. Call add_words again with it.",
                ));
            }
            match ctx.dictionary_within(ctx.timeout().min(left))?.fetch(&word) {
                Ok(fetched) => store_fetched(ctx, &word, fetched)?.0,
                Err(DictionaryError::NotFound(_)) => {
                    return Ok(WordResult::new(typed, "not_found")
                        .with_message("Wiktionary has no English entry for it; not added."))
                }
                Err(err) => {
                    let next = if err.is_transient() {
                        "Not added; try again later."
                    } else {
                        "Not added; trying again will not help."
                    };
                    return Ok(WordResult::new(typed, "network_error")
                        .with_message(format!("{err}. {next}")));
                }
            }
        }
    };
    let outcome = ctx
        .db
        .add_from_agent(cached.id, reason, limit, day_start, Utc::now())?;
    let mut result = match outcome {
        AgentAddOutcome::Added => {
            let result = WordResult::new(typed, "added");
            if cached.entry.has_audio() {
                result
            } else {
                result.with_message(
                    "No pronunciation recording, so it will not appear in listening practice.",
                )
            }
        }
        AgentAddOutcome::AlreadySaved => WordResult::new(typed, "already_saved"),
        AgentAddOutcome::Dismissed => WordResult::new(typed, "dismissed")
            .with_message("The user removed this word from their list. Do not suggest it again."),
        AgentAddOutcome::LimitReached => limit_reached(typed, limit),
    };
    result.entry = Some(cached);
    Ok(result)
}

fn limit_reached(word: &str, limit: u32) -> WordResult {
    let message = if limit == 0 {
        "The user has turned off adding words by agents (mcp_daily_add_limit = 0).".to_string()
    } else {
        format!(
            "Agents already added {limit} words today, the user's daily limit \
             (mcp_daily_add_limit). Not added."
        )
    };
    WordResult::new(word, "limit_reached").with_message(message)
}
