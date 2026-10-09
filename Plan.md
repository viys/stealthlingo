# StealthLingo — CLI 摸鱼式外语学习工具规划

> **产品定位：** 一个使用 Rust 开发、以英语 Wiktionary（通过 Wikimedia 官方 API）作为词典数据来源的终端外语学习工具。它不是 GUI 应用，也不是单纯的终端词典，而是让用户在碎片时间里快速查词、记忆、拼写和听音拼写的低干扰学习工具。
>
> **核心理念：** 打开终端，几秒进入练习；每轮只花 3–5 分钟；按一下 Esc 就能藏起来；本地保留学习进度；不用网络也能复习已经缓存的词。

> **当前状态（v0.1.0）：** P0 全部完成，并已加入全屏 TUI、老板键、英美音选择、可点击查词结果（Windows）和预编译二进制发布。早期版本使用的 Free Dictionary API 已经弃用，只保留读取旧缓存的兼容代码。

## 1. 产品原则

1. **终端优先。** 所有功能都在终端里完成，不开发 GUI。在交互式终端中默认打开全屏 TUI；`--plain` 或非终端输入输出（管道等）时回退到逐行交互，两种界面共用同一套学习逻辑。
2. **碎片化学习。** 支持短时长会话和随时退出，不强制完成长课程。
3. **键盘驱动。** 全屏界面使用单键操作（翻卡不需要回车），尽量减少菜单和重复输入。
4. **低干扰、可隐藏。** Esc 立刻隐藏界面并回到启动它的 shell，再按 Esc 恢复；隐藏期间会话计时暂停。
5. **词典提供数据，软件提供学习能力。** Wiktionary 提供词义、词性、例句、音标、发音、同义词和反义词；复习计划、答题、学习记录由 StealthLingo 自己实现。
6. **离线优先。** 查询过的单词和播放过的音频保存在本地，网络不可用时仍可复习已缓存内容。
7. **如实处理缺失数据。** 音频、音标、例句、同义词等字段都可能缺失；缺少音频时听音拼写跳过该词。
8. **先做好核心闭环。** 不加入账号、云同步、AI 对话、复杂插件系统或完整课程体系。

## 2. 核心使用体验

### 快速开始

```bash
# 打开全屏应用：首页显示今日待复习数和菜单
stealthlingo

# 进行 3 分钟的快速学习
stealthlingo study --minutes 3

# 只练习拼写 / 听音拼写
stealthlingo study --mode spelling --minutes 5
stealthlingo study --mode listening --count 10

# 逐行交互（不进入全屏界面）
stealthlingo --plain
```

全屏首页菜单：`r` 复习、`s` 记忆卡片、`p` 拼写、`l` 听音拼写、`/` 查词、`w` 词库、`t` 统计、`q` 退出。

`--plain` 模式下默认命令直接给出有用的下一步：有到期词条时优先复习；没有到期词时学习新词；词库为空时提示如何查询并收藏第一个单词。

### 示例学习过程

```text
StealthLingo · Quick Study
Session: 3 minutes
Due: 12 words

Word: ephemeral

[Space] Reveal meaning   [a] Audio   [s] Skip   [q] Quit
```

揭晓后：

```text
Pronunciation: UK /ɪˈfɛm(ə)ɹəl/ · US /ɪˈfɛm(ə)ɹəl/
Meaning: lasting for a very short time
Part of speech: adjective
Example: The beauty of youth is ephemeral.

[1] Again  [2] Hard  [3] Good  [4] Easy
```

听音拼写模式：

```text
Listening spelling · 4 / 10

♪ playing UK
[Enter on empty] Replay   [Ctrl+R] Other accent   [Tab] Show answer   [Ctrl+N] Skip
Your spelling: _
```

## 3. 功能规划

### P0 — v0.1 可用 MVP（已完成）

- [x] `lookup <word>`：查询 Wiktionary 并展示词典信息；`--all` 显示全部定义、例句、同义词、反义词和来源。
- [x] 展示可用的音标（按 UK / US 标注）、词性、定义、例句、同义词和反义词。
- [x] 播放查询结果中存在的发音音频（Wikimedia Commons 的 OGG 录音）。
- [x] `add <word>`：把单词加入本地学习词库，可附带个人笔记 `--note`。
- [x] `words` / `search` / `remove`：查看、搜索和移除已收藏的词。
- [x] `study`：以卡片形式进行单词记忆。
- [x] `study --mode spelling`：根据定义输入单词拼写（定义和例句中的目标词会被遮住）。
- [x] `study --mode listening`：播放单词音频，隐藏词形，要求用户输入拼写。
- [x] `review`：复习已到期的词条。
- [x] 本地保存单词数据、答题记录和下一次复习时间。
- [x] `stats`：展示收藏数、今日练习量、准确率、到期复习数。
- [x] 支持 Ctrl+C / `q` 等退出方式，已提交的答题即时保存。
- [x] 处理无网络、单词不存在、API 返回异常、无发音音频、字段缺失等情况。

