# StealthLingo

**English** | [简体中文](docs/i18n/README.zh-CN.md) | [繁體中文](docs/i18n/README.zh-TW.md) | [日本語](docs/i18n/README.ja.md) | [한국어](docs/i18n/README.ko.md) | [Español](docs/i18n/README.es.md) | [Português (Brasil)](docs/i18n/README.pt-BR.md) | [Русский](docs/i18n/README.ru.md) | [Tiếng Việt](docs/i18n/README.vi.md)

[![Release](https://img.shields.io/github/v/release/viys/stealthlingo)](https://github.com/viys/stealthlingo/releases) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Practise a few English words while you slack off. Press **Esc** and you’re at a shell prompt in this terminal; press Esc once more and you’re back. Esc is not typed into that shell. The timer pauses while it’s hidden.

That gap is whatever you already have — a build, a meeting you are not really in, the wait before an AI reply. Look a word up, save it, and practise with flashcards, spelling, or listening. Definitions come from English [Wiktionary](https://en.wiktionary.org/) (no API key). Progress lives in a local SQLite database, so saved words work offline. An optional MCP server lets a coding agent add words it notices; StealthLingo itself never calls an AI model.

![Home — daily goal, due words, and the practice menu](docs/images/home.png)

Full guides live in the **[Wiki](https://github.com/viys/stealthlingo/wiki)** (keys, every command, study modes, MCP, backups, FAQ, development). English vocabulary only in v0.1.

## Why StealthLingo

A gap you can leave at once: a short session, Esc when someone walks by, and a list that grows from words you actually meet — in docs, code, or chat.

| You get | You don’t get |
|---|---|
| Full-screen TUI or `--plain` prompts | Accounts, cloud sync, or a GUI |
| Esc hides the screen and opens a shell; Esc returns. The timer pauses while it is hidden | A window you have to quit to dismiss |
| Flashcards, spelling, missing letters, listening, mixed, retry mistakes | AI-written definitions |
| Daily word goal + spaced repetition (simplified SM-2) | Forced long courses |
| UK / US audio when Wiktionary has it | Network required for review |

## Install

Prebuilt binaries for Windows, macOS, and Linux — no Rust required:

```powershell
# Windows (PowerShell)
powershell -ExecutionPolicy Bypass -c "irm https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.ps1 | iex"
```

```bash
# macOS / Linux
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.sh | sh
```

Or from source (Rust 1.88+; Linux needs `libasound2-dev` to build):

```bash
cargo install --git https://github.com/viys/stealthlingo
```

## Quick start

```bash
stealthlingo                           # full-screen app; Esc hides it
stealthlingo study --minutes 3         # a session that fits the gap
stealthlingo lookup ephemeral          # pronunciations + main definitions (--all for everything)
stealthlingo add ephemeral --note "lasting a short time"
stealthlingo --plain                   # line-by-line prompts instead
```

On the home screen: `r` review · `a` practice · `/` look up · `w` word list · `t` stats · `c` settings · **Esc** hide · **Ctrl+C** quit (`q` only goes back).

In the shell that Esc opens, commands such as `lookup` and `add` work as usual. The full-screen app is already open, so starting it again there only reminds you to press Esc.

Practice modes (`a`, or jump in with the letter): `s` flashcards · `p` spelling · `m` missing letters · `i` listening · `x` mixed · `e` retry mistakes.

## Commands

| Command | What it does |
|---|---|
| `stealthlingo` | Open the full-screen app |
| `lookup <WORD> [--all]` | Query Wiktionary (falls back to cache offline) |
| `add` / `remove` | Save or remove a word |
| `words` / `search` | List or search (`--status`, `--pos`, `--due`) |
| `study` / `review` | Practise (`--mode`, `--mistakes`, `--minutes`, `--count`) |
| `stats` | Progress and accuracy |
| `audio <WORD>` | Play pronunciation |
| `config` | Show or change settings |
| `export` / `import` | `.json` backup or `.csv` word list |
| `mcp` | MCP server for AI agents |
| `links` | Clickable lookups (Windows) |

```bash
stealthlingo study --mode spelling --count 10
stealthlingo study --mode listening --minutes 5
stealthlingo study --mistakes
stealthlingo review
```

## AI agents (MCP)

While you are actually working, an agent can save words for the next time you slack off. Point your client at StealthLingo (e.g. Cursor `~/.cursor/mcp.json`):

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

Agents can look words up and add them to your list. They cannot remove words, change settings, or record answers. Details: [Wiki · AI Agents](https://github.com/viys/stealthlingo/wiki/AI-Agents-MCP).

## Data & backups

Default data directory: `%APPDATA%\stealthlingo\data` (Windows), `~/Library/Application Support/stealthlingo` (macOS), `~/.local/share/stealthlingo` (Linux). Override with `STEALTHLINGO_HOME`.

```bash
stealthlingo export backup.json    # full backup
stealthlingo import backup.json    # merge (most recent progress wins)
```

## Contributing

Issues and pull requests are welcome. Build and test notes: [Wiki · Development](https://github.com/viys/stealthlingo/wiki/Development).

## License

MIT — see [LICENSE](LICENSE).
