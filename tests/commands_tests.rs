use std::io::{Read, Write};
use std::net::TcpListener;

use chrono::Utc;
use stealthlingo::commands::links::{self, LinkAction};
use stealthlingo::commands::{cached_or_fetch, fetch_word, Context, Freshness};
use stealthlingo::config::Paths;
use stealthlingo::dictionary::{self, legacy, wiktionary, Endpoints};
use stealthlingo::storage::AddOutcome;

const HELLO: &str = include_str!("fixtures/hello.json");
const WIKT_REST: &str = include_str!("fixtures/wiktionary_ephemeral.json");

/// A context whose Wiktionary endpoints refuse connections, so any network
/// request fails fast without leaving the machine.
fn offline_context(dir: &tempfile::TempDir) -> Context {
    let mut ctx = Context::load_from(Paths::at(dir.path().to_path_buf())).unwrap();
    ctx.endpoints = Endpoints {
        rest_base: "http://127.0.0.1:9/definition".to_string(),
        action_api: "http://127.0.0.1:9/api.php".to_string(),
    };
    ctx.config.http_timeout_secs = 2;
    ctx
}

#[test]
fn cached_wiktionary_words_are_used_without_the_network() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = offline_context(&dir);
    let entry = wiktionary::parse_definitions("ephemeral", WIKT_REST).unwrap();
    ctx.db
        .cache_word(
            "ephemeral",
            &entry,
            WIKT_REST,
            dictionary::SOURCE,
            Utc::now(),
        )
        .unwrap();

    let (cached, freshness) = cached_or_fetch(&ctx, "Ephemeral").unwrap();
    assert!(matches!(freshness, Freshness::Cached));
    assert_eq!(cached.entry, entry);
}

#[test]
fn words_cached_by_older_versions_are_refreshed_but_kept_when_offline() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = offline_context(&dir);
    let backup = format!(
        r#"{{"format":"stealthlingo-backup","version":1,"exported_at":"2026-03-01T08:00:00.000000Z",
            "words":[{{"language_code":"en","headword":"hello","display_word":"hello",
            "raw_response_json":{raw},"has_audio":true,
            "source_fetched_at":"2026-03-01T08:00:00.000000Z","created_at":"2026-03-01T08:00:00.000000Z"}}]}}"#,
        raw = serde_json::to_string(HELLO).unwrap()
    );
    ctx.db
        .import_backup(&serde_json::from_str(&backup).unwrap())
        .unwrap();

    let (cached, freshness) = cached_or_fetch(&ctx, "hello").unwrap();
    assert!(
        matches!(freshness, Freshness::Fallback(_)),
        "an entry from another dictionary must trigger a Wiktionary refresh"
    );
    assert_ne!(cached.source, dictionary::SOURCE);
    assert_eq!(cached.entry, legacy::parse("hello", HELLO).unwrap());
}

#[test]
fn add_and_remove_links_change_the_word_list() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = offline_context(&dir);
    let entry = wiktionary::parse_definitions("ephemeral", WIKT_REST).unwrap();
    ctx.db
        .cache_word(
            "ephemeral",
            &entry,
            WIKT_REST,
            dictionary::SOURCE,
            Utc::now(),
        )
        .unwrap();
    let saved = || cached_or_fetch(&ctx, "ephemeral").unwrap().0.in_collection;

    links::open(&ctx, &LinkAction::Add("ephemeral".to_string()).link()).unwrap();
    assert!(saved());
    // Clicking again is harmless.
    links::open(&ctx, &LinkAction::Add("ephemeral".to_string()).link()).unwrap();
    assert!(saved());

    links::open(&ctx, &LinkAction::Remove("Ephemeral".to_string()).link()).unwrap();
    assert!(!saved());
    links::open(&ctx, &LinkAction::Remove("ephemeral".to_string()).link()).unwrap();

    // A link removal is recoverable: adding the word again restores it.
    let id = cached_or_fetch(&ctx, "ephemeral").unwrap().0.id;
    assert_eq!(
        ctx.db.add_to_collection(id, None, Utc::now()).unwrap(),
        AddOutcome::Restored
    );
}

