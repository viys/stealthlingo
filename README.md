# stealthlingo

A terminal-first language learning tool for stealthy study sessions.

StealthLingo is a small Rust CLI for vocabulary practice in spare minutes: look a
word up, save it, and run 3–5 minute sessions of flashcards, spelling or
listening-spelling. Dictionary data comes from a source you choose
([Wiktionary](https://en.wiktionary.org/) by default, the
[Free Dictionary API](https://dictionaryapi.dev/) or [Merriam-Webster](https://dictionaryapi.com/)); scheduling, answer checking and
your progress live in a local SQLite database, so saved words can be reviewed offline.

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
stealthlingo lookup ephemeral          # show definitions, phonetics, examples
stealthlingo add ephemeral --note 短暂的 # save it, with an optional personal note
stealthlingo                           # today's practice: due reviews first, else new words
```

## Commands

| Command | What it does |
|---|---|
| `stealthlingo` | Review due words; if nothing is due, study new ones; with an empty list, explain how to start |
| `lookup <WORD>` | Query the API (refreshes the cache; falls back to the cached copy when offline) |
| `add <WORD> [--note TEXT]` | Save a word. Saving twice is harmless; `--note` updates the note |
| `remove <WORD>` | Remove a word from your list (cached dictionary data and history are kept) |
| `words` | List saved words with status and due time |
| `search <QUERY>` | Search saved words and notes |
| `study [--mode memory\|spelling\|listening] [--minutes N] [--count N]` | Study session |
| `review [--count N] [--minutes N]` | Flashcard review of due words only |
| `stats` | Saved words, due count, today's answers and accuracy |
| `audio <WORD>` | Play the pronunciation |
| `config` / `config set <KEY> <VALUE>` | Show or change settings and data locations |
| `export <PATH>` / `import <PATH>` | `.json` full backup, or `.csv` word list |

Without `--minutes` or `--count`, a session lasts `default_minutes` (5).
Commands that look words up (`lookup`, `add`, `audio`, `import`, and sessions) also
accept `--source <wiktionary|free-dictionary|merriam-webster>` to use another
dictionary just this once. A cached word that came from a different source is
fetched again from the one you asked for.

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
stealthlingo config set dictionary_source merriam-webster
stealthlingo config set merriam_webster_key <KEY>
stealthlingo config set free_dictionary_url https://api.dictionaryapi.dev/api/v2/entries/en
```

### Dictionary sources

| Source | Key | Notes |
|---|---|---|
| `wiktionary` (default) | none | Wikimedia's official API: definitions and examples, plus IPA, synonyms, antonyms and Wikimedia Commons recordings (OGG) from the page source. |
| `free-dictionary` | none | Community-run, derived from Wiktionary; MP3 audio for many words. Has no SLA and is sometimes down. |
| `merriam-webster` | free key | Collegiate Dictionary: concise definitions, MW-style pronunciation (`\i-ˈfem-rəl\`) and MP3 audio. Register at [dictionaryapi.com](https://dictionaryapi.com/register/index) and request the *Collegiate Dictionary* API. |

Switch for good with `config set dictionary_source <SOURCE>`, or for a single command:

```bash
stealthlingo lookup hostel --source free-dictionary
```

Each cached word remembers which source it came from (`lookup` shows it). Looking
a word up again with another source replaces the cached entry; your study progress
and notes are kept. When a dictionary answers with a different headword (Merriam-Webster
maps `ran` to `run`), the typed form is remembered, so `add ran` and `remove ran`
refer to the same saved word. `config` masks the Merriam-Webster key when printing
it, and it is never included in error messages.

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
- `lookup` refreshes from the dictionary, and shows the cached copy if it is unreachable.
  When a word is not cached and the dictionary is down, the error suggests trying
  another source.
- `add` and `audio` use the cache when possible and only go online for new words.
- Pronunciation audio is downloaded the first time it is played and reused after that.

## About the dictionary data

- Entries carry their source URL and license, which `lookup` shows. Free Dictionary
  and Wiktionary content is CC BY-SA (3.0 or 4.0); Merriam-Webster content is
  subject to its [API terms](https://dictionaryapi.com/info/terms-of-service)
  (free for non-commercial use, with request limits).
- Coverage varies by word: phonetics, audio, examples, synonyms and antonyms can all
  be missing. StealthLingo handles every field as optional; words without audio
  are skipped in listening practice.
- None of the sources is a translation service. Chinese (or any other language)
  hints come from your own `--note`, never from the dictionary.
- The services are free and can be slow or down. Respect their usage terms, and do
  not redistribute cached data in bulk without honoring the original licenses.

## Development

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

The code is split into `dictionary` (one module per source, all mapped to a shared
`Entry` model), `storage` (SQLite),
`learning` (scheduling, answer checking, sessions), `audio` and `commands`.
Scheduling is a pure function with its own tests in `tests/scheduling_tests.rs`.

## License

MIT, see [LICENSE](LICENSE).
