# StealthLingo

[English](../../README.md) | [简体中文](README.zh-CN.md) | **繁體中文** | [日本語](README.ja.md) | [한국어](README.ko.md) | [Español](README.es.md) | [Português (Brasil)](README.pt-BR.md) | [Русский](README.ru.md) | [Tiếng Việt](README.vi.md)

[![Release](https://img.shields.io/github/v/release/viys/stealthlingo)](https://github.com/viys/stealthlingo/releases) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](../../LICENSE)

摸魚的時候，背幾個英語單字。按 **Esc**，介面立刻變回終端機；再按一次又回來。隱藏期間計時暫停。

這段空檔是你本來就有的任何一會兒——一次編譯、一場你並沒真正在開的會，或者 AI 還沒出結果。查一個詞、收藏它，用閃卡、拼寫或聽力練幾分鐘。詞典資料來自英文 [Wiktionary](https://en.wiktionary.org/)（無需 API 金鑰）。進度保存在本機 SQLite，已收藏的詞可離線練習。可選的 MCP 伺服器讓程式設計助手把注意到的詞加進列表——StealthLingo 本身從不呼叫 AI 模型。

![首頁 — 今日目標、待複習詞和練習選單](../images/home.png)

完整說明見 **[Wiki](https://github.com/viys/stealthlingo/wiki)**（英文：按鍵、全部命令、練習模式、MCP、備份、FAQ、開發）。v0.1 僅支援英語。

## 為什麼是 StealthLingo

用來填一段隨時能抽身的空檔：練一小會兒，有人過來就 Esc，詞表來自你在文件、程式碼或聊天裡真正遇到的詞。

| 你會得到 | 你不會遇到 |
|---|---|
| 全螢幕 TUI 或 `--plain` 逐行提示 | 帳號、雲端同步或 GUI |
| Esc 隱藏介面；隱藏期間計時暫停 | 得結束程式才能把螢幕讓出去 |
| 閃卡、拼寫、缺字母、聽力、混合、錯詞重練 | AI 產生的釋義 |
| 每日單字目標 + 間隔複習（簡化 SM-2） | 強制長課程 |
| Wiktionary 有錄音時的 UK / US 音訊 | 複習必須連網 |

## 安裝

Windows / macOS / Linux 預編譯套件——無需 Rust：

```powershell
# Windows (PowerShell)
powershell -ExecutionPolicy Bypass -c "irm https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.ps1 | iex"
```

```bash
# macOS / Linux
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.sh | sh
```

或從原始碼安裝（Rust 1.88+；Linux 建置需要 `libasound2-dev`）：

```bash
cargo install --git https://github.com/viys/stealthlingo
```

## 快速開始

```bash
stealthlingo                           # 全螢幕應用；Esc 隱藏
stealthlingo study --minutes 3         # 剛好填上這點空檔
stealthlingo lookup ephemeral          # 發音與主要釋義（--all 顯示全部）
stealthlingo add ephemeral --note "lasting a short time"
stealthlingo --plain                   # 改為逐行提示
```

首頁：`r` 複習 · `a` 練習 · `/` 查詞 · `w` 詞庫 · `t` 統計 · `c` 設定 · **Esc** 隱藏 · **Ctrl+C** 結束（`q` 只返回上一畫面）。

練習模式（`a`，或直接按字母進入）：`s` 閃卡 · `p` 拼寫 · `m` 缺字母 · `i` 聽力 · `x` 混合 · `e` 錯詞重練。

## 命令

| 命令 | 作用 |
|---|---|
| `stealthlingo` | 開啟全螢幕應用 |
| `lookup <WORD> [--all]` | 查詢 Wiktionary（離線回退快取） |
| `add` / `remove` | 收藏或移除單字 |
| `words` / `search` | 列出或搜尋（`--status`、`--pos`、`--due`） |
| `study` / `review` | 練習（`--mode`、`--mistakes`、`--minutes`、`--count`） |
| `stats` | 進度與正確率 |
| `audio <WORD>` | 播放發音 |
| `config` | 檢視或修改設定 |
| `export` / `import` | `.json` 備份或 `.csv` 詞表 |
| `mcp` | 供 AI agent 使用的 MCP 伺服器 |
| `links` | 可點擊查詞（Windows） |

```bash
stealthlingo study --mode spelling --count 10
stealthlingo study --mode listening --minutes 5
stealthlingo study --mistakes
stealthlingo review
```

## AI 助手（MCP）

你真正在做事的時候，助手可以把單字先存好，下次摸魚就能直接練。在客戶端 MCP 設定中指向 StealthLingo（例如 Cursor 的 `~/.cursor/mcp.json`）：

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

Agent 可以查詞並加入列表，但不能刪詞、改設定或記答題。詳見 [Wiki · AI Agents](https://github.com/viys/stealthlingo/wiki/AI-Agents-MCP)。

## 資料與備份

預設資料目錄：Windows `%APPDATA%\stealthlingo\data`，macOS `~/Library/Application Support/stealthlingo`，Linux `~/.local/share/stealthlingo`。可用 `STEALTHLINGO_HOME` 覆寫。

```bash
stealthlingo export backup.json    # 完整備份
stealthlingo import backup.json    # 合併（以最近複習進度為準）
```

## 參與貢獻

歡迎提出 Issue 和 Pull Request。建置與測試見 [Wiki · Development](https://github.com/viys/stealthlingo/wiki/Development)。

## 授權條款

MIT — 見 [LICENSE](../../LICENSE)。
