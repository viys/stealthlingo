use chrono::Utc;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use stealthlingo::cli::{StudyArgs, StudyMode};
use stealthlingo::commands::study::SessionRequest;
use stealthlingo::commands::Context;
use stealthlingo::config::Paths;
use stealthlingo::dictionary::{self, wiktionary};
use stealthlingo::learning::spelling::missing_letters;
use stealthlingo::tui::{App, Start};

const WIKT_REST: &str = include_str!("fixtures/wiktionary_ephemeral.json");

fn context(dir: &tempfile::TempDir, saved: bool) -> Context {
    let ctx = Context::load_from(Paths::at(dir.path().to_path_buf())).unwrap();
    if saved {
        let entry = wiktionary::parse_definitions("ephemeral", WIKT_REST).unwrap();
        let id = ctx
            .db
            .cache_word(
                "ephemeral",
                &entry,
                WIKT_REST,
                dictionary::SOURCE,
                Utc::now(),
            )
            .unwrap();
        ctx.db.add_to_collection(id, None, Utc::now()).unwrap();
    }
    ctx
}

/// Saves copies of the "ephemeral" entry under other headwords.
fn save_words(ctx: &Context, words: &[&str]) {
    let entry = wiktionary::parse_definitions("ephemeral", WIKT_REST).unwrap();
    for word in words {
        let mut entry = entry.clone();
        entry.word = word.to_string();
        let id = ctx
            .db
            .cache_word(word, &entry, WIKT_REST, dictionary::SOURCE, Utc::now())
            .unwrap();
        ctx.db.add_to_collection(id, None, Utc::now()).unwrap();
    }
}

fn set_goal(ctx: &mut Context, goal: u32) {
    ctx.config.daily_goal = goal;
    ctx.save_config().unwrap();
}

fn screen(app: &mut App) -> String {
    screen_sized(app, 100, 32)
}

fn screen_sized(app: &mut App, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| app.render(frame)).unwrap();
    let buffer = terminal.backend().buffer();
    let width = buffer.area.width as usize;
    buffer
        .content()
        .chunks(width)
        .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

fn press(app: &mut App, ctx: &mut Context, code: KeyCode) {
    app.handle_key(ctx, KeyEvent::new(code, KeyModifiers::NONE));
}

fn type_text(app: &mut App, ctx: &mut Context, text: &str) {
    for c in text.chars() {
        press(app, ctx, KeyCode::Char(c));
    }
}

fn session(mode: StudyMode) -> Start {
    Start::Session(SessionRequest {
        mode,
        ..SessionRequest::study(&StudyArgs::default())
    })
}

fn answers(ctx: &Context) -> i64 {
    let now = Utc::now();
    ctx.db
        .stats(now, now - chrono::Duration::days(1))
        .unwrap()
        .total_attempts
}

#[test]
fn home_explains_an_empty_word_list() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir, false);
    let mut app = App::new(&mut ctx, Start::Home).unwrap();
    let text = screen(&mut app);
    assert!(text.contains("Your word list is empty"), "{text}");
    assert!(text.contains("Esc hide"), "{text}");

    // Starting practice with nothing saved stays on the home screen.
    press(&mut app, &mut ctx, KeyCode::Char('s'));
    assert!(app.flash_text().unwrap().contains("word list is empty"));
    press(&mut app, &mut ctx, KeyCode::Char('q'));
    assert!(app.should_quit());
}

#[test]
fn menu_keys_keep_the_highlight_when_nothing_can_start() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir, true);
    let mut app = App::new(&mut ctx, Start::Home).unwrap();

    // The saved word has no recording, so listening practice cannot start.
    press(&mut app, &mut ctx, KeyCode::Char('L'));
    assert!(app.flash_text().unwrap().contains("listening practice"));
    let home = screen(&mut app);
    assert!(home.contains("› l  Listening practice"), "{home}");

    // Enter retries listening instead of opening another mode.
    press(&mut app, &mut ctx, KeyCode::Enter);
    let home = screen(&mut app);
    assert!(home.contains("Home"), "{home}");
    assert!(app.flash_text().unwrap().contains("listening practice"));
}

