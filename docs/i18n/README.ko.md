# StealthLingo

[English](../../README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md) | **한국어** | [Español](README.es.md) | [Português (Brasil)](README.pt-BR.md) | [Русский](README.ru.md) | [Tiếng Việt](README.vi.md)

[![Release](https://img.shields.io/github/v/release/viys/stealthlingo)](https://github.com/viys/stealthlingo/releases) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](../../LICENSE) [![M8ven Score](https://m8ven.ai/badge/mcp/viys-stealthlingo-17ubze?v=b87b5893849d721e2b3074b76e72c0c2)](https://m8ven.ai/mcp/viys-stealthlingo-17ubze?s=readme)

농땡이 치는 동안 영어 단어를 조금 익히세요. **Esc**를 누르면 이 터미널에서 셸이 열리고 명령을 칠 수 있습니다. 한 번 더 누르면 돌아옵니다. Esc는 그 셸로 전달되지 않습니다. 숨겨 둔 동안 타이머는 멈춥니다.

그 틈은 원래 있던 무엇이든 됩니다. 빌드, 제대로 참석하지 않은 회의, AI 답변을 기다리는 시간. 단어를 찾아 저장하고, 플래시카드, 철자, 듣기로 몇 분만 연습하세요. 사전 데이터는 영어 [Wiktionary](https://en.wiktionary.org/)에서 가져옵니다(API 키 불필요). 진행 상황은 로컬 SQLite에 있으므로 저장한 단어는 오프라인에서도 연습할 수 있습니다. 선택적 MCP 서버로 코딩 에이전트가 눈에 띈 단어를 목록에 넣을 수 있습니다. StealthLingo 자체는 AI 모델을 호출하지 않습니다.

![홈 — 오늘의 목표, 복습 예정, 연습 메뉴](../images/home.png)

전체 가이드는 **[Wiki](https://github.com/viys/stealthlingo/wiki)**(영어: 키, 모든 명령, 학습 모드, MCP, 백업, FAQ, 개발). v0.1은 영어만 지원합니다.

## 왜 StealthLingo인가

바로 빠져나올 수 있는 틈을 위한 앱입니다. 짧게 연습하고, 누가 오면 Esc. 단어 목록은 문서, 코드, 채팅에서 실제로 만난 단어로 자랍니다.

| 얻는 것 | 없는 것 |
|---|---|
| 전체 화면 TUI 또는 `--plain` 프롬프트 | 계정, 클라우드 동기화, GUI |
| Esc로 숨기고 셸이 열림. Esc로 복귀. 숨겨 둔 동안 타이머는 멈춤 | 화면을 치우려고 앱을 종료하는 일 |
| 플래시카드, 철자, 빈칸, 듣기, 혼합, 오답 재연습 | AI가 쓴 정의 |
| 일일 단어 목표 + 간격 반복(단순 SM-2) | 강제 긴 코스 |
| 녹음이 있을 때 UK / US 오디오 | 복습에 필수인 네트워크 |

## 설치

Windows / macOS / Linux 미리 빌드된 바이너리 — Rust 불필요:

```powershell
# Windows (PowerShell)
powershell -ExecutionPolicy Bypass -c "irm https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.ps1 | iex"
```

```bash
# macOS / Linux
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.sh | sh
```

또는 소스에서(Rust 1.88+; Linux 빌드에는 `libasound2-dev` 필요):

```bash
cargo install --git https://github.com/viys/stealthlingo
```

업데이트:

```bash
stealthlingo update            # 최신 릴리스 설치
stealthlingo update --check    # 새 릴리스가 있는지만 알림
```

`update`는 위 설치 스크립트로 설치한 사본을 교체합니다. `cargo install`로 설치한 사본이면 대신 실행할 명령을 출력합니다. StealthLingo는 하루에 한 번(홈 화면과 `lookup` 이후) 새 릴리스를 확인하고, 있으면 헤더에 `update X.Y.Z`를 표시합니다. `stealthlingo config set check_updates off`로 끌 수 있습니다.

## 빠른 시작

```bash
stealthlingo                           # 전체 화면; Esc로 숨김
stealthlingo study --minutes 3         # 그 틈에 맞는 연습
stealthlingo lookup ephemeral          # 발음과 주요 정의(--all로 전체)
stealthlingo add ephemeral --note "lasting a short time"
stealthlingo --plain                   # 줄 단위 프롬프트
```

홈: `r` 복습 · `a` 연습 · `/` 검색 · `w` 단어 목록 · `t` 통계 · `c` 설정 · **Esc** 숨기기 · **Ctrl+C** 종료(`q`는 뒤로만).

Esc로 열린 셸에서도 `lookup`, `add` 같은 명령은 평소처럼 쓸 수 있습니다. 전체 화면 앱은 이미 열려 있으므로, 그 셸에서 다시 실행하면 Esc를 눌러 돌아가라는 안내만 표시됩니다.

연습 모드(`a`, 또는 글자로 바로): `s` 플래시카드 · `p` 철자 · `m` 빈칸 · `i` 듣기 · `x` 혼합 · `e` 오답 재연습.

## 명령

| 명령 | 역할 |
|---|---|
| `stealthlingo` | 전체 화면 앱 열기 |
| `lookup <WORD> [--all]` | Wiktionary 조회(오프라인 시 캐시) |
| `add` / `remove` | 단어 저장 / 제거 |
| `words` / `search` | 목록 또는 검색(`--status`, `--pos`, `--due`) |
| `study` / `review` | 연습(`--mode`, `--mistakes`, `--minutes`, `--count`) |
| `stats` | 진행과 정답률 |
| `audio <WORD>` | 발음 재생 |
| `config` | 설정 보기/변경 |
| `export` / `import` | `.json` 백업 또는 `.csv` 단어 목록 |
| `mcp` | AI 에이전트용 MCP 서버 |
| `links` | 클릭 가능한 조회(Windows) |
| `update [--check]` | 최신 릴리스 설치(`--check`는 알림만) |

```bash
stealthlingo study --mode spelling --count 10
stealthlingo study --mode listening --minutes 5
stealthlingo study --mistakes
stealthlingo review
```

## AI 에이전트(MCP)

실제로 일하는 동안 에이전트가 단어를 저장해 둘 수 있습니다. 다음에 농땡이 칠 때 바로 연습합니다. 클라이언트 MCP 설정에서 StealthLingo를 가리키세요(예: Cursor `~/.cursor/mcp.json`):

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

에이전트는 단어를 조회하고 목록에 추가할 수 있습니다. 삭제, 설정 변경, 답안 기록은 할 수 없습니다. 자세한 내용: [Wiki · AI Agents](https://github.com/viys/stealthlingo/wiki/AI-Agents-MCP).

## 데이터와 백업

기본 데이터 디렉터리: Windows `%APPDATA%\stealthlingo\data`, macOS `~/Library/Application Support/stealthlingo`, Linux `~/.local/share/stealthlingo`. `STEALTHLINGO_HOME`으로 덮어쓸 수 있습니다.

```bash
stealthlingo export backup.json    # 전체 백업
stealthlingo import backup.json    # 병합(가장 최근 진행 우선)
```

## 기여

Issue와 Pull Request를 환영합니다. 빌드와 테스트: [Wiki · Development](https://github.com/viys/stealthlingo/wiki/Development).

## 라이선스

MIT — [LICENSE](../../LICENSE) 참조.
