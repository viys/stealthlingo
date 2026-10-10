# stealthlingo

[English](../../README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | **日本語** | [한국어](README.ko.md) | [Español](README.es.md) | [Português (Brasil)](README.pt-BR.md) | [Русский](README.ru.md) | [Tiếng Việt](README.vi.md)

こっそり学べる、ターミナル中心の語学学習ツールです。

StealthLingo は、スキマ時間に語彙を練習するための小さな Rust 製 CLI です。単語を調べて保存し、フラッシュカード・スペリング・リスニング（聞き取って書く）の 3〜5 分のセッションを行えます。辞書データは英語版 [Wiktionary](https://en.wiktionary.org/) から Wikimedia の公式 API 経由で取得し、API キーは不要です。復習スケジュール、正誤判定、学習の進捗はローカルの SQLite データベースに保存されるため、保存した単語はオフラインでも復習できます。

v0.1 で学習できる言語は英語のみです。

## インストール

Windows（x64）、macOS（Intel と Apple シリコン）、Linux（x64 と ARM64）向けのビルド済みプログラムを [Releases ページ](https://github.com/viys/stealthlingo/releases)で公開しています。Rust は不要です。インストーラーは `stealthlingo` とリンク用ヘルパー `stealthlingo-link` を `~/.cargo/bin` に配置し、そのフォルダーを `PATH` に追加します。

Windows（PowerShell）：

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.ps1 | iex"
```

macOS と Linux：

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.sh | sh
```

または、Releases ページからお使いのシステム向けのアーカイブをダウンロードして展開し、`stealthlingo` と `stealthlingo-link` を同じフォルダーに置いてください。Linux では音声再生に ALSA（`libasound2`）を使います。デスクトップ向けディストリビューションには標準で含まれています。

### ソースからビルド

最近の安定版 Rust ツールチェーン（1.88 以上）と C コンパイラが必要です（SQLite は同梱）。

```bash
cargo install --git https://github.com/viys/stealthlingo
# または、クローンしたリポジトリで
cargo install --path .
cargo run -- --help
```

Linux でビルドするには ALSA の開発パッケージ（例：`libasound2-dev`）も必要です。

## クイックスタート

```bash
stealthlingo lookup ephemeral          # 発音と主な語義を表示（--all ですべて表示）
stealthlingo add ephemeral --note 儚い  # 単語を保存（個人メモは任意）
stealthlingo                           # フルスクリーンアプリを開く：練習、検索、単語一覧、統計
```

## フルスクリーン画面

ターミナルで `stealthlingo`、`study`、`review` を実行するとフルスクリーン画面が開きます。ホーム画面には今日の復習予定とメニュー（`r` 復習、`s` フラッシュカード、`p` スペリング、`l` リスニング、`/` 単語検索、`w` 単語一覧、`t` 統計、`q` 終了）が表示されます。`↑`/`↓`（または `k`/`j`）と `Enter` でも操作できます。リストや長いページも同じキーでスクロールします。

- **Esc で即座にすべてを隠し**、起動前のシェルを表示します。もう一度 Esc を押すと戻ります。非表示の間はセッションのタイマーが止まります。
- **フラッシュカード**では Enter は不要です。`Space` で答えを表示、`1`〜`4` で評価、`a` で発音を再生、`s` でスキップ、`q` でセッション終了。
- **アクセント**：`a` はまず既定のアクセント（`accent` 設定）を再生します。同じ単語でもう一度押すと録音されている別のアクセントを再生し、以降も順に切り替わります。次に再生されるアクセントはフッターに、再生中のアクセントはヘッダーに表示されます。
- **スペリングとリスニング**：単語を入力して `Enter`。`Tab` で答えを表示、`Ctrl+N` でスキップ。リスニングでは空のまま `Enter` を押すと録音を再生し直し、`Ctrl+R` で別のアクセントを再生します。間違えると、余分な文字や足りない文字が表示されます。
- **単語一覧**：`/` で絞り込み、`Enter` で保存済みの項目を開く（オフライン可）、`a` で再生、`n` でメモを編集、`d` に続けて `y` で単語を削除（もう一度保存すると進捗が復元されます）。`PgUp`/`PgDn` と `g`/`G` で長いリストを移動できます。
- **単語検索**：単語を入力して `Enter`。その後 `s` で保存または削除、`Tab` で簡易表示と詳細表示を切り替え、`/` で新しい検索、`q` で戻ります。
- `Ctrl+C` は実行中のセッションを終了し、絞り込みやメモの編集を取り消し、それ以外の場面ではアプリを終了します。回答はその都度すぐに保存されます。
- 画面には 40×12 文字以上のターミナルが必要です。高さが足りないウィンドウでは枠が縮み、ホームの概要が 1 行にまとまり、メニューがスクロールするので、選択中の項目と解答欄は常に表示されます。

`--plain` を付けた場合（またはパイプなど、入出力がターミナルでない場合）は、下で説明する 1 行ずつのプロンプトを使います。`NO_COLOR` を設定すると色を無効にできます。

## コマンド

| コマンド | 内容 |
|---|---|
| `stealthlingo` | フルスクリーンアプリを開く。`--plain` 付きの場合：期限が来た単語を復習し、なければ新しい単語を学習。単語一覧が空なら始め方を案内 |
| `lookup <WORD> [--all]` | Wiktionary を検索（キャッシュを更新。オフライン時はキャッシュを表示） |
| `add <WORD> [--note TEXT]` | 単語を保存。2 回保存しても問題なし。`--note` でメモを更新 |
| `remove <WORD>` | 単語一覧から削除（キャッシュした辞書データと回答履歴は残ります） |
| `words` | 保存した単語を状態と復習期限付きで一覧表示 |
| `search <QUERY>` | 保存した単語とメモを検索 |
| `study [--mode memory\|spelling\|listening] [--minutes N] [--count N]` | 学習セッション |
| `review [--count N] [--minutes N]` | 期限が来た単語だけをフラッシュカードで復習 |
| `stats` | 状態別の保存単語数、期限が来た単語数、今日と累計の回答数と正答率、次の復習 |
| `audio <WORD> [--accent uk\|us]` | 発音を再生（既定は `accent` 設定） |
| `config` / `config set <KEY> <VALUE>` | 設定とデータの保存場所を表示・変更 |
| `links [status\|install\|uninstall]` | `lookup` の出力を Ctrl+クリックして発音を再生、単語を追加・削除（Windows） |
| `export <PATH>` / `import <PATH>` | `.json` は完全バックアップ、`.csv` は単語リスト |

`--minutes` も `--count` も指定しない場合、セッションは `default_minutes`（5 分）続きます。

`lookup` は発音を 1 行にまとめて英国式、米国式の順に表示し（`Pronunciation: UK /njuː/ · US /nu/`）、`audio` が再生する録音のアクセントを示します（両方ある場合は `--accent uk|us` を案内）。既定では続けて、最大 3 つの品詞についてそれぞれ最初の 3 つの語義を表示します。`lookup --all`（または `-a`）では、すべての語義を例文・類義語・対義語・出典とともに表示します。語義はターミナルの幅で折り返され、続きの行はインデントされます。

### クリックできる検索結果（Windows）

```bash
stealthlingo links install   # インストール後に一度だけ
```

これで現在のユーザーに `stealthlingo://` リンクのハンドラーが登録されます（管理者権限は不要）。以降、`lookup` は録音のある発音をターミナルのハイパーリンクとして表示します。Ctrl+クリックすると、ブラウザーやウィンドウを開かずにバックグラウンドで録音を再生します。最後の行は `+ Add to word list`、保存済みの単語なら `Saved in your word list · Remove` になり、Ctrl+クリックでコマンドを入力せずに単語を追加・削除できます（変更を確認するには `lookup` をもう一度実行してください）。これらのリンクは他のプログラムからも開けるため、リンクによる削除では学習の進捗が保持されます。リンクまたは `add` で再び追加すると進捗が復元されます。一方、`remove` コマンドでは最初からやり直しになります。ハイパーリンクを表示できるターミナル（Windows Terminal、VS Code / Cursor）で動作し、それ以外では `lookup` は `add` と `audio` のコマンドを添えたプレーンテキストを表示します。`links status` で検出結果を確認でき、`FORCE_HYPERLINK=1`（または `0`）で検出を上書きし、`links uninstall` でハンドラーを削除します。エラーはデータディレクトリの `link-errors.log` に記録されます。

### 学習モード

以下のキーは 1 行ずつのプロンプト（`--plain`）用です。フルスクリーン画面では上で説明した 1 キー操作を使います。

- **memory（記憶）**：単語を見て Enter で意味を表示し、自己評価します：`1` Again（もう一度）、`2` Hard（難しい）、`3` Good（できた）、`4` Easy（簡単）。`a` で発音を聞けます。
- **spelling（スペリング）**：語義を読んで単語を入力します（語義と例文では答えの単語が伏せられます）。`?` でギブアップして答えを表示します。
- **listening（リスニング）**：発音を聞いて単語を入力します。`r` で再生し直し。発音の録音がある単語だけが出題されます。

どのモードでも、`s` は回答を記録せずに現在の単語をスキップし、`q` はセッションを終了します。Ctrl+C でもセッションを終了でき、もう一度押すとすぐにプログラムが終了します。回答は送信のたびにすぐ保存されるので、途中でやめても済んだ分は失われず、答えていない問題が不正解として記録されることもありません。

正誤判定では大文字・小文字と前後の空白は無視されますが、アポストロフィとハイフンは区別されます（`well-being` と `wellbeing` は別）。

### 復習スケジュール

簡略化した SM-2 アルゴリズムで、単語が次に出題される時期を決めます：

- **Again**：10 分後に再出題。進捗はリセット。
- **Hard**：通常より短い間隔。
- **Good**：1 日、次に 3 日、その後は単語ごとの易しさ係数（ease factor）に応じて間隔が伸びます。
- **Easy**：より長い間隔になり、以後その単語は易しい扱いになります。

スペリングとリスニングでは、正解は Good、不正解は Again として扱われます。間違えた単語は同じセッションの最後にもう一度出題されます。期限が来た復習は常に新しい単語より先に出題され、まだ学習していない単語は 1 日に最大 `daily_new_limit`（既定 10）個まで追加されます。間隔が 21 日以上になった単語は `mastered`（習得済み）と表示されます。

## 設定

```bash
stealthlingo config set daily_new_limit 15
stealthlingo config set default_minutes 3
stealthlingo config set http_timeout_secs 10
stealthlingo config set accent us        # 米国式の録音を優先して再生（既定：uk）
```

1 つのアクセントでしか録音されていない単語では、常にその録音が再生されます。

### 辞書

StealthLingo は Wikimedia 公式の Wiktionary API を使います。語義と例文は REST API から、IPA（アクセント別）、類義語、対義語、Wikimedia Commons の録音（OGG）はページのソースから取得します。入力した形と辞書の見出し語が異なる場合（`Ephemeral` と `ephemeral` など）は入力した形を記憶するので、`add` と `remove` は同じ保存済みの単語を指します。

v0.1.0 リリース前の開発版では Free Dictionary API や Merriam-Webster も使えました。それらからキャッシュした単語は引き続きオフラインで読むことができ、次に検索したときに Wiktionary のデータに置き換えられます（学習の進捗とメモは保持されます）。`config.json` に残っている古い設定は無視されます。

## データディレクトリ

`stealthlingo config` で正確なパスを表示できます。既定の場所：

| OS | 場所 |
|---|---|
| Windows | `%APPDATA%\stealthlingo\data` |
| macOS | `~/Library/Application Support/stealthlingo` |
| Linux | `~/.local/share/stealthlingo` |

`STEALTHLINGO_HOME` を設定すると別のディレクトリを使えます（ポータブルな利用や実験に便利です）。中には `stealthlingo.db`（SQLite、バージョン管理されたマイグレーション付き）、`config.json`、`audio/`（ダウンロードした発音ファイル）が入ります。

### バックアップと単語リスト

```bash
stealthlingo export backup.json   # すべて：キャッシュ、進捗、回答履歴
stealthlingo import backup.json   # マージ。同じファイルを再インポートしても変化なし
stealthlingo export words.csv     # word, note, status, due_at, definition
stealthlingo import examples/words.csv   # "word" 列（なければ先頭の列）。"note" 列は任意
```

JSON のインポートで同じ単語が両方にある場合は、より最近復習した進捗が優先されます。CSV のインポートでは、まだキャッシュされていない単語をそれぞれ検索するため、ネット接続が必要です。

## オフライン時の動作

- 学習、復習、`words`、`search`、`stats` はネットワークに一切アクセスしません。
- `lookup` は Wiktionary から更新し、接続できない場合はキャッシュを表示します。
- `add` と `audio` はできるだけキャッシュを使い、新しい単語のときだけオンラインになります。
- 発音の音声は最初の再生時にダウンロードされ、以後は再利用されます。

## 辞書データについて

- 各項目には出典 URL とライセンスが付いており、`lookup --all` で確認できます。Wiktionary のコンテンツは CC BY-SA 4.0 でライセンスされています。
- 収録状況は単語によって異なり、発音記号、音声、例文、類義語、対義語はいずれも欠けている場合があります。StealthLingo はすべての項目を任意として扱い、音声のない単語はリスニング練習でスキップされます。
- Wiktionary は翻訳サービスではありません。日本語（やその他の言語）のヒントは辞書からではなく、自分で付けた `--note` から表示されます。
- このサービスは無料で、遅かったり停止していたりすることがあります。Wikimedia の利用規約を守り、ライセンスに従わずにキャッシュしたデータを大量に再配布しないでください。

## 開発

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

コードは `dictionary`（Wiktionary クライアントと、`Entry` モデルに変換するパーサー。リリース前の開発版がキャッシュしたデータ用のデコーダーも含む）、`storage`（SQLite）、`learning`（スケジュール、正誤判定、セッション）、`audio`、`commands`、`tui`（ratatui で作ったフルスクリーン画面）に分かれています。どちらの画面も同じ `learning::session::Session` を使います。スケジュールは純粋関数で、`tests/scheduling_tests.rs` に専用のテストがあります。

### リリース

リリースは [dist](https://github.com/axodotdev/cargo-dist) でビルドします（`dist-workspace.toml`、`.github/workflows/release.yml`）。`Cargo.toml` の `version` を上げてコミットし、対応するタグをプッシュします：

```bash
git tag v0.2.0
git push origin v0.2.0
```

すると GitHub Actions が全プラットフォーム向けにビルドし、アーカイブとインストーラーを GitHub Release として公開します。`dist plan` でビルド内容を事前に確認できます。`dist-workspace.toml` を変更したら、`dist generate` でワークフローを更新してください。

## ライセンス

MIT。詳しくは [LICENSE](../../LICENSE) を参照してください。