#[test]
fn short_terminals_show_the_whole_home_menu() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir, true);
    let mut app = App::new(&mut ctx, Start::Home).unwrap();
    let home = screen_sized(&mut app, 100, 13);
    assert!(home.contains("Answered 0"), "{home}");
    assert!(home.contains("r  Review due words"), "{home}");
    assert!(home.contains("q  Quit"), "{home}");
}

#[test]
fn home_menu_scrolls_to_the_highlighted_item() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir, false);
    let mut app = App::new(&mut ctx, Start::Home).unwrap();
    // Too narrow for two columns.
    let home = screen_sized(&mut app, 60, 12);
    assert!(home.contains("Your word list is empty"), "{home}");
    assert!(!home.contains("Quit"), "{home}");

    press(&mut app, &mut ctx, KeyCode::Up);
    press(&mut app, &mut ctx, KeyCode::Up);
    let home = screen_sized(&mut app, 60, 12);
    assert!(home.contains("› q  Quit"), "{home}");
    let small = screen_sized(&mut app, 40, 12);
    assert!(small.contains("› q  Quit"), "{small}");
}

#[test]
fn short_terminals_keep_the_spelling_answer_visible() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir, true);
    let mut app = App::new(&mut ctx, session(StudyMode::Spelling)).unwrap();
    type_text(&mut app, &mut ctx, "ephem");
    let card = screen_sized(&mut app, 100, 12);
    assert!(card.contains("› ephem"), "{card}");
}

#[test]
fn stats_scroll_when_they_do_not_fit() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir, true);
    let mut app = App::new(&mut ctx, Start::Home).unwrap();
    press(&mut app, &mut ctx, KeyCode::Char('t'));
    let stats = screen_sized(&mut app, 100, 12);
    assert!(stats.contains("↑↓ scroll"), "{stats}");
    assert!(!stats.contains("Accuracy"), "{stats}");
    for _ in 0..20 {
        press(&mut app, &mut ctx, KeyCode::Down);
    }
    let stats = screen_sized(&mut app, 100, 12);
    assert!(stats.contains("Accuracy"), "{stats}");
}

#[test]
fn flashcards_reveal_and_rate_with_single_keys() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir, true);
    let mut app = App::new(&mut ctx, session(StudyMode::Memory)).unwrap();
    let front = screen(&mut app);
    assert!(front.contains("ephemeral"), "{front}");
    assert!(front.contains("Space reveal"), "{front}");
    assert!(
        !front.contains("Lasting"),
        "the meaning must stay hidden: {front}"
    );

    press(&mut app, &mut ctx, KeyCode::Char(' '));
    let back = screen(&mut app);
    assert!(back.contains("adjective"), "{back}");
    assert!(back.contains("1 again"), "{back}");

    press(&mut app, &mut ctx, KeyCode::Char('3'));
    assert_eq!(answers(&ctx), 1);
    let summary = screen(&mut app);
    assert!(summary.contains("All done for now"), "{summary}");
    assert!(summary.contains("1 correct (100%)"), "{summary}");

    press(&mut app, &mut ctx, KeyCode::Enter);
    assert!(screen(&mut app).contains("Home"));
}