### P1 — 让日常使用更顺手

已完成：

- [x] `study --count 10`：按题量开始练习。
- [x] CSV / JSON 导出和导入个人词库及学习进度。
- [x] 单词详情刷新：`lookup` 总会联网刷新缓存，离线时回退到缓存；发音数据没取全的条目在下次使用时自动补全。
- [x] 清晰展示多个定义和多个发音变体：默认展示前三个词性各前三条定义，`--all` 展示全部；发音按 UK、US 顺序标注。
- [x] 英美音选择：`accent` 配置决定默认口音，`audio --accent uk|us` 临时指定；全屏界面中重复按 `a` 在已录制的口音之间轮换。
- [x] 拼错时提示多余或缺少的字母。
- [x] 错题在同一会话末尾再出现一次。

待做：

- [ ] 缺字母拼写，例如 `e_ph_m_r_l`。
- [ ] 错词重练模式，优先复习最近写错的词（可利用 `lapses` 和 `practice_attempts`）。
- [ ] `study --mode mixed`：混合记忆、拼写和听音拼写。
- [ ] 按词性、熟悉程度和到期状态筛选词库。

### P2 — 后续探索

- [x] 全屏 TUI（ratatui），不影响核心 CLI 命令；`--plain` 或非终端时回退到逐行交互。
- [x] 老板键：Esc 一键隐藏 / 恢复，隐藏期间会话计时暂停。
- [x] 可点击的查词结果（Windows）：`links install` 注册 `stealthlingo://` 协议后，在支持超链接的终端中 Ctrl+点击发音即可播放，点击末行即可收藏或移除单词。
- [x] 预编译二进制和一键安装脚本（Windows / macOS / Linux）。
- [ ] 可点击链接支持 macOS / Linux。
- [ ] 句子听写：需要另外的句子音频或 TTS 来源，Wiktionary 不为例句提供音频。
- [ ] 自定义词组和专题练习。
- [ ] 更多语言：为每种语言确认 Wiktionary 对应版本的字段覆盖率和解析方式后再支持。
- [ ] 可选 AI 辅助释义或生成例句；不作为核心依赖。

## 4. CLI 命令设计

```text
stealthlingo [--plain] [COMMAND]

Commands:
  lookup <WORD> [--all]           查询 Wiktionary（刷新缓存，离线时用缓存）
  add <WORD> [--note TEXT]        收藏单词；重复收藏无副作用，--note 更新笔记
  remove <WORD>                   从个人词库移除单词（保留词典缓存和历史记录）
  words                           列出已收藏单词
  search <QUERY>                  搜索本地词库和个人笔记
  study [--mode M] [--minutes N] [--count N]
                                  开始学习会话，M = memory | spelling | listening
  review [--minutes N] [--count N]
                                  只复习到期词条
  stats                           查看学习统计
  audio <WORD> [--accent uk|us]   播放单词发音
  config [show | set KEY VALUE]   查看或修改配置及数据目录
  links [status|install|uninstall]
                                  可点击查词结果（Windows）
  export <PATH>                   导出（.json 完整备份 / .csv 词表）
  import <PATH>                   导入（.json 备份 / 含 word 列的 .csv）
```

不带 `--minutes` 或 `--count` 时，会话时长为 `default_minutes`（默认 5 分钟）。

配置项（`config.json`）：

| 键 | 默认值 | 含义 |
|---|---|---|
| `daily_new_limit` | 10 | 每天最多引入的新词数 |
| `default_minutes` | 5 | 默认会话时长 |
| `http_timeout_secs` | 10 | 一次查词（含全部请求）的总超时 |
| `accent` | `UK` | 默认播放的口音（`uk` / `us`） |

旧版本的 `dictionary_source`、`free_dictionary_url`、`merriam_webster_key` 等配置项已移除：读取时忽略，下次保存时丢弃；尝试 `config set` 会提示“现在始终使用 Wiktionary”。

## 5. Wiktionary 接入

### Endpoint

