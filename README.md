# stealthlingo

A terminal-first language learning tool for stealthy study sessions.

StealthLingo is a small Rust CLI for vocabulary practice in spare minutes: look a
word up, save it, and run 3–5 minute sessions of flashcards, spelling or
listening-spelling. Dictionary data comes from English
[Wiktionary](https://en.wiktionary.org/) through Wikimedia's official API, no key
needed; scheduling, answer checking and your progress live in a local SQLite
database, so saved words can be reviewed offline.

English is the only supported language in v0.1.

## Install

Requires a recent stable Rust toolchain (1.88+) and a C compiler (SQLite is bundled).

```bash
cargo install --path .
# or run from the checkout
cargo run -- --help
```

On Linux, audio playback needs the ALSA development package (e.g. `libasound2-dev`)
at build time.

## Quick start

```bash
stealthlingo lookup ephemeral          # pronunciation and the main definitions (--all for everything)
stealthlingo add ephemeral --note 短暂的 # save it, with an optional personal note
stealthlingo                           # today's practice: due reviews first, else new words
```

## Commands

| Command | What it does |
|---|---|
| `stealthlingo` | Review due words; if nothing is due, study new ones; with an empty list, explain how to start |
| `lookup <WORD> [--all]` | Query Wiktionary (refreshes the cache; falls back to the cached copy when offline) |
| `add <WORD> [--note TEXT]` | Save a word. Saving twice is harmless; `--note` updates the note |
| `remove <WORD>` | Remove a word from your list (cached dictionary data and history are kept) |
| `words` | List saved words with status and due time |
| `search <QUERY>` | Search saved words and notes |
| `study [--mode memory\|spelling\|listening] [--minutes N] [--count N]` | Study session |
| `review [--count N] [--minutes N]` | Flashcard review of due words only |
| `stats` | Saved words, due count, today's answers and accuracy |
| `audio <WORD>` | Play the pronunciation |
| `config` / `config set <KEY> <VALUE>` | Show or change settings and data locations |
| `links [status\|install\|uninstall]` | Ctrl+click in `lookup` to play pronunciations and add or remove words (Windows) |
| `export <PATH>` / `import <PATH>` | `.json` full backup, or `.csv` word list |

Without `--minutes` or `--count`, a session lasts `default_minutes` (5).

`lookup` shows pronunciations on one line, UK first, then US
(`Pronunciation: UK /njuː/ · US /nu/`), and names the accent of the recording
that `audio` plays. By default it then lists the first three definitions of up
to three parts of speech. `lookup --all` (or `-a`)
shows every definition with examples, synonyms, antonyms and the source.
Definitions wrap to the terminal width with their continuation lines indented.

### Clickable lookups (Windows)

```bash
stealthlingo links install   # once, after the first `cargo install`
```

This registers a `stealthlingo://` link handler for your user (no admin rights
needed). `lookup` then turns each pronunciation that has a recording into a
terminal hyperlink: Ctrl+click it and the recording plays in the background,
without opening a browser or a window. The last line becomes
`+ Add to word list`, or `Saved in your word list · Remove` for saved words;
Ctrl+click it to add or remove the word without typing a command (run `lookup`
again to see the change). Because other programs can open these links too, a
removal by link keeps the word's study progress: adding the word again, by link
or with `add`, restores it. The `remove` command starts the word over instead. It works in terminals that render hyperlinks (Windows
Terminal, VS Code / Cursor); elsewhere `lookup` prints plain text with the
`add` and `audio` commands. `links status` shows what was detected,
`FORCE_HYPERLINK=1` (or `0`) overrides the detection, and `links uninstall`
removes the handler. Errors are written to `link-errors.log` in the data
directory.

### Study modes

- **memory**: see the word, press Enter to reveal the meaning, then rate yourself
  `1` Again, `2` Hard, `3` Good, `4` Easy. Press `a` to hear the word.
- **spelling**: read the definition (the word itself is blanked out of definitions
  and examples) and type the word. `?` gives up and shows the answer.
- **listening**: hear the pronunciation and type the word. `r` replays. Only words
  with pronunciation audio are used.

In every mode `s` skips the current word without recording an answer and `q` ends
the session. Ctrl+C also ends the session; a second Ctrl+C exits immediately.
Every submitted answer is saved right away, so quitting never loses finished work,
and an unanswered question is never recorded as wrong.

Answer checking ignores case and surrounding whitespace, but apostrophes and
hyphens count (`well-being` is not `wellbeing`).

### Scheduling

A simplified SM-2 algorithm decides when a word comes back:

- **Again**: back in 10 minutes; progress resets.
- **Hard**: shorter interval than normal.
- **Good**: 1 day, then 3 days, then the interval grows by the word's ease factor.
- **Easy**: longer interval, and the word gets easier from then on.

Spelling and listening answers count as Good when correct and Again when wrong.
A missed word comes back once more at the end of the same session. Due reviews
always come before new words, and at most `daily_new_limit` (default 10)
never-studied words are introduced per day. Words with an interval of 21 days or
more are shown as `mastered`.

## Configuration

```bash
stealthlingo config set daily_new_limit 15
stealthlingo config set default_minutes 3
stealthlingo config set http_timeout_secs 10
```

### Dictionary

StealthLingo uses Wikimedia's official Wiktionary API: definitions and examples
come from the REST API, and IPA (labelled by accent), synonyms, antonyms and
Wikimedia Commons recordings (OGG) from the page source. When the typed form and
the dictionary headword differ (`Ephemeral` and `ephemeral`), the typed form is
remembered, so `add` and `remove` refer to the same saved word.

Older versions could also use the Free Dictionary API or Merriam-Webster. Words
cached from those are still readable offline, and are replaced with Wiktionary
data the next time they are looked up (study progress and notes are kept). Their
old settings in `config.json` are ignored.

## Data directory

`stealthlingo config` prints the exact paths. By default:

| OS | Location |
|---|---|
| Windows | `%APPDATA%\stealthlingo\data` |
| macOS | `~/Library/Application Support/stealthlingo` |
| Linux | `~/.local/share/stealthlingo` |

Set `STEALTHLINGO_HOME` to use another directory (handy for a portable setup or
for experiments). It contains `stealthlingo.db` (SQLite, with versioned
migrations), `config.json`, and `audio/` (downloaded pronunciation files).

### Backups and word lists

```bash
stealthlingo export backup.json   # everything: cache, progress, answer history
stealthlingo import backup.json   # merge; re-importing the same file changes nothing
stealthlingo export words.csv     # word, note, status, due_at, definition
stealthlingo import examples/words.csv   # needs a "word" column; "note" is optional
```

When the same word exists on both sides of a JSON import, the most recently
reviewed progress wins. CSV import looks up each word that is not cached yet, so
it needs a connection.

## Offline behavior

- Studying, reviewing, `words`, `search` and `stats` never touch the network.
- `lookup` refreshes from Wiktionary, and shows the cached copy if it is unreachable.
- `add` and `audio` use the cache when possible and only go online for new words.
- Pronunciation audio is downloaded the first time it is played and reused after that.

## About the dictionary data

- Entries carry their source URL and license, which `lookup --all` shows. Wiktionary
  content is licensed CC BY-SA 4.0.
- Coverage varies by word: phonetics, audio, examples, synonyms and antonyms can all
  be missing. StealthLingo handles every field as optional; words without audio
  are skipped in listening practice.
- Wiktionary is not a translation service. Chinese (or any other language)
  hints come from your own `--note`, never from the dictionary.
- The service is free and can be slow or down. Respect Wikimedia's usage terms, and
  do not redistribute cached data in bulk without honoring the license.

## Development

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

The code is split into `dictionary` (Wiktionary client and parsers mapped to an
`Entry` model, plus a decoder for data cached by v0.1), `storage` (SQLite),
`learning` (scheduling, answer checking, sessions), `audio` and `commands`.
Scheduling is a pure function with its own tests in `tests/scheduling_tests.rs`.

## License

MIT, see [LICENSE](LICENSE).