#[test]
fn missed_spellings_show_the_difference_and_come_back() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir, true);
    let mut app = App::new(&mut ctx, session(StudyMode::Spelling)).unwrap();
    let prompt = screen(&mut app);
    assert!(prompt.contains("9 letters"), "{prompt}");
    assert!(
        !prompt.contains("ephemeral"),
        "the answer must be masked: {prompt}"
    );

    type_text(&mut app, &mut ctx, "ephemral");
    press(&mut app, &mut ctx, KeyCode::Enter);
    let feedback = screen(&mut app);
    assert!(feedback.contains("Not quite"), "{feedback}");
    assert!(feedback.contains("you typed  ephemral"), "{feedback}");

    // The missed word is asked again at the end of the session.
    press(&mut app, &mut ctx, KeyCode::Enter);
    type_text(&mut app, &mut ctx, "Ephemeral");
    press(&mut app, &mut ctx, KeyCode::Enter);
    assert!(screen(&mut app).contains("Correct"));

    press(&mut app, &mut ctx, KeyCode::Enter);
    let summary = screen(&mut app);
    assert!(summary.contains("Missed"), "{summary}");
    assert!(summary.contains("1 correct (50%)"), "{summary}");
    assert_eq!(answers(&ctx), 2);
}

#[test]
fn ctrl_c_ends_the_session_without_recording_the_open_question() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir, true);
    let mut app = App::new(&mut ctx, session(StudyMode::Spelling)).unwrap();
    type_text(&mut app, &mut ctx, "ephem");
    let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);

    app.handle_key(&mut ctx, ctrl_c);
    assert!(!app.should_quit());
    assert!(screen(&mut app).contains("Session ended"));
    assert_eq!(answers(&ctx), 0);

    app.handle_key(&mut ctx, ctrl_c);
    assert!(app.should_quit());
}

#[test]
fn hiding_keeps_the_session_going() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir, true);
    let mut app = App::new(&mut ctx, session(StudyMode::Memory)).unwrap();
    app.set_hidden(true);
    assert!(app.is_hidden());
    app.set_hidden(false);
    press(&mut app, &mut ctx, KeyCode::Char(' '));
    press(&mut app, &mut ctx, KeyCode::Char('4'));
    assert_eq!(answers(&ctx), 1);
}

#[test]
fn home_sessions_end_once_the_daily_goal_is_reached() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir, false);
    save_words(&ctx, &["alpha", "beta", "gamma"]);
    set_goal(&mut ctx, 2);
    let mut app = App::new(&mut ctx, Start::Home).unwrap();
    let home = screen(&mut app);
    assert!(home.contains("0 / 2 words · 2 to go"), "{home}");

    press(&mut app, &mut ctx, KeyCode::Char('s'));
    assert!(screen(&mut app).contains("goal 0/2"));
    for _ in 0..2 {
        press(&mut app, &mut ctx, KeyCode::Char(' '));
        press(&mut app, &mut ctx, KeyCode::Char('3'));
    }
    let summary = screen(&mut app);
    assert!(summary.contains("That's today's goal"), "{summary}");
    assert!(
        summary.contains("Daily goal reached: 2 words today"),
        "{summary}"
    );
    assert_eq!(answers(&ctx), 2);

    press(&mut app, &mut ctx, KeyCode::Enter);
    let home = screen(&mut app);
    assert!(home.contains("2 / 2 words · reached"), "{home}");
}

#[test]
fn repeated_misses_do_not_count_toward_the_goal() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir, false);
    save_words(&ctx, &["alpha", "beta"]);
    set_goal(&mut ctx, 3);
    let mut app = App::new(&mut ctx, Start::Home).unwrap();
    press(&mut app, &mut ctx, KeyCode::Char('s'));
    // alpha (missed), beta, then alpha again at the end.
    for grade in ['1', '3', '3'] {
        press(&mut app, &mut ctx, KeyCode::Char(' '));
        press(&mut app, &mut ctx, KeyCode::Char(grade));
    }
    assert_eq!(answers(&ctx), 3);
    let summary = screen(&mut app);
    assert!(summary.contains("All done for now"), "{summary}");
    assert!(
        summary.contains("Nothing left to practise today, 1 word short"),
        "{summary}"
    );
}

