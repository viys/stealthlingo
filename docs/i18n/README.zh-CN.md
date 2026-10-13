# StealthLingo

[English](../../README.md) | **简体中文** | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Español](README.es.md) | [Português (Brasil)](README.pt-BR.md) | [Русский](README.ru.md) | [Tiếng Việt](README.vi.md)

[![Release](https://img.shields.io/github/v/release/viys/stealthlingo)](https://github.com/viys/stealthlingo/releases) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](../../LICENSE) [![M8ven Score](https://m8ven.ai/badge/mcp/viys-stealthlingo-17ubze?v=b87b5893849d721e2b3074b76e72c0c2)](https://m8ven.ai/mcp/viys-stealthlingo-17ubze?s=readme)

摸鱼的时候，背几个英语单词。按 **Esc**，当前终端里会打开一个 shell，可以打命令；再按一次 Esc 回来。Esc 不会送进这个 shell。隐藏期间计时暂停。

这段空档是你本来就有的任何一会儿——一次编译、一场你并没真正在开的会，或者 AI 还没出结果。查一个词、收藏它，用闪卡、拼写或听力练几分钟。词典数据来自英文 [Wiktionary](https://en.wiktionary.org/)（无需 API 密钥）。进度保存在本地 SQLite，已收藏的词可离线练习。可选的 MCP 服务器让编程助手把注意到的词加进列表——StealthLingo 本身从不调用 AI 模型。

![首页 — 今日目标、待复习词和练习菜单](../images/home.png)

完整说明见 **[Wiki](https://github.com/viys/stealthlingo/wiki)**（英文：按键、全部命令、练习模式、MCP、备份、FAQ、开发）。v0.1 仅支持英语。

## 为什么是 StealthLingo

用来填一段随时能抽身的空档：练一小会儿，有人过来就 Esc，词表来自你在文档、代码或聊天里真正遇到的词。

| 你会得到 | 你不会遇到 |
|---|---|
| 全屏 TUI 或 `--plain` 逐行提示 | 账号、云同步或 GUI |
| Esc 隐藏界面并打开 shell；再按 Esc 回来。隐藏期间计时暂停 | 得退出程序才能把屏幕让出去 |
| 闪卡、拼写、缺字母、听力、混合、错词重练 | AI 生成的释义 |
| 每日单词目标 + 间隔复习（简化 SM-2） | 强制长课程 |
| Wiktionary 有录音时的 UK / US 音频 | 复习必须联网 |

## 安装

Windows / macOS / Linux 预编译包——无需 Rust：

```powershell
# Windows (PowerShell)
powershell -ExecutionPolicy Bypass -c "irm https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.ps1 | iex"
```

```bash
# macOS / Linux
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.sh | sh
```

或从源码安装（Rust 1.88+；Linux 构建需要 `libasound2-dev`）：

```bash
cargo install --git https://github.com/viys/stealthlingo
```

更新：

```bash
stealthlingo update            # 安装最新版本
stealthlingo update --check    # 只报告是否有新版本
```

`update` 会替换用上面安装脚本装的副本；用 `cargo install` 装的副本则打印需要运行的命令。StealthLingo 每天检查一次新版本（在首页和 `lookup` 之后），有新版本时在标题栏显示 `update X.Y.Z`；`stealthlingo config set check_updates off` 可以关闭。

## 快速开始

```bash
stealthlingo                           # 全屏应用；Esc 隐藏
stealthlingo study --minutes 3         # 刚好填上这点空档
stealthlingo lookup ephemeral          # 发音与主要释义（--all 显示全部）
stealthlingo add ephemeral --note "lasting a short time"
stealthlingo --plain                   # 改为逐行提示
```

首页：`r` 复习 · `a` 练习 · `/` 查词 · `w` 词库 · `t` 统计 · `c` 设置 · **Esc** 隐藏 · **Ctrl+C** 退出（`q` 只返回上一屏）。

在 Esc 打开的 shell 里，`lookup`、`add` 等命令照常可用。全屏界面已经开着，所以在那里再次启动它只会提示你按 Esc 返回。

练习模式（`a`，或直接按字母进入）：`s` 闪卡 · `p` 拼写 · `m` 缺字母 · `i` 听力 · `x` 混合 · `e` 错词重练。

## 命令

| 命令 | 作用 |
|---|---|
| `stealthlingo` | 打开全屏应用 |
| `lookup <WORD> [--all]` | 查询 Wiktionary（离线回退缓存） |
| `add` / `remove` | 收藏或移除单词 |
| `words` / `search` | 列出或搜索（`--status`、`--pos`、`--due`） |
| `study` / `review` | 练习（`--mode`、`--mistakes`、`--minutes`、`--count`） |
| `stats` | 进度与正确率 |
| `audio <WORD>` | 播放发音 |
| `config` | 查看或修改配置 |
| `export` / `import` | `.json` 备份或 `.csv` 词表 |
| `mcp` | 供 AI agent 使用的 MCP 服务器 |
| `links` | 可点击查词（Windows） |
| `update [--check]` | 安装最新版本（`--check` 只报告） |

```bash
stealthlingo study --mode spelling --count 10
stealthlingo study --mode listening --minutes 5
stealthlingo study --mistakes
stealthlingo review
```

## AI 助手（MCP）

你真正在干活的时候，助手可以把单词先存好，下次摸鱼就能直接练。在客户端 MCP 配置中指向 StealthLingo（例如 Cursor 的 `~/.cursor/mcp.json`）：

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

Agent 可以查词并加入列表，但不能删词、改设置或记答题。详见 [Wiki · AI Agents](https://github.com/viys/stealthlingo/wiki/AI-Agents-MCP)。

## 数据与备份

默认数据目录：Windows `%APPDATA%\stealthlingo\data`，macOS `~/Library/Application Support/stealthlingo`，Linux `~/.local/share/stealthlingo`。可用 `STEALTHLINGO_HOME` 覆盖。

```bash
stealthlingo export backup.json    # 完整备份
stealthlingo import backup.json    # 合并（以最近复习进度为准）
```

## 参与贡献

欢迎提 Issue 和 Pull Request。构建与测试见 [Wiki · Development](https://github.com/viys/stealthlingo/wiki/Development)。

## 许可证

MIT — 见 [LICENSE](../../LICENSE)。
