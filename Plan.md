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
9. **对 agent 开放，但不内置 AI。** 通过 MCP 把"查词、看词库、收藏单词"暴露给用户自己使用的 AI agent；StealthLingo 本身不调用任何模型。agent 只能决定收藏哪些词，释义仍然完全来自 Wiktionary；agent 也不能删除单词或伪造答题记录（见第 8 节）。

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

全屏首页菜单：`r` 复习、`s` 记忆卡片、`p` 拼写、`l` 听音拼写、`/` 查词、`w` 词库、`t` 统计、`c` 设置（规划中）、`q` 退出。首页同时显示今日目标进度，例如 `Today 6 / 10 words`（规划中，见第 4 节 `daily_goal`）。

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
- [ ] 每日学习目标：新增 `daily_goal`（每天练习的单词数量，不按时间计），首页和 `stats` 显示今日进度，达成时给出一行提示；不强制、不打断会话。
- [ ] 所有配置项都可由用户自己设置：全屏设置界面（首页按 `c`）、`config set`、`config reset`，以及直接编辑 `config.json`（见第 4 节）。

### P2 — 后续探索

- [x] 全屏 TUI（ratatui），不影响核心 CLI 命令；`--plain` 或非终端时回退到逐行交互。
- [x] 老板键：Esc 一键隐藏 / 恢复，隐藏期间会话计时暂停。
- [x] 可点击的查词结果（Windows）：`links install` 注册 `stealthlingo://` 协议后，在支持超链接的终端中 Ctrl+点击发音即可播放，点击末行即可收藏或移除单词。
- [x] 预编译二进制和一键安装脚本（Windows / macOS / Linux）。
- [ ] 可点击链接支持 macOS / Linux (暂无测试条件延缓)。
- [ ] MCP 服务器（`stealthlingo mcp`）：让 Cursor、Claude 等 agent 查词、查看词库，并把推荐学习的单词加入学习列表（见第 8 节）。
- [ ] 句子听写：需要另外的句子音频或 TTS 来源，Wiktionary 不为例句提供音频。
- [ ] 自定义词组和专题练习。
- [ ] 更多语言：为每种语言确认 Wiktionary 对应版本的字段覆盖率和解析方式后再支持。
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
  config [show | set KEY VALUE | reset [KEY]]
                                  查看或修改配置及数据目录；reset 恢复默认值（规划中）
  links [status|install|uninstall]
                                  可点击查词结果（Windows）
  export <PATH>                   导出（.json 完整备份 / .csv 词表）
  import <PATH>                   导入（.json 备份 / 含 word 列的 .csv）
  mcp                             以 MCP 服务器方式运行（stdio），供 AI agent 调用（规划中）