一次查词发两个请求，共享 `http_timeout_secs` 的总时间预算：

```text
# 1. 定义和例句（Wikimedia REST API）
GET https://en.wiktionary.org/api/rest_v1/page/definition/{title}

# 2. 页面 wikitext：IPA、发音音频、同义词、反义词（MediaWiki Action API）
GET https://en.wiktionary.org/w/api.php?action=parse&page={title}&prop=wikitext&format=json&formatversion=2&redirects=1

# 音频文件（Wikimedia Commons）
https://commons.wikimedia.org/wiki/Special:FilePath/{file}
```

只解析英语部分：REST 响应取 `en` 语言键，wikitext 取 `==English==` 段落。不需要 API key。

### 接入原则

1. 在 `dictionary` 模块中封装 API，CLI 命令和 TUI 只接触内部 `Entry` 模型，不依赖远端 JSON 或 wikitext 结构。
2. 用 `serde` 解析 REST 响应，HTML 定义转为纯文本；wikitext 用轻量的模板解析器提取 `{{IPA}}`、`{{audio}}`、`{{a|...}}`/`{{accent|...}}`、`{{syn}}`、`{{ant}}`。
3. 可选字段全部按可缺失处理。第二个请求（wikitext）是尽力而为的：失败或超时时仍返回定义，并把条目记为 `wiktionary-partial`，下次使用时自动补全。
4. Wiktionary 页面标题区分大小写：先按用户输入查询，查不到再用小写重试（`Ephemeral` → `ephemeral`）；输入形式记录为别名，保证 `add`、`remove` 指向同一个词。
5. 设置 HTTP 超时和带项目地址的 User-Agent；将“查无此词”、无法连接、超时、服务端异常分别提示。
6. 查词或显式刷新时访问 API；常规复习从 SQLite 读取，不重复请求远端。
7. 音频按 UK / US 标注：优先使用模板中的口音标签，其次继承上级列表项，最后根据文件名推断；每个录音只保留一个口音标签。
8. 音频播放封装为单独模块：首次播放时下载到数据目录的 `audio/`，之后从本地重放。没有可播放音频时，听音拼写跳过该词。
9. 不假设 Wiktionary 能提供中文翻译、学习进度、等级体系、可靠的词频排名或句子音频。
10. 遵守 Wikimedia 的使用条款。Wiktionary 内容以 CC BY-SA 4.0 授权，每个条目保存来源 URL 和许可证，`lookup --all` 会显示；不批量再分发缓存数据。

### 旧数据兼容

v0.1 早期版本把 Free Dictionary API 的原始响应存进数据库，中间版本还支持过 Merriam-Webster。现在：

- 不再访问这些服务；`dictionary::legacy` 只负责解码旧缓存行和第 1 版 JSON 备份。
- 旧缓存可以离线阅读和学习，下次 `lookup` 时被 Wiktionary 数据替换，学习进度和笔记保留。

### 中文提示的处理

Wiktionary 不是翻译服务。英语定义作为主要提示，用户可以用 `add --note` 或在词库界面按 `n` 给收藏词添加个人中文释义。这个字段属于用户补充信息，不会伪装成词典返回的内容。

## 6. 学习模式设计

全屏界面和 `--plain` 逐行交互都驱动同一个 `learning::session::Session`。任何模式下都可以跳过当前词（不记录答案）或结束会话；已提交的答案立即写入数据库，未作答的题目不会被记为答错。

### 6.1 记忆模式

1. 显示单词，可按 `a` 播放发音（再按一次切换到另一种口音）。
2. 用户尝试回忆含义，按 `Space`（plain 模式为 Enter）揭晓。
3. 展示词性、定义、可用例句及音标。
4. 用户选择 `1 Again / 2 Hard / 3 Good / 4 Easy`。
5. 保存答题记录并计算下次复习时间。

### 6.2 拼写模式

“定义 → 拼写”：用户看到英文定义（其中的目标词被遮住），输入对应词条。`Tab`（plain 模式为 `?`）放弃并显示答案。

答题判断规则：

- 去掉输入首尾空白，合并中间多余空白。
- 英语单词忽略大小写差异。
- 撇号、连字符等标点必须一致（`well-being` 不等于 `wellbeing`）。
- 答错后显示标准答案，并标出多余或缺少的字母。
- “本题是否正确”和“用于排程的评分”分开存储：拼写和听音答对记为 Good，答错记为 Again。

### 6.3 听音拼写模式

