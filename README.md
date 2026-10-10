# stealthlingo

**English** | [简体中文](docs/i18n/README.zh-CN.md) | [繁體中文](docs/i18n/README.zh-TW.md) | [日本語](docs/i18n/README.ja.md) | [한국어](docs/i18n/README.ko.md) | [Español](docs/i18n/README.es.md) | [Português (Brasil)](docs/i18n/README.pt-BR.md) | [Русский](docs/i18n/README.ru.md) | [Tiếng Việt](docs/i18n/README.vi.md)

A terminal-first language learning tool for stealthy study sessions.

StealthLingo is a small Rust CLI for vocabulary practice in spare minutes: look a
word up, save it, and practise it with flashcards, spelling, missing letters,
listening or a mix of them, toward a daily word goal. Esc hides the whole screen
at once. Dictionary data comes from English
[Wiktionary](https://en.wiktionary.org/) through Wikimedia's official API, no key
needed; scheduling, answer checking and your progress live in a local SQLite
database, so saved words can be reviewed offline. An AI agent can also add the
words you may not know while you read or code (see [AI agents](#ai-agents-mcp)).

English is the only supported language in v0.1.

![StealthLingo home screen: today's goal, due words and the practice menu](docs/images/home.png)

## Install

Prebuilt programs for Windows (x64), macOS (Intel and Apple silicon) and Linux
(x64 and ARM64) are on the
[Releases page](https://github.com/viys/stealthlingo/releases); no Rust needed.
The installers put `stealthlingo` and its link helper `stealthlingo-link` in
`~/.cargo/bin` and add that folder to your `PATH`.

Windows (PowerShell):

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.ps1 | iex"
```

macOS and Linux:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.sh | sh
```

Or download the archive for your system from the Releases page, unpack it and
keep `stealthlingo` and `stealthlingo-link` in the same folder. On Linux, audio
playback uses ALSA (`libasound2`), which desktop distributions ship by default.

### From source

Requires a recent stable Rust toolchain (1.88+) and a C compiler (SQLite is bundled).

```bash
cargo install --git https://github.com/viys/stealthlingo
# or, from a checkout
cargo install --path .
cargo run -- --help
```

On Linux, building needs the ALSA development package (e.g. `libasound2-dev`).

## Quick start

```bash
stealthlingo lookup ephemeral          # pronunciation and the main definitions (--all for everything)
stealthlingo add ephemeral --note 短暂的 # save it, with an optional personal note
stealthlingo                           # open the full-screen app: practice, look up, browse, stats
```

## Full-screen interface

In a terminal, `stealthlingo`, `study` and `review` open a full-screen interface.
The home screen shows what is due today, your progress toward the daily goal and
a menu (`r` review, `s` flashcards, `p` spelling, `m` missing letters,
`l` listening, `x` mixed, `e` retry mistakes, `/` look up, `w` word list,
`t` stats, `c` settings, `q` quit); `↑`/`↓` (or `k`/`j`) and `Enter` work too.
Lists and long pages scroll with the same keys.

![Esc hides the app and shows the shell; Esc again brings it back](docs/images/boss-key.webp)

- **Esc hides everything instantly** and shows the shell you started from; press
  Esc again to come back. The session clock pauses while hidden.
- **Flashcards** need no Enter: `Space` reveals, `1`–`4` rate, `a` plays the word,
  `s` skips, `q` ends the session.
- **Accents**: `a` plays your default accent (the `accent` setting) first; pressing
  it again on the same word plays the other recorded accent, and so on. The footer
  shows which accent comes next, the header which one is playing.
- **Spelling and listening**: type the word and press `Enter`. `Tab` shows the
  answer, `Ctrl+N` skips. In listening practice `Enter` on an empty line replays
  the recording and `Ctrl+R` plays the other accent. A wrong answer shows which
  letters were extra or missing. Missing-letter questions show the word with
  some letters blanked out (`e _ h e _ e r _ l`); type the whole word.
- **Word list**: `/` filters by text, `s` cycles through statuses, `p` through
  parts of speech, and `u` shows only words that are due. `Enter` opens the
  saved entry (offline), `a` plays it, `n` edits the note, `d` then `y` removes
  the word (saving it again restores its progress). `PgUp`/`PgDn` and `g`/`G`
  jump through long lists. Words an AI agent added are marked `agent`, and
  the details under the selected word show the agent's reason.
- **Settings** (`c`): `←`/`→` change the selected value, `PgUp`/`PgDn` by 10,
  `Enter` types a number, `r` restores the default and `R` restores every
  setting. Each change is saved immediately; values outside the allowed range
  are not saved.
- **Look up**: type a word and press `Enter`; then `s` saves or removes it, `a`
  plays it, `Tab` switches between the brief and the full entry, `/` starts a
  new lookup and `q` goes back.
- `Ctrl+C` ends the current session, cancels a filter or note, and otherwise quits.
  Every answer is saved as soon as it is given.
- The interface needs a terminal of at least 40×12 characters. In short windows the
  boxes shrink, the home summary fits on one line and the menu splits into two
  columns (wide windows) or scrolls, so the selected item and the answer field
  stay visible.

![A flashcard with its meaning revealed, waiting for a rating](docs/images/flashcard.png)

![A wrong spelling answer: the missing letter is marked in the answer](docs/images/spelling.png)

![Looking a word up: UK and US pronunciations and the main definitions](docs/images/lookup.png)

`--plain` (or any non-terminal input/output, such as a pipe) uses the line-by-line
prompts described below instead. `NO_COLOR` turns colors off.

## Commands

| Command | What it does |
|---|---|
| `stealthlingo` | Open the full-screen app. With `--plain`: review due words; if nothing is due, study new ones; with an empty list, explain how to start |
| `lookup <WORD> [--all]` | Query Wiktionary (refreshes the cache; falls back to the cached copy when offline) |
| `add <WORD> [--note TEXT]` | Save a word. Saving twice is harmless; `--note` updates the note |
| `remove <WORD>` | Remove a word from your list (cached dictionary data and history are kept) |
| `words [--status S] [--pos P] [--due]` | List saved words with status, due time and who added them, optionally filtered |
| `search <QUERY> [--status S] [--pos P] [--due]` | Search saved words and notes |
| `study [--mode memory\|spelling\|letters\|listening\|mixed] [--mistakes] [--minutes N] [--count N]` | Study session |
| `review [--count N] [--minutes N]` | Flashcard review of due words only |
| `stats` | Saved words by status, due count, daily goal and the last 7 days, today's and all-time answers with accuracy, next review |
| `audio <WORD> [--accent uk\|us]` | Play the pronunciation (default: the `accent` setting) |
| `config` / `config set <KEY> <VALUE>` / `config reset [KEY]` | Show, change or restore settings; show data locations |
| `links [status\|install\|uninstall]` | Ctrl+click in `lookup` to play pronunciations and add or remove words (Windows) |
| `export <PATH>` / `import <PATH>` | `.json` full backup, or `.csv` word list |
| `mcp` | Serve AI agents over MCP on stdin/stdout; started by the agent's client (see AI agents below) |

Without `--minutes` or `--count`, a session lasts `default_minutes` (5). Sessions
started from the home screen, or by `stealthlingo --plain`, instead run until the
daily goal is reached (see [Daily goal](#daily-goal)).

The word list filters combine: `--status` is one of `new`, `learning`, `review`
or `mastered`; `--pos` matches the start of a part of speech (`adj`, `n`,
`verb`); `--due` keeps the words whose review time has come.

```bash
stealthlingo words --status learning --pos adj
stealthlingo words --due
```

`lookup` shows pronunciations on one line, UK first, then US
(`Pronunciation: UK /njuː/ · US /nu/`), and names the accent of the recording
that `audio` plays (or suggests `--accent uk|us` when there are both). By default it then lists the first three definitions of up
to three parts of speech. `lookup --all` (or `-a`)
shows every definition with examples, synonyms, antonyms and the source.
Definitions wrap to the terminal width with their continuation lines indented.

### Clickable lookups (Windows)

```bash
stealthlingo links install   # once, after installing
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

The keys below are those of the line-by-line prompts (`--plain`); the full-screen
interface uses single keys as described above.

- **memory**: see the word, press Enter to reveal the meaning, then rate yourself
  `1` Again, `2` Hard, `3` Good, `4` Easy. Press `a` to hear the word.
- **spelling**: read the definition (the word itself is blanked out of definitions
  and examples) and type the word. `?` gives up and shows the answer.
- **letters**: like spelling, but about half of the letters are shown
  (`e _ h e _ e r _ l`). The blanks stay the same for each word. Answers count
  as spelling practice.
- **listening**: hear the pronunciation and type the word. `r` replays. Only words
  with pronunciation audio are used.
- **mixed**: words you have never studied start as flashcards; the others take
  turns between flashcards, spelling and (when there is a recording) listening.

`--mistakes` practises the words you answered wrong in the last 30 days, or
forgot after having learned them, the most recent mistakes first, whether or not
they are due. It works with every mode; on the home screen, `e` uses mixed
practice.

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

### Daily goal

`daily_goal` (default 10) is the number of different words to practise each day;
answering the same word again does not count twice. The home screen, the stats
page and `stats` show today's progress, and `stats` also shows the last 7 days.
Until the goal is reached, sessions started from the home screen (or by
`stealthlingo --plain`) end when it is reached rather than after
`default_minutes`; if you run out of words first, the session says how many were
missing. Explicit `--minutes` or `--count` always win. `0` turns the goal off.

## Configuration

Change settings on the settings screen (`c` on the home screen) or with
`config`:

```bash
stealthlingo config set daily_goal 20
stealthlingo config set daily_new_limit 15
stealthlingo config set default_minutes 3
stealthlingo config set accent us        # play American recordings first (default: uk)
stealthlingo config reset daily_goal     # back to the default
stealthlingo config reset                # every setting, after confirmation (--yes skips it)
```

| Key | Default | Range | Meaning |
|---|---|---|---|
| `daily_goal` | 10 | 0–500, 0 = off | Different words to practise per day |
| `daily_new_limit` | 10 | 0–200, 0 = review only | Never-studied words introduced per day |
| `default_minutes` | 5 | 1–120 | Length of a session without `--minutes` or `--count` |
| `accent` | uk | uk / us | Accent played first |
| `mcp_daily_add_limit` | 30 | 0–200, 0 = off | Words AI agents may add per day through MCP |
| `http_timeout_secs` | 10 | 1–120 | Network timeout |

`config` lists every value next to its default and range. If `config.json`
holds a value outside the range, StealthLingo warns and uses the nearest
allowed value without rewriting the file. Words recorded in only one accent
always play that recording.

### Dictionary

StealthLingo uses Wikimedia's official Wiktionary API: definitions and examples
come from the REST API, and IPA (labelled by accent), synonyms, antonyms and
Wikimedia Commons recordings (OGG) from the page source. When the typed form and
the dictionary headword differ (`Ephemeral` and `ephemeral`), the typed form is
remembered, so `add` and `remove` refer to the same saved word.

Development builds from before the v0.1.0 release could also use the Free
Dictionary API or Merriam-Webster. Words cached from those are still readable offline, and are replaced with Wiktionary
data the next time they are looked up (study progress and notes are kept). Their
old settings in `config.json` are ignored.

## AI agents (MCP)

`stealthlingo mcp` runs a [Model Context Protocol](https://modelcontextprotocol.io)
server on stdin/stdout, so an AI agent (Cursor, Claude Desktop or another MCP
client) can look words up and add the ones you may not know to your study list
while you read or code. StealthLingo itself never calls an AI model. Add it to
your client's MCP configuration (for Cursor, `~/.cursor/mcp.json`, or
`.cursor/mcp.json` in a project):

```json
{
  "mcpServers": {
    "stealthlingo": {
      "command": "stealthlingo",
      "args": ["mcp"]
    }
  }
}
```

If `stealthlingo` is not on your `PATH`, use the full path of the executable as
`command`. Then ask the agent, for example, to add the words on this page that
you might not know.

| Tool | What it does |
|---|---|
| `get_study_status` | Saved, due and not-yet-started words, today's goal progress and accuracy, and how many words agents may still add today |
| `list_words` | Saved words with status, due time, audio, lapses and a short definition; `query`, `status` and `limit` narrow it. Notes are never shared |
| `lookup_word` | The Wiktionary entry for a word (cache first), without saving it |
| `add_words` | Save up to 20 words per call, each with an optional `reason`; reports the result word by word |

Agents only choose words:

- Definitions, examples and pronunciations come from Wiktionary only. An agent
  can say where a word came up (`reason`), but cannot write definitions,
  translations or your notes. Words Wiktionary does not have are not added.
- There is no tool to remove words, change settings or record answers.
- Words you removed (with `remove`, `d` in the word list, or a link) are never
  added back by an agent; saving one yourself makes it available again.
- Agents add at most `mcp_daily_add_limit` (default 30) words per day, and
  removing one does not give the slot back. `0` turns adding off. Changes apply
  to the next call, without restarting the client.
- Words an agent added are studied like any other word. The word list marks them
  `agent` and shows the agent's reason; `words` shows who added each word in the
  `BY` column. JSON backups keep both, and the words you removed.

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
stealthlingo import examples/words.csv   # "word" column (else the first one); "note" is optional
```

When the same word exists on both sides of a JSON import, the most recently
reviewed progress wins. CSV import looks up each word that is not cached yet, so
it needs a connection.

## Offline behavior

- Studying, reviewing, `words`, `search` and `stats` never touch the network.
- `lookup` refreshes from Wiktionary, and shows the cached copy if it is unreachable.
- `add` and `audio` use the cache when possible and only go online for new words.
- In `mcp`, `get_study_status` and `list_words` never touch the network;
  `lookup_word` and `add_words` only go online for words that are not cached.
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
`Entry` model, plus a decoder for data cached by pre-release builds), `storage` (SQLite),
`learning` (scheduling, answer checking, sessions), `audio`, `commands` and `tui`
(the full-screen interface, built with ratatui). Both interfaces drive the same
`learning::session::Session`.
Scheduling is a pure function with its own tests in `tests/scheduling_tests.rs`.

### Releasing

Releases are built by [dist](https://github.com/axodotdev/cargo-dist)
(`dist-workspace.toml`, `.github/workflows/release.yml`). Bump `version` in
`Cargo.toml`, commit, then push a matching tag:

```bash
git tag v0.2.0
git push origin v0.2.0
```

GitHub Actions then builds every platform and publishes the archives and
installers as a GitHub Release. `dist plan` previews what will be built; after
changing `dist-workspace.toml`, run `dist generate` to update the workflow.

## License

MIT, see [LICENSE](LICENSE).
