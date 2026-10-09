# StealthLingo — CLI 摸鱼式外语学习工具规划

> **产品定位：** 一个使用 Rust 开发、以 Free Dictionary API 作为主要词典数据来源的 CLI 外语学习工具。它不是 GUI 应用，也不是单纯的终端词典，而是让用户在碎片时间里快速查词、记忆、拼写和听音拼写的低干扰学习工具。
>
> **核心理念：** 打开终端，几秒进入练习；每轮只花 3–5 分钟；本地保留学习进度；不用网络也能复习已经缓存的词。

## 1. 产品原则

1. **CLI 优先。** 所有核心功能通过命令行完成。第一版不开发 GUI，也不以全屏 TUI 为前提。
2. **碎片化学习。** 支持短时长会话和随时退出，不强制完成长课程。
3. **键盘驱动。** 尽量减少菜单和重复输入，让学习流程短而顺手。
4. **API 提供词典数据，软件提供学习能力。** Free Dictionary API 用于查询词义、词性、音标、发音和可用例句；复习计划、答题、学习记录由 StealthLingo 自己实现。
5. **离线优先。** 查询过并加入词库的单词保存在本地，网络不可用时仍可复习已缓存内容。
6. **如实处理缺失数据。** API 的音频、音标、例句等字段都可能缺失；缺少音频时应跳过听力题或切换题型。
7. **先做好核心闭环。** 不在第一版加入账号、云同步、AI 对话、复杂插件系统或完整课程体系。

## 2. 核心使用体验

### 快速开始

```bash
# 直接进入默认的今日练习
stealthlingo

# 进行 3 分钟的快速学习
stealthlingo study --minutes 3

# 只练习拼写
stealthlingo study --mode spelling --minutes 5

# 只练习听音拼写
stealthlingo study --mode listening --minutes 3
```

默认命令应直接提供有用的下一步：如果有到期复习词条，优先开始复习；如果尚未建立词库，提示用户查询并收藏第一个单词。不要每次启动都先展示复杂菜单。

### 示例学习过程

```text
StealthLingo · Quick Study
Session: 3 minutes
Due: 12 words

Word: ephemeral

[Enter] Reveal meaning   [Q] Quit
```

揭晓后：

```text
Meaning: lasting for a very short time
Part of speech: adjective
Example: The beauty of youth is ephemeral.

[1] Again  [2] Hard  [3] Good  [4] Easy
```

听音拼写模式：

```text
Listening spelling · 4 / 10

Playing pronunciation...
[R] Replay audio  [Enter] Submit answer  [Q] Quit
Your spelling: _
```

以上只是交互草图。初版应优先使用稳定、清晰的普通终端交互；只有在键盘操作和音频体验确实需要时，再引入全屏 TUI。

## 3. 功能规划

### P0 — v0.1 可用 MVP

- [x] `lookup <word>`：查询 Free Dictionary API 并展示词典信息。
- [x] 展示可用的音标、词性、定义、例句、同义词和反义词。
- [x] 播放查询结果中存在的发音音频。
- [x] `add <word>`：把单词加入本地学习词库。
- [x] `words`：查看、搜索和移除已收藏的词。
- [x] `study`：以卡片形式进行单词记忆。
- [x] `study --mode spelling`：根据释义或定义输入单词拼写。
- [x] `study --mode listening`：播放单词音频，隐藏词形，要求用户输入拼写。
- [x] `review`：复习已到期的词条。
- [x] 本地保存单词数据、答题记录和下一次复习时间。
- [x] `stats`：展示收藏数、今日练习量、准确率、到期复习数。
- [x] 支持 Ctrl+C / `q` 等退出方式，并尽可能保留已完成的答题记录。
- [x] 处理无网络、单词不存在、API 返回异常、无发音音频、字段缺失等情况。

### P1 — 让日常使用更顺手

- [ ] 缺字母拼写，例如 `e_ph_m_r_l`。
- [ ] 错词重练模式，优先复习最近写错的词。
- [x] `study --count 10`：按题量开始练习。
- [ ] `study --mode mixed`：混合记忆、拼写和听音拼写。
- [ ] 按词性、熟悉程度和到期状态筛选词库。
- [x] CSV / JSON 导出和导入个人词库及学习进度。
- [ ] 单词详情刷新：用户主动请求后更新缓存的词典数据。
- [ ] 清晰展示 API 中的多个定义和多个发音变体。

### P2 — 后续探索

- [ ] 句子听写：需要另外的句子音频或 TTS 来源，不能假设 Free Dictionary API 为每条例句提供音频。
- [ ] 自定义词组和专题练习。
- [ ] 更多语言：为每种语言确认数据来源与字段覆盖率后再支持。
- [x] 可选全屏 TUI，但不影响核心 CLI 命令（`--plain` 或非终端时回退到逐行交互）。
- [ ] 可选 AI 辅助释义或生成例句；不是首版依赖。

## 4. CLI 命令设计