```

不带 `--minutes` 或 `--count` 时，会话时长为 `default_minutes`（默认 5 分钟）。规划中：从首页或默认命令开始、且今天还没达成 `daily_goal` 时，改为一直练到今天练过的不同单词数达到目标为止（见下文"每日学习目标"）。

配置项（`config.json`）。下表中的每一项都由用户自己决定，默认值只是第一次使用时的起点：

| 键 | 默认值 | 允许范围 | 含义 |
|---|---|---|---|
| `daily_goal` | 10 | 0–500，0 表示不设目标 | 每天的学习目标：当天练习过的单词数量（规划中） |
| `daily_new_limit` | 10 | 0–200，0 表示只复习不学新词 | 每天最多引入的新词数 |
| `default_minutes` | 5 | 1–120 | 默认会话时长（分钟） |
| `http_timeout_secs` | 10 | 1–120 | 一次查词（含全部请求）的总超时 |
| `accent` | `UK` | `uk` / `us` | 默认播放的口音 |
| `mcp_daily_add_limit` | 30 | 0–200，0 表示禁止 agent 收藏 | 每天最多允许 agent 通过 MCP 新收藏的单词数（规划中） |

旧版本的 `dictionary_source`、`free_dictionary_url`、`merriam_webster_key` 等配置项已移除：读取时忽略，下次保存时丢弃；尝试 `config set` 会提示“现在始终使用 Wiktionary”。

#### 用户如何设置

1. **全屏设置界面（规划中）：** 首页按 `c` 打开，上表所有配置项都能在这里修改，不需要退出全屏界面或另开终端，详见下文"全屏设置界面"。
2. **命令行：** `config set KEY VALUE` 修改单项。目前已支持 `daily_new_limit`、`default_minutes`、`http_timeout_secs`、`accent`；规划中补上 `daily_goal` 和 `mcp_daily_add_limit`，并统一按上表范围校验。`config reset KEY` 恢复单项默认值，`config reset` 恢复全部（会先确认）；`config show` 列出所有项、当前值和默认值。
3. **直接编辑 `config.json`：** 缺少的键使用默认值；值超出范围时启动时提示该项无效，并按最近的边界值运行（如 `daily_new_limit: 300` 按 200），不改写用户的文件；之后在设置界面或用 `config set` 修改任意一项时，才会把校正后的值一并写回。`daily_new_limit` 以前没有上限，按边界值而不是默认值运行，可以避免老用户的设置被悄悄改小。

#### 全屏设置界面（规划中）

```text
StealthLingo · Settings

 Study
 > Daily goal (words)        ◀  10 ▶   0–500, 0 = off
   New words per day         ◀  10 ▶   0–200, 0 = review only
   Default session (min)     ◀   5 ▶   1–120
 Audio
   Default accent            ◀  UK ▶   UK / US
 Agent (MCP)
   Agent adds per day        ◀  30 ▶   0–200, 0 = off
 Network
   Lookup timeout (s)        ◀  10 ▶   1–120

 Words practised today, each counted once. Default: 10
 Saved to C:\Users\...\stealthlingo\config.json