#[test]
fn reviews_short_of_the_goal_do_not_claim_nothing_is_left() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir, false);
    save_words(&ctx, &["alpha", "beta", "gamma"]);
    rusqlite::Connection::open(ctx.paths.database())
        .unwrap()
        .execute(
            "UPDATE user_words SET status = 'review', due_at = '2020-01-01T00:00:00Z'
             WHERE word_id = (SELECT id FROM words WHERE headword_normalized = 'alpha')",
            [],
        )
        .unwrap();
    set_goal(&mut ctx, 3);
    let mut app = App::new(&mut ctx, Start::Home).unwrap();
    press(&mut app, &mut ctx, KeyCode::Char('r'));
    press(&mut app, &mut ctx, KeyCode::Char(' '));
    press(&mut app, &mut ctx, KeyCode::Char('3'));
    assert_eq!(answers(&ctx), 1);
    let summary = screen(&mut app);
    assert!(summary.contains("All done for now"), "{summary}");
    assert!(
        !summary.contains("Nothing left to practise"),
        "beta and gamma can still be studied: {summary}"
    );
    assert!(
        summary.contains("Today: 1 / 3 words · 2 to go"),
        "{summary}"
    );
}

#[test]
fn sessions_with_explicit_limits_ignore_the_goal() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir, false);
    save_words(&ctx, &["alpha", "beta"]);
    set_goal(&mut ctx, 1);
    let mut app = App::new(&mut ctx, session(StudyMode::Memory)).unwrap();
    for _ in 0..2 {
        press(&mut app, &mut ctx, KeyCode::Char(' '));
        press(&mut app, &mut ctx, KeyCode::Char('3'));
    }
    assert_eq!(answers(&ctx), 2);
    let summary = screen(&mut app);
    assert!(summary.contains("All done for now"), "{summary}");
    assert!(
        summary.contains("Daily goal reached: 2 words today"),
        "{summary}"
    );
}

#[test]
fn missing_letter_questions_show_part_of_the_word() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir, true);
    let mut app = App::new(&mut ctx, session(StudyMode::Letters)).unwrap();
    let card = screen(&mut app);
    let pattern = missing_letters("ephemeral");
    assert!(card.contains(&pattern), "{card}");
    assert!(card.contains("Missing letters"), "{card}");
    assert!(!card.contains("ephemeral"), "{card}");

    type_text(&mut app, &mut ctx, "ephemeral");
    press(&mut app, &mut ctx, KeyCode::Enter);
    assert!(screen(&mut app).contains("Correct"));
    assert_eq!(answers(&ctx), 1);
}

#[test]
fn mixed_practice_starts_new_words_as_flashcards() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir, true);
    let mut app = App::new(&mut ctx, session(StudyMode::Mixed)).unwrap();
    let card = screen(&mut app);
    assert!(card.contains("Mixed · Memory"), "{card}");
    assert!(card.contains("Space reveal"), "{card}");
    press(&mut app, &mut ctx, KeyCode::Char(' '));
    press(&mut app, &mut ctx, KeyCode::Char('1'));
    // The missed word comes back, now asked another way.
    let card = screen(&mut app);
    assert!(card.contains("Mixed · "), "{card}");
    assert_eq!(answers(&ctx), 1);
}

#[test]
fn mistakes_practise_only_words_answered_wrong() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir, false);
    save_words(&ctx, &["alpha", "beta"]);
    let mistakes = || {
        Start::Session(SessionRequest {
            mode: StudyMode::Spelling,
            mistakes: true,
            ..SessionRequest::study(&StudyArgs::default())
        })
    };
    let app = App::new(&mut ctx, mistakes()).unwrap();
    assert!(app
        .flash_text()
        .unwrap()
        .contains("No mistakes to practise"));

    let mut app = App::new(&mut ctx, session(StudyMode::Spelling)).unwrap();
    type_text(&mut app, &mut ctx, "alpah");
    press(&mut app, &mut ctx, KeyCode::Enter);
    press(&mut app, &mut ctx, KeyCode::Enter);
    type_text(&mut app, &mut ctx, "beta");
    press(&mut app, &mut ctx, KeyCode::Enter);

    let mut app = App::new(&mut ctx, mistakes()).unwrap();
    let card = screen(&mut app);
    assert!(card.contains("Mistakes"), "{card}");
    assert!(card.contains("0/1"), "only alpha was missed: {card}");
    type_text(&mut app, &mut ctx, "alpha");
    press(&mut app, &mut ctx, KeyCode::Enter);
    press(&mut app, &mut ctx, KeyCode::Enter);
    assert!(screen(&mut app).contains("All done for now"));
}

