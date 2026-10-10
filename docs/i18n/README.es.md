# stealthlingo

[English](../../README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | **Español** | [Português (Brasil)](README.pt-BR.md) | [Русский](README.ru.md) | [Tiếng Việt](README.vi.md)

Una herramienta de aprendizaje de idiomas pensada para la terminal, para estudiar con discreción.

StealthLingo es una pequeña CLI en Rust para practicar vocabulario en ratos libres: busca una palabra, guárdala y practícala con tarjetas (flashcards), ortografía, letras que faltan, dictado auditivo o una mezcla de todo, con un objetivo diario de palabras. Esc oculta toda la pantalla de golpe. Los datos del diccionario provienen del [Wiktionary](https://en.wiktionary.org/) en inglés a través de la API oficial de Wikimedia, sin necesidad de clave; la programación de repasos, la corrección de respuestas y tu progreso se guardan en una base de datos SQLite local, así que puedes repasar las palabras guardadas sin conexión. Un agente de IA también puede añadir las palabras que quizá no conozcas mientras lees o programas (ver [Agentes de IA](#agentes-de-ia-mcp)).

En la v0.1 solo se puede estudiar inglés.

![Pantalla de inicio de StealthLingo: objetivo de hoy, palabras pendientes y menú de práctica](../images/home.png)

## Instalación

En la [página de Releases](https://github.com/viys/stealthlingo/releases) hay programas precompilados para Windows (x64), macOS (Intel y Apple silicon) y Linux (x64 y ARM64); no necesitas Rust. Los instaladores colocan `stealthlingo` y su programa auxiliar de enlaces `stealthlingo-link` en `~/.cargo/bin` y añaden esa carpeta a tu `PATH`.

Windows (PowerShell):

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.ps1 | iex"
```

macOS y Linux:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.sh | sh
```

También puedes descargar el archivo comprimido para tu sistema desde la página de Releases, descomprimirlo y mantener `stealthlingo` y `stealthlingo-link` en la misma carpeta. En Linux, la reproducción de audio usa ALSA (`libasound2`), que las distribuciones de escritorio incluyen por defecto.

### Desde el código fuente

Requiere una versión estable reciente de Rust (1.88+) y un compilador de C (SQLite viene incluido).

```bash
cargo install --git https://github.com/viys/stealthlingo
# o, desde una copia local del repositorio
cargo install --path .
cargo run -- --help
```

En Linux, para compilar también hace falta el paquete de desarrollo de ALSA (p. ej. `libasound2-dev`).

## Inicio rápido

```bash
stealthlingo lookup ephemeral             # pronunciación y definiciones principales (--all para todo)
stealthlingo add ephemeral --note efímero # guárdala, con una nota personal opcional
stealthlingo                              # abre la aplicación a pantalla completa: practicar, buscar, explorar, estadísticas
```

## Interfaz a pantalla completa

En una terminal, `stealthlingo`, `study` y `review` abren una interfaz a pantalla completa. La pantalla de inicio muestra lo que toca repasar hoy, tu avance hacia el objetivo diario y un menú (`r` repasar, `s` tarjetas, `p` ortografía, `m` letras que faltan, `l` escucha, `x` práctica mixta, `e` repetir errores, `/` buscar, `w` lista de palabras, `t` estadísticas, `c` ajustes, `q` salir); también funcionan `↑`/`↓` (o `k`/`j`) y `Enter`. Las listas y las páginas largas se desplazan con las mismas teclas.

![Esc oculta la aplicación y muestra la terminal; otro Esc la recupera](../images/boss-key.webp)

- **Esc lo oculta todo al instante** y muestra la terminal desde la que lo abriste; pulsa Esc otra vez para volver. El reloj de la sesión se detiene mientras está oculta.
- Las **tarjetas** no necesitan Enter: `Space` muestra la respuesta, `1`–`4` califica, `a` reproduce la palabra, `s` la salta y `q` termina la sesión.
- **Acentos**: `a` reproduce primero tu acento predeterminado (el ajuste `accent`); si la vuelves a pulsar en la misma palabra, reproduce el otro acento grabado, y así sucesivamente. El pie de pantalla indica qué acento sonará a continuación y la cabecera, cuál se está reproduciendo.
- **Ortografía y escucha**: escribe la palabra y pulsa `Enter`. `Tab` muestra la respuesta y `Ctrl+N` la salta. En la práctica de escucha, `Enter` con la línea vacía repite la grabación y `Ctrl+R` reproduce el otro acento. Si fallas, se muestran las letras que sobraban o faltaban. Las preguntas de letras que faltan muestran la palabra con algunas letras ocultas (`e _ h e _ e r _ l`); escribe la palabra completa.
- **Lista de palabras**: `/` filtra por texto, `s` recorre los estados, `p` las categorías gramaticales y `u` muestra solo las palabras pendientes. `Enter` abre la entrada guardada (sin conexión), `a` la reproduce, `n` edita la nota y `d` seguido de `y` elimina la palabra (si la vuelves a guardar, recupera su progreso). `PgUp`/`PgDn` y `g`/`G` permiten moverse por listas largas. Las palabras que añadió un agente de IA llevan la marca `agent`, y al seleccionarlas los detalles de abajo muestran el motivo que dio el agente.
- **Ajustes** (`c`): `←`/`→` cambian el valor seleccionado, `PgUp`/`PgDn` de 10 en 10, `Enter` permite escribir un número, `r` restaura el valor predeterminado y `R` restaura todos los ajustes. Cada cambio se guarda al instante; los valores fuera del rango permitido no se guardan.
- **Buscar**: escribe una palabra y pulsa `Enter`; después, `s` la guarda o la elimina, `a` la reproduce, `Tab` alterna entre la entrada breve y la completa, `/` inicia una nueva búsqueda y `q` vuelve atrás.
- `Ctrl+C` termina la sesión en curso, cancela un filtro o una nota y, en los demás casos, cierra la aplicación. Cada respuesta se guarda en cuanto la das.
- La interfaz necesita una terminal de al menos 40×12 caracteres. En ventanas bajas, los recuadros se reducen, el resumen de inicio cabe en una línea y el menú se divide en dos columnas (ventanas anchas) o se desplaza, para que el elemento seleccionado y el campo de respuesta sigan visibles.

![Una tarjeta con el significado a la vista, esperando la calificación](../images/flashcard.png)

![Una respuesta de ortografía incorrecta: la letra que faltaba aparece marcada en la respuesta](../images/spelling.png)

![Búsqueda de una palabra: pronunciaciones británica y estadounidense y las definiciones principales](../images/lookup.png)

`--plain` (o cualquier entrada/salida que no sea una terminal, como una tubería) usa en su lugar las preguntas línea a línea descritas más abajo. `NO_COLOR` desactiva los colores.

## Comandos

| Comando | Qué hace |
|---|---|
| `stealthlingo` | Abre la aplicación a pantalla completa. Con `--plain`: repasa las palabras pendientes; si no hay ninguna, estudia palabras nuevas; con la lista vacía, explica cómo empezar |
| `lookup <WORD> [--all]` | Consulta Wiktionary (actualiza la caché; sin conexión, usa la copia guardada) |
| `add <WORD> [--note TEXT]` | Guarda una palabra. Guardarla dos veces no causa problemas; `--note` actualiza la nota |
| `remove <WORD>` | Quita una palabra de tu lista (se conservan los datos del diccionario en caché y el historial) |
| `words [--status S] [--pos P] [--due]` | Lista las palabras guardadas con su estado, la fecha del próximo repaso y quién las añadió, con filtros opcionales |
| `search <QUERY> [--status S] [--pos P] [--due]` | Busca en las palabras guardadas y en las notas |
| `study [--mode memory\|spelling\|letters\|listening\|mixed] [--mistakes] [--minutes N] [--count N]` | Sesión de estudio |
| `review [--count N] [--minutes N]` | Repaso con tarjetas solo de las palabras pendientes |
| `stats` | Palabras guardadas por estado, pendientes, objetivo diario y últimos 7 días, respuestas de hoy y totales con su porcentaje de aciertos, próximo repaso |
| `audio <WORD> [--accent uk\|us]` | Reproduce la pronunciación (por defecto, según el ajuste `accent`) |
| `config` / `config set <KEY> <VALUE>` / `config reset [KEY]` | Muestra, cambia o restaura los ajustes; muestra las ubicaciones de los datos |
| `links [status\|install\|uninstall]` | Ctrl+clic en `lookup` para reproducir pronunciaciones y añadir o quitar palabras (Windows) |
| `export <PATH>` / `import <PATH>` | `.json` copia de seguridad completa, o `.csv` lista de palabras |
| `mcp` | Atiende a agentes de IA por MCP en stdin/stdout; lo inicia el cliente del agente (ver Agentes de IA más abajo) |

Sin `--minutes` ni `--count`, una sesión dura `default_minutes` (5). Las sesiones iniciadas desde la pantalla de inicio, o con `stealthlingo --plain`, duran en cambio hasta alcanzar el objetivo diario (ver [Objetivo diario](#objetivo-diario)).

Los filtros de la lista de palabras se combinan: `--status` es `new`, `learning`, `review` o `mastered`; `--pos` coincide con el comienzo de una categoría gramatical (`adj`, `n`, `verb`); `--due` deja solo las palabras cuyo repaso ya toca.

```bash
stealthlingo words --status learning --pos adj
stealthlingo words --due
```

`lookup` muestra las pronunciaciones en una línea, primero la británica (UK) y luego la estadounidense (US) (`Pronunciation: UK /njuː/ · US /nu/`), e indica el acento de la grabación que reproduce `audio` (o sugiere `--accent uk|us` cuando hay ambas). Por defecto, a continuación muestra las tres primeras definiciones de hasta tres categorías gramaticales. `lookup --all` (o `-a`) muestra todas las definiciones con ejemplos, sinónimos, antónimos y la fuente. Las definiciones se ajustan al ancho de la terminal, con las líneas de continuación sangradas.

### Búsquedas con enlaces (Windows)

```bash
stealthlingo links install   # una sola vez, después de instalar
```

Esto registra un manejador de enlaces `stealthlingo://` para tu usuario (sin permisos de administrador). A partir de entonces, `lookup` convierte cada pronunciación con grabación en un hipervínculo de terminal: con Ctrl+clic, la grabación suena en segundo plano, sin abrir un navegador ni una ventana. La última línea pasa a ser `+ Add to word list`, o `Saved in your word list · Remove` si la palabra ya está guardada; con Ctrl+clic puedes añadir o quitar la palabra sin escribir ningún comando (vuelve a ejecutar `lookup` para ver el cambio). Como otros programas también pueden abrir estos enlaces, quitar una palabra mediante un enlace conserva su progreso: si la vuelves a añadir, por enlace o con `add`, lo recupera. En cambio, el comando `remove` hace que la palabra empiece de cero. Funciona en terminales que muestran hipervínculos (Windows Terminal, VS Code / Cursor); en las demás, `lookup` imprime texto plano con los comandos `add` y `audio`. `links status` muestra lo que se detectó, `FORCE_HYPERLINK=1` (o `0`) anula la detección y `links uninstall` elimina el manejador. Los errores se escriben en `link-errors.log`, en el directorio de datos.

### Modos de estudio

Las teclas siguientes son las de las preguntas línea a línea (`--plain`); la interfaz a pantalla completa usa teclas únicas, como se describe arriba.

- **memory (memoria)**: ves la palabra, pulsas Enter para ver el significado y te autoevalúas: `1` Again (otra vez), `2` Hard (difícil), `3` Good (bien), `4` Easy (fácil). Pulsa `a` para oír la palabra.
- **spelling (ortografía)**: lee la definición (la palabra se oculta en las definiciones y los ejemplos) y escríbela. `?` te rinde y muestra la respuesta.
- **letters (letras que faltan)**: como ortografía, pero se muestra aproximadamente la mitad de las letras (`e _ h e _ e r _ l`). Los huecos son siempre los mismos para cada palabra. Las respuestas cuentan como práctica de ortografía.
- **listening (escucha)**: escucha la pronunciación y escribe la palabra. `r` la repite. Solo se usan palabras con audio de pronunciación.
- **mixed (mixto)**: las palabras que nunca has estudiado empiezan como tarjetas; las demás alternan entre tarjetas, ortografía y (si hay grabación) escucha.

`--mistakes` practica las palabras que respondiste mal en los últimos 30 días, o que olvidaste después de haberlas aprendido, empezando por los errores más recientes, estén pendientes o no. Funciona con cualquier modo; en la pantalla de inicio, `e` usa la práctica mixta.

En todos los modos, `s` salta la palabra actual sin registrar respuesta y `q` termina la sesión. Ctrl+C también termina la sesión; un segundo Ctrl+C sale de inmediato. Cada respuesta enviada se guarda al momento, así que salir nunca hace perder el trabajo terminado, y una pregunta sin responder nunca se registra como incorrecta.

La corrección ignora mayúsculas y minúsculas y los espacios al principio y al final, pero los apóstrofos y los guiones cuentan (`well-being` no es `wellbeing`).

### Programación de repasos

Un algoritmo SM-2 simplificado decide cuándo vuelve una palabra:

- **Again**: vuelve en 10 minutos; el progreso se reinicia.
- **Hard**: intervalo más corto de lo normal.
- **Good**: 1 día, luego 3 días, y después el intervalo crece según el factor de facilidad (ease factor) de la palabra.
- **Easy**: intervalo más largo, y la palabra se vuelve más fácil a partir de entonces.

En ortografía y escucha, una respuesta correcta cuenta como Good y una incorrecta como Again. Una palabra fallada vuelve una vez más al final de la misma sesión. Los repasos pendientes siempre van antes que las palabras nuevas, y cada día se introducen como máximo `daily_new_limit` (10 por defecto) palabras nunca estudiadas. Las palabras con un intervalo de 21 días o más se muestran como `mastered` (dominadas).

### Objetivo diario

`daily_goal` (10 por defecto) es el número de palabras distintas que practicar cada día; responder otra vez la misma palabra no cuenta dos veces. La pantalla de inicio, la página de estadísticas y `stats` muestran el avance de hoy, y `stats` también muestra los últimos 7 días. Hasta alcanzar el objetivo, las sesiones iniciadas desde la pantalla de inicio (o con `stealthlingo --plain`) terminan al alcanzarlo en lugar de tras `default_minutes`; si te quedas sin palabras antes, la sesión indica cuántas faltaron. Un `--minutes` o `--count` explícito siempre tiene prioridad. `0` desactiva el objetivo.

## Configuración

Cambia los ajustes en la pantalla de ajustes (`c` en la pantalla de inicio) o con `config`:

```bash
stealthlingo config set daily_goal 20
stealthlingo config set daily_new_limit 15
stealthlingo config set default_minutes 3
stealthlingo config set accent us        # reproducir primero las grabaciones estadounidenses (por defecto: uk)
stealthlingo config reset daily_goal     # volver al valor predeterminado
stealthlingo config reset                # todos los ajustes, tras confirmar (--yes omite la confirmación)
```

| Clave | Predeterminado | Rango | Significado |
|---|---|---|---|
| `daily_goal` | 10 | 0–500, 0 = desactivado | Palabras distintas que practicar al día |
| `daily_new_limit` | 10 | 0–200, 0 = solo repaso | Palabras nunca estudiadas que se introducen al día |
| `default_minutes` | 5 | 1–120 | Duración de una sesión sin `--minutes` ni `--count` |
| `accent` | uk | uk / us | Acento que se reproduce primero |
| `mcp_daily_add_limit` | 30 | 0–200, 0 = desactivado | Palabras que los agentes de IA pueden añadir al día mediante MCP |
| `http_timeout_secs` | 10 | 1–120 | Tiempo de espera de red |

`config` muestra cada valor junto a su valor predeterminado y su rango. Si `config.json` contiene un valor fuera de rango, StealthLingo avisa y usa el valor permitido más cercano sin reescribir el archivo. Las palabras grabadas en un solo acento siempre reproducen esa grabación.

### Diccionario

StealthLingo usa la API oficial de Wiktionary de Wikimedia: las definiciones y los ejemplos vienen de la API REST, y el AFI (etiquetado por acento), los sinónimos, los antónimos y las grabaciones de Wikimedia Commons (OGG), del código fuente de la página. Cuando la forma escrita y el lema del diccionario difieren (`Ephemeral` y `ephemeral`), se recuerda la forma escrita, de modo que `add` y `remove` se refieren a la misma palabra guardada.

Las versiones de desarrollo anteriores a la publicación de v0.1.0 también podían usar Free Dictionary API o Merriam-Webster. Las palabras guardadas en caché desde esas fuentes se pueden seguir leyendo sin conexión y se sustituyen por datos de Wiktionary la próxima vez que se buscan (el progreso y las notas se conservan). Sus antiguos ajustes en `config.json` se ignoran.

## Agentes de IA (MCP)

`stealthlingo mcp` ejecuta un servidor [Model Context Protocol](https://modelcontextprotocol.io) en stdin/stdout, para que un agente de IA (Cursor, Claude Desktop u otro cliente MCP) pueda buscar palabras y añadir a tu lista de estudio las que quizá no conozcas mientras lees o programas. StealthLingo nunca llama por sí mismo a un modelo de IA. Añádelo a la configuración MCP de tu cliente (en Cursor, `~/.cursor/mcp.json`, o `.cursor/mcp.json` dentro de un proyecto):

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

Si `stealthlingo` no está en tu `PATH`, pon la ruta completa del ejecutable en `command`. Después pide al agente, por ejemplo, que añada las palabras de esta página que quizá no conozcas.

| Herramienta | Qué hace |
|---|---|
| `get_study_status` | Palabras guardadas, pendientes y sin empezar, el progreso y la precisión de hoy respecto al objetivo, y cuántas palabras pueden añadir todavía los agentes hoy |
| `list_words` | Palabras guardadas con estado, fecha de repaso, audio, olvidos y una definición breve; `query`, `status` y `limit` la acotan. Las notas nunca se comparten |
| `lookup_word` | La entrada de Wiktionary de una palabra (primero la caché), sin guardarla |
| `add_words` | Guarda hasta 20 palabras por llamada, cada una con un `reason` opcional; informa del resultado palabra por palabra |

Los agentes solo eligen palabras:

- Las definiciones, los ejemplos y las pronunciaciones vienen solo de Wiktionary. Un agente puede decir dónde apareció una palabra (`reason`), pero no puede escribir definiciones, traducciones ni tus notas. Las palabras que Wiktionary no tiene no se añaden.
- No hay ninguna herramienta para eliminar palabras, cambiar ajustes o registrar respuestas.
- Las palabras que eliminaste (con `remove`, `d` en la lista de palabras o un enlace) nunca las vuelve a añadir un agente; si la guardas tú, vuelve a estar disponible.
- Los agentes añaden como máximo `mcp_daily_add_limit` (30 por defecto) palabras al día, y eliminar una no devuelve el cupo. `0` desactiva la función. Los cambios se aplican en la siguiente llamada, sin reiniciar el cliente.
- Las palabras que añadió un agente se estudian como cualquier otra. La lista de palabras las marca con `agent` y muestra el motivo del agente; `words` indica quién añadió cada palabra en la columna `BY`. Las copias JSON conservan ambos datos y las palabras que eliminaste.

## Directorio de datos

`stealthlingo config` muestra las rutas exactas. Por defecto:

| SO | Ubicación |
|---|---|
| Windows | `%APPDATA%\stealthlingo\data` |
| macOS | `~/Library/Application Support/stealthlingo` |
| Linux | `~/.local/share/stealthlingo` |

Define `STEALTHLINGO_HOME` para usar otro directorio (útil para una instalación portátil o para hacer pruebas). Contiene `stealthlingo.db` (SQLite, con migraciones versionadas), `config.json` y `audio/` (archivos de pronunciación descargados).

### Copias de seguridad y listas de palabras

```bash
stealthlingo export backup.json   # todo: caché, progreso, historial de respuestas
stealthlingo import backup.json   # combina; volver a importar el mismo archivo no cambia nada
stealthlingo export words.csv     # word, note, status, due_at, definition
stealthlingo import examples/words.csv   # columna "word" (o la primera); "note" es opcional
```

Si una misma palabra existe en ambos lados de una importación JSON, gana el progreso repasado más recientemente. La importación CSV busca cada palabra que aún no está en caché, así que necesita conexión.

## Funcionamiento sin conexión

- Estudiar, repasar, `words`, `search` y `stats` nunca usan la red.
- `lookup` actualiza desde Wiktionary y muestra la copia en caché si no puede conectarse.
- `add` y `audio` usan la caché siempre que pueden y solo se conectan para palabras nuevas.
- En `mcp`, `get_study_status` y `list_words` nunca usan la red; `lookup_word` y `add_words` solo se conectan para palabras que no están en la caché.
- El audio de pronunciación se descarga la primera vez que se reproduce y después se reutiliza.

## Sobre los datos del diccionario

- Las entradas incluyen su URL de origen y su licencia, que `lookup --all` muestra. El contenido de Wiktionary tiene licencia CC BY-SA 4.0.
- La cobertura varía según la palabra: pueden faltar la transcripción fonética, el audio, los ejemplos, los sinónimos y los antónimos. StealthLingo trata todos los campos como opcionales; las palabras sin audio se omiten en la práctica de escucha.
- Wiktionary no es un servicio de traducción. Las pistas en español (o en cualquier otro idioma) vienen de tu propia `--note`, nunca del diccionario.
- El servicio es gratuito y puede ser lento o no estar disponible. Respeta las condiciones de uso de Wikimedia y no redistribuyas datos de la caché de forma masiva sin cumplir la licencia.

## Desarrollo

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

El código se divide en `dictionary` (cliente y analizadores de Wiktionary que generan un modelo `Entry`, más un decodificador para los datos guardados en caché por las versiones previas a la publicación), `storage` (SQLite), `learning` (programación, corrección de respuestas, sesiones), `audio`, `commands` y `tui` (la interfaz a pantalla completa, hecha con ratatui). Ambas interfaces usan la misma `learning::session::Session`. La programación de repasos es una función pura con sus propias pruebas en `tests/scheduling_tests.rs`.

### Publicación

Las versiones se compilan con [dist](https://github.com/axodotdev/cargo-dist) (`dist-workspace.toml`, `.github/workflows/release.yml`). Sube `version` en `Cargo.toml`, haz commit y luego publica una etiqueta que coincida:

```bash
git tag v0.2.0
git push origin v0.2.0
```

Después, GitHub Actions compila para todas las plataformas y publica los archivos y los instaladores como una GitHub Release. `dist plan` muestra una vista previa de lo que se compilará; tras cambiar `dist-workspace.toml`, ejecuta `dist generate` para actualizar el flujo de trabajo.

## Licencia

MIT, consulta [LICENSE](../../LICENSE).
