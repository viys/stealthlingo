# stealthlingo

[English](../../README.md) | **简体中文** | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Español](README.es.md) | [Português (Brasil)](README.pt-BR.md) | [Русский](README.ru.md) | [Tiếng Việt](README.vi.md)

一个以终端为主、适合悄悄学习的语言学习工具。

StealthLingo 是一个用 Rust 写的小型命令行工具，用来在零碎时间里练词汇：查一个单词，把它存下来，然后进行 3–5 分钟的闪卡、拼写或听音拼写练习。词典数据来自英文 [Wiktionary（维基词典）](https://en.wiktionary.org/)，通过 Wikimedia 官方 API 获取，无需密钥；复习调度、答案判定和学习进度都保存在本地 SQLite 数据库中，因此已保存的单词可以离线复习。

v0.1 只支持学习英语。

## 安装

[Releases 页面](https://github.com/viys/stealthlingo/releases)提供 Windows（x64）、macOS（Intel 和 Apple 芯片）以及 Linux（x64 和 ARM64）的预编译程序，无需安装 Rust。安装脚本会把 `stealthlingo` 和它的链接辅助程序 `stealthlingo-link` 放进 `~/.cargo/bin`，并把该目录加入 `PATH`。

Windows（PowerShell）：

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.ps1 | iex"
```

macOS 和 Linux：

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.sh | sh
```

也可以从 Releases 页面下载对应系统的压缩包，解压后让 `stealthlingo` 和 `stealthlingo-link` 保持在同一个文件夹里。在 Linux 上，音频播放依赖 ALSA（`libasound2`），桌面发行版默认都已自带。

### 从源码构建

需要较新的稳定版 Rust 工具链（1.88+）和 C 编译器（SQLite 已内置）。

```bash
cargo install --git https://github.com/viys/stealthlingo
# 或者在克隆下来的仓库里
cargo install --path .
cargo run -- --help
```

在 Linux 上构建还需要 ALSA 开发包（例如 `libasound2-dev`）。

## 快速上手

```bash
stealthlingo lookup ephemeral            # 显示发音和主要释义（--all 显示全部）
stealthlingo add ephemeral --note 短暂的  # 保存单词，可附带个人笔记
stealthlingo                             # 打开全屏应用：练习、查词、浏览单词、统计
```

## 全屏界面

在终端里运行 `stealthlingo`、`study` 和 `review` 会打开全屏界面。首页显示今天待复习的内容和一个菜单（`r` 复习、`s` 闪卡、`p` 拼写、`l` 听力、`/` 查词、`w` 单词列表、`t` 统计、`q` 退出）；也可以用 `↑`/`↓`（或 `k`/`j`）加 `Enter` 操作。列表和较长的页面也用同样的键滚动。

- **按 Esc 立刻隐藏全部内容**，显示你启动程序前的命令行；再按一次 Esc 回来。隐藏期间练习计时会暂停。
- **闪卡**不需要按 Enter：`Space` 显示答案，`1`–`4` 打分，`a` 播放发音，`s` 跳过，`q` 结束本次练习。
- **口音**：`a` 先播放默认口音（`accent` 设置）；对同一个单词再按一次会播放另一种已录制的口音，依此类推。底栏提示下一次播放哪种口音，顶栏显示正在播放哪种。
- **拼写和听力**：输入单词后按 `Enter`。`Tab` 显示答案，`Ctrl+N` 跳过。听力练习中在空行按 `Enter` 会重播录音，`Ctrl+R` 播放另一种口音。答错时会标出多了或少了哪些字母。
- **单词列表**：`/` 筛选，`Enter` 打开已保存的词条（离线可用），`a` 播放发音，`n` 编辑笔记，先按 `d` 再按 `y` 删除单词（重新保存会恢复其学习进度）。`PgUp`/`PgDn` 和 `g`/`G` 可以在长列表中快速跳转。
- **查词**：输入单词后按 `Enter`；之后 `s` 保存或删除该词，`Tab` 在简要和完整词条之间切换，`/` 开始新的查询，`q` 返回。
- `Ctrl+C` 结束当前练习、取消筛选或笔记编辑，其余情况下退出程序。每个答案在作答后立即保存。
- 界面需要至少 40×12 个字符的终端。窗口较矮时，边框会收紧，首页摘要压缩为一行，菜单可以滚动，保证选中的菜单项和答题输入框始终可见。

使用 `--plain`（或者输入/输出不是终端，例如通过管道）时，会改用下文介绍的逐行提示模式。设置 `NO_COLOR` 可关闭颜色。

## 命令

| 命令 | 作用 |
|---|---|
| `stealthlingo` | 打开全屏应用。加 `--plain` 时：复习到期的单词；没有到期单词就学习新词；单词列表为空时说明如何开始 |
| `lookup <WORD> [--all]` | 查询 Wiktionary（刷新缓存；离线时退回到缓存的副本） |
| `add <WORD> [--note TEXT]` | 保存单词。重复保存没有副作用；`--note` 会更新笔记 |
| `remove <WORD>` | 从单词列表中删除单词（缓存的词典数据和答题记录会保留） |
| `words` | 列出已保存的单词及其状态和到期时间 |
| `search <QUERY>` | 搜索已保存的单词和笔记 |
| `study [--mode memory\|spelling\|listening] [--minutes N] [--count N]` | 开始一次练习 |
| `review [--count N] [--minutes N]` | 只用闪卡复习到期的单词 |
| `stats` | 按状态统计的已保存单词数、到期数、今日和历史累计的答题数及正确率、下次复习时间 |
| `audio <WORD> [--accent uk\|us]` | 播放发音（默认使用 `accent` 设置） |
| `config` / `config set <KEY> <VALUE>` | 查看或修改设置和数据位置 |
| `links [status\|install\|uninstall]` | 在 `lookup` 输出中 Ctrl+点击即可播放发音、添加或删除单词（Windows） |
| `export <PATH>` / `import <PATH>` | `.json` 为完整备份，`.csv` 为单词列表 |

不指定 `--minutes` 或 `--count` 时，一次练习持续 `default_minutes`（5）分钟。

`lookup` 会在一行中显示发音，先英音后美音（`Pronunciation: UK /njuː/ · US /nu/`），并注明 `audio` 播放的是哪种口音的录音（两种都有时会提示使用 `--accent uk|us`）。默认接着列出最多三个词性、每个词性的前三条释义。`lookup --all`（或 `-a`）显示所有释义以及例句、同义词、反义词和来源。释义会按终端宽度换行，续行带缩进。

### 可点击的查词结果（Windows）

```bash
stealthlingo links install   # 安装后执行一次即可
```

这会为当前用户注册一个 `stealthlingo://` 链接处理程序（不需要管理员权限）。之后 `lookup` 会把每个有录音的发音变成终端超链接：Ctrl+点击即可在后台播放录音，不会打开浏览器或任何窗口。最后一行会变成 `+ Add to word list`，已保存的单词则显示 `Saved in your word list · Remove`；Ctrl+点击它即可添加或删除单词，无需输入命令（再次运行 `lookup` 可看到变化）。由于其他程序也能打开这些链接，通过链接删除的单词会保留学习进度：之后再通过链接或 `add` 添加，进度会恢复。而 `remove` 命令会让该单词从头开始。此功能需要终端支持超链接（Windows Terminal、VS Code / Cursor）；其他终端中 `lookup` 会输出纯文本，并给出对应的 `add` 和 `audio` 命令。`links status` 显示检测结果，`FORCE_HYPERLINK=1`（或 `0`）可覆盖检测，`links uninstall` 移除处理程序。错误会写入数据目录下的 `link-errors.log`。

### 练习模式

下面的按键适用于逐行提示模式（`--plain`）；全屏界面使用上文介绍的单键操作。

- **memory（记忆）**：先看单词，按 Enter 显示释义，然后自评：`1` Again（忘了）、`2` Hard（困难）、`3` Good（良好）、`4` Easy（简单）。按 `a` 听发音。
- **spelling（拼写）**：阅读释义（释义和例句中的目标单词会被遮住），然后输入单词。`?` 表示放弃并显示答案。
- **listening（听力）**：听发音并输入单词。`r` 重播。只会使用有发音录音的单词。

所有模式中，`s` 跳过当前单词且不记录答案，`q` 结束练习。Ctrl+C 也会结束练习；再按一次 Ctrl+C 立即退出。每个提交的答案都会立即保存，所以中途退出不会丢失已完成的部分，未作答的题目也不会被记为答错。

答案判定忽略大小写和首尾空白，但撇号和连字符必须一致（`well-being` 不等于 `wellbeing`）。

### 复习调度

由简化版 SM-2 算法决定单词何时再次出现：

- **Again**：10 分钟后再出现；进度重置。
- **Hard**：间隔比正常更短。
- **Good**：先 1 天，再 3 天，之后间隔按该单词的难易系数（ease factor）增长。
- **Easy**：间隔更长，并且该单词之后会变得更“容易”。

拼写和听力练习中，答对算作 Good，答错算作 Again。答错的单词会在本次练习结束前再出现一次。到期的复习总是排在新词之前，每天最多引入 `daily_new_limit`（默认 10）个从未学过的新词。间隔达到 21 天及以上的单词显示为 `mastered`（已掌握）。

## 配置

```bash
stealthlingo config set daily_new_limit 15
stealthlingo config set default_minutes 3
stealthlingo config set http_timeout_secs 10
stealthlingo config set accent us        # 优先播放美音录音（默认：uk）
```

只有一种口音录音的单词总是播放那段录音。

### 词典

StealthLingo 使用 Wikimedia 官方的 Wiktionary API：释义和例句来自 REST API，国际音标（按口音标注）、同义词、反义词和 Wikimedia Commons 录音（OGG）来自页面源码。当输入的形式和词典词头不同时（例如 `Ephemeral` 和 `ephemeral`），会记住输入的形式，因此 `add` 和 `remove` 指向的是同一个已保存单词。

v0.1.0 发布之前的开发版本还可以使用 Free Dictionary API 或 Merriam-Webster。从这些来源缓存的单词仍可离线查看，并会在下次查询时替换为 Wiktionary 数据（学习进度和笔记会保留）。`config.json` 中对应的旧设置会被忽略。

## 数据目录

`stealthlingo config` 会打印确切路径。默认位置：

| 系统 | 位置 |
|---|---|
| Windows | `%APPDATA%\stealthlingo\data` |
| macOS | `~/Library/Application Support/stealthlingo` |
| Linux | `~/.local/share/stealthlingo` |

设置 `STEALTHLINGO_HOME` 可改用其他目录（适合便携使用或做实验）。该目录包含 `stealthlingo.db`（SQLite，带版本化迁移）、`config.json` 和 `audio/`（已下载的发音文件）。

### 备份与单词列表

```bash
stealthlingo export backup.json   # 全部内容：缓存、进度、答题记录
stealthlingo import backup.json   # 合并导入；重复导入同一文件不会产生变化
stealthlingo export words.csv     # word、note、status、due_at、definition
stealthlingo import examples/words.csv   # 使用 "word" 列（没有则用第一列）；"note" 列可选
```

JSON 导入时，如果同一个单词在两边都存在，以最近复习过的进度为准。CSV 导入会查询每个尚未缓存的单词，因此需要联网。

## 离线行为

- 练习、复习、`words`、`search` 和 `stats` 从不访问网络。
- `lookup` 会从 Wiktionary 刷新数据，无法连接时显示缓存的副本。
- `add` 和 `audio` 尽量使用缓存，只有遇到新单词时才联网。
- 发音音频在第一次播放时下载，之后重复使用。

## 关于词典数据

- 词条附带来源 URL 和许可证，可通过 `lookup --all` 查看。Wiktionary 内容采用 CC BY-SA 4.0 许可。
- 不同单词的数据完整度不一：音标、音频、例句、同义词和反义词都可能缺失。StealthLingo 把每个字段都视为可选；没有音频的单词会在听力练习中被跳过。
- Wiktionary 不是翻译服务。中文（或其他语言）提示来自你自己的 `--note`，而不是词典。
- 该服务免费，可能较慢或暂时不可用。请遵守 Wikimedia 的使用条款，未遵守许可证要求时不要批量再分发缓存数据。

## 开发

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

代码分为 `dictionary`（Wiktionary 客户端和解析器，映射到 `Entry` 模型，另有一个解码器用于读取发布前开发版本缓存的数据）、`storage`（SQLite）、`learning`（复习调度、答案判定、练习会话）、`audio`、`commands` 和 `tui`（基于 ratatui 的全屏界面）。两种界面驱动的是同一个 `learning::session::Session`。复习调度是一个纯函数，在 `tests/scheduling_tests.rs` 中有单独的测试。

### 发布

发布由 [dist](https://github.com/axodotdev/cargo-dist) 构建（`dist-workspace.toml`、`.github/workflows/release.yml`）。先修改 `Cargo.toml` 中的 `version` 并提交，然后推送对应的标签：

```bash
git tag v0.2.0
git push origin v0.2.0
```

GitHub Actions 随后会构建所有平台，并把压缩包和安装脚本发布为 GitHub Release。`dist plan` 可以预览将要构建的内容；修改 `dist-workspace.toml` 后，运行 `dist generate` 更新工作流。

## 许可证

MIT，详见 [LICENSE](../../LICENSE)。
