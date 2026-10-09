use chrono::{DateTime, Duration, TimeZone, Utc};
use stealthlingo::cli::StudyMode;
use stealthlingo::dictionary::{free_dictionary, Entry, Source};
use stealthlingo::learning::session::build_queue;
use stealthlingo::learning::{schedule, Grade, PracticeMode, Status};
use stealthlingo::storage::{AddOutcome, Database, NewAttempt};

const HELLO: &str = include_str!("fixtures/hello.json");
const SPARSE: &str = include_str!("fixtures/sparse.json");

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, 1, 8, 0, 0).unwrap()
}

fn raw_for(word: &str) -> String {
    format!(
        r#"[{{"word":"{word}","meanings":[{{"partOfSpeech":"noun","definitions":[{{"definition":"meaning of {word}"}}]}}]}}]"#
    )
}

/// Caches and saves a word; returns its id.
fn save(db: &Database, word: &str, raw: &str, now: DateTime<Utc>) -> i64 {
    let entry: Entry = free_dictionary::parse(word, raw).unwrap();
    let id = db
        .cache_word(word, &entry, raw, Source::FreeDictionary, now)
        .unwrap();
    assert_eq!(
        db.add_to_collection(id, None, now).unwrap(),
        AddOutcome::Added
    );
    id
}

fn answer(db: &mut Database, word_id: i64, grade: Grade, now: DateTime<Utc>) {
    let item = db
        .list_words(None)
        .unwrap()
        .into_iter()
        .find(|w| w.word_id == word_id)
        .unwrap();
    let state = db
        .due_items(now + Duration::days(3650), false)
        .unwrap()
        .into_iter()
        .chain(db.new_items(1000, false).unwrap())
        .find(|i| i.word_id == word_id)
        .unwrap()
        .schedule;
    let next = schedule(&state, grade, now);
    db.record_attempt(
        &NewAttempt {
            word_id,
            mode: PracticeMode::Spelling,
            expected: &item.display_word,
            submitted: Some(&item.display_word),
            is_correct: grade != Grade::Again,
            grade,
            duration_ms: Some(1200),
        },
        &next,
        now,
    )
    .unwrap();
}

#[test]
fn migrations_run_once_and_persist_data() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested").join("db.sqlite");
    {
        let db = Database::open(&path).unwrap();
        assert_eq!(
            db.schema_version().unwrap(),
            Database::latest_schema_version()
        );
        save(&db, "hello", HELLO, t0());
    }
    let db = Database::open(&path).unwrap();
    assert_eq!(
        db.schema_version().unwrap(),
        Database::latest_schema_version()
    );
    assert_eq!(db.collection_size().unwrap(), 1);
}

#[test]
fn saving_is_idempotent_and_updates_notes() {
    let db = Database::open_in_memory().unwrap();
    let entry = free_dictionary::parse("hello", HELLO).unwrap();
    let id = db
        .cache_word("hello", &entry, HELLO, Source::FreeDictionary, t0())
        .unwrap();
    let again = db
        .cache_word("hello", &entry, HELLO, Source::FreeDictionary, t0())
        .unwrap();
    assert_eq!(id, again);

    assert_eq!(
        db.add_to_collection(id, None, t0()).unwrap(),
        AddOutcome::Added
    );
    assert_eq!(
        db.add_to_collection(id, None, t0()).unwrap(),
        AddOutcome::AlreadySaved
    );
    assert_eq!(
        db.add_to_collection(id, Some("你好"), t0()).unwrap(),
        AddOutcome::NoteUpdated
    );
    assert_eq!(db.collection_size().unwrap(), 1);

    let cached = db.find_cached("hello").unwrap().unwrap();
    assert!(cached.in_collection);
    assert_eq!(cached.note.as_deref(), Some("你好"));
    assert!(cached.entry.has_audio());
    assert!(db.find_cached("missing").unwrap().is_none());
}

#[test]
fn remove_keeps_cache_but_drops_from_list() {
    let db = Database::open_in_memory().unwrap();
    save(&db, "hello", HELLO, t0());
    assert!(db.remove_from_collection("hello").unwrap());
    assert!(!db.remove_from_collection("hello").unwrap());
    assert_eq!(db.collection_size().unwrap(), 0);
    let cached = db.find_cached("hello").unwrap().unwrap();
    assert!(!cached.in_collection);
}