[↑↓] Select  [←→] Adjust  [PgUp/PgDn] ±10  [Enter] Type value  [r] Reset  [R] Reset all  [q] Back
```

- **入口：** 首页菜单 `c`；首页的 `Today 6 / 10 words` 行旁提示 `c` 可调整目标。和词库、统计界面一样，按 `q` 或 Backspace 返回首页（输入数值时 Backspace 用于删除数字）。
- **分组：** 按学习、音频、Agent（MCP）、网络分组，最常改的 `daily_goal` 放在最上面。每项显示当前值和允许范围，选中项在底部显示一句说明和默认值。
- **调整数值：** `←` / `→`（或 `-` / `+`）每次加减 1，`PgUp` / `PgDn` 每次加减 10，到达范围边界时停住不越界。`accent` 用 `←` / `→` 在 UK / US 间切换。
- **直接输入：** 选中数值项按 Enter 进入输入状态，只接受数字；再按 Enter 确认，`↑` / `↓` 移开则放弃输入。超出范围时底部提示允许范围，不保存。
- **目标可达性提示：** `daily_goal` 超过"今天到期的复习词数 + 今天剩余的新词名额"时，在该项下方提示"按当前词库和 `daily_new_limit`，今天最多能练 N 个词"，但仍允许保存。
- **恢复默认：** `r` 恢复当前项；`R` 恢复全部，需要再按 `y` 确认。
- **保存：** 每次修改校验通过后立即写入 `config.json`（先写临时文件再替换，避免中途退出时损坏文件），不需要单独的"保存"按钮；底部短暂显示 `Saved`。写入失败时提示原因，界面上的值退回到修改前。
- **与其他功能的关系：**
  - 老板键 Esc 在设置界面照常生效。
  - 窄终端下隐藏"允许范围"一列，只保留名称和数值。
  - 底部显示配置文件路径和数据目录，与 `config show` 一致。
- **单一来源：** 每个配置项的键名、显示名、默认值、范围、步长和说明集中定义在 `config.rs` 的一张元数据表里；设置界面、`config set` / `config reset` / `config show` 和 `config.json` 加载时的校验都使用这张表，保证三种方式的规则一致。
- **测试：** `tests/tui_tests.rs` 增加设置界面用例：选择与调整、范围边界停住、直接输入超范围被拒绝、修改后 `config.json` 内容正确、`r` / `R` 恢复默认、返回首页后目标进度按新的 `daily_goal` 显示。目标模式另加用例：错题重做不增加进度、练够不同单词数即结束、队列不足时提前结束并给出提示。

#### 何时生效

- 每次开始会话、回到首页、进入统计页时重新读取配置，所以在设置界面或另一个终端里修改后，不需要重启程序。
- `daily_goal`、`daily_new_limit`、`mcp_daily_add_limit` 都按本地日期计算"今天"。今天调小 `daily_new_limit` 时，已经开始学习的新词不受影响，只是今天不再引入更多新词。
- MCP 服务器在每次处理 `add_words` 前读取最新的 `mcp_daily_add_limit`，用户修改后无需重启 agent 客户端。agent 不能修改任何配置。

#### 每日学习目标

- 学习目标按**数量**计算，不按时间计算。`default_minutes` 只决定一次会话默认多长，与每日目标无关。
- `daily_goal` 统计当天在 `practice_attempts` 中至少有一次已提交答案的**不同单词数**：任何模式都算，跳过的不算；同一个词当天练多次（如答错后在会话末尾重做，或先记忆后拼写）只算一个。
- 默认值 10 与 `daily_new_limit` 的默认值相同，保证刚安装、还没有到期复习的用户也能达成目标。
- 首页显示 `Today 6 / 10 words`，未达成时提示还差几个词；`stats` 显示今日进度和最近 7 天的达成情况；达成目标时会话结束页给出一行提示，之后照常可以继续学习。
- 首页和 `--plain` 的默认命令在今天还没达成目标时，以"目标模式"开始会话：会话在今天练过的不同单词数达到 `daily_goal` 时结束，而不是按答题次数结束（现有的 `--count` 按 `max_answers` 计数，会话末尾的错题重做也算一次，不能直接复用）。已达成或 `daily_goal = 0` 时仍按 `default_minutes` 计时。命令行显式传入 `--minutes` 或 `--count` 时以参数为准。
- 目标模式的出题队列仍然是"到期复习词 + `daily_new_limit` 剩余名额内的新词"，可练的词可能少于剩余目标。队列练完时提前结束，并提示"今天可练的词已练完，目标还差 N 个"，同时建议调大 `daily_new_limit` 或收藏更多单词。
- 目标只是提示，不会阻止退出、不弹窗、不影响老板键。`daily_goal = 0` 时不显示任何目标相关信息。

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

Wiktionary 不是翻译服务。英语定义作为主要提示，用户可以用 `add --note` 或在词库界面按 `n` 给收藏词添加个人中文释义。这个字段属于用户补充信息，不会伪装成词典返回的内容，也只能由用户自己填写：通过 MCP 收藏单词的 agent 不能写入它。

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

## 8. MCP 接入：让 agent 推荐单词（规划中）

### 目标

用户在 Cursor、Claude Desktop 等支持 [MCP（Model Context Protocol）](https://modelcontextprotocol.io) 的 agent 里读文档、写代码或聊天时，可以直接说"把这段里我可能不认识的词加到学习列表"。agent 通过 MCP 调用 StealthLingo：先了解词库和学习负担，再把推荐的单词连同推荐理由加入词库。之后这些词和手动收藏的词一样，进入常规的记忆 / 拼写 / 听音拼写和间隔复习。

StealthLingo 只做被调用的一方：选哪些词由 agent 决定，学习节奏仍由本地排程控制。

**释义只来自 Wiktionary。** agent 只能告诉 StealthLingo"收藏哪个词、为什么推荐"，不能提交释义、翻译、例句、音标或个人笔记。卡片、拼写题和词库中显示的释义、词性、例句、音标全部是 Wiktionary 返回并缓存的数据；Wiktionary 查不到的词不会被加入。

### 运行方式

```bash
stealthlingo mcp        # 以 stdio 传输运行 MCP 服务器，由 agent 客户端启动，不直接给人使用
```

客户端配置示例（Cursor 的 `~/.cursor/mcp.json` 或项目内 `.cursor/mcp.json`；Claude Desktop 等客户端格式相同）：

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

安装脚本已把 `stealthlingo` 放进 PATH；不在 PATH 时把 `command` 写成可执行文件的绝对路径。README 增加一节说明上述配置。

### 工具设计

第一版只提供 4 个工具，覆盖"了解现状 → 查证单词 → 加入词库"的完整流程：

| 工具 | 只读 | 作用 |
|---|---|---|
| `get_study_status` | 是 | 返回词库大小、到期复习数、尚未开始学习的新词数、用户设置的 `daily_goal` 与今天已练习的单词数、`daily_new_limit`、今日准确率、今天 agent 已收藏数和剩余额度 |
| `list_words` | 是 | 列出已收藏单词：词、状态、到期时间、是否有音频、`lapses`、Wiktionary 简短释义；支持 `query`、`status` 筛选和 `limit`（默认 100） |
| `lookup_word` | 是 | 查询单词（缓存优先，未缓存时联网），原样返回 Wiktionary 的词性、定义、例句、音标、是否有音频、来源 URL 和许可证；不会收藏 |
| `add_words` | 否 | 批量把推荐的单词加入学习列表，逐词返回结果 |

`add_words` 的输入：

```json
{
  "words": [
    { "word": "ephemeral", "reason": "出现在你正在读的文章第二段" },
    { "word": "idempotent", "reason": "代码评审里频繁出现" }
  ]
}
```

- `word`：必填，单个英语词或词组，最长 64 个字符，不允许换行和控制字符。实际收藏的词头以 Wiktionary 为准（沿用大小写回退和别名规则）。
- `reason`：可选，推荐理由，只描述在哪里遇到、为什么值得学，最长 200 个字符。单独存为 `added_reason`，只在词库详情里标注为"推荐理由（agent）"显示，**不出现在学习卡片和拼写题中**，也不当作释义使用。
- 没有 `note`、`definition` 之类的字段；输入中出现未知字段时整个调用以参数错误拒绝，避免 agent 误以为释义已被采纳。
- 每次调用最多 20 个词；超出时整个调用以参数错误拒绝，提示 agent 分批。

`add_words` 的逐词结果（`structuredContent`，同时附一段人类可读的文本摘要）：

| `status` | 含义 |
|---|---|
| `added` | 新加入学习列表 |
| `already_saved` | 已在词库中，不改动进度、笔记和原有的推荐理由 |
| `dismissed` | 用户以前主动移除过这个词，不重新加入；agent 不应再推荐 |
| `not_found` | Wiktionary 查无此词，未加入 |
| `network_error` | 未缓存且联网失败（无法连接、超时、服务端错误），未加入，可稍后重试 |
| `deferred` | 本次调用的联网时间预算用完，未处理，可重新调用 |
| `limit_reached` | 已达到今天的 `mcp_daily_add_limit`，未加入 |
| `invalid` | 输入不合法（空、过长、含控制字符） |

每个成功结果附带实际词头（如输入 `Ephemeral` 得到 `ephemeral`）、Wiktionary 的简短释义和 `has_audio`，方便 agent 向用户汇报"这个词没有发音，不会出现在听音拼写里"。agent 向用户转述释义时应以这里返回的 Wiktionary 内容为准。

### 典型调用流程

```text
用户：把这篇文章里我可能不认识的词加到学习列表
agent → get_study_status   词库 120 词，待开始的新词 15 个，每天引入 10 个，今日 agent 额度剩 30
agent → list_words         排除已收藏的词，参考 lapses 高的词判断用户水平
agent → add_words([...8 个词，各附 reason])
StealthLingo → 5 added · 1 already_saved · 1 dismissed · 1 not_found
agent → 告诉用户：已加入 5 个词，会按每天 10 个新词的节奏出现在学习中
```

服务器在 `initialize` 响应的 `instructions` 中给 agent 写明这些约定：先查看状态和词库再推荐；待开始的新词已经很多时少加或不加；返回 `dismissed` 的词是用户不想学的，不要再推荐；每个词尽量给出 `reason`，只写出处和推荐原因，不写释义或翻译；词义以 `lookup_word` / `add_words` 返回的 Wiktionary 内容为准。

### 协议实现

1. **传输：** stdio，每行一条 UTF-8 编码的 JSON-RPC 2.0 消息。stdout 只输出协议消息；诊断信息写 stderr。MCP 模式下不进入 TUI、不播放音频、不读取键盘。
2. **方法：** 实现 `initialize`、`notifications/initialized`、`ping`、`tools/list`、`tools/call`。未知方法返回 `-32601`，JSON 解析失败返回 `-32700`，参数错误返回 `-32602`；通知（无 `id`）不回复。stdin 关闭时正常退出。
3. **版本协商：** 支持 `2025-06-18` 和 `2025-03-26`。客户端请求的版本受支持时原样返回，否则返回支持的最新版本。`2025-06-18` 下工具声明 `outputSchema` 并返回 `structuredContent`；旧版本只返回文本内容。
4. **工具标注：** 只读工具标 `readOnlyHint: true`；`add_words` 标 `destructiveHint: false`、`idempotentHint: true`，便于客户端决定是否需要用户确认。
5. **错误分层：** 协议层错误用 JSON-RPC error；工具执行中的业务失败（如所有词都查不到、数据库写入失败）返回 `isError: true` 的工具结果，让 agent 能看到原因并调整；部分成功不算错误。
6. **不引入 SDK：** 官方 Rust SDK（`rmcp`）依赖 tokio 异步运行时，与第 9 节"不引入异步运行时"的原则冲突。这里需要的协议子集很小，用 `serde_json` 手写同步的请求循环即可。以后如需 HTTP 传输、resources 或 prompts 再重新评估。

### 与现有模块的关系

- 新增 `src/mcp/` 模块：`protocol.rs`（JSON-RPC 与 MCP 消息类型）、`server.rs`（读写循环、分发、版本协商）、`tools.rs`（工具定义、参数校验、结果组装）。
- 现有 `commands::add::run` 直接 `println!`，不能在 MCP 模式下复用。把"查缓存或联网 → 收藏"抽成返回结构化结果的函数（如 `commands::add::add_word(ctx, word, options) -> AddReport`），CLI、TUI 和 MCP 共用；CLI 负责把结果打印成现在的文字。
- `Database::add_to_collection` 增加参数：来源（`user` / `mcp`）和推荐理由。MCP 调用时笔记参数固定为空，不会写入或修改 `personal_note`；CLI `add --note` 保持现有行为。
- `list_words` 返回的 `WordSummary` 增加 `lapses`、`added_via`，`get_study_status` 复用 `Database::stats`、`count_due`、`count_new_ready`。
- 词典访问沿用 `cached_or_fetch`：已缓存的词不联网；未缓存的词逐个联网。整次 `add_words` 有 30 秒总预算，每个词的超时取 `http_timeout_secs` 与剩余预算中较小的值，保证整次调用不超过常见 MCP 客户端的工具调用超时（约 60 秒）；预算用完后剩余的词标为 `deferred`。
- 用户移除单词时（`remove` 命令、词库界面、链接移除）在 `dismissed_words` 中记一笔；用户自己再次 `add` 时清除这条记录。`add_words` 遇到有记录的词直接返回 `dismissed`，不联网、不恢复归档的学习进度。

### 安全与边界

- **不提供破坏性工具：** 不开放 `remove`、答题 / 记录练习、修改排程、`config set`、导入导出。agent 只能"添加"，添加的词可以在词库界面或用 `remove` 删除。
- **尊重用户的移除：** 用户移除过的词（无论是否由 agent 加入）不会被 agent 重新加入，`add_words` 返回 `dismissed`；只有用户自己 `add` 才能把它加回来。
- **不写入词条内容：** agent 不能提交或修改释义、例句、音标和个人笔记；词条内容只来自 Wiktionary，个人笔记只能由用户自己写。
- **防止失控循环：** 每次最多 20 个词，每天最多 `mcp_daily_add_limit` 个新收藏。额度按 `mcp_add_log` 中今天（本地日期）的记录数统计：每个 `added` 结果追加一条，之后即使用户删掉这个词也不会退回额度；`already_saved`、`dismissed` 等未加入的结果不计数。这个上限由用户在设置界面或 `config set` 中调整，设为 0 即关闭 agent 收藏。
- **不淹没学习：** agent 加入的词状态为 `new`，仍受用户设置的 `daily_new_limit` 控制进入学习的速度，不会一次性挤占复习任务。
- **来源可见：** 词库界面给 agent 加入的词显示 `agent` 标记，详情里显示推荐理由；`words` 命令输出加一列来源。
- **并发：** MCP 服务器和 TUI / CLI 是独立进程，共用同一个 SQLite 文件。依赖已有的 5 秒 `busy_timeout`，并考虑开启 WAL 减少读写互斥；TUI 回到首页或进入词库时重新读取数据，让 agent 刚加入的词及时出现。
- **只在本机：** 只支持 stdio，由本地客户端拉起，不监听任何网络端口。

### 测试

- `tests/mcp_tests.rs`：用内存中的输入输出驱动服务器循环，覆盖初始化与版本协商、`tools/list`、未知方法、参数错误、通知不回复。
- 用 fixtures 预先写入词典缓存，测试 `add_words` 的 `added` / `already_saved` / `dismissed` / `invalid` / `limit_reached`；验证用户 `remove`（包括链接归档）后 agent 再加入返回 `dismissed`、用户自己 `add` 后恢复正常；验证删掉 agent 今天加的词后额度不会退回；验证带 `note` 等未知字段的调用被拒绝，`personal_note` 不被改动，`reason` 不出现在学习卡片和拼写题中；查无此词和联网失败通过可注入的词典客户端模拟，测试不访问网络。
- 手动验收：在 Cursor 中配置服务器，让 agent 从一段英文中推荐单词并加入，确认词库界面出现带 `agent` 标记的词，且第二天按 `daily_new_limit` 进入学习。

## 9. Rust 技术栈

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

不引入 Web 框架、ORM、异步运行时或插件机制；`reqwest` 阻塞接口足以满足需求。MCP 服务器用 `serde_json` 手写，不新增依赖。发布使用 [dist](https://github.com/axodotdev/cargo-dist)。

## 10. 目录结构

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
│   ├── 004_archived_words.sql
│   └── 005_word_origin.sql       # 规划中：added_via、added_reason、dismissed_words、mcp_add_log
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
│   ├── mcp/                     # 规划中：MCP 服务器
│   │   ├── protocol.rs          # JSON-RPC 与 MCP 消息类型
│   │   ├── server.rs            # stdio 读写循环、分发、版本协商
│   │   └── tools.rs             # 工具定义、参数校验、结果组装
│   └── tui/                     # home / study / words / lookup / stats / settings（规划中）等界面
└── tests/
    ├── commands_tests.rs
    ├── dictionary_tests.rs
    ├── mcp_tests.rs             # 规划中
    ├── scheduling_tests.rs
    ├── storage_tests.rs
    ├── tui_tests.rs
    └── fixtures/                # Wiktionary 响应样本，以及旧版 Free Dictionary 样本
```