/// Answers every request on a local port with the first route whose path
/// prefix matches (404 otherwise). Returns the base URL.
fn serve(routes: Vec<(&'static str, u16, String)>) -> String {
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
            let request = String::from_utf8_lossy(&request);
            let path = request.split_whitespace().nth(1).unwrap_or("");
            let (status, body) = routes
                .iter()
                .find(|(prefix, _, _)| path.starts_with(prefix))
                .map_or((404, ""), |(_, status, body)| (*status, body.as_str()));
            let _ = write!(
                stream,
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    base
}

fn use_server(ctx: &mut Context, base: &str) {
    ctx.endpoints = Endpoints {
        rest_base: format!("{base}/definition"),
        action_api: format!("{base}/api.php"),
    };
}

#[test]
fn entries_fetched_without_pronunciations_are_refreshed_later() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = offline_context(&dir);
    let wikitext_down = serve(vec![
        ("/definition/", 200, WIKT_REST.to_string()),
        ("/api.php", 503, String::new()),
    ]);
    let page = serde_json::json!({ "parse": { "wikitext":
        "==English==\n===Pronunciation===\n* {{IPA|en|/ɪˈfɛm(ə)ɹəl/|a=UK}}\n\
         ** {{audio|en|En-uk-ephemeral.ogg|a=UK}}\n===Adjective===\n" } });
    let working = serve(vec![
        ("/definition/", 200, WIKT_REST.to_string()),
        ("/api.php", 200, page.to_string()),
    ]);

    use_server(&mut ctx, &wikitext_down);
    let (cached, _) = fetch_word(&ctx, "ephemeral").unwrap();
    assert_eq!(cached.source, dictionary::PARTIAL_SOURCE);
    assert!(!cached.entry.has_audio());

    // The next use refreshes it.
    use_server(&mut ctx, &working);
    let (cached, freshness) = cached_or_fetch(&ctx, "ephemeral").unwrap();
    assert!(matches!(freshness, Freshness::Fresh));
    assert_eq!(cached.source, dictionary::SOURCE);
    assert!(cached.entry.has_audio());

    // A later partial fetch does not replace the complete copy.
    use_server(&mut ctx, &wikitext_down);
    let (cached, _) = fetch_word(&ctx, "ephemeral").unwrap();
    assert_eq!(cached.source, dictionary::SOURCE);
    assert!(cached.entry.has_audio());
}

#[test]
fn old_entries_missing_from_wiktionary_are_not_looked_up_again() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = offline_context(&dir);
    let backup = format!(
        r#"{{"format":"stealthlingo-backup","version":1,"exported_at":"2026-03-01T08:00:00.000000Z",
            "words":[{{"language_code":"en","headword":"hello","display_word":"hello",
            "raw_response_json":{raw},"has_audio":true,
            "source_fetched_at":"2026-03-01T08:00:00.000000Z","created_at":"2026-03-01T08:00:00.000000Z"}}]}}"#,
        raw = serde_json::to_string(HELLO).unwrap()
    );
    ctx.db
        .import_backup(&serde_json::from_str(&backup).unwrap())
        .unwrap();
    use_server(&mut ctx, &serve(Vec::new()));

    let (cached, _) = cached_or_fetch(&ctx, "hello").unwrap();
    assert_eq!(cached.entry, legacy::parse("hello", HELLO).unwrap());
    let cached = ctx.db.find_cached("hello").unwrap().unwrap();
    assert_eq!(cached.source, dictionary::SOURCE);
}

#[test]
fn uncached_words_need_the_network() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = offline_context(&dir);
    let Err(err) = cached_or_fetch(&ctx, "hello") else {
        panic!("lookup must fail offline");
    };
    let err = err.to_string();
    assert!(err.contains("not in your local cache"), "{err}");
}
