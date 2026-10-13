# StealthLingo

[English](../../README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | **日本語** | [한국어](README.ko.md) | [Español](README.es.md) | [Português (Brasil)](README.pt-BR.md) | [Русский](README.ru.md) | [Tiếng Việt](README.vi.md)

[![Release](https://img.shields.io/github/v/release/viys/stealthlingo)](https://github.com/viys/stealthlingo/releases) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](../../LICENSE) [![M8ven Score](https://m8ven.ai/badge/mcp/viys-stealthlingo-17ubze?v=b87b5893849d721e2b3074b76e72c0c2)](https://m8ven.ai/mcp/viys-stealthlingo-17ubze?s=readme)

サボっているあいだに、英単語を少しやる。**Esc** を押すと、このターミナルでシェルが開いてコマンドを打てる。もう一度押すと練習に戻る。Esc はそのシェルには渡されない。隠れているあいだ、タイマーは止まる。

その隙間は、もともと手元にある何でもよい。ビルド、真面目に出ていない会議、AI の返答待ち。単語を調べて保存し、フラッシュカード、スペリング、リスニングで数分練習する。辞書データは英語 [Wiktionary](https://en.wiktionary.org/) から取得する（API キー不要）。進捗はローカル SQLite にあり、保存した単語はオフラインでも練習できる。任意の MCP サーバーで、エージェントが気づいた単語をリストに追加できる。StealthLingo 自身は AI モデルを呼びません。

![ホーム — 今日の目標、復習予定、練習メニュー](../images/home.png)

詳細は **[Wiki](https://github.com/viys/stealthlingo/wiki)**（英語：キー、全コマンド、練習モード、MCP、バックアップ、FAQ、開発）。v0.1 は英語のみ対応です。

## なぜ StealthLingo か

すぐに抜けられる隙間のためにある。短く練習し、人が来たら Esc。単語リストはドキュメント、コード、チャットで実際に出会った語から育ちます。

| できること | ないもの |
|---|---|
| フルスクリーン TUI または `--plain` プロンプト | アカウント、クラウド同期、GUI |
| Esc で隠し、シェルが開く。もう一度 Esc で戻る。隠れているあいだタイマーは止まる | 画面をしまうためにアプリを終了すること |
| フラッシュカード、スペリング、穴あき、リスニング、ミックス、間違いの再練習 | AI が書いた定義 |
| 毎日の単語目標 + 間隔反復（簡易 SM-2） | 強制の長時間コース |
| 録音がある場合の UK / US 音声 | 復習に必須のネットワーク |

## インストール

Windows / macOS / Linux のバイナリ——Rust は不要です：

```powershell
# Windows (PowerShell)
powershell -ExecutionPolicy Bypass -c "irm https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.ps1 | iex"
```

```bash
# macOS / Linux
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.sh | sh
```

またはソースから（Rust 1.88+；Linux のビルドには `libasound2-dev` が必要）：

```bash
cargo install --git https://github.com/viys/stealthlingo
```

アップデート：

```bash
stealthlingo update            # 最新リリースをインストール
stealthlingo update --check    # 新しいリリースがあるかだけ表示
```

`update` は上のインストールスクリプトで入れたコピーを置き換えます。`cargo install` で入れたコピーの場合は、代わりに実行すべきコマンドを表示します。StealthLingo は 1 日 1 回（ホーム画面と `lookup` の後に）新しいリリースを確認し、あればヘッダーに `update X.Y.Z` を表示します。`stealthlingo config set check_updates off` でオフにできます。

## クイックスタート

```bash
stealthlingo                           # フルスクリーン；Esc で隠す
stealthlingo study --minutes 3         # その隙間にちょうど入る
stealthlingo lookup ephemeral          # 発音と主な定義（--all ですべて）
stealthlingo add ephemeral --note "lasting a short time"
stealthlingo --plain                   # 行ごとのプロンプト
```

ホーム：`r` 復習 · `a` 練習 · `/` 検索 · `w` 単語リスト · `t` 統計 · `c` 設定 · **Esc** 隠す · **Ctrl+C** 終了（`q` は戻るだけ）。

Esc で開いたシェルでも、`lookup` や `add` などのコマンドは普段どおり使えます。フルスクリーン画面はすでに開いているため、そこでもう一度起動すると Esc で戻るよう案内が表示されるだけです。

練習モード（`a`、または文字キーで直接）：`s` フラッシュカード · `p` スペリング · `m` 穴あき · `i` リスニング · `x` ミックス · `e` 間違いの再練習。

## コマンド

| コマンド | 内容 |
|---|---|
| `stealthlingo` | フルスクリーンアプリを開く |
| `lookup <WORD> [--all]` | Wiktionary を照会（オフライン時はキャッシュ） |
| `add` / `remove` | 単語の保存 / 削除 |
| `words` / `search` | 一覧または検索（`--status`、`--pos`、`--due`） |
| `study` / `review` | 練習（`--mode`、`--mistakes`、`--minutes`、`--count`） |
| `stats` | 進捗と正答率 |
| `audio <WORD>` | 発音を再生 |
| `config` | 設定の表示・変更 |
| `export` / `import` | `.json` バックアップまたは `.csv` 単語リスト |
| `mcp` | AI エージェント向け MCP サーバー |
| `links` | クリック可能な lookup（Windows） |
| `update [--check]` | 最新リリースをインストール（`--check` は表示のみ） |

```bash
stealthlingo study --mode spelling --count 10
stealthlingo study --mode listening --minutes 5
stealthlingo study --mistakes
stealthlingo review
```

## AI エージェント（MCP）

実際に作業しているあいだ、エージェントが単語を保存できる。次にサボるときは、もう練習できる。クライアントの MCP 設定で StealthLingo を指定します（例：Cursor の `~/.cursor/mcp.json`）：

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

エージェントは単語の照会とリストへの追加ができます。削除、設定変更、解答の記録はできません。詳細：[Wiki · AI Agents](https://github.com/viys/stealthlingo/wiki/AI-Agents-MCP)。

## データとバックアップ

既定のデータディレクトリ：Windows `%APPDATA%\stealthlingo\data`、macOS `~/Library/Application Support/stealthlingo`、Linux `~/.local/share/stealthlingo`。`STEALTHLINGO_HOME` で上書きできます。

```bash
stealthlingo export backup.json    # 完全バックアップ
stealthlingo import backup.json    # マージ（直近の進捗を優先）
```

## 貢献

Issue と Pull Request を歓迎します。ビルドとテスト：[Wiki · Development](https://github.com/viys/stealthlingo/wiki/Development)。

## ライセンス

MIT — [LICENSE](../../LICENSE) を参照。