fn saved_config(ctx: &Context) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(ctx.paths.config_file()).unwrap()).unwrap()
}

#[test]
fn settings_adjust_values_within_their_range_and_save_at_once() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir, true);
    let mut app = App::new(&mut ctx, Start::Home).unwrap();
    press(&mut app, &mut ctx, KeyCode::Char('c'));
    let settings = screen(&mut app);
    assert!(settings.contains("Settings"), "{settings}");
    assert!(settings.contains("› Daily goal (words)"), "{settings}");
    assert!(settings.contains("0–500, 0 = off"), "{settings}");

    press(&mut app, &mut ctx, KeyCode::Right);
    assert_eq!(app.flash_text(), Some("Saved"));
    assert_eq!(saved_config(&ctx)["daily_goal"], 11);
    press(&mut app, &mut ctx, KeyCode::PageUp);
    assert_eq!(ctx.config.daily_goal, 21);

    // Stops at the bottom of the range.
    for _ in 0..4 {
        press(&mut app, &mut ctx, KeyCode::PageDown);
    }
    assert_eq!(ctx.config.daily_goal, 0);
    press(&mut app, &mut ctx, KeyCode::Left);
    assert_eq!(saved_config(&ctx)["daily_goal"], 0);

    // The accent switches between UK and US.
    for _ in 0..3 {
        press(&mut app, &mut ctx, KeyCode::Down);
    }
    assert!(screen(&mut app).contains("› Default accent"));
    press(&mut app, &mut ctx, KeyCode::Right);
    assert_eq!(saved_config(&ctx)["accent"], "US");

    // A goal of 0 hides the goal on the home screen.
    press(&mut app, &mut ctx, KeyCode::Char('q'));
    let home = screen(&mut app);
    assert!(!home.contains("Daily goal"), "{home}");
}

#[test]
fn typed_settings_outside_the_range_are_not_saved() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir, true);
    let mut app = App::new(&mut ctx, Start::Home).unwrap();
    press(&mut app, &mut ctx, KeyCode::Char('c'));
    press(&mut app, &mut ctx, KeyCode::Enter);
    type_text(&mut app, &mut ctx, "9x99");
    press(&mut app, &mut ctx, KeyCode::Enter);
    assert!(
        app.flash_text().unwrap().contains("0 to 500"),
        "{:?}",
        app.flash_text()
    );
    assert_eq!(ctx.config.daily_goal, 10);
    assert!(!ctx.paths.config_file().exists());

    // Moving away drops the typed value.
    press(&mut app, &mut ctx, KeyCode::Down);
    assert!(screen(&mut app).contains("› New words per day"));
    press(&mut app, &mut ctx, KeyCode::Up);
    press(&mut app, &mut ctx, KeyCode::Enter);
    type_text(&mut app, &mut ctx, "3");
    assert!(screen(&mut app).contains("› 3"));
    press(&mut app, &mut ctx, KeyCode::Enter);
    assert_eq!(saved_config(&ctx)["daily_goal"], 3);

    // The goal is more than one saved word allows today.
    let settings = screen(&mut app);
    assert!(settings.contains("at most 1 word can"), "{settings}");

    press(&mut app, &mut ctx, KeyCode::Backspace);
    let home = screen(&mut app);
    assert!(home.contains("0 / 3 words · 3 to go"), "{home}");
}