## 11. 本地数据模型

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
- 规划中（迁移 `005_word_origin.sql`）：`added_via`（`user` / `mcp`，默认 `user`）、`added_reason`（agent 给出的推荐理由，可为空）

### `archived_user_words`

通过链接点击“Remove”移除的单词会连同排程状态归档。因为其他程序也能触发这些链接，这类移除可以恢复：再次收藏时还原进度。`remove` 命令则让该词从头开始。迁移 005 同时给这张表加上 `added_via`、`added_reason`，恢复时一并还原。

### `dismissed_words`（规划中，迁移 005）

- `word_id`、`dismissed_at`
- 用户移除单词（`remove`、词库界面、链接移除）时写入，用户自己再次收藏时删除。MCP 的 `add_words` 遇到这里有记录的词返回 `dismissed`，不重新加入。

### `mcp_add_log`（规划中，迁移 005）

- `id`、`word_id`、`added_at`
- agent 每成功加入一个词追加一条，只增不删，用于统计 `mcp_daily_add_limit` 的当日用量，用户删词不会退回额度。

### `practice_attempts`

- `id`、`word_id`
- `mode`（`memory`、`spelling`、`listening_spelling`）
- `expected_answer`、`submitted_answer`
- `is_correct` 与 `grade`（`again`、`hard`、`good`、`easy`）分开存储
- `duration_ms`（可选）、`created_at`