```text
stealthlingo [OPTIONS] [COMMAND]

Commands:
  lookup <WORD>       查询外部词典
  add <WORD>          收藏单词
  remove <WORD>       从个人词库移除单词
  words               列出已收藏单词
  search <QUERY>      搜索本地词库
  study               开始学习会话
  review              复习到期词条
  stats               查看学习统计
  audio <WORD>        播放单词发音
  config              查看或修改配置
  export <PATH>       导出词库与学习记录
  import <PATH>       导入词库
```

建议参数：

```bash
stealthlingo lookup ephemeral
stealthlingo add ephemeral
stealthlingo words
stealthlingo search ephem
stealthlingo study --minutes 3
stealthlingo study --mode spelling --count 10
stealthlingo study --mode listening --count 10
stealthlingo review
stealthlingo stats
```

命令名称在实施时可以微调，但应避免同一件事存在太多同义命令。默认入口要简短，管理操作才使用更明确的子命令。

## 5. Free Dictionary API 接入

### Endpoint

```text
GET https://api.dictionaryapi.dev/api/v2/entries/en/{word}
```

首版先聚焦英语。API 通常可提供词头、音标信息、发音音频 URL、词性、定义、部分例句、同义词和反义词等内容，但不同单词的字段覆盖不一致。

### 接入原则

1. 在 `dictionary` 模块中封装 API，不让 CLI 命令直接依赖远端 JSON 结构。
2. 用 `serde` 解析响应，把可用字段转换成内部数据模型。
3. 可选字段全部按可缺失处理，不因为没有音频或例句而让查询失败。
4. 设置 HTTP 超时；将“查无此词”、网络失败、服务端异常分别提示。
5. 查词或显式刷新时访问 API；常规复习从 SQLite 读取，不重复请求远端。
6. 将协议相对音频地址（例如 `//...`）规范化为 HTTPS。
7. 音频播放封装为单独模块。若没有可播放音频，听音拼写模式应跳过该词或改用其他题型。
8. 不假设 API 能提供中文翻译、学习进度、等级体系、可靠的词频排名或句子音频。
9. 对外发布前查看 API 当前的数据使用、署名和缓存相关要求；“免费查询”不等于可无限制重新分发所有内容。

### 中文提示的处理

Free Dictionary API 的重点是英语词典数据，不应把它当作稳定的中英翻译 API。第一版可以采用英语定义作为主要提示，并允许用户可选地给收藏词添加个人中文释义。这个字段属于用户补充信息，不应伪装成 API 返回内容。

## 6. 学习模式设计

### 6.1 记忆模式

1. 显示单词，可选择播放发音。
2. 用户尝试回忆含义。
3. 展示词性、定义、可用例句及音标。
4. 用户选择 `Again / Hard / Good / Easy`。
5. 保存答题记录并计算下次复习时间。

### 6.2 拼写模式

首版支持“定义 → 拼写”。用户看到英文定义，输入对应词条。之后可加入个人中文释义提示或缺字母练习。

答题判断规则：

- 去掉输入首尾空白。
- 英语单词默认忽略大小写差异。
- 不要随意删除撇号、连字符等可能影响拼写的标点。
- 答错后显示标准答案，并记录错误。
- 将“本题是否正确”和“用户对长期记忆的主观评价”分开存储。

### 6.3 听音拼写模式

1. 选取存在可用音频的已收藏单词。
2. 不显示单词拼写，只播放发音。
3. 用户可按键重播并提交拼写。
4. 显示对错、正确拼写、定义和音标。
5. 记录用时与结果，安排后续复习。

注意：单词发音和句子听写是两件不同的事。第一版专注于单词级听音拼写，避免引入并不存在的句子音频。

## 7. 复习计划

采用简单、可测试的间隔重复算法。第一版可实现基础 SM-2 风格的排程或更简单的规则，关键是行为明确且可测试，而不是尽早引入复杂算法。

- `Again`：标记为不熟悉，较快安排重练。
- `Hard`：缩短正常复习间隔。
- `Good`：按正常幅度增加间隔。
- `Easy`：更大幅度增加间隔。

排程函数应独立于 API、CLI 和 SQLite。测试边界至少包括新词第一次学习、答错、连续答对、到期时间计算和时间边界。

会话选题建议：

1. 先安排到期复习词。
2. 新词数量应有限制，避免新词淹没复习任务。
3. 最近答错的词可以更早重现，但应避免无限重复同一道题。
4. 听力模式只选择有可用音频的词条。

## 8. Rust 技术栈

建议从小而清晰的依赖集合开始：

| Crate | 职责 |
|---|---|
| `clap` | 子命令、参数解析、自动帮助信息 |
| `reqwest` | 调用 Free Dictionary API |
| `serde` / `serde_json` | JSON 序列化和反序列化 |
| `rusqlite` | SQLite 本地存储，初版可评估 `bundled` feature |
| `directories` | 跨平台应用数据目录 |
| `anyhow` | 应用层错误上下文 |
| `chrono` | 复习时间与学习日期 |
| `rodio` 或经验证的跨平台音频方案 | 播放发音音频；先做最小可行性验证 |
| `csv` | 后续导入导出 |