1. 只选取有可用音频的已收藏单词。
2. 不显示单词拼写，只播放发音（默认口音优先）。
3. 空行回车重播，`Ctrl+R` 播放另一种口音（plain 模式用 `r` 重播）。
4. 显示对错、正确拼写、定义和音标。
5. 记录用时与结果，安排后续复习。

单词发音和句子听写是两件不同的事。目前专注于单词级听音拼写。

## 7. 复习计划

采用简化的 SM-2 算法。排程是不依赖 API、CLI 和 SQLite 的纯函数，测试在 `tests/scheduling_tests.rs`。

- `Again`：10 分钟后再出现，进度重置，`lapses` 加一。
- `Hard`：比正常间隔更短。
- `Good`：第一次 1 天，第二次 3 天，之后按该词的 ease factor 增长。
- `Easy`：更长的间隔，并提高该词的 ease factor。
- 间隔达到 21 天及以上的词显示为 `mastered`。

会话选题：

1. 先安排到期复习词。
2. 每天最多引入 `daily_new_limit` 个从未学过的新词，避免新词淹没复习任务。
3. 本次会话答错的词在会话末尾再出现一次，不会无限重复。
4. 听力模式只选择有可用音频的词条。

## 8. Rust 技术栈

| Crate | 职责 |
|---|---|
| `clap` | 子命令、参数解析、自动帮助信息 |
| `reqwest`（blocking + rustls） | 调用 Wiktionary API、下载音频 |
| `serde` / `serde_json` | JSON 序列化和反序列化 |
| `rusqlite`（`bundled`） | SQLite 本地存储，无需系统 SQLite |
| `directories` | 跨平台应用数据目录 |
| `anyhow` / `thiserror` | 应用层错误上下文 / 词典错误分类 |
| `chrono` | 复习时间与学习日期 |
| `rodio` | 播放 OGG（vorbis）、MP3、WAV、FLAC 音频；Linux 依赖 ALSA |
| `ratatui`（crossterm 后端） | 全屏 TUI |
| `unicode-width` | 终端中按显示宽度换行和对齐 |
| `ctrlc` | Ctrl+C 处理，保证终端状态被恢复 |
| `csv` | 导入导出 |