## 12. 错误处理和离线行为

- 查词失败时显示具体原因（查无此词、无法连接、超时、服务端错误），不 panic。
- 学习、复习、`words`、`search`、`stats` 从不联网。
- `lookup` 联网刷新，连不上时显示缓存副本；`add` 和 `audio` 优先用缓存，只在遇到新词时联网。
- 发音数据未取全的条目照常可用，之后自动补全。
- 音频首次播放时下载并缓存；下载或播放失败时提示原因，不影响答题。
- 用户中途退出时，已经提交的答题保留；未提交答案不记为答错。第二次 Ctrl+C 立即退出，并恢复终端状态。
- 数据库写入失败时，不报告保存成功。
- 重复收藏同一个单词是幂等的；JSON 导入可重复执行，同一个词以最近复习过的进度为准。
- MCP 的 `add_words` 逐词报告结果：离线时已缓存的词照常加入，未缓存的词标为 `network_error`，不影响同批其他词。

## 13. 开发阶段

| 阶段 | 内容 | 状态 |
|---|---|---|
| 1. CLI 骨架 | `--help`、`--version`、默认命令、配置和数据目录 | 完成 |
| 2. 词典查询与缓存 | 词典接入、解析、缓存、各类错误处理 | 完成（已从 Free Dictionary 迁移到 Wiktionary） |
| 3. 收藏与记忆复习 | `add`、`words`、`review`、间隔排程 | 完成 |
| 4. 拼写与听音拼写 | 两种拼写模式、重播、错词记录、答题规则测试 | 完成 |
| 5. 统计与导入导出 | `stats`、JSON / CSV 导入导出、数据库迁移 | 完成 |
| 6. 全屏界面 | ratatui TUI、老板键、英美音轮换、小终端自适应布局 | 完成，布局仍在打磨 |
| 7. 发布 | dist 构建五个平台的二进制，生成 shell / PowerShell 安装脚本 | 完成 |
| 8. 日常使用增强 | P1 待做项：缺字母拼写、错词重练、混合模式、词库筛选、每日学习目标、全屏设置界面与 `config reset` | 下一步 |
| 9. MCP 接入 | 抽出结构化的收藏函数、迁移 005（来源、移除记录、agent 收藏日志）、`stealthlingo mcp` 与 4 个工具、词库界面显示来源、README 配置说明 | 规划中 |