首版不必引入 Web 框架、ORM、异步任务系统或插件机制。若采用 `reqwest` 的阻塞接口即可满足首版需求，除非后续出现明确的并发需求。

## 9. 建议目录结构

```text
stealthlingo/
├── Cargo.toml
├── README.md
├── LICENSE
├── migrations/
│   └── 001_initial.sql
├── src/
│   ├── main.rs
│   ├── cli.rs
│   ├── config.rs
│   ├── error.rs
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── lookup.rs
│   │   ├── add.rs
│   │   ├── words.rs
│   │   ├── study.rs
│   │   ├── review.rs
│   │   └── stats.rs
│   ├── dictionary/
│   │   ├── mod.rs
│   │   ├── client.rs
│   │   └── models.rs
│   ├── learning/
│   │   ├── mod.rs
│   │   ├── spelling.rs
│   │   ├── session.rs
│   │   └── scheduling.rs
│   ├── audio/
│   │   └── mod.rs
│   └── storage/
│       ├── mod.rs
│       ├── database.rs
│       └── repository.rs
└── tests/
    ├── dictionary_tests.rs
    ├── scheduling_tests.rs
    └── storage_tests.rs
```

## 10. 本地数据模型

保持模型简单，至少保存以下数据：

### `words`

- `id`
- `language_code`（首版 `en`）
- `headword_normalized`（规范化后的词头）
- `display_word`
- `raw_response_json` 或规范化缓存
- `source_fetched_at`
- `created_at`

### `user_words`

- `word_id`
- `status`（`new`、`learning`、`review`、`mastered`）
- `personal_note`（可选的个人中文释义或记忆提示）
- `due_at`
- `interval_days`
- `repetitions`
- 排程所需的其他参数
- `added_at`、`last_reviewed_at`

### `practice_attempts`

- `id`
- `word_id`
- `mode`（`memory`、`spelling`、`listening_spelling`）
- `expected_answer`
- `submitted_answer`
- `is_correct`
- `duration_ms`（可选）
- `created_at`

词典响应可先以原始 JSON 缓存，再抽取 CLI 和学习逻辑常用的字段。数据库应使用版本化迁移，避免后续更新破坏用户的学习进度。

## 11. 错误处理和离线行为

- API 查词失败时，显示具体原因，不要直接 panic。
- 已缓存的词条可在无网络时继续学习。
- 未缓存的单词需要网络查询时，给出明确提示。
- 音频 URL 失效时提供重试或跳过选项。
- 用户中途退出时，已经提交的答题应保留；未提交答案不要误记成答错。
- 数据库写入失败时，不要报告保存成功。
- 对重复收藏同一个单词采取幂等处理，不要创建重复记录。

## 12. 开发计划和验收标准

### 阶段 1：CLI 骨架

- 初始化 Cargo 项目。
- 实现 `--help`、`--version`、默认命令和配置目录。
- 验收：在目标操作系统上可以正常运行，帮助信息清晰。

### 阶段 2：词典查询与缓存

- 接入 Free Dictionary API。
- 解析定义、音标、例句和可选音频。
- 保存查询缓存。
- 验收：正常单词、查无此词、网络错误、字段缺失都能正常处理。

### 阶段 3：收藏与记忆复习

- 实现 `add`、`words`、`review` 和基础间隔排程。
- 验收：退出重启后词库与复习进度仍然存在。

### 阶段 4：拼写与听音拼写

- 实现定义到拼写、音频到拼写、重播音频、提交答案和错词记录。
- 验收：音频缺失时能跳过；答题判断规则有自动化测试。

### 阶段 5：统计与发布

- 实现 `stats`、导入导出、数据库迁移和跨平台构建说明。
- 验收：可以查看基本学习统计，且有可靠备份方式。

## 13. v0.1 发布清单

- [x] 默认命令可直接开始今日学习，或给出清晰的下一步。
- [x] 可查询、收藏和搜索单词。
- [x] 可查看 API 返回的定义、词性、音标、例句和可用音频。
- [x] 可执行记忆、拼写、听音拼写三种练习。
- [x] 学习进度保存在本地，复习不依赖每次联网。
- [x] 能查看今日学习量、正确率和待复习数。
- [x] 错误提示足够清晰，缺失字段不会导致程序崩溃。
- [x] README 说明安装、使用、数据目录、API 数据边界和许可注意事项。

## 14. 首版明确不做的事情

- 不开发 GUI。
- 不把全屏 TUI 作为首版前提。
- 不开发账号、云端同步或服务端。
- 不把 Free Dictionary API 当作中英翻译服务。
- 不假设所有单词都有音频或例句。
- 不在首版实现句子级听写、AI 会话或大型游戏系统。
- 不为尚未验证的需求提前搭建复杂插件架构。

**首版目标：让用户在终端输入一条简短命令，就能用几分钟查词、记忆、拼写或听音拼写，并让每次练习都积累为下一次更有效的复习。**
