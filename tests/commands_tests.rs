use chrono::Utc;
use stealthlingo::commands::{cached_or_fetch, Context, Freshness};
use stealthlingo::config::Paths;
use stealthlingo::dictionary::{free_dictionary, Source};

const HELLO: &str = include_str!("fixtures/hello.json");

/// A context whose Free Dictionary URL refuses connections, so any network
/// request fails fast without leaving the machine.
fn offline_context(dir: &tempfile::TempDir) -> Context {
    let mut ctx = Context::load_from(Paths::at(dir.path().to_path_buf())).unwrap();
    ctx.config.dictionary_source = Source::FreeDictionary;
    ctx.config.free_dictionary_url = "http://127.0.0.1:9/entries".to_string();
    ctx.config.http_timeout_secs = 2;
    let entry = free_dictionary::parse("hello", HELLO).unwrap();
    ctx.db
        .cache_word("hello", &entry, HELLO, Source::FreeDictionary, Utc::now())
        .unwrap();
    ctx
}

#[test]
fn cached_words_are_used_without_an_override() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = offline_context(&dir);
    let (_, freshness) = cached_or_fetch(&ctx, "Hello").unwrap();
    assert!(matches!(freshness, Freshness::Cached));

    ctx.source_override = Some(Source::FreeDictionary);
    let (_, freshness) = cached_or_fetch(&ctx, "hello").unwrap();
    assert!(matches!(freshness, Freshness::Cached));
}

#[test]
fn a_different_source_override_refreshes_cached_words() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = offline_context(&dir);
    // Point the "other" source at the same dead endpoint by overriding with
    // Free Dictionary while the cached copy claims another source.
    let entry = free_dictionary::parse("hello", HELLO).unwrap();
    ctx.db
        .cache_word("hello", &entry, HELLO, Source::MerriamWebster, Utc::now())
        .unwrap();
    ctx.source_override = Some(Source::FreeDictionary);

    let (cached, freshness) = cached_or_fetch(&ctx, "hello").unwrap();
    assert!(
        matches!(freshness, Freshness::Fallback(_)),
        "the override must trigger a network refresh"
    );
    assert_eq!(cached.source, Source::MerriamWebster);
}