## 14. v0.1 发布清单

- [x] 默认命令打开全屏应用，或在 `--plain` 下直接开始今日学习 / 给出清晰的下一步。
- [x] 可查询、收藏和搜索单词。
- [x] 可查看 Wiktionary 返回的定义、词性、音标、例句和可用音频。
- [x] 可执行记忆、拼写、听音拼写三种练习。
- [x] 学习进度保存在本地，复习不依赖每次联网。
- [x] 能查看今日学习量、正确率和待复习数。
- [x] 错误提示足够清晰，缺失字段不会导致程序崩溃。
- [x] README 说明安装、使用、数据目录、词典数据边界和许可注意事项。
- [x] 推送 `v*` 标签后，GitHub Actions 自动构建并发布 Release。

## 15. 明确不做的事情

- 不开发 GUI。
- 不开发账号、云端同步或服务端。MCP 服务器只通过 stdio 供本机 agent 调用，不监听网络端口。
- 不内置 AI、不调用模型 API；推荐单词由用户自己的 agent 完成。
- 不通过 MCP 开放删除单词、提交答题、修改排程或配置等操作。
- 不接受 agent 或 AI 提供的释义、翻译、例句或音标；词条内容只来自 Wiktionary。
- 不把 Wiktionary 当作中英翻译服务。
- 不假设所有单词都有音频、音标或例句。
- 不在近期实现句子级听写、AI 会话或大型游戏系统。
- 不为尚未验证的需求提前搭建复杂插件架构。

**目标：让用户在终端输入一条简短命令，就能用几分钟查词、记忆、拼写或听音拼写，随时一键隐藏，并让每次练习都积累为下一次更有效的复习。**
