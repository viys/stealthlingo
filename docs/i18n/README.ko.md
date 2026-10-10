# stealthlingo

[English](../../README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md) | **한국어** | [Español](README.es.md) | [Português (Brasil)](README.pt-BR.md) | [Русский](README.ru.md) | [Tiếng Việt](README.vi.md)

터미널 중심으로, 티 나지 않게 공부하기 좋은 언어 학습 도구입니다.

StealthLingo는 자투리 시간에 어휘를 연습하기 위한 작은 Rust CLI입니다. 단어를 찾아 저장하고, 플래시카드·철자·듣고 받아쓰기 연습을 3~5분 단위로 할 수 있습니다. 사전 데이터는 영어 [Wiktionary(위키낱말사전)](https://en.wiktionary.org/)에서 Wikimedia 공식 API로 가져오며 API 키가 필요 없습니다. 복습 일정, 정답 판정, 학습 진도는 로컬 SQLite 데이터베이스에 저장되므로 저장한 단어는 오프라인에서도 복습할 수 있습니다.

v0.1에서는 영어 학습만 지원합니다.

## 설치

Windows(x64), macOS(Intel 및 Apple 실리콘), Linux(x64 및 ARM64)용 미리 빌드된 프로그램이 [Releases 페이지](https://github.com/viys/stealthlingo/releases)에 있으며, Rust가 필요 없습니다. 설치 스크립트는 `stealthlingo`와 링크 도우미 `stealthlingo-link`를 `~/.cargo/bin`에 넣고 그 폴더를 `PATH`에 추가합니다.

Windows(PowerShell):

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.ps1 | iex"
```

macOS 및 Linux:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.sh | sh
```

또는 Releases 페이지에서 사용 중인 시스템용 압축 파일을 내려받아 풀고, `stealthlingo`와 `stealthlingo-link`를 같은 폴더에 두세요. Linux에서는 오디오 재생에 ALSA(`libasound2`)를 사용하며, 데스크톱 배포판에는 기본으로 포함되어 있습니다.

### 소스에서 빌드

최신 안정 버전 Rust 툴체인(1.88 이상)과 C 컴파일러가 필요합니다(SQLite는 함께 포함됨).

```bash
cargo install --git https://github.com/viys/stealthlingo
# 또는 클론한 저장소에서
cargo install --path .
cargo run -- --help
```

Linux에서 빌드하려면 ALSA 개발 패키지(예: `libasound2-dev`)도 필요합니다.

## 빠른 시작

```bash
stealthlingo lookup ephemeral            # 발음과 주요 뜻 표시(--all이면 전부)
stealthlingo add ephemeral --note 덧없는  # 단어 저장, 개인 메모는 선택
stealthlingo                             # 전체 화면 앱 열기: 연습, 검색, 단어 목록, 통계
```

## 전체 화면 인터페이스

터미널에서 `stealthlingo`, `study`, `review`를 실행하면 전체 화면 인터페이스가 열립니다. 홈 화면에는 오늘 복습할 내용과 메뉴(`r` 복습, `s` 플래시카드, `p` 철자, `l` 듣기, `/` 단어 검색, `w` 단어 목록, `t` 통계, `q` 종료)가 표시됩니다. `↑`/`↓`(또는 `k`/`j`)와 `Enter`로도 조작할 수 있으며, 목록과 긴 페이지도 같은 키로 스크롤합니다.

- **Esc를 누르면 즉시 모든 것을 숨기고** 프로그램을 실행했던 셸을 보여 줍니다. Esc를 다시 누르면 돌아옵니다. 숨겨진 동안에는 세션 타이머가 멈춥니다.
- **플래시카드**는 Enter가 필요 없습니다. `Space`로 답 보기, `1`~`4`로 평가, `a`로 발음 재생, `s`로 건너뛰기, `q`로 세션 종료.
- **억양**: `a`는 먼저 기본 억양(`accent` 설정)을 재생합니다. 같은 단어에서 다시 누르면 녹음된 다른 억양을 재생하고, 이후 차례로 바뀝니다. 하단 바에는 다음에 재생될 억양이, 상단 바에는 재생 중인 억양이 표시됩니다.
- **철자와 듣기**: 단어를 입력하고 `Enter`. `Tab`은 정답 보기, `Ctrl+N`은 건너뛰기입니다. 듣기 연습에서는 빈 줄에서 `Enter`를 누르면 녹음을 다시 재생하고, `Ctrl+R`은 다른 억양을 재생합니다. 틀리면 어떤 글자가 더 들어갔거나 빠졌는지 보여 줍니다.
- **단어 목록**: `/` 필터, `Enter` 저장된 항목 열기(오프라인 가능), `a` 재생, `n` 메모 편집, `d` 다음 `y`로 단어 삭제(다시 저장하면 진도가 복원됨). `PgUp`/`PgDn`과 `g`/`G`로 긴 목록을 빠르게 이동합니다.
- **단어 검색**: 단어를 입력하고 `Enter`. 그다음 `s`로 저장 또는 삭제, `Tab`으로 간단히 보기/전체 보기 전환, `/`로 새 검색, `q`로 돌아갑니다.
- `Ctrl+C`는 진행 중인 세션을 끝내고, 필터나 메모 편집을 취소하며, 그 밖의 경우에는 프로그램을 종료합니다. 모든 답은 입력하는 즉시 저장됩니다.
- 인터페이스에는 최소 40×12 문자 크기의 터미널이 필요합니다. 창 높이가 낮으면 상자가 줄어들고 홈 요약이 한 줄로 합쳐지며 메뉴가 스크롤되어, 선택한 항목과 답 입력란이 항상 보입니다.

`--plain`을 쓰거나 입출력이 터미널이 아닐 때(예: 파이프)는 아래에서 설명하는 한 줄씩 묻는 프롬프트 방식을 사용합니다. `NO_COLOR`를 설정하면 색을 끕니다.

## 명령어

| 명령어 | 기능 |
|---|---|
| `stealthlingo` | 전체 화면 앱 열기. `--plain`이면: 복습할 단어를 복습하고, 없으면 새 단어를 학습하며, 단어 목록이 비어 있으면 시작 방법을 안내 |
| `lookup <WORD> [--all]` | Wiktionary 조회(캐시를 갱신하며, 오프라인이면 캐시된 사본을 표시) |
| `add <WORD> [--note TEXT]` | 단어 저장. 두 번 저장해도 문제없음. `--note`는 메모를 갱신 |
| `remove <WORD>` | 단어 목록에서 삭제(캐시된 사전 데이터와 답안 기록은 유지) |
| `words` | 저장한 단어를 상태 및 복습 예정 시각과 함께 표시 |
| `search <QUERY>` | 저장한 단어와 메모 검색 |
| `study [--mode memory\|spelling\|listening] [--minutes N] [--count N]` | 학습 세션 |
| `review [--count N] [--minutes N]` | 복습할 단어만 플래시카드로 복습 |
| `stats` | 상태별 저장 단어 수, 복습할 단어 수, 오늘과 전체 답안 수 및 정답률, 다음 복습 |
| `audio <WORD> [--accent uk\|us]` | 발음 재생(기본값: `accent` 설정) |
| `config` / `config set <KEY> <VALUE>` | 설정과 데이터 위치 보기 또는 변경 |
| `links [status\|install\|uninstall]` | `lookup`에서 Ctrl+클릭으로 발음 재생, 단어 추가·삭제(Windows) |
| `export <PATH>` / `import <PATH>` | `.json`은 전체 백업, `.csv`는 단어 목록 |

`--minutes`나 `--count`를 지정하지 않으면 세션은 `default_minutes`(5분) 동안 진행됩니다.

`lookup`은 발음을 한 줄에 영국식, 미국식 순서로 표시하고(`Pronunciation: UK /njuː/ · US /nu/`), `audio`가 재생할 녹음의 억양을 알려 줍니다(둘 다 있으면 `--accent uk|us`를 안내). 기본적으로 이어서 최대 세 개 품사의 처음 세 가지 뜻을 보여 줍니다. `lookup --all`(또는 `-a`)은 모든 뜻을 예문, 유의어, 반의어, 출처와 함께 보여 줍니다. 뜻은 터미널 너비에 맞게 줄바꿈되고 이어지는 줄은 들여쓰기됩니다.

### 클릭할 수 있는 검색 결과(Windows)

```bash
stealthlingo links install   # 설치 후 한 번만
```

현재 사용자에 대해 `stealthlingo://` 링크 처리기를 등록합니다(관리자 권한 불필요). 그러면 `lookup`은 녹음이 있는 각 발음을 터미널 하이퍼링크로 바꿉니다. Ctrl+클릭하면 브라우저나 창을 열지 않고 백그라운드에서 녹음을 재생합니다. 마지막 줄은 `+ Add to word list`가 되고, 이미 저장한 단어라면 `Saved in your word list · Remove`가 됩니다. 이 줄을 Ctrl+클릭하면 명령을 입력하지 않고 단어를 추가하거나 삭제할 수 있습니다(변경 사항을 보려면 `lookup`을 다시 실행). 다른 프로그램도 이 링크를 열 수 있으므로, 링크로 삭제한 단어는 학습 진도가 유지됩니다. 링크나 `add`로 다시 추가하면 진도가 복원됩니다. 반면 `remove` 명령은 단어를 처음부터 다시 시작하게 합니다. 하이퍼링크를 표시하는 터미널(Windows Terminal, VS Code / Cursor)에서 동작하며, 그 밖의 터미널에서는 `lookup`이 `add`와 `audio` 명령을 함께 적은 일반 텍스트를 출력합니다. `links status`는 감지 결과를 보여 주고, `FORCE_HYPERLINK=1`(또는 `0`)로 감지 결과를 덮어쓸 수 있으며, `links uninstall`은 처리기를 제거합니다. 오류는 데이터 디렉터리의 `link-errors.log`에 기록됩니다.

### 학습 모드

아래 키는 한 줄씩 묻는 프롬프트(`--plain`)용이며, 전체 화면 인터페이스에서는 위에서 설명한 단일 키를 사용합니다.

- **memory(기억)**: 단어를 보고 Enter를 눌러 뜻을 확인한 뒤 스스로 평가합니다: `1` Again(다시), `2` Hard(어려움), `3` Good(좋음), `4` Easy(쉬움). `a`를 누르면 발음을 들을 수 있습니다.
- **spelling(철자)**: 뜻을 읽고(뜻과 예문에서는 정답 단어가 가려짐) 단어를 입력합니다. `?`를 입력하면 포기하고 정답을 봅니다.
- **listening(듣기)**: 발음을 듣고 단어를 입력합니다. `r`은 다시 재생입니다. 발음 녹음이 있는 단어만 출제됩니다.

모든 모드에서 `s`는 답을 기록하지 않고 현재 단어를 건너뛰고, `q`는 세션을 끝냅니다. Ctrl+C로도 세션을 끝낼 수 있으며, 한 번 더 누르면 즉시 종료합니다. 제출한 답은 바로 저장되므로 중간에 그만둬도 끝낸 부분은 사라지지 않고, 답하지 않은 문제가 오답으로 기록되는 일도 없습니다.

정답 판정은 대소문자와 앞뒤 공백을 무시하지만, 아포스트로피와 하이픈은 구분합니다(`well-being`과 `wellbeing`은 다름).

### 복습 일정

단순화한 SM-2 알고리즘이 단어가 다시 나올 시점을 정합니다:

- **Again**: 10분 뒤 다시 출제되고 진도가 초기화됩니다.
- **Hard**: 보통보다 짧은 간격.
- **Good**: 1일, 그다음 3일, 이후에는 단어의 쉬움 계수(ease factor)만큼 간격이 늘어납니다.
- **Easy**: 더 긴 간격이 적용되고, 이후 그 단어는 더 쉬운 단어로 취급됩니다.

철자와 듣기에서는 맞히면 Good, 틀리면 Again으로 계산합니다. 틀린 단어는 같은 세션이 끝나기 전에 한 번 더 나옵니다. 복습할 단어는 항상 새 단어보다 먼저 나오고, 한 번도 학습하지 않은 단어는 하루 최대 `daily_new_limit`(기본 10)개까지 새로 추가됩니다. 간격이 21일 이상인 단어는 `mastered`(숙달)로 표시됩니다.

## 설정

```bash
stealthlingo config set daily_new_limit 15
stealthlingo config set default_minutes 3
stealthlingo config set http_timeout_secs 10
stealthlingo config set accent us        # 미국식 녹음을 먼저 재생(기본값: uk)
```

한 가지 억양으로만 녹음된 단어는 항상 그 녹음을 재생합니다.

### 사전

StealthLingo는 Wikimedia 공식 Wiktionary API를 사용합니다. 뜻과 예문은 REST API에서, IPA(억양별 표기), 유의어, 반의어, Wikimedia Commons 녹음(OGG)은 페이지 원문에서 가져옵니다. 입력한 형태와 사전의 표제어가 다르면(`Ephemeral`과 `ephemeral`) 입력한 형태를 기억하므로, `add`와 `remove`는 같은 저장 단어를 가리킵니다.

v0.1.0 출시 이전의 개발 빌드에서는 Free Dictionary API나 Merriam-Webster도 사용할 수 있었습니다. 그때 캐시된 단어는 여전히 오프라인에서 읽을 수 있고, 다음에 검색할 때 Wiktionary 데이터로 교체됩니다(학습 진도와 메모는 유지). `config.json`에 남은 예전 설정은 무시됩니다.

## 데이터 디렉터리

`stealthlingo config`가 정확한 경로를 출력합니다. 기본 위치:

| OS | 위치 |
|---|---|
| Windows | `%APPDATA%\stealthlingo\data` |
| macOS | `~/Library/Application Support/stealthlingo` |
| Linux | `~/.local/share/stealthlingo` |

`STEALTHLINGO_HOME`을 설정하면 다른 디렉터리를 사용할 수 있습니다(포터블 설치나 실험에 유용). 이 디렉터리에는 `stealthlingo.db`(SQLite, 버전별 마이그레이션 포함), `config.json`, `audio/`(내려받은 발음 파일)가 있습니다.

### 백업과 단어 목록

```bash
stealthlingo export backup.json   # 전부: 캐시, 진도, 답안 기록
stealthlingo import backup.json   # 병합. 같은 파일을 다시 가져와도 바뀌는 것 없음
stealthlingo export words.csv     # word, note, status, due_at, definition
stealthlingo import examples/words.csv   # "word" 열(없으면 첫 번째 열), "note" 열은 선택
```

JSON 가져오기에서 같은 단어가 양쪽에 있으면 가장 최근에 복습한 진도를 따릅니다. CSV 가져오기는 아직 캐시되지 않은 단어를 하나씩 조회하므로 인터넷 연결이 필요합니다.

## 오프라인 동작

- 학습, 복습, `words`, `search`, `stats`는 네트워크를 전혀 사용하지 않습니다.
- `lookup`은 Wiktionary에서 새로 가져오고, 연결할 수 없으면 캐시된 사본을 보여 줍니다.
- `add`와 `audio`는 가능하면 캐시를 쓰고, 새 단어일 때만 온라인에 접속합니다.
- 발음 오디오는 처음 재생할 때 내려받고 그 뒤로는 재사용합니다.

## 사전 데이터에 관하여

- 항목마다 출처 URL과 라이선스가 있으며 `lookup --all`로 볼 수 있습니다. Wiktionary 콘텐츠는 CC BY-SA 4.0 라이선스를 따릅니다.
- 단어마다 수록 정도가 다릅니다. 발음 기호, 오디오, 예문, 유의어, 반의어 모두 없을 수 있습니다. StealthLingo는 모든 항목을 선택 사항으로 처리하며, 오디오가 없는 단어는 듣기 연습에서 건너뜁니다.
- Wiktionary는 번역 서비스가 아닙니다. 한국어(또는 다른 언어) 힌트는 사전이 아니라 직접 작성한 `--note`에서 나옵니다.
- 이 서비스는 무료이며 느리거나 중단될 수 있습니다. Wikimedia 이용 약관을 지키고, 라이선스를 따르지 않은 채 캐시된 데이터를 대량으로 재배포하지 마세요.

## 개발

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

코드는 `dictionary`(Wiktionary 클라이언트와 `Entry` 모델로 변환하는 파서, 그리고 출시 전 개발 빌드가 캐시한 데이터를 읽는 디코더), `storage`(SQLite), `learning`(복습 일정, 정답 판정, 세션), `audio`, `commands`, `tui`(ratatui로 만든 전체 화면 인터페이스)로 나뉩니다. 두 인터페이스는 같은 `learning::session::Session`을 사용합니다. 복습 일정은 순수 함수이며 `tests/scheduling_tests.rs`에 전용 테스트가 있습니다.

### 릴리스

릴리스는 [dist](https://github.com/axodotdev/cargo-dist)로 빌드합니다(`dist-workspace.toml`, `.github/workflows/release.yml`). `Cargo.toml`의 `version`을 올려 커밋한 뒤, 같은 버전의 태그를 푸시합니다:

```bash
git tag v0.2.0
git push origin v0.2.0
```

그러면 GitHub Actions가 모든 플랫폼용으로 빌드하고 압축 파일과 설치 스크립트를 GitHub Release로 게시합니다. `dist plan`으로 빌드될 내용을 미리 볼 수 있고, `dist-workspace.toml`을 바꾼 뒤에는 `dist generate`로 워크플로를 갱신하세요.

## 라이선스

MIT, [LICENSE](../../LICENSE)를 참고하세요.
