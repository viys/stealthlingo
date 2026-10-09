use chrono::Utc;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use stealthlingo::cli::StudyMode;
use stealthlingo::commands::study::SessionRequest;
use stealthlingo::commands::Context;
use stealthlingo::config::Paths;
use stealthlingo::dictionary::{self, wiktionary};
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
        due_only: false,
        minutes: None,
        count: None,
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
    let home = screen_sized(&mut app, 100, 12);
    assert!(home.contains("Your word list is empty"), "{home}");
    assert!(!home.contains("Quit"), "{home}");

    press(&mut app, &mut ctx, KeyCode::Up);
    press(&mut app, &mut ctx, KeyCode::Up);
    let home = screen_sized(&mut app, 100, 12);
    assert!(home.contains("› q  Quit"), "{home}");
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