#[test]
fn search_matches_words_and_notes_literally() {
    let db = Database::open_in_memory().unwrap();
    save(&db, "hello", HELLO, t0());
    let id = save(&db, "ephemeral", SPARSE, t0());
    db.add_to_collection(id, Some("短暂的 100%"), t0()).unwrap();
    save(&db, "ice_cream", &raw_for("ice_cream"), t0());

    let names = |q: &str| -> Vec<String> {
        db.list_words(Some(q))
            .unwrap()
            .into_iter()
            .map(|w| w.display_word)
            .collect()
    };
    assert_eq!(names("HEL"), vec!["hello"]);
    assert_eq!(names("短暂"), vec!["ephemeral"]);
    assert_eq!(names("%"), vec!["ephemeral"]);
    assert_eq!(names("_"), vec!["ice_cream"]);
    assert!(names("zzz").is_empty());
    assert_eq!(db.list_words(None).unwrap().len(), 3);
}

#[test]
fn due_and_new_selection_follows_schedule() {
    let mut db = Database::open_in_memory().unwrap();
    let hello = save(&db, "hello", HELLO, t0());
    let ephemeral = save(&db, "ephemeral", SPARSE, t0() + Duration::seconds(1));

    assert!(db.due_items(t0(), false).unwrap().is_empty());
    let new: Vec<i64> = db
        .new_items(10, false)
        .unwrap()
        .iter()
        .map(|i| i.word_id)
        .collect();
    assert_eq!(new, vec![hello, ephemeral]);
    assert_eq!(
        db.new_items(10, true).unwrap().len(),
        1,
        "only hello has audio"
    );

    answer(&mut db, hello, Grade::Good, t0());
    assert_eq!(db.new_items(10, false).unwrap().len(), 1);
    assert!(db
        .due_items(t0() + Duration::hours(23), false)
        .unwrap()
        .is_empty());
    let due = db.due_items(t0() + Duration::days(1), false).unwrap();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].schedule.status, Status::Learning);
    assert_eq!(due[0].schedule.repetitions, 1);
    assert_eq!(db.count_due(t0() + Duration::days(1)).unwrap(), 1);
    assert_eq!(db.next_due_at().unwrap(), Some(t0() + Duration::days(1)));
}

#[test]
fn queue_respects_daily_new_limit_and_audio() {
    let mut db = Database::open_in_memory().unwrap();
    let ids: Vec<i64> = (0..5)
        .map(|i| {
            let word = format!("word{i}");
            save(&db, &word, &raw_for(&word), t0() + Duration::seconds(i))
        })
        .collect();
    let day_start = t0() - Duration::hours(8);

    let queue = build_queue(&db, StudyMode::Memory, false, 3, t0(), day_start).unwrap();
    assert_eq!(queue.len(), 3);

    answer(&mut db, ids[0], Grade::Again, t0());
    answer(&mut db, ids[1], Grade::Good, t0());
    let later = t0() + Duration::minutes(15);
    let queue = build_queue(&db, StudyMode::Memory, false, 3, later, day_start).unwrap();
    let words: Vec<i64> = queue.iter().map(|i| i.word_id).collect();
    // word0 is due again; only one more new word fits under the limit of 3.
    assert_eq!(words, vec![ids[0], ids[2]]);

    let review_only = build_queue(&db, StudyMode::Memory, true, 3, later, day_start).unwrap();
    assert_eq!(review_only.len(), 1);

    let listening = build_queue(&db, StudyMode::Listening, false, 10, later, day_start).unwrap();
    assert!(listening.is_empty(), "none of these words has audio");
}

