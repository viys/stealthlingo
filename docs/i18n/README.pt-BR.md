# StealthLingo

[English](../../README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Español](README.es.md) | **Português (Brasil)** | [Русский](README.ru.md) | [Tiếng Việt](README.vi.md)

[![Release](https://img.shields.io/github/v/release/viys/stealthlingo)](https://github.com/viys/stealthlingo/releases) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](../../LICENSE)

Pratique algumas palavras em inglês enquanto você enrola. Pressione **Esc** e um shell abre neste terminal; pressione Esc de novo e você retorna. Esc não é digitado nesse shell. O timer pausa enquanto está oculta.

Essa pausa é qualquer uma que você já tem: um build, uma reunião na qual você não está de verdade, a espera de uma resposta da IA. Procure uma palavra, salve e pratique com flashcards, soletração ou escuta. Os dados do dicionário vêm do [Wiktionary](https://en.wiktionary.org/) em inglês (sem chave de API). O progresso fica em SQLite local, então palavras salvas funcionam offline. Um servidor MCP opcional deixa um agente de código adicionar as palavras que notar; o StealthLingo nunca chama um modelo de IA.

![Início — meta do dia, palavras pendentes e o menu](../images/home.png)

Guias completos na **[Wiki](https://github.com/viys/stealthlingo/wiki)** (teclas, comandos, modos, MCP, backups, FAQ, desenvolvimento). Apenas inglês no v0.1.

## Por que StealthLingo

Uma pausa da qual você sai na hora: uma sessão curta, Esc se alguém aparecer, e uma lista que cresce com palavras que você realmente encontra — em docs, código ou chat.

| Você tem | Você não tem |
|---|---|
| TUI em tela cheia ou prompts `--plain` | Contas, sync na nuvem ou GUI |
| Esc oculta a tela e abre um shell; Esc volta. O timer pausa enquanto está oculta | Ter que sair do app para tirá-lo da vista |
| Flashcards, soletração, letras faltando, escuta, misto, refazer erros | Definições escritas por IA |
| Meta diária + repetição espaçada (SM-2 simplificado) | Cursos longos obrigatórios |
| Áudio UK / US quando o Wiktionary tem | Rede obrigatória para revisar |

## Instalação

Binários para Windows, macOS e Linux — sem precisar de Rust:

```powershell
# Windows (PowerShell)
powershell -ExecutionPolicy Bypass -c "irm https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.ps1 | iex"
```

```bash
# macOS / Linux
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.sh | sh
```

Ou a partir do código-fonte (Rust 1.88+; no Linux é preciso `libasound2-dev` para compilar):

```bash
cargo install --git https://github.com/viys/stealthlingo
```

## Início rápido

```bash
stealthlingo                           # app em tela cheia; Esc oculta
stealthlingo study --minutes 3         # uma sessão que cabe na pausa
stealthlingo lookup ephemeral          # pronúncias + definições principais (--all para tudo)
stealthlingo add ephemeral --note "lasting a short time"
stealthlingo --plain                   # prompts linha a linha
```

Na tela inicial: `r` revisar · `a` praticar · `/` buscar · `w` lista · `t` estatísticas · `c` configurações · **Esc** ocultar · **Ctrl+C** sair (`q` só volta).

No shell que o Esc abre, comandos como `lookup` e `add` funcionam normalmente. O app em tela cheia já está aberto, então iniciá-lo de novo ali só lembra você de apertar Esc.

Modos de prática (`a`, ou a letra direto): `s` flashcards · `p` soletração · `m` letras faltando · `i` escuta · `x` misto · `e` refazer erros.

## Comandos

| Comando | O que faz |
|---|---|
| `stealthlingo` | Abrir o app em tela cheia |
| `lookup <WORD> [--all]` | Consultar o Wiktionary (cache offline) |
| `add` / `remove` | Salvar ou remover uma palavra |
| `words` / `search` | Listar ou buscar (`--status`, `--pos`, `--due`) |
| `study` / `review` | Praticar (`--mode`, `--mistakes`, `--minutes`, `--count`) |
| `stats` | Progresso e precisão |
| `audio <WORD>` | Tocar a pronúncia |
| `config` | Ver ou alterar configurações |
| `export` / `import` | Backup `.json` ou lista `.csv` |
| `mcp` | Servidor MCP para agentes de IA |
| `links` | Lookups clicáveis (Windows) |

```bash
stealthlingo study --mode spelling --count 10
stealthlingo study --mode listening --minutes 5
stealthlingo study --mistakes
stealthlingo review
```

## Agentes de IA (MCP)

Enquanto você trabalha de verdade, um agente pode guardar palavras para a próxima vez que você enrolar. Aponte seu cliente para o StealthLingo (ex.: Cursor `~/.cursor/mcp.json`):

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

Agentes podem buscar palavras e adicioná-las à sua lista. Não podem remover palavras, mudar configurações nem registrar respostas. Detalhes: [Wiki · AI Agents](https://github.com/viys/stealthlingo/wiki/AI-Agents-MCP).

## Dados e backups

Diretório de dados padrão: `%APPDATA%\stealthlingo\data` (Windows), `~/Library/Application Support/stealthlingo` (macOS), `~/.local/share/stealthlingo` (Linux). Substitua com `STEALTHLINGO_HOME`.

```bash
stealthlingo export backup.json    # backup completo
stealthlingo import backup.json    # mesclar (o progresso mais recente vence)
```

## Contribuir

Issues e pull requests são bem-vindos. Como compilar e testar: [Wiki · Development](https://github.com/viys/stealthlingo/wiki/Development).

## Licença

MIT — veja [LICENSE](../../LICENSE).
