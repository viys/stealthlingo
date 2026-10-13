# StealthLingo

[English](../../README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | **Español** | [Português (Brasil)](README.pt-BR.md) | [Русский](README.ru.md) | [Tiếng Việt](README.vi.md)

[![Release](https://img.shields.io/github/v/release/viys/stealthlingo)](https://github.com/viys/stealthlingo/releases) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](../../LICENSE)

Practica unas palabras de inglés mientras haces el vago. Pulsa **Esc** y aparece un shell en esta terminal; pulsa Esc otra vez y regresas. Esc no se escribe en ese shell. El temporizador se pausa mientras está oculta.

Ese hueco es cualquiera que ya tengas: una compilación, una reunión a la que no estás prestando atención, la espera de una respuesta de la IA. Busca una palabra, guárdala y practica con tarjetas, ortografía o escucha. Los datos del diccionario vienen del [Wiktionary](https://en.wiktionary.org/) en inglés (sin clave de API). El progreso vive en SQLite local, así que las palabras guardadas funcionan sin conexión. Un servidor MCP opcional permite que un agente de código añada las palabras que note; StealthLingo nunca llama a un modelo de IA.

![Inicio — objetivo del día, palabras pendientes y el menú](../images/home.png)

Guías completas en la **[Wiki](https://github.com/viys/stealthlingo/wiki)** (teclas, comandos, modos, MCP, copias de seguridad, FAQ, desarrollo). Solo inglés en v0.1.

## Por qué StealthLingo

Un hueco del que puedes salir al momento: una sesión corta, Esc si alguien se acerca, y una lista que crece con palabras que de verdad encuentras — en docs, código o chat.

| Obtienes | No obtienes |
|---|---|
| TUI a pantalla completa o prompts `--plain` | Cuentas, sincronización en la nube ni GUI |
| Esc oculta la pantalla y abre un shell; Esc vuelve. El temporizador se pausa | Tener que salir de la app para quitarla de la vista |
| Tarjetas, ortografía, letras que faltan, escucha, mixto, reintentar errores | Definiciones escritas por IA |
| Meta diaria + repetición espaciada (SM-2 simplificado) | Cursos largos obligatorios |
| Audio UK / US cuando Wiktionary lo tiene | Red obligatoria para repasar |

## Instalación

Binarios para Windows, macOS y Linux — no hace falta Rust:

```powershell
# Windows (PowerShell)
powershell -ExecutionPolicy Bypass -c "irm https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.ps1 | iex"
```

```bash
# macOS / Linux
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.sh | sh
```

O desde el código fuente (Rust 1.88+; en Linux hace falta `libasound2-dev` para compilar):

```bash
cargo install --git https://github.com/viys/stealthlingo
```

## Inicio rápido

```bash
stealthlingo                           # app a pantalla completa; Esc la oculta
stealthlingo study --minutes 3         # una sesión que cabe en el hueco
stealthlingo lookup ephemeral          # pronunciaciones + definiciones principales (--all para todo)
stealthlingo add ephemeral --note "lasting a short time"
stealthlingo --plain                   # prompts línea a línea
```

En inicio: `r` repasar · `a` practicar · `/` buscar · `w` lista · `t` estadísticas · `c` ajustes · **Esc** ocultar · **Ctrl+C** salir (`q` solo vuelve atrás).

En la shell que abre Esc, comandos como `lookup` y `add` funcionan con normalidad. La app a pantalla completa ya está abierta, así que volver a iniciarla ahí solo te recuerda que pulses Esc.

Modos de práctica (`a`, o la letra directa): `s` tarjetas · `p` ortografía · `m` letras que faltan · `i` escucha · `x` mixto · `e` reintentar errores.

## Comandos

| Comando | Qué hace |
|---|---|
| `stealthlingo` | Abrir la app a pantalla completa |
| `lookup <WORD> [--all]` | Consultar Wiktionary (caché sin conexión) |
| `add` / `remove` | Guardar o quitar una palabra |
| `words` / `search` | Listar o buscar (`--status`, `--pos`, `--due`) |
| `study` / `review` | Practicar (`--mode`, `--mistakes`, `--minutes`, `--count`) |
| `stats` | Progreso y precisión |
| `audio <WORD>` | Reproducir pronunciación |
| `config` | Ver o cambiar ajustes |
| `export` / `import` | Backup `.json` o lista `.csv` |
| `mcp` | Servidor MCP para agentes de IA |
| `links` | Lookups clicables (Windows) |

```bash
stealthlingo study --mode spelling --count 10
stealthlingo study --mode listening --minutes 5
stealthlingo study --mistakes
stealthlingo review
```

## Agentes de IA (MCP)

Mientras trabajas de verdad, un agente puede guardar palabras para la próxima vez que hagas el vago. Apunta tu cliente a StealthLingo (p. ej. Cursor `~/.cursor/mcp.json`):

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

Los agentes pueden buscar palabras y añadirlas a tu lista. No pueden borrar palabras, cambiar ajustes ni registrar respuestas. Detalles: [Wiki · AI Agents](https://github.com/viys/stealthlingo/wiki/AI-Agents-MCP).

## Datos y copias de seguridad

Directorio de datos por defecto: `%APPDATA%\stealthlingo\data` (Windows), `~/Library/Application Support/stealthlingo` (macOS), `~/.local/share/stealthlingo` (Linux). Se puede cambiar con `STEALTHLINGO_HOME`.

```bash
stealthlingo export backup.json    # backup completo
stealthlingo import backup.json    # fusionar (gana el progreso más reciente)
```

## Contribuir

Se agradecen issues y pull requests. Cómo compilar y probar: [Wiki · Development](https://github.com/viys/stealthlingo/wiki/Development).

## Licencia

MIT — ver [LICENSE](../../LICENSE).