#[test]
fn attempts_are_recorded_and_counted_in_stats() {
    let mut db = Database::open_in_memory().unwrap();
    let hello = save(&db, "hello", HELLO, t0());
    let ephemeral = save(&db, "ephemeral", SPARSE, t0());
    let yesterday = t0() - Duration::days(1);
    answer(&mut db, hello, Grade::Good, yesterday);
    answer(&mut db, hello, Grade::Again, t0());
    answer(&mut db, ephemeral, Grade::Good, t0());

    let day_start = t0() - Duration::hours(1);
    assert_eq!(db.count_new_started_since(day_start).unwrap(), 1);
    let stats = db.stats(t0(), day_start).unwrap();
    assert_eq!(stats.total_words, 2);
    assert_eq!(stats.attempts_today, 2);
    assert_eq!(stats.correct_today, 1);
    assert_eq!(stats.total_attempts, 3);
    assert_eq!(stats.total_correct, 2);
    assert_eq!(stats.new_words_today, 1);
    assert_eq!(stats.due_now, 0);
    assert!(stats.status_counts.contains(&(Status::Learning, 2)));
    assert!(stats.status_counts.contains(&(Status::New, 0)));
    assert_eq!(stats.next_due, Some(t0() + Duration::minutes(10)));
}

#[test]
fn backup_round_trip_and_idempotent_import() {
    let mut source = Database::open_in_memory().unwrap();
    let hello = save(&source, "hello", HELLO, t0());
    source.add_to_collection(hello, Some("你好"), t0()).unwrap();
    save(&source, "ephemeral", SPARSE, t0());
    let unsaved = free_dictionary::parse("lookup", &raw_for("lookup")).unwrap();
    source
        .cache_word(
            "lookup",
            &unsaved,
            &raw_for("lookup"),
            Source::FreeDictionary,
            t0(),
        )
        .unwrap();
    answer(&mut source, hello, Grade::Good, t0());
    answer(&mut source, hello, Grade::Good, t0() + Duration::days(1));

    let backup = source.export_backup(t0()).unwrap();
    let json = serde_json::to_string(&backup).unwrap();
    let parsed = serde_json::from_str(&json).unwrap();

    let mut target = Database::open_in_memory().unwrap();
    let first = target.import_backup(&parsed).unwrap();
    assert_eq!(first.words_added, 3);
    assert_eq!(first.progress_added, 2);
    assert_eq!(first.attempts_added, 2);

    let second = target.import_backup(&parsed).unwrap();
    assert_eq!(second.words_added, 0);
    assert_eq!(second.progress_updated, 0);
    assert_eq!(second.attempts_added, 0);

    let mut again = target.export_backup(t0()).unwrap();
    let mut original = backup.clone();
    again.words.sort_by(|a, b| a.headword.cmp(&b.headword));
    original.words.sort_by(|a, b| a.headword.cmp(&b.headword));
    assert_eq!(again, original);

    let stats = target.stats(t0() + Duration::days(30), t0()).unwrap();
    assert_eq!(stats.total_words, 2);
    assert_eq!(stats.total_attempts, 2);
}

#[test]
fn import_prefers_more_recent_progress() {
    let mut a = Database::open_in_memory().unwrap();
    let id = save(&a, "hello", HELLO, t0());
    answer(&mut a, id, Grade::Good, t0());
    let older = a.export_backup(t0()).unwrap();
    answer(&mut a, id, Grade::Good, t0() + Duration::days(1));
    let newer = a.export_backup(t0()).unwrap();

    let mut b = Database::open_in_memory().unwrap();
    b.import_backup(&newer).unwrap();
    let summary = b.import_backup(&older).unwrap();
    assert_eq!(summary.progress_updated, 0);
    let item = &b.due_items(t0() + Duration::days(365), false).unwrap()[0];
    assert_eq!(item.schedule.repetitions, 2);
}

#[test]
fn import_rejects_foreign_or_invalid_backups() {
    let mut db = Database::open_in_memory().unwrap();
    let mut backup = db.export_backup(t0()).unwrap();
    backup.format = "something-else".into();
    assert!(db.import_backup(&backup).is_err());

    let mut source = Database::open_in_memory().unwrap();
    let id = save(&source, "hello", HELLO, t0());
    answer(&mut source, id, Grade::Good, t0());
    let mut bad = source.export_backup(t0()).unwrap();
    bad.words[0].attempts[0].grade = "perfect".into();
    assert!(db.import_backup(&bad).is_err());
    assert_eq!(
        db.collection_size().unwrap(),
        0,
        "failed import must roll back"
    );
}

