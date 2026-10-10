# AGENTS.md

StealthLingo 是一个用 Rust 编写的终端英语单词学习工具：查词（英语 Wiktionary）、收藏、记忆卡片 / 拼写 / 听音拼写，间隔复习进度保存在本地 SQLite。交互式终端默认打开全屏 TUI（ratatui），`--plain` 或非终端时回退到逐行交互。

- 产品规划、功能状态和设计决策：`Plan.md`（中文，开始新功能前先读对应章节）
- 用户文档：`README.md`（英文），翻译在 `docs/i18n/README.<lang>.md`

## 常用命令

```bash
cargo build
cargo run -- --help
cargo run -- lookup ephemeral
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

提交前三条检查（test / clippy / fmt）都必须通过。最低 Rust 版本 1.88（`Cargo.toml` 的 `rust-version`），不要使用更新版本才有的 API。Linux 构建需要 `libasound2-dev`。

手动试用时设置 `STEALTHLINGO_HOME` 指向临时目录，避免改动自己的真实词库：

```powershell
$env:STEALTHLINGO_HOME = "$env:TEMP\sl-dev"; cargo run -- stats
```

## 代码结构

| 路径 | 职责 |
|---|---|
| `src/cli.rs` | clap 命令定义 |
| `src/commands/` | 每个子命令一个文件；`mod.rs` 里的 `Context`、`fetch_word`、`cached_or_fetch` 是查词与缓存的入口 |
| `src/dictionary/` | Wiktionary 客户端与解析，对外只暴露 `Entry` 模型；`legacy.rs` 只用于读旧缓存 |
| `src/storage/` | SQLite 连接、版本化迁移（`database.rs`）、查询（`repository.rs`）、JSON 备份 |
| `src/learning/` | 排程（纯函数 SM-2）、会话 `Session`、拼写判定 |
| `src/audio/` | 发音下载缓存与播放（rodio） |
| `src/tui/` | 全屏界面，各屏一个文件 |
| `src/bin/stealthlingo-link.rs` | `stealthlingo://` 链接处理程序（Windows，无窗口） |
| `migrations/` | 按编号递增的 SQL 迁移 |
| `tests/` | 集成测试与 `fixtures/`（Wiktionary 响应样本） |

## 架构约束

- **两种界面共用一套逻辑。** TUI 和 `--plain` 都驱动 `learning::session::Session`；学习规则改在 `learning/`，不要在界面层各写一份。
- **命令和 TUI 只接触 `Entry`。** 不要在 `dictionary` 模块之外解析 Wiktionary 的 JSON 或 wikitext。
- **所有词典字段都可能缺失**（音标、音频、例句、同义词等），一律按 `Option` / 空集合处理，不能 panic。没有音频的词不进入听音拼写。
- **离线优先。** 学习、复习、`words`、`search`、`stats` 绝不联网；`lookup` 联网失败时回退缓存；`add`、`audio` 优先用缓存。
- **释义只来自 Wiktionary。** 个人笔记（`personal_note`）只能由用户自己写，不要把翻译或 AI 生成的内容当成词典数据。
- **已提交的答案立即写库**，未作答不记为答错；数据库写入失败时不能显示保存成功。
- **不引入异步运行时、Web 框架、ORM 或插件机制。** HTTP 使用 `reqwest` 的 blocking 接口。新增依赖前先确认确实必要。
- 不做 GUI、账号、云同步，程序本身不调用 AI。完整清单见 `Plan.md` 第 15 节。

## 数据库迁移

- 新增迁移文件 `migrations/00N_描述.sql`，并在 `src/storage/database.rs` 的 `MIGRATIONS` 数组末尾追加。
- **不要修改已有的迁移文件**，用户数据库靠 `user_version` 按顺序升级。
- 迁移必须保留现有学习进度；涉及 `user_words` 时同步考虑 `archived_user_words` 和 JSON 备份（`storage/backup.rs`）的导入导出。

## 测试

- 测试不得访问真实网络。需要联网失败时，用 `tests/commands_tests.rs` 里 `offline_context` 的做法：把 `Endpoints` 指向 `127.0.0.1:9`。需要词典数据时，用 `tests/fixtures/` 里的样本解析后写入缓存。
- 数据目录用 `tempfile::tempdir()` 加 `Paths::at(...)`，不要读写真实的用户目录。
- 排程算法的测试放在 `tests/scheduling_tests.rs`，TUI 行为的测试放在 `tests/tui_tests.rs`。
- 修 bug 时先补一个能复现问题的测试。

## 代码风格

- 用 `rustfmt` 默认格式；clippy 零警告。
- 应用层错误用 `anyhow`（加上 `.context(...)` 说明上下文），词典错误用 `error.rs` 里的 `thiserror` 类型。错误信息要告诉用户原因和下一步怎么做。
- 代码注释、标识符、面向用户的输出一律用英文。注释只写代码本身表达不出来的约束或原因。
- 终端文字按显示宽度（`unicode-width`）处理对齐和换行，不要按字节或字符数计算。
- TUI 需要在 40×12 的终端里依然可用；修改布局后检查小窗口下的表现。Esc 老板键必须在每个界面都生效。

## 文档同步

- 用户可见的行为变化（命令、参数、按键、配置项、默认值）需要同步更新 `README.md`。
- **只要文档有变更，就必须在同一次修改中同步所有语言的 README**，不能只改英文版，也不能留到以后再补：
  - 英文原文：`README.md`
  - 翻译：`docs/i18n/README.zh-CN.md`、`README.zh-TW.md`、`README.ja.md`、`README.ko.md`、`README.es.md`、`README.pt-BR.md`、`README.ru.md`、`README.vi.md`
- 以英文版为准。每个翻译的章节结构、命令、代码块、按键、配置项和默认值都要和英文版一致，只翻译说明文字；命令、参数名、配置键和代码块内容保持原样。
- 翻译文件里的相对链接要从 `docs/i18n/` 出发（例如 `../../README.md`、`../../LICENSE`）。
- 每个 README 标题下方都有一行语言切换栏，当前语言加粗、不带链接。新增或删除一种语言时，要更新全部 README 的切换栏，并同步本节的文件清单。
- 完成文档修改后，逐个检查所有语言的 README 都已更新，再报告完成。
- 完成或调整 `Plan.md` 中的功能项时，更新对应的复选框和"开发阶段"表。

## Git 与发布

- 提交信息使用 Conventional Commits，英文书写，例如 `feat(tui): ...`、`fix: ...`、`docs: ...`、`ci: ...`。
- 发布由 dist 完成：修改 `Cargo.toml` 的 `version` 并提交，推送对应的 `vX.Y.Z` 标签，由 `.github/workflows/release.yml` 构建。修改 `dist-workspace.toml` 后运行 `dist generate`，不要手改生成的 workflow。
- 未经用户要求，不要打标签、推送或发布。