#[test]
fn settings_reset_one_or_all_values() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir, true);
    let mut app = App::new(&mut ctx, Start::Home).unwrap();
    press(&mut app, &mut ctx, KeyCode::Char('c'));
    press(&mut app, &mut ctx, KeyCode::Right);
    press(&mut app, &mut ctx, KeyCode::Down);
    press(&mut app, &mut ctx, KeyCode::Left);
    assert_eq!(ctx.config.daily_new_limit, 9);

    press(&mut app, &mut ctx, KeyCode::Char('r'));
    assert_eq!(saved_config(&ctx)["daily_new_limit"], 10);
    assert_eq!(ctx.config.daily_goal, 11);

    press(&mut app, &mut ctx, KeyCode::Char('R'));
    assert!(screen(&mut app).contains("Reset every setting?"));
    press(&mut app, &mut ctx, KeyCode::Char('n'));
    assert_eq!(ctx.config.daily_goal, 11);
    press(&mut app, &mut ctx, KeyCode::Char('R'));
    press(&mut app, &mut ctx, KeyCode::Char('y'));
    assert_eq!(saved_config(&ctx)["daily_goal"], 10);
}

#[test]
fn settings_fit_a_small_terminal() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir, true);
    let mut app = App::new(&mut ctx, Start::Home).unwrap();
    press(&mut app, &mut ctx, KeyCode::Char('c'));
    let small = screen_sized(&mut app, 40, 12);
    assert!(small.contains("› Daily goal (words)"), "{small}");
    assert!(!small.contains("0–500"), "{small}");
    for _ in 0..5 {
        press(&mut app, &mut ctx, KeyCode::Down);
    }
    let small = screen_sized(&mut app, 40, 12);
    assert!(small.contains("› Lookup timeout (s)"), "{small}");
}

#[test]
fn word_list_filters_and_removes_words() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir, true);
    let mut app = App::new(&mut ctx, Start::Home).unwrap();
    press(&mut app, &mut ctx, KeyCode::Char('w'));
    let list = screen(&mut app);
    assert!(list.contains("Word list (1)"), "{list}");
    assert!(list.contains("› ephemeral"), "{list}");

    press(&mut app, &mut ctx, KeyCode::Char('/'));
    type_text(&mut app, &mut ctx, "zzz");
    assert!(screen(&mut app).contains("No saved words match"));
    app.handle_key(
        &mut ctx,
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
    );
    assert!(!app.should_quit(), "Ctrl+C clears the filter first");

    press(&mut app, &mut ctx, KeyCode::Char('d'));
    assert!(screen(&mut app).contains("Remove \"ephemeral\"?"));
    press(&mut app, &mut ctx, KeyCode::Char('y'));
    assert_eq!(ctx.db.collection_size().unwrap(), 0);
    assert!(screen(&mut app).contains("No saved words yet"));
}

#[test]
fn word_list_keys_filter_by_status_part_of_speech_and_due() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir, true);
    save_words(&ctx, &["brisk"]);
    let mut app = App::new(&mut ctx, Start::Home).unwrap();
    press(&mut app, &mut ctx, KeyCode::Char('w'));
    assert!(screen(&mut app).contains("Word list (2)"));

    press(&mut app, &mut ctx, KeyCode::Char('s'));
    let new = screen(&mut app);
    assert!(new.contains("Word list (2)"), "{new}");
    assert!(new.contains("Showing 2 of 2: new."), "{new}");

    press(&mut app, &mut ctx, KeyCode::Char('s'));
    let learning = screen(&mut app);
    assert!(learning.contains("Word list (0 of 2)"), "{learning}");
    assert!(learning.contains("No saved words match"), "{learning}");
    for _ in 0..3 {
        press(&mut app, &mut ctx, KeyCode::Char('s'));
    }
    assert!(screen(&mut app).contains("Showing all words."));

    press(&mut app, &mut ctx, KeyCode::Char('p'));
    let adjectives = screen(&mut app);
    assert!(adjectives.contains("adjective"), "{adjectives}");
    assert!(adjectives.contains("Word list (2)"), "{adjectives}");

    press(&mut app, &mut ctx, KeyCode::Char('u'));
    let due = screen_sized(&mut app, 40, 12);
    assert!(due.contains("adjective · due"), "{due}");
    assert!(
        due.contains("Showing 0 of 2"),
        "new words are never due: {due}"
    );
    press(&mut app, &mut ctx, KeyCode::Esc);
    assert!(!app.should_quit());
}

