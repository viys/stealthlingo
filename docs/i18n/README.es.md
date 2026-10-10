# stealthlingo

[English](../../README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | **Español** | [Português (Brasil)](README.pt-BR.md) | [Русский](README.ru.md) | [Tiếng Việt](README.vi.md)

Una herramienta de aprendizaje de idiomas pensada para la terminal, para estudiar con discreción.

StealthLingo es una pequeña CLI en Rust para practicar vocabulario en ratos libres: busca una palabra, guárdala y haz sesiones de 3 a 5 minutos de tarjetas (flashcards), ortografía o dictado auditivo. Los datos del diccionario provienen del [Wiktionary](https://en.wiktionary.org/) en inglés a través de la API oficial de Wikimedia, sin necesidad de clave; la programación de repasos, la corrección de respuestas y tu progreso se guardan en una base de datos SQLite local, así que puedes repasar las palabras guardadas sin conexión.

En la v0.1 solo se puede estudiar inglés.

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

En una terminal, `stealthlingo`, `study` y `review` abren una interfaz a pantalla completa. La pantalla de inicio muestra lo que toca repasar hoy y un menú (`r` repasar, `s` tarjetas, `p` ortografía, `l` escucha, `/` buscar, `w` lista de palabras, `t` estadísticas, `q` salir); también funcionan `↑`/`↓` (o `k`/`j`) y `Enter`. Las listas y las páginas largas se desplazan con las mismas teclas.

- **Esc lo oculta todo al instante** y muestra la terminal desde la que lo abriste; pulsa Esc otra vez para volver. El reloj de la sesión se detiene mientras está oculta.
- Las **tarjetas** no necesitan Enter: `Space` muestra la respuesta, `1`–`4` califica, `a` reproduce la palabra, `s` la salta y `q` termina la sesión.
- **Acentos**: `a` reproduce primero tu acento predeterminado (el ajuste `accent`); si la vuelves a pulsar en la misma palabra, reproduce el otro acento grabado, y así sucesivamente. El pie de pantalla indica qué acento sonará a continuación y la cabecera, cuál se está reproduciendo.
- **Ortografía y escucha**: escribe la palabra y pulsa `Enter`. `Tab` muestra la respuesta y `Ctrl+N` la salta. En la práctica de escucha, `Enter` con la línea vacía repite la grabación y `Ctrl+R` reproduce el otro acento. Si fallas, se muestran las letras que sobraban o faltaban.
- **Lista de palabras**: `/` filtra, `Enter` abre la entrada guardada (sin conexión), `a` la reproduce, `n` edita la nota y `d` seguido de `y` elimina la palabra (si la vuelves a guardar, recupera su progreso). `PgUp`/`PgDn` y `g`/`G` permiten moverse por listas largas.
- **Buscar**: escribe una palabra y pulsa `Enter`; después, `s` la guarda o la elimina, `Tab` alterna entre la entrada breve y la completa, `/` inicia una nueva búsqueda y `q` vuelve atrás.
- `Ctrl+C` termina la sesión en curso, cancela un filtro o una nota y, en los demás casos, cierra la aplicación. Cada respuesta se guarda en cuanto la das.
- La interfaz necesita una terminal de al menos 40×12 caracteres. En ventanas bajas, los recuadros se reducen, el resumen de inicio cabe en una línea y el menú se desplaza, para que el elemento seleccionado y el campo de respuesta sigan visibles.

`--plain` (o cualquier entrada/salida que no sea una terminal, como una tubería) usa en su lugar las preguntas línea a línea descritas más abajo. `NO_COLOR` desactiva los colores.

## Comandos

| Comando | Qué hace |
|---|---|
| `stealthlingo` | Abre la aplicación a pantalla completa. Con `--plain`: repasa las palabras pendientes; si no hay ninguna, estudia palabras nuevas; con la lista vacía, explica cómo empezar |
| `lookup <WORD> [--all]` | Consulta Wiktionary (actualiza la caché; sin conexión, usa la copia guardada) |
| `add <WORD> [--note TEXT]` | Guarda una palabra. Guardarla dos veces no causa problemas; `--note` actualiza la nota |
| `remove <WORD>` | Quita una palabra de tu lista (se conservan los datos del diccionario en caché y el historial) |
| `words` | Lista las palabras guardadas con su estado y la fecha del próximo repaso |
| `search <QUERY>` | Busca en las palabras guardadas y en las notas |
| `study [--mode memory\|spelling\|listening] [--minutes N] [--count N]` | Sesión de estudio |
| `review [--count N] [--minutes N]` | Repaso con tarjetas solo de las palabras pendientes |
| `stats` | Palabras guardadas por estado, pendientes, respuestas de hoy y totales con su porcentaje de aciertos, próximo repaso |
| `audio <WORD> [--accent uk\|us]` | Reproduce la pronunciación (por defecto, según el ajuste `accent`) |
| `config` / `config set <KEY> <VALUE>` | Muestra o cambia los ajustes y las ubicaciones de los datos |
| `links [status\|install\|uninstall]` | Ctrl+clic en `lookup` para reproducir pronunciaciones y añadir o quitar palabras (Windows) |
| `export <PATH>` / `import <PATH>` | `.json` copia de seguridad completa, o `.csv` lista de palabras |

Sin `--minutes` ni `--count`, una sesión dura `default_minutes` (5).

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
- **listening (escucha)**: escucha la pronunciación y escribe la palabra. `r` la repite. Solo se usan palabras con audio de pronunciación.

En todos los modos, `s` salta la palabra actual sin registrar respuesta y `q` termina la sesión. Ctrl+C también termina la sesión; un segundo Ctrl+C sale de inmediato. Cada respuesta enviada se guarda al momento, así que salir nunca hace perder el trabajo terminado, y una pregunta sin responder nunca se registra como incorrecta.

La corrección ignora mayúsculas y minúsculas y los espacios al principio y al final, pero los apóstrofos y los guiones cuentan (`well-being` no es `wellbeing`).

### Programación de repasos

Un algoritmo SM-2 simplificado decide cuándo vuelve una palabra:

- **Again**: vuelve en 10 minutos; el progreso se reinicia.
- **Hard**: intervalo más corto de lo normal.
- **Good**: 1 día, luego 3 días, y después el intervalo crece según el factor de facilidad (ease factor) de la palabra.
- **Easy**: intervalo más largo, y la palabra se vuelve más fácil a partir de entonces.

En ortografía y escucha, una respuesta correcta cuenta como Good y una incorrecta como Again. Una palabra fallada vuelve una vez más al final de la misma sesión. Los repasos pendientes siempre van antes que las palabras nuevas, y cada día se introducen como máximo `daily_new_limit` (10 por defecto) palabras nunca estudiadas. Las palabras con un intervalo de 21 días o más se muestran como `mastered` (dominadas).

## Configuración

```bash
stealthlingo config set daily_new_limit 15
stealthlingo config set default_minutes 3
stealthlingo config set http_timeout_secs 10
stealthlingo config set accent us        # reproducir primero las grabaciones estadounidenses (por defecto: uk)
```

Las palabras grabadas en un solo acento siempre reproducen esa grabación.

### Diccionario

StealthLingo usa la API oficial de Wiktionary de Wikimedia: las definiciones y los ejemplos vienen de la API REST, y el AFI (etiquetado por acento), los sinónimos, los antónimos y las grabaciones de Wikimedia Commons (OGG), del código fuente de la página. Cuando la forma escrita y el lema del diccionario difieren (`Ephemeral` y `ephemeral`), se recuerda la forma escrita, de modo que `add` y `remove` se refieren a la misma palabra guardada.

Las versiones de desarrollo anteriores a la publicación de v0.1.0 también podían usar Free Dictionary API o Merriam-Webster. Las palabras guardadas en caché desde esas fuentes se pueden seguir leyendo sin conexión y se sustituyen por datos de Wiktionary la próxima vez que se buscan (el progreso y las notas se conservan). Sus antiguos ajustes en `config.json` se ignoran.

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