#[test]
fn schema_is_migrated_to_latest_version() {
    let db = Database::open_in_memory().unwrap();
    assert_eq!(db.schema_version().unwrap(), 3);
    assert_eq!(Database::latest_schema_version(), 3);
}

#[test]
fn aliases_resolve_to_the_dictionary_headword() {
    let db = Database::open_in_memory().unwrap();
    let run = save(&db, "run", &raw_for("run"), t0());
    db.add_alias("ran", run).unwrap();
    // Aliasing a word to its own headword is a no-op.
    db.add_alias("run", run).unwrap();

    let cached = db.find_cached("ran").unwrap().unwrap();
    assert_eq!(cached.id, run);
    assert_eq!(cached.headword, "run");
    assert!(cached.in_collection);
    assert_eq!(db.list_words(None).unwrap().len(), 1);

    // A real entry for the typed form wins over the alias.
    let raw = raw_for("ran");
    let entry = free_dictionary::parse("ran", &raw).unwrap();
    let ran = db
        .cache_word("ran", &entry, &raw, Source::FreeDictionary, t0())
        .unwrap();
    assert_eq!(db.find_cached("ran").unwrap().unwrap().id, ran);
    assert!(
        !db.remove_from_collection("ran").unwrap(),
        "ran is not saved"
    );

    let db = Database::open_in_memory().unwrap();
    let run = save(&db, "run", &raw_for("run"), t0());
    db.add_alias("ran", run).unwrap();
    assert!(db.remove_from_collection("ran").unwrap());
    assert_eq!(db.collection_size().unwrap(), 0);
}

#[test]
fn caches_entries_from_other_sources() {
    let db = Database::open_in_memory().unwrap();
    let raw = include_str!("fixtures/wiktionary_ephemeral.json");
    let entry = stealthlingo::dictionary::wiktionary::parse_definitions("ephemeral", raw).unwrap();
    let id = db
        .cache_word("ephemeral", &entry, raw, Source::Wiktionary, t0())
        .unwrap();
    db.add_to_collection(id, None, t0()).unwrap();

    let cached = db.find_cached("ephemeral").unwrap().unwrap();
    assert_eq!(cached.source, Source::Wiktionary);
    assert_eq!(cached.entry, entry);
    let listed = &db.list_words(None).unwrap()[0];
    assert_eq!(listed.summary, entry.short_summary());
    assert_eq!(db.new_items(10, false).unwrap()[0].entry, entry);

    // Re-fetching from another source replaces the cached entry.
    let hello = free_dictionary::parse("ephemeral", SPARSE).unwrap();
    db.cache_word("ephemeral", &hello, SPARSE, Source::FreeDictionary, t0())
        .unwrap();
    let cached = db.find_cached("ephemeral").unwrap().unwrap();
    assert_eq!(cached.source, Source::FreeDictionary);
    assert!(cached.in_collection);

    let mut target = Database::open_in_memory().unwrap();
    let backup = db.export_backup(t0()).unwrap();
    assert_eq!(backup.words[0].source, "free-dictionary");
    target.import_backup(&backup).unwrap();
    assert_eq!(
        target.find_cached("ephemeral").unwrap().unwrap().entry,
        hello
    );
}

#[test]
fn imports_version_1_backups() {
    let raw = serde_json::to_string(HELLO).unwrap();
    let raw = raw.trim_matches('"');
    let json = format!(
        r#"{{"format":"stealthlingo-backup","version":1,"exported_at":"2026-03-01T08:00:00.000000Z",
            "words":[{{"language_code":"en","headword":"hello","display_word":"hello",
            "raw_response_json":"{raw}","has_audio":true,
            "source_fetched_at":"2026-03-01T08:00:00.000000Z","created_at":"2026-03-01T08:00:00.000000Z"}}]}}"#
    );
    let backup = serde_json::from_str(&json).unwrap();
    let mut db = Database::open_in_memory().unwrap();
    let summary = db.import_backup(&backup).unwrap();
    assert_eq!(summary.words_added, 1);
    let cached = db.find_cached("hello").unwrap().unwrap();
    assert_eq!(cached.source, Source::FreeDictionary);
    assert!(cached.entry.has_audio());
}