#[test]
fn word_list_marks_words_added_by_agents_with_their_reason() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir, true);
    let entry = wiktionary::parse_definitions("ephemeral", WIKT_REST).unwrap();
    let mut entry = entry.clone();
    entry.word = "brisk".to_string();
    let id = ctx
        .db
        .cache_word("brisk", &entry, WIKT_REST, dictionary::SOURCE, Utc::now())
        .unwrap();
    let now = Utc::now();
    ctx.db
        .add_from_agent(id, Some("came up in a code review"), 30, now, now)
        .unwrap();

    let mut app = App::new(&mut ctx, Start::Home).unwrap();
    press(&mut app, &mut ctx, KeyCode::Char('w'));
    let list = screen(&mut app);
    let brisk = list.lines().find(|l| l.contains("brisk")).unwrap();
    assert!(brisk.contains("agent"), "{list}");
    let ephemeral = list.lines().find(|l| l.contains("ephemeral")).unwrap();
    assert!(!ephemeral.contains("agent"), "{list}");
    assert!(
        list.contains("suggested by an AI agent: came up in a code review"),
        "{list}"
    );
    assert!(
        screen_sized(&mut app, 40, 12).contains("brisk"),
        "the marker must not push the word out of a small window"
    );

    press(&mut app, &mut ctx, KeyCode::Char('j'));
    assert!(!screen(&mut app).contains("suggested by an AI agent"));
}

#[test]
fn lookups_ignore_spaces_around_the_word() {
    for typed in [
        " ephemeral",
        "ephemeral  ",
        "\u{3000}ephemeral",
        "\u{200B}ephemeral",
        "\u{FEFF} ephemeral\u{2060}",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = context(&dir, true);
        ctx.endpoints = dictionary::Endpoints {
            rest_base: "http://127.0.0.1:9/definition".to_string(),
            action_api: "http://127.0.0.1:9/api.php".to_string(),
        };
        ctx.config.http_timeout_secs = 2;
        let mut app = App::new(&mut ctx, Start::Home).unwrap();
        press(&mut app, &mut ctx, KeyCode::Char('/'));
        type_text(&mut app, &mut ctx, typed);
        press(&mut app, &mut ctx, KeyCode::Enter);
        assert!(app.has_background_work(), "{typed:?}");
        app.background_work(&mut ctx);
        let entry = screen(&mut app);
        assert!(entry.contains("adjective"), "{typed:?}: {entry}");
    }
}

#[test]
fn blank_lookups_ask_for_a_word() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir, false);
    let mut app = App::new(&mut ctx, Start::Home).unwrap();
    press(&mut app, &mut ctx, KeyCode::Char('/'));
    type_text(&mut app, &mut ctx, " \u{200B} ");
    press(&mut app, &mut ctx, KeyCode::Enter);
    assert!(!app.has_background_work());
    assert!(app.flash_text().unwrap().contains("Type a word"));
    type_text(&mut app, &mut ctx, "x");
    assert!(screen(&mut app).contains("Look up › x"));
}

#[test]
fn saved_entries_open_offline_from_the_word_list() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(&dir, true);
    let mut app = App::new(&mut ctx, Start::Home).unwrap();
    press(&mut app, &mut ctx, KeyCode::Char('w'));
    press(&mut app, &mut ctx, KeyCode::Enter);
    let entry = screen(&mut app);
    assert!(entry.contains("in your word list"), "{entry}");
    assert!(entry.contains("adjective"), "{entry}");
    assert!(entry.contains("s remove"), "{entry}");
    press(&mut app, &mut ctx, KeyCode::Char('q'));
    assert!(screen(&mut app).contains("Word list"));
}
