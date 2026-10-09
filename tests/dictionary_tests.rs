use std::time::Duration;

use stealthlingo::dictionary::{
    decode_cached, free_dictionary, markup, merriam_webster, normalize_audio_url,
    normalize_headword, wiktionary, DictionaryClient, Source, SourceSettings,
};
use stealthlingo::error::DictionaryError;

const HELLO: &str = include_str!("fixtures/hello.json");
const SPARSE: &str = include_str!("fixtures/sparse.json");
const NOT_FOUND: &str = include_str!("fixtures/not_found.json");
const WIKT_REST: &str = include_str!("fixtures/wiktionary_ephemeral.json");
const WIKT_PAGE: &str = include_str!("fixtures/wiktionary_ephemeral_wikitext.json");
const MW: &str = include_str!("fixtures/mw_ephemeral.json");

const FREE_DICTIONARY_URL: &str = "https://api.dictionaryapi.dev/api/v2/entries/en/";

fn client(source: Source, key: Option<&str>) -> DictionaryClient {
    DictionaryClient::new(
        SourceSettings {
            source,
            free_dictionary_url: FREE_DICTIONARY_URL.to_string(),
            merriam_webster_key: key.map(str::to_string),
        },
        Duration::from_secs(1),
    )
    .unwrap()
}

// ---- Free Dictionary API ----

#[test]
fn parses_full_entry_and_merges_homographs() {
    let entry = free_dictionary::parse("hello", HELLO).unwrap();
    assert_eq!(entry.word, "hello");
    assert_eq!(entry.phonetic.as_deref(), Some("/həˈləʊ/"));
    // The duplicate phonetic from the second array element is merged away.
    assert_eq!(entry.phonetics.len(), 2);
    assert_eq!(
        entry.audio_url(),
        Some("https://ssl.gstatic.com/dictionary/static/sounds/20200429/hello--_gb_1.mp3")
    );
    assert_eq!(entry.parts_of_speech(), vec!["exclamation", "noun", "verb"]);
    assert_eq!(entry.first_example(), Some("hello there, Katie!"));
    let noun = &entry.meanings[1];
    assert_eq!(noun.synonyms, vec!["salutation"]);
    assert_eq!(noun.antonyms, vec!["goodbye"]);
    assert_eq!(
        entry.source_urls,
        vec!["https://en.wiktionary.org/wiki/hello"]
    );
    assert_eq!(entry.license.as_deref(), Some("CC BY-SA 3.0"));
    assert_eq!(
        entry.short_summary(),
        "exclamation: used as a greeting or to begin a phone conversation."
    );
}

#[test]
fn tolerates_missing_and_blank_fields() {
    let entry = free_dictionary::parse("ephemeral", SPARSE).unwrap();
    assert_eq!(entry.word, "ephemeral");
    assert!(entry.phonetic.is_none());
    assert!(entry.phonetics.is_empty());
    assert!(!entry.has_audio());
    assert_eq!(entry.meanings.len(), 1);
    assert_eq!(entry.meanings[0].definitions.len(), 1);
    assert!(entry.first_example().is_none());
    assert!(entry.license.is_none());
}