不引入 Web 框架、ORM、异步运行时或插件机制；`reqwest` 阻塞接口足以满足需求。发布使用 [dist](https://github.com/axodotdev/cargo-dist)。

## 9. 目录结构

```text
stealthlingo/
├── Cargo.toml
├── dist-workspace.toml          # dist 发布配置
├── README.md
├── LICENSE
├── .github/workflows/release.yml
├── examples/words.csv
├── migrations/
│   ├── 001_initial.sql
│   ├── 002_dictionary_sources.sql
│   ├── 003_word_aliases.sql
│   └── 004_archived_words.sql
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── cli.rs
│   ├── config.rs
│   ├── error.rs
│   ├── time.rs
│   ├── bin/
│   │   └── stealthlingo-link.rs # stealthlingo:// 链接的无窗口处理程序
│   ├── commands/                # 每个子命令一个文件，另有 links.rs、io.rs（导入导出）
│   ├── dictionary/
│   │   ├── client.rs            # HTTP 请求、超时预算、大小写回退
│   │   ├── wiktionary.rs        # REST 定义解析 + wikitext 解析
│   │   ├── markup.rs            # HTML 转纯文本
│   │   ├── models.rs            # Entry / Meaning / Phonetic
│   │   └── legacy.rs            # 旧版 Free Dictionary 缓存解码
│   ├── learning/
│   │   ├── scheduling.rs
│   │   ├── session.rs
│   │   └── spelling.rs
│   ├── audio/
│   ├── storage/
│   │   ├── database.rs          # 连接与版本化迁移
│   │   ├── repository.rs
│   │   └── backup.rs            # JSON 备份
│   └── tui/                     # home / study / words / lookup / stats 等界面
└── tests/
    ├── commands_tests.rs
    ├── dictionary_tests.rs
    ├── scheduling_tests.rs
    ├── storage_tests.rs
    ├── tui_tests.rs
    └── fixtures/                # Wiktionary 响应样本，以及旧版 Free Dictionary 样本
```

## 10. 本地数据模型

数据目录下有 `stealthlingo.db`（SQLite）、`config.json`、`audio/`（下载的发音文件）和 `link-errors.log`。数据库使用版本化迁移，升级不会破坏学习进度。

### `words`（词典缓存）

- `id`、`language_code`（目前只有 `en`）、`headword_normalized`、`display_word`
- `raw_response_json`：原始响应
- `entry_json`：规范化后的 `Entry`；为空表示 v0.1 的 Free Dictionary 旧数据
- `source`：`wiktionary`、`wiktionary-partial`，或旧数据的 `free-dictionary` 等
- `has_audio`、`source_fetched_at`、`created_at`

### `word_aliases`

用户输入形式与词典词头不同时（如 `Ephemeral` → `ephemeral`）记录别名，之后查询输入形式也能命中缓存。

### `user_words`（学习词库与排程状态）

- `word_id`
- `status`（`new`、`learning`、`review`、`mastered`）
- `personal_note`（个人中文释义或记忆提示）
- `due_at`、`interval_days`、`repetitions`、`ease_factor`、`lapses`
- `added_at`、`last_reviewed_at`

### `archived_user_words`

通过链接点击“Remove”移除的单词会连同排程状态归档。因为其他程序也能触发这些链接，这类移除可以恢复：再次收藏时还原进度。`remove` 命令则让该词从头开始。

### `practice_attempts`

- `id`、`word_id`
- `mode`（`memory`、`spelling`、`listening_spelling`）
- `expected_answer`、`submitted_answer`
- `is_correct` 与 `grade`（`again`、`hard`、`good`、`easy`）分开存储
- `duration_ms`（可选）、`created_at`

## 11. 错误处理和离线行为

- 查词失败时显示具体原因（查无此词、无法连接、超时、服务端错误），不 panic。
- 学习、复习、`words`、`search`、`stats` 从不联网。
- `lookup` 联网刷新，连不上时显示缓存副本；`add` 和 `audio` 优先用缓存，只在遇到新词时联网。
- 发音数据未取全的条目照常可用，之后自动补全。
- 音频首次播放时下载并缓存；下载或播放失败时提示原因，不影响答题。
- 用户中途退出时，已经提交的答题保留；未提交答案不记为答错。第二次 Ctrl+C 立即退出，并恢复终端状态。
- 数据库写入失败时，不报告保存成功。
- 重复收藏同一个单词是幂等的；JSON 导入可重复执行，同一个词以最近复习过的进度为准。

## 12. 开发阶段

| 阶段 | 内容 | 状态 |
|---|---|---|
| 1. CLI 骨架 | `--help`、`--version`、默认命令、配置和数据目录 | 完成 |
| 2. 词典查询与缓存 | 词典接入、解析、缓存、各类错误处理 | 完成（已从 Free Dictionary 迁移到 Wiktionary） |
| 3. 收藏与记忆复习 | `add`、`words`、`review`、间隔排程 | 完成 |
| 4. 拼写与听音拼写 | 两种拼写模式、重播、错词记录、答题规则测试 | 完成 |
| 5. 统计与导入导出 | `stats`、JSON / CSV 导入导出、数据库迁移 | 完成 |
| 6. 全屏界面 | ratatui TUI、老板键、英美音轮换、小终端自适应布局 | 完成，布局仍在打磨 |
| 7. 发布 | dist 构建五个平台的二进制，生成 shell / PowerShell 安装脚本 | 完成 |
| 8. 日常使用增强 | P1 待做项：缺字母拼写、错词重练、混合模式、词库筛选 | 下一步 |

## 13. v0.1 发布清单

- [x] 默认命令打开全屏应用，或在 `--plain` 下直接开始今日学习 / 给出清晰的下一步。
- [x] 可查询、收藏和搜索单词。
- [x] 可查看 Wiktionary 返回的定义、词性、音标、例句和可用音频。
- [x] 可执行记忆、拼写、听音拼写三种练习。
- [x] 学习进度保存在本地，复习不依赖每次联网。
- [x] 能查看今日学习量、正确率和待复习数。
- [x] 错误提示足够清晰，缺失字段不会导致程序崩溃。
- [x] README 说明安装、使用、数据目录、词典数据边界和许可注意事项。
- [x] 推送 `v*` 标签后，GitHub Actions 自动构建并发布 Release。

## 14. 明确不做的事情

- 不开发 GUI。
- 不开发账号、云端同步或服务端。
- 不把 Wiktionary 当作中英翻译服务。
- 不假设所有单词都有音频、音标或例句。
- 不在近期实现句子级听写、AI 会话或大型游戏系统。
- 不为尚未验证的需求提前搭建复杂插件架构。

**目标：让用户在终端输入一条简短命令，就能用几分钟查词、记忆、拼写或听音拼写，随时一键隐藏，并让每次练习都积累为下一次更有效的复习。**
