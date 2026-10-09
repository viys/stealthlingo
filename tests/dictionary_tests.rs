use std::time::Duration;

use stealthlingo::commands::links::{self, LinkAction};
use stealthlingo::commands::lookup::{
    next_steps, render_entry, render_entry_to_width, wrap, Detail,
};
use stealthlingo::dictionary::{
    accent_from_audio_url, accent_name, decode_cached, legacy, markup, normalize_audio_url,
    normalize_headword, wiktionary, Definition, DictionaryClient, Endpoints, Entry, Meaning,
    Phonetic,
};
use stealthlingo::error::DictionaryError;

/// Free Dictionary responses as cached by v0.1.
const HELLO: &str = include_str!("fixtures/hello.json");
const SPARSE: &str = include_str!("fixtures/sparse.json");
const WIKT_REST: &str = include_str!("fixtures/wiktionary_ephemeral.json");
const WIKT_PAGE: &str = include_str!("fixtures/wiktionary_ephemeral_wikitext.json");

fn client() -> DictionaryClient {
    DictionaryClient::new(Endpoints::default(), Duration::from_secs(1)).unwrap()
}

// ---- Data cached by v0.1 (Free Dictionary responses) ----

#[test]
fn parses_full_entry_and_merges_homographs() {
    let entry = legacy::parse("hello", HELLO).unwrap();
    assert_eq!(entry.word, "hello");
    assert_eq!(entry.phonetic.as_deref(), Some("/həˈləʊ/"));
    // The duplicate phonetic from the second array element is merged away.
    assert_eq!(entry.phonetics.len(), 2);
    assert_eq!(
        entry.audio_url(),
        Some("https://ssl.gstatic.com/dictionary/static/sounds/20200429/hello--_gb_1.mp3")
    );
    assert_eq!(entry.phonetics[0].accent.as_deref(), Some("UK"));
    assert_eq!(entry.phonetics[1].accent, None);
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
    let entry = legacy::parse("ephemeral", SPARSE).unwrap();
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
    let entry = legacy::parse(" Serendipity ", r#"[{"meanings": []}]"#).unwrap();
    assert_eq!(entry.word, "Serendipity");
    assert_eq!(entry.short_summary(), "(no definition available)");
    assert!(matches!(
        legacy::parse("x", "{not json").unwrap_err(),
        DictionaryError::Parse(_)
    ));
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
    assert!(DictionaryError::Server(503).is_transient());
    assert!(!DictionaryError::NotFound("x".into()).is_transient());
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
    assert_eq!(
        extras.ipa,
        vec![
            accented("/ɪˈfɛm(ə)ɹəl/", Some("UK")),
            accented("/əˈfɛm(ə)ɹəl/", Some("UK")),
            accented("/ɪˈfɛm(ə)ɹəl/", Some("US")),
        ]
    );
    assert_eq!(
        extras.audio_files,
        vec![accented("en-us-ephemeral.ogg", Some("US"))]
    );
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

fn accented(value: &str, accent: Option<&str>) -> (String, Option<String>) {
    (value.to_string(), accent.map(str::to_string))
}

#[test]
fn labels_wiktionary_pronunciations_with_accents() {
    let wikitext = "==English==\n===Pronunciation===\n\
        * {{IPA|en|/ˈhɑstəl/|a=GA}}\n\
        ** {{audio|en|En-us-hostel.ogg}}\n\
        * {{IPA|en|/ˈhɒstəl/|a=RP}}\n\
        * {{IPA|en|/ˈhɔsʈəl/|a=Indic}}\n\
        * {{a|UK|US}} {{IPA|en|/x/}}\n\
        * {{IPA|en|/y/}}\n\
        * {{audio|en|LL-Q1860 (eng)-hostel.wav}}\n";
    let extras = wiktionary::parse_wikitext(wikitext);
    assert_eq!(
        extras.ipa,
        vec![
            accented("/ˈhɑstəl/", Some("US")),
            accented("/ˈhɒstəl/", Some("UK")),
            accented("/ˈhɔsʈəl/", Some("India")),
            accented("/x/", Some("UK")),
            accented("/x/", Some("US")),
            accented("/y/", None),
        ]
    );
    // The sub-bullet recording inherits the accent of the transcription above it.
    assert_eq!(
        extras.audio_files,
        vec![
            accented("En-us-hostel.ogg", Some("US")),
            accented("LL-Q1860 (eng)-hostel.wav", None),
        ]
    );
}

#[test]
fn recognises_accent_labels_and_audio_file_names() {
    assert_eq!(accent_name("RP"), Some("UK"));
    assert_eq!(accent_name(" General American "), Some("US"));
    assert_eq!(accent_name("cot-caught"), None);
    assert_eq!(
        accent_from_audio_url("https://example.com/sounds/hello-uk.mp3"),
        Some("UK")
    );
    assert_eq!(
        accent_from_audio_url("https://example.com/hello--_gb_1.mp3"),
        Some("UK")
    );
    assert_eq!(accent_from_audio_url("En-us-hostel.ogg"), Some("US"));
    assert_eq!(accent_from_audio_url("us.mp3"), None);
    assert_eq!(
        accent_from_audio_url("https://example.com/ephemer01.mp3"),
        None
    );
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
    let url = wiktionary::wikitext_url(wiktionary::ACTION_API, "ice cream").unwrap();
    assert!(url.as_str().contains("page=ice_cream"), "{url}");
    assert!(url.as_str().contains("prop=wikitext"), "{url}");
    assert!(wiktionary::wikitext_url("not a url", "x").is_err());
    assert_eq!(
        client().entry_url("ice cream").unwrap().as_str(),
        "https://en.wiktionary.org/api/rest_v1/page/definition/ice_cream"
    );
    assert_eq!(
        client().entry_url("a/b").unwrap().as_str(),
        "https://en.wiktionary.org/api/rest_v1/page/definition/a%2Fb"
    );
}

#[test]
fn unreachable_wiktionary_is_a_transient_error() {
    let endpoints = Endpoints {
        rest_base: "http://127.0.0.1:9/definition".to_string(),
        action_api: "http://127.0.0.1:9/api.php".to_string(),
    };
    let err = DictionaryClient::new(endpoints, Duration::from_secs(2))
        .unwrap()
        .fetch("hello")
        .unwrap_err();
    assert!(err.is_transient(), "{err}");
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

// ---- Lookup rendering ----

fn phonetic(text: Option<&str>, audio: Option<&str>, accent: Option<&str>) -> Phonetic {
    Phonetic {
        text: text.map(str::to_string),
        audio_url: audio.map(str::to_string),
        accent: accent.map(str::to_string),
    }
}

#[test]
fn renders_uk_and_us_pronunciations_on_one_line() {
    let entry = Entry {
        word: "hostel".to_string(),
        phonetics: vec![
            phonetic(Some("/ˈhɑstəl/"), None, Some("US")),
            phonetic(None, Some("https://x/En-us-hostel.ogg"), Some("US")),
            phonetic(Some("/ˈhɒstəl/"), None, Some("UK")),
            phonetic(Some("/ˈhɒstl̩/"), None, Some("UK")),
            phonetic(Some("/ˈhɔsʈəl/"), None, Some("India")),
            phonetic(Some("/n(j)ʉw/"), None, None),
        ],
        ..Entry::default()
    };
    let out = render_entry(&entry, Detail::Brief, false);
    assert!(
        out.starts_with("hostel\n\nPronunciation: UK /ˈhɒstəl/ · US /ˈhɑstəl/\n"),
        "{out}"
    );
    assert_eq!(
        next_steps(&entry, false, false),
        "Save it: `stealthlingo add hostel`. Listen: `stealthlingo audio hostel` (US)."
    );
    assert_eq!(
        next_steps(&entry, true, false),
        "Saved in your word list. Listen: `stealthlingo audio hostel` (US)."
    );
}

#[test]
fn links_only_pronunciations_that_have_a_recording() {
    let entry = Entry {
        word: "ice cream".to_string(),
        phonetics: vec![
            phonetic(Some("/ˈaɪs kɹiːm/"), None, Some("UK")),
            phonetic(Some("/ˈaɪs ˌkɹim/"), None, Some("US")),
            phonetic(None, Some("https://x/En-us-ice_cream.ogg"), Some("US")),
        ],
        ..Entry::default()
    };
    let us_link = links::audio_link("ice cream", Some("US"));
    assert_eq!(us_link, "stealthlingo://audio/ice%20cream?accent=US");
    assert_eq!(
        render_entry(&entry, Detail::Brief, true)
            .lines()
            .nth(2)
            .unwrap(),
        format!(
            "Pronunciation: UK /ˈaɪs kɹiːm/ · {}",
            links::hyperlink(&us_link, "US /ˈaɪs ˌkɹim/")
        )
    );
    // The pronunciation is clickable, so the closing line only adds or removes.
    assert_eq!(
        next_steps(&entry, false, true),
        links::hyperlink("stealthlingo://add/ice%20cream", "+ Add to word list")
    );
    assert_eq!(
        next_steps(&entry, true, true),
        format!(
            "Saved in your word list · {}",
            links::hyperlink("stealthlingo://remove/ice%20cream", "Remove")
        )
    );

    // Recordings exist, but none for the transcription shown: keep the command.
    let entry = Entry {
        word: "hi".to_string(),
        phonetics: vec![
            phonetic(Some("/haːj/"), None, Some("India")),
            phonetic(None, Some("https://x/hi-au.ogg"), Some("Australia")),
        ],
        ..Entry::default()
    };
    assert_eq!(
        next_steps(&entry, false, true),
        format!(
            "{} · Listen: `stealthlingo audio hi` (Australia)",
            links::hyperlink("stealthlingo://add/hi", "+ Add to word list")
        )
    );
}

#[test]
fn parses_links() {
    assert_eq!(
        LinkAction::parse("stealthlingo://audio/ice%20cream?accent=US").unwrap(),
        LinkAction::Audio {
            word: "ice cream".to_string(),
            accent: Some("US".to_string())
        }
    );
    for action in [
        LinkAction::Audio {
            word: "o'clock".to_string(),
            accent: None,
        },
        LinkAction::Add("ice cream".to_string()),
        LinkAction::Remove("café".to_string()),
    ] {
        assert_eq!(LinkAction::parse(&action.link()).unwrap(), action);
    }
    assert!(LinkAction::parse("https://audio/hello").is_err());
    assert!(LinkAction::parse("stealthlingo://audio/").is_err());
    assert!(LinkAction::parse("stealthlingo://delete/hello").is_err());
}

#[test]
fn renders_other_and_missing_pronunciations() {
    // The unlabelled transcription is the general one and beats a minor accent.
    let entry = Entry {
        word: "hi".to_string(),
        phonetics: vec![
            phonetic(Some("/haɪ/"), None, None),
            phonetic(None, Some("https://x/en-us-hi.ogg"), Some("US")),
            phonetic(None, Some("https://x/En-uk-hi.ogg"), Some("UK")),
            phonetic(Some("/haːj/"), None, Some("India")),
        ],
        ..Entry::default()
    };
    assert!(render_entry(&entry, Detail::Brief, false).contains("Pronunciation: /haɪ/\n"));
    assert_eq!(
        next_steps(&entry, false, false),
        "Save it: `stealthlingo add hi`. Listen: `stealthlingo audio hi --accent uk|us`."
    );

    let entry = Entry {
        word: "g'day".to_string(),
        phonetics: vec![phonetic(Some("/ɡəˈdæɪ/"), None, Some("Australia"))],
        ..Entry::default()
    };
    assert!(
        render_entry(&entry, Detail::Brief, false).contains("Pronunciation: Australia /ɡəˈdæɪ/\n")
    );

    let entry = Entry {
        word: "ice cream".to_string(),
        phonetics: vec![
            phonetic(Some("/a/"), Some("https://x/ice-cream.mp3"), None),
            phonetic(Some("/b/"), None, None),
        ],
        ..Entry::default()
    };
    assert!(render_entry(&entry, Detail::Brief, false).contains("Pronunciation: /a/, /b/\n"));
    assert_eq!(
        next_steps(&entry, false, false),
        "Save it: `stealthlingo add \"ice cream\"`. Listen: `stealthlingo audio \"ice cream\"`."
    );

    let entry = Entry {
        word: "x".to_string(),
        ..Entry::default()
    };
    assert!(render_entry(&entry, Detail::Brief, false).contains("Pronunciation: not available\n"));
    assert_eq!(
        next_steps(&entry, false, false),
        "Run `stealthlingo add x` to save it for practice."
    );
}

fn meaning(part_of_speech: &str, definitions: usize, synonyms: &[&str]) -> Meaning {
    Meaning {
        part_of_speech: Some(part_of_speech.to_string()),
        definitions: (1..=definitions)
            .map(|i| Definition {
                definition: format!("{part_of_speech} sense {i}"),
                example: (i == 1).then(|| format!("{part_of_speech} example")),
            })
            .collect(),
        synonyms: synonyms.iter().map(|s| s.to_string()).collect(),
        ..Meaning::default()
    }
}

fn run_entry() -> Entry {
    Entry {
        word: "run".to_string(),
        meanings: vec![
            meaning("symbol", 1, &[]),
            meaning("verb", 5, &["sprint"]),
            meaning("noun", 2, &[]),
            meaning("adjective", 1, &[]),
            meaning("adverb", 1, &[]),
        ],
        source_urls: vec!["https://en.wiktionary.org/wiki/run".to_string()],
        license: Some("CC BY-SA 4.0".to_string()),
        ..Entry::default()
    }
}

#[test]
fn brief_lookup_shows_a_few_definitions_of_the_main_parts_of_speech() {
    let entry = run_entry();
    let out = render_entry(&entry, Detail::Brief, false);
    assert_eq!(
        out,
        "run\n\nPronunciation: not available\n\
         \nverb\n  1. verb sense 1\n  2. verb sense 2\n  3. verb sense 3\n\
         \nnoun\n  1. noun sense 1\n  2. noun sense 2\n\
         \nadjective\n  1. adjective sense 1\n"
    );
    let full = render_entry_to_width(&entry, Detail::Full, false, 80);
    assert_eq!(
        full,
        "run\n\nPronunciation: not available\n\
         \nsymbol\n  1. symbol sense 1\n     e.g. symbol example\n\
         \nverb\n  1. verb sense 1\n     e.g. verb example\n  2. verb sense 2\n  3. verb sense 3\n  \
         4. verb sense 4\n  5. verb sense 5\n  Synonyms: sprint\n\
         \nnoun\n  1. noun sense 1\n     e.g. noun example\n  2. noun sense 2\n\
         \nadjective\n  1. adjective sense 1\n     e.g. adjective example\n\
         \nadverb\n  1. adverb sense 1\n     e.g. adverb example\n\
         \nSource: https://en.wiktionary.org/wiki/run (CC BY-SA 4.0)\n"
    );
}

#[test]
fn full_lookup_aligns_numbers_and_wraps_long_lines() {
    let entry = Entry {
        word: "set".to_string(),
        meanings: vec![meaning("verb", 10, &[])],
        ..Entry::default()
    };
    let full = render_entry_to_width(&entry, Detail::Full, false, 80);
    assert!(
        full.contains("\n   1. verb sense 1\n      e.g. verb example\n"),
        "{full}"
    );
    assert!(
        full.contains("\n   9. verb sense 9\n  10. verb sense 10\n"),
        "{full}"
    );

    assert_eq!(
        wrap("To move forward quickly upon two feet.", "  1. ", 24),
        "  1. To move forward\n     quickly upon two\n     feet.\n"
    );
    assert_eq!(
        wrap("https://example.com/a-very-long-url x", "  ", 10),
        "  https://example.com/a-very-long-url\n  x\n"
    );
}

#[test]
fn brief_lookup_keeps_symbols_when_nothing_else_is_left() {
    let entry = Entry {
        word: "ISO".to_string(),
        meanings: vec![Meaning {
            part_of_speech: Some("symbol".to_string()),
            definitions: vec![Definition {
                definition: "A language code.".to_string(),
                ..Definition::default()
            }],
            ..Meaning::default()
        }],
        source_urls: vec!["https://en.wiktionary.org/wiki/ISO".to_string()],
        ..Entry::default()
    };
    assert!(
        render_entry(&entry, Detail::Brief, false).ends_with("\nsymbol\n  1. A language code.\n")
    );
}

#[test]
fn prefers_uk_or_us_recordings() {
    let entry = Entry {
        phonetics: vec![
            phonetic(None, Some("au.mp3"), Some("Australia")),
            phonetic(None, Some("us.mp3"), Some("US")),
        ],
        ..Entry::default()
    };
    assert_eq!(entry.audio_url(), Some("us.mp3"));
}