#[test]
fn entry_without_word_falls_back_to_request() {
    let entry = free_dictionary::parse(" Serendipity ", r#"[{"meanings": []}]"#).unwrap();
    assert_eq!(entry.word, "Serendipity");
    assert_eq!(entry.short_summary(), "(no definition available)");
}

#[test]
fn classifies_free_dictionary_responses() {
    assert_eq!(
        free_dictionary::interpret("qwzx", 404, NOT_FOUND).unwrap_err(),
        DictionaryError::NotFound("qwzx".to_string())
    );
    assert_eq!(
        free_dictionary::interpret("x", 404, "<html>not here</html>").unwrap_err(),
        DictionaryError::Server(404)
    );
    let server = free_dictionary::interpret("x", 522, "error code: 522").unwrap_err();
    assert_eq!(server, DictionaryError::Server(522));
    assert!(server.is_transient());
    assert!(!DictionaryError::NotFound("x".into()).is_transient());
    assert_eq!(
        free_dictionary::interpret("x", 200, "[]").unwrap_err(),
        DictionaryError::NotFound("x".to_string())
    );
    assert!(matches!(
        free_dictionary::interpret("x", 200, "{not json").unwrap_err(),
        DictionaryError::Parse(_)
    ));
    assert_eq!(
        free_dictionary::interpret("hello", 200, HELLO)
            .unwrap()
            .word,
        "hello"
    );
}

// ---- Wiktionary ----

#[test]
fn parses_wiktionary_definitions() {
    let entry = wiktionary::parse_definitions("ephemeral", WIKT_REST).unwrap();
    assert_eq!(entry.word, "ephemeral");
    assert_eq!(entry.parts_of_speech(), vec!["adjective", "noun"]);
    let adjective = &entry.meanings[0];
    // The blank definition is dropped and only English usages are kept.
    assert_eq!(adjective.definitions.len(), 2);
    assert_eq!(
        adjective.definitions[0].definition,
        "Lasting for a short period of time."
    );
    assert_eq!(
        adjective.definitions[0].example.as_deref(),
        Some("The ephemeral beauty of cherry blossoms & snow.")
    );
    assert_eq!(
        adjective.definitions[1].definition,
        "(biology) Existing for only one day, as with some flowers."
    );
    assert_eq!(
        adjective.definitions[1].example.as_deref(),
        Some("Ephemeral flowers open at dawn.")
    );
    assert_eq!(
        entry.source_urls,
        vec!["https://en.wiktionary.org/wiki/ephemeral"]
    );
    assert_eq!(entry.license.as_deref(), Some(wiktionary::LICENSE));
}

#[test]
fn classifies_wiktionary_responses() {
    assert_eq!(
        wiktionary::interpret_definitions("qwzx", 404, r#"{"status":404}"#).unwrap_err(),
        DictionaryError::NotFound("qwzx".to_string())
    );
    assert_eq!(
        wiktionary::interpret_definitions("x", 503, "").unwrap_err(),
        DictionaryError::Server(503)
    );
    // A page that only has non-English sections is not an English word.
    assert_eq!(
        wiktionary::parse_definitions("x", r#"{"fr":[{"definitions":[{"definition":"y"}]}]}"#)
            .unwrap_err(),
        DictionaryError::NotFound("x".to_string())
    );
}

#[test]
fn extracts_pronunciation_and_related_words_from_wikitext() {
    let wikitext = wiktionary::extract_wikitext(WIKT_PAGE).unwrap();
    let extras = wiktionary::parse_wikitext(&wikitext);
    assert_eq!(extras.ipa, vec!["/ɪˈfɛm(ə)ɹəl/", "/əˈfɛm(ə)ɹəl/"]);
    assert_eq!(extras.audio_files, vec!["en-us-ephemeral.ogg"]);
    assert_eq!(
        extras.synonyms,
        vec![
            (
                "adjective".to_string(),
                vec![
                    "transient".to_string(),
                    "fleeting".to_string(),
                    "short-lived".to_string()
                ]
            ),
            ("noun".to_string(), vec!["ephemeron".to_string()]),
        ]
    );
    assert_eq!(
        extras.antonyms,
        vec![(
            "adjective".to_string(),
            vec!["permanent".to_string(), "lasting".to_string()]
        )]
    );

    let mut entry = wiktionary::parse_definitions("ephemeral", WIKT_REST).unwrap();
    wiktionary::apply_extras(&mut entry, extras);
    assert_eq!(entry.phonetic.as_deref(), Some("/ɪˈfɛm(ə)ɹəl/"));
    assert_eq!(
        entry.audio_url(),
        Some("https://commons.wikimedia.org/wiki/Special:FilePath/en-us-ephemeral.ogg")
    );
    assert_eq!(entry.meanings[0].synonyms[0], "transient");
    assert_eq!(entry.meanings[0].antonyms, vec!["permanent", "lasting"]);
    assert_eq!(entry.meanings[1].synonyms, vec!["ephemeron"]);
}

#[test]
fn wikitext_without_english_section_has_no_extras() {
    let extras = wiktionary::parse_wikitext("==French==\n* {{IPA|fr|/a/}}\n");
    assert!(extras.ipa.is_empty() && extras.audio_files.is_empty());
    assert!(wiktionary::extract_wikitext(r#"{"error":{"code":"missingtitle"}}"#).is_none());
}

#[test]
fn builds_wiktionary_urls() {
    assert_eq!(
        wiktionary::page_url("ice cream"),
        "https://en.wiktionary.org/wiki/ice_cream"
    );
    let url = wiktionary::wikitext_url("ice cream");
    assert!(url.as_str().contains("page=ice_cream"), "{url}");
    assert!(url.as_str().contains("prop=wikitext"), "{url}");
    assert_eq!(
        client(Source::Wiktionary, None)
            .entry_url("ice cream")
            .unwrap()
            .as_str(),
        "https://en.wiktionary.org/api/rest_v1/page/definition/ice_cream"
    );
}

// ---- Merriam-Webster ----

#[test]
fn parses_merriam_webster_entries() {
    let entry = merriam_webster::parse("ephemeral", MW).unwrap();
    assert_eq!(entry.word, "ephemeral");
    // `ephemerality` is a different headword and is left out.
    assert_eq!(entry.parts_of_speech(), vec!["adjective", "noun"]);
    assert_eq!(entry.phonetic.as_deref(), Some("\\i-ˈfem-rəl\\"));
    assert_eq!(
        entry.audio_url(),
        Some("https://media.merriam-webster.com/audio/prons/en/us/mp3/e/ephemer01.mp3")
    );
    let adjective = &entry.meanings[0];
    assert_eq!(
        adjective
            .definitions
            .iter()
            .map(|d| d.definition.as_str())
            .collect::<Vec<_>>(),
        vec!["lasting one day only", "lasting a very short time"]
    );
    assert_eq!(
        adjective.definitions[0].example.as_deref(),
        Some("an ephemeral fever")
    );
    assert_eq!(
        entry.source_urls,
        vec!["https://www.merriam-webster.com/dictionary/ephemeral"]
    );
}

#[test]
fn merriam_webster_suggestions_mean_not_found() {
    assert_eq!(
        merriam_webster::parse("ephemerl", r#"["ephemeral","ephemera"]"#).unwrap_err(),
        DictionaryError::NotFound("ephemerl".to_string())
    );
    assert_eq!(
        merriam_webster::parse("x", "[]").unwrap_err(),
        DictionaryError::NotFound("x".to_string())
    );
}

#[test]
fn merriam_webster_key_problems_are_config_errors() {
    let invalid =
        merriam_webster::parse("x", "Invalid API key. Not subscribed for this reference.")
            .unwrap_err();
    assert!(matches!(invalid, DictionaryError::Config(_)));
    assert!(!invalid.is_transient());
    assert!(matches!(
        merriam_webster::interpret("x", 403, "").unwrap_err(),
        DictionaryError::Config(_)
    ));

    let missing = client(Source::MerriamWebster, None)
        .entry_url("x")
        .unwrap_err();
    assert!(
        missing.to_string().contains("merriam_webster_key"),
        "{missing}"
    );
    assert!(client(Source::MerriamWebster, Some("  "))
        .entry_url("x")
        .is_err());
    assert_eq!(
        client(Source::MerriamWebster, Some("abc"))
            .entry_url("ice cream")
            .unwrap()
            .as_str(),
        "https://www.dictionaryapi.com/api/v3/references/collegiate/json/ice%20cream?key=abc"
    );
}

#[test]
fn merriam_webster_audio_directories() {
    let url = |f| merriam_webster::audio_url(f).unwrap();
    assert!(url("bixabc01").ends_with("/bix/bixabc01.mp3"));
    assert!(url("ggwhiz01").ends_with("/gg/ggwhiz01.mp3"));
    assert!(url("3d000001").ends_with("/number/3d000001.mp3"));
    assert!(url("_xyz").ends_with("/number/_xyz.mp3"));
    assert!(url("Hello001").ends_with("/h/Hello001.mp3"));
    assert_eq!(merriam_webster::audio_url(""), None);
}

// ---- Shared ----

#[test]
fn sources_parse_from_config_values() {
    assert_eq!(Source::parse("wiktionary"), Some(Source::Wiktionary));
    assert_eq!(
        Source::parse("Merriam_Webster"),
        Some(Source::MerriamWebster)
    );
    assert_eq!(
        Source::parse(" free-dictionary "),
        Some(Source::FreeDictionary)
    );
    assert_eq!(Source::parse("oxford"), None);
    assert_eq!(Source::default(), Source::Wiktionary);
    assert_eq!(
        Source::names(),
        "wiktionary, free-dictionary, merriam-webster"
    );
}

#[test]
fn decodes_cached_entries_of_every_generation() {
    // Rows from v0.1 only have the Free Dictionary response.
    let legacy = decode_cached("hello", HELLO, None).unwrap();
    assert_eq!(legacy.word, "hello");

    let entry = wiktionary::parse_definitions("ephemeral", WIKT_REST).unwrap();
    let json = serde_json::to_string(&entry).unwrap();
    let decoded = decode_cached("ephemeral", WIKT_REST, Some(&json)).unwrap();
    assert_eq!(decoded, entry);
}

#[test]
fn strips_markup() {
    assert_eq!(
        markup::html_to_text("<i>a</i>&nbsp;&lt;b&gt; &#39;c&#x27;  <br>d"),
        "a <b> 'c' d"
    );
    assert_eq!(
        markup::strip_mw_markup("{bc}a {it}quick{/it} {a_link|fox} {dx}see{/dx}"),
        "a quick fox see"
    );
}

#[test]
fn normalizes_audio_urls() {
    assert_eq!(
        normalize_audio_url("//ssl.gstatic.com/a.mp3").as_deref(),
        Some("https://ssl.gstatic.com/a.mp3")
    );
    assert_eq!(
        normalize_audio_url(" https://x/a.mp3 ").as_deref(),
        Some("https://x/a.mp3")
    );
    assert_eq!(normalize_audio_url(""), None);
    assert_eq!(normalize_audio_url("   "), None);
}

#[test]
fn normalizes_headwords() {
    assert_eq!(normalize_headword("  Ice   Cream "), "ice cream");
    assert_eq!(normalize_headword("O'Clock"), "o'clock");
    assert_eq!(normalize_headword("   "), "");
}

#[test]
fn error_messages_blame_the_right_side() {
    let outage = DictionaryError::Server(522).to_string();
    assert!(outage.contains("down right now"), "{outage}");
    assert!(outage.contains("not your connection"), "{outage}");
    assert!(DictionaryError::Server(522).is_service_side());

    let timeout = DictionaryError::Timeout(10);
    assert!(timeout.to_string().contains("10s"));
    assert!(timeout.is_service_side() && timeout.is_transient());

    let unreachable = DictionaryError::Unreachable("connection timed out".into());
    assert!(unreachable
        .to_string()
        .contains("check your internet connection"));
    assert!(!unreachable.is_service_side() && unreachable.is_transient());

    assert!(DictionaryError::Server(429)
        .to_string()
        .contains("rate-limiting"));
    assert!(DictionaryError::Server(503)
        .to_string()
        .contains("internal error"));
}

#[test]
fn builds_encoded_free_dictionary_urls() {
    let client = client(Source::FreeDictionary, None);
    assert_eq!(client.source(), Source::FreeDictionary);
    assert_eq!(
        client.entry_url("ice cream").unwrap().as_str(),
        "https://api.dictionaryapi.dev/api/v2/entries/en/ice%20cream"
    );
    assert_eq!(
        client.entry_url("a/b").unwrap().as_str(),
        "https://api.dictionaryapi.dev/api/v2/entries/en/a%2Fb"
    );
}
