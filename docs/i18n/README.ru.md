# StealthLingo

[English](../../README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Español](README.es.md) | [Português (Brasil)](README.pt-BR.md) | **Русский** | [Tiếng Việt](README.vi.md)

[![Release](https://img.shields.io/github/v/release/viys/stealthlingo)](https://github.com/viys/stealthlingo/releases) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](../../LICENSE)

Несколько английских слов, пока вы отлыниваете. Нажмите **Esc** — снова обычный терминал; нажмите Esc ещё раз — вы вернулись. Пока экран скрыт, таймер на паузе.

Эта пауза — любая, которая у вас уже есть: сборка, встреча, на которой вас по сути нет, ожидание ответа ИИ. Найдите слово, сохраните и потренируйтесь с карточками, правописанием или аудированием. Данные словаря берутся из английского [Wiktionary](https://en.wiktionary.org/) (без API-ключа). Прогресс хранится в локальном SQLite, поэтому сохранённые слова работают офлайн. Опциональный MCP-сервер позволяет агенту добавлять замеченные слова; сам StealthLingo никогда не вызывает модель ИИ.

![Главная — дневная цель, слова к повторению и меню](../images/home.png)

Полные руководства в **[Wiki](https://github.com/viys/stealthlingo/wiki)** (клавиши, команды, режимы, MCP, бэкапы, FAQ, разработка). В v0.1 только английский.

## Почему StealthLingo

Пауза, из которой можно выйти сразу: короткая сессия, Esc когда кто-то подходит, и список из слов, которые вы реально встречаете — в доках, коде или чате.

| Есть | Нет |
|---|---|
| Полноэкранный TUI или промпты `--plain` | Аккаунты, облачная синхронизация, GUI |
| Esc скрывает экран; пока он скрыт, таймер на паузе | Выход из программы, чтобы убрать её с глаз |
| Карточки, правописание, пропуски букв, аудирование, смешанный режим, повтор ошибок | Определения от ИИ |
| Дневная цель + интервальные повторения (упрощённый SM-2) | Обязательные длинные курсы |
| Аудио UK / US, если есть в Wiktionary | Сеть обязательна для повторения |

## Установка

Готовые сборки для Windows, macOS и Linux — Rust не нужен:

```powershell
# Windows (PowerShell)
powershell -ExecutionPolicy Bypass -c "irm https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.ps1 | iex"
```

```bash
# macOS / Linux
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.sh | sh
```

Или из исходников (Rust 1.88+; для сборки на Linux нужен `libasound2-dev`):

```bash
cargo install --git https://github.com/viys/stealthlingo
```

## Быстрый старт

```bash
stealthlingo                           # полноэкранное приложение; Esc скрывает
stealthlingo study --minutes 3         # сессия как раз на эту паузу
stealthlingo lookup ephemeral          # произношение и основные определения (--all для всего)
stealthlingo add ephemeral --note "lasting a short time"
stealthlingo --plain                   # построчные промпты
```

На главной: `r` повторение · `a` практика · `/` поиск · `w` список · `t` статистика · `c` настройки · **Esc** скрыть · **Ctrl+C** выход (`q` только назад).

Режимы практики (`a` или буква напрямую): `s` карточки · `p` правописание · `m` пропуски · `i` аудирование · `x` смешанный · `e` повтор ошибок.

## Команды

| Команда | Что делает |
|---|---|
| `stealthlingo` | Открыть полноэкранное приложение |
| `lookup <WORD> [--all]` | Запрос к Wiktionary (офлайн — из кэша) |
| `add` / `remove` | Сохранить или удалить слово |
| `words` / `search` | Список или поиск (`--status`, `--pos`, `--due`) |
| `study` / `review` | Практика (`--mode`, `--mistakes`, `--minutes`, `--count`) |
| `stats` | Прогресс и точность |
| `audio <WORD>` | Воспроизвести произношение |
| `config` | Показать или изменить настройки |
| `export` / `import` | Бэкап `.json` или список `.csv` |
| `mcp` | MCP-сервер для ИИ-агентов |
| `links` | Кликабельный lookup (Windows) |

```bash
stealthlingo study --mode spelling --count 10
stealthlingo study --mode listening --minutes 5
stealthlingo study --mistakes
stealthlingo review
```

## ИИ-агенты (MCP)

Пока вы действительно работаете, агент может сохранить слова к следующему разу, когда вы отлыниваете. Укажите StealthLingo в MCP-конфиге клиента (например Cursor `~/.cursor/mcp.json`):

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

Агенты могут искать слова и добавлять их в список. Удалять слова, менять настройки и записывать ответы они не могут. Подробнее: [Wiki · AI Agents](https://github.com/viys/stealthlingo/wiki/AI-Agents-MCP).

## Данные и бэкапы

Каталог данных по умолчанию: `%APPDATA%\stealthlingo\data` (Windows), `~/Library/Application Support/stealthlingo` (macOS), `~/.local/share/stealthlingo` (Linux). Переопределяется через `STEALTHLINGO_HOME`.

```bash
stealthlingo export backup.json    # полный бэкап
stealthlingo import backup.json    # слияние (побеждает более свежий прогресс)
```

## Участие

Issues и pull request приветствуются. Сборка и тесты: [Wiki · Development](https://github.com/viys/stealthlingo/wiki/Development).

## Лицензия

MIT — см. [LICENSE](../../LICENSE).
