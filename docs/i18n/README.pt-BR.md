# stealthlingo

[English](../../README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Español](README.es.md) | **Português (Brasil)** | [Русский](README.ru.md) | [Tiếng Việt](README.vi.md)

Uma ferramenta de aprendizado de idiomas feita para o terminal, para estudar com discrição.

O StealthLingo é uma pequena CLI em Rust para praticar vocabulário nos minutos livres: pesquise uma palavra, salve-a e faça sessões de 3 a 5 minutos de flashcards, ortografia ou ditado por áudio. Os dados do dicionário vêm do [Wiktionary](https://en.wiktionary.org/) em inglês pela API oficial da Wikimedia, sem precisar de chave; o agendamento das revisões, a correção das respostas e o seu progresso ficam em um banco de dados SQLite local, então as palavras salvas podem ser revisadas offline.

Na v0.1, só é possível estudar inglês.

## Instalação

Há programas pré-compilados para Windows (x64), macOS (Intel e Apple silicon) e Linux (x64 e ARM64) na [página de Releases](https://github.com/viys/stealthlingo/releases); não é preciso ter Rust. Os instaladores colocam o `stealthlingo` e o seu auxiliar de links `stealthlingo-link` em `~/.cargo/bin` e adicionam essa pasta ao seu `PATH`.

Windows (PowerShell):

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.ps1 | iex"
```

macOS e Linux:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.sh | sh
```

Ou baixe o arquivo compactado para o seu sistema na página de Releases, descompacte-o e mantenha `stealthlingo` e `stealthlingo-link` na mesma pasta. No Linux, a reprodução de áudio usa o ALSA (`libasound2`), que as distribuições desktop já trazem por padrão.

### A partir do código-fonte

Requer uma versão estável recente do Rust (1.88+) e um compilador C (o SQLite já vem incluído).

```bash
cargo install --git https://github.com/viys/stealthlingo
# ou, a partir de um clone do repositório
cargo install --path .
cargo run -- --help
```

No Linux, a compilação também precisa do pacote de desenvolvimento do ALSA (por exemplo, `libasound2-dev`).

## Início rápido

```bash
stealthlingo lookup ephemeral             # pronúncia e principais definições (--all para tudo)
stealthlingo add ephemeral --note efêmero # salva a palavra, com uma nota pessoal opcional
stealthlingo                              # abre o app em tela cheia: praticar, pesquisar, navegar, estatísticas
```

## Interface em tela cheia

Em um terminal, `stealthlingo`, `study` e `review` abrem uma interface em tela cheia. A tela inicial mostra o que está pendente hoje e um menu (`r` revisar, `s` flashcards, `p` ortografia, `l` audição, `/` pesquisar, `w` lista de palavras, `t` estatísticas, `q` sair); `↑`/`↓` (ou `k`/`j`) e `Enter` também funcionam. Listas e páginas longas rolam com as mesmas teclas.

- **Esc esconde tudo na hora** e mostra o terminal de onde você abriu o programa; pressione Esc de novo para voltar. O cronômetro da sessão fica pausado enquanto a interface está escondida.
- Os **flashcards** não precisam de Enter: `Space` revela a resposta, `1`–`4` avaliam, `a` toca a palavra, `s` pula e `q` encerra a sessão.
- **Sotaques**: `a` toca primeiro o seu sotaque padrão (a configuração `accent`); pressionar de novo na mesma palavra toca o outro sotaque gravado, e assim por diante. O rodapé mostra qual sotaque vem a seguir, e o cabeçalho, qual está tocando.
- **Ortografia e audição**: digite a palavra e pressione `Enter`. `Tab` mostra a resposta e `Ctrl+N` pula. Na prática de audição, `Enter` em uma linha vazia repete a gravação e `Ctrl+R` toca o outro sotaque. Uma resposta errada mostra quais letras sobraram ou faltaram.
- **Lista de palavras**: `/` filtra, `Enter` abre a entrada salva (offline), `a` toca a palavra, `n` edita a nota e `d` seguido de `y` remove a palavra (salvá-la de novo restaura o progresso). `PgUp`/`PgDn` e `g`/`G` navegam por listas longas.
- **Pesquisar**: digite uma palavra e pressione `Enter`; depois, `s` salva ou remove a palavra, `Tab` alterna entre a entrada resumida e a completa, `/` inicia uma nova pesquisa e `q` volta.
- `Ctrl+C` encerra a sessão atual, cancela um filtro ou uma nota e, nos outros casos, fecha o programa. Cada resposta é salva assim que é dada.
- A interface precisa de um terminal com pelo menos 40×12 caracteres. Em janelas baixas, os quadros encolhem, o resumo da tela inicial cabe em uma linha e o menu rola, para que o item selecionado e o campo de resposta continuem visíveis.

`--plain` (ou qualquer entrada/saída que não seja um terminal, como um pipe) usa os prompts linha a linha descritos abaixo. `NO_COLOR` desativa as cores.

## Comandos

| Comando | O que faz |
|---|---|
| `stealthlingo` | Abre o app em tela cheia. Com `--plain`: revisa as palavras pendentes; se não houver nenhuma, estuda palavras novas; com a lista vazia, explica como começar |
| `lookup <WORD> [--all]` | Consulta o Wiktionary (atualiza o cache; sem conexão, usa a cópia em cache) |
| `add <WORD> [--note TEXT]` | Salva uma palavra. Salvar duas vezes não causa problema; `--note` atualiza a nota |
| `remove <WORD>` | Remove uma palavra da sua lista (os dados do dicionário em cache e o histórico são mantidos) |
| `words` | Lista as palavras salvas com status e data da próxima revisão |
| `search <QUERY>` | Pesquisa nas palavras salvas e nas notas |
| `study [--mode memory\|spelling\|listening] [--minutes N] [--count N]` | Sessão de estudo |
| `review [--count N] [--minutes N]` | Revisão com flashcards apenas das palavras pendentes |
| `stats` | Palavras salvas por status, pendentes, respostas de hoje e de todo o período com a taxa de acerto, próxima revisão |
| `audio <WORD> [--accent uk\|us]` | Toca a pronúncia (padrão: a configuração `accent`) |
| `config` / `config set <KEY> <VALUE>` | Mostra ou altera as configurações e os locais dos dados |
| `links [status\|install\|uninstall]` | Ctrl+clique no `lookup` para tocar pronúncias e adicionar ou remover palavras (Windows) |
| `export <PATH>` / `import <PATH>` | `.json` backup completo, ou `.csv` lista de palavras |

Sem `--minutes` ou `--count`, uma sessão dura `default_minutes` (5).

`lookup` mostra as pronúncias em uma linha, primeiro a britânica (UK) e depois a americana (US) (`Pronunciation: UK /njuː/ · US /nu/`), e informa o sotaque da gravação que o `audio` toca (ou sugere `--accent uk|us` quando há as duas). Por padrão, em seguida lista as três primeiras definições de até três classes gramaticais. `lookup --all` (ou `-a`) mostra todas as definições com exemplos, sinônimos, antônimos e a fonte. As definições quebram na largura do terminal, com as linhas de continuação recuadas.

### Pesquisas clicáveis (Windows)

```bash
stealthlingo links install   # uma vez, depois de instalar
```

Isso registra um manipulador de links `stealthlingo://` para o seu usuário (sem precisar de permissões de administrador). A partir daí, o `lookup` transforma cada pronúncia que tem gravação em um hiperlink no terminal: com Ctrl+clique, a gravação toca em segundo plano, sem abrir navegador nem janela. A última linha vira `+ Add to word list`, ou `Saved in your word list · Remove` para palavras já salvas; com Ctrl+clique você adiciona ou remove a palavra sem digitar nenhum comando (execute `lookup` de novo para ver a mudança). Como outros programas também podem abrir esses links, remover uma palavra por link mantém o progresso de estudo: adicioná-la de novo, por link ou com `add`, restaura esse progresso. Já o comando `remove` faz a palavra recomeçar do zero. Funciona em terminais que exibem hiperlinks (Windows Terminal, VS Code / Cursor); nos outros, o `lookup` imprime texto simples com os comandos `add` e `audio`. `links status` mostra o que foi detectado, `FORCE_HYPERLINK=1` (ou `0`) substitui a detecção e `links uninstall` remove o manipulador. Os erros são gravados em `link-errors.log`, no diretório de dados.

### Modos de estudo

As teclas abaixo são as dos prompts linha a linha (`--plain`); a interface em tela cheia usa teclas únicas, como descrito acima.

- **memory (memória)**: veja a palavra, pressione Enter para revelar o significado e avalie a si mesmo: `1` Again (de novo), `2` Hard (difícil), `3` Good (bom), `4` Easy (fácil). Pressione `a` para ouvir a palavra.
- **spelling (ortografia)**: leia a definição (a própria palavra fica oculta nas definições e nos exemplos) e digite a palavra. `?` desiste e mostra a resposta.
- **listening (audição)**: ouça a pronúncia e digite a palavra. `r` repete. Só são usadas palavras com áudio de pronúncia.

Em todos os modos, `s` pula a palavra atual sem registrar resposta e `q` encerra a sessão. Ctrl+C também encerra a sessão; um segundo Ctrl+C sai imediatamente. Cada resposta enviada é salva na hora, então sair nunca faz perder o que já foi feito, e uma pergunta não respondida nunca é registrada como errada.

A correção ignora maiúsculas e minúsculas e espaços no início e no fim, mas apóstrofos e hifens contam (`well-being` não é `wellbeing`).

### Agendamento

Um algoritmo SM-2 simplificado decide quando uma palavra volta:

- **Again**: volta em 10 minutos; o progresso é reiniciado.
- **Hard**: intervalo mais curto que o normal.
- **Good**: 1 dia, depois 3 dias, e então o intervalo cresce pelo fator de facilidade (ease factor) da palavra.
- **Easy**: intervalo mais longo, e a palavra passa a ser considerada mais fácil daí em diante.

Em ortografia e audição, respostas certas contam como Good e erradas como Again. Uma palavra errada volta mais uma vez no fim da mesma sessão. Revisões pendentes sempre vêm antes de palavras novas, e no máximo `daily_new_limit` (padrão 10) palavras nunca estudadas são introduzidas por dia. Palavras com intervalo de 21 dias ou mais aparecem como `mastered` (dominadas).

## Configuração

```bash
stealthlingo config set daily_new_limit 15
stealthlingo config set default_minutes 3
stealthlingo config set http_timeout_secs 10
stealthlingo config set accent us        # tocar primeiro as gravações americanas (padrão: uk)
```

Palavras gravadas em um só sotaque sempre tocam essa gravação.

### Dicionário

O StealthLingo usa a API oficial do Wiktionary, da Wikimedia: definições e exemplos vêm da API REST, e o IPA (identificado por sotaque), sinônimos, antônimos e gravações do Wikimedia Commons (OGG) vêm do código-fonte da página. Quando a forma digitada e o verbete do dicionário são diferentes (`Ephemeral` e `ephemeral`), a forma digitada é lembrada, de modo que `add` e `remove` se referem à mesma palavra salva.

Versões de desenvolvimento anteriores ao lançamento da v0.1.0 também podiam usar a Free Dictionary API ou o Merriam-Webster. Palavras armazenadas em cache a partir dessas fontes continuam legíveis offline e são substituídas por dados do Wiktionary na próxima vez que forem pesquisadas (o progresso de estudo e as notas são mantidos). As configurações antigas dessas fontes em `config.json` são ignoradas.

## Diretório de dados

`stealthlingo config` mostra os caminhos exatos. Por padrão:

| SO | Local |
|---|---|
| Windows | `%APPDATA%\stealthlingo\data` |
| macOS | `~/Library/Application Support/stealthlingo` |
| Linux | `~/.local/share/stealthlingo` |

Defina `STEALTHLINGO_HOME` para usar outro diretório (útil para uma instalação portátil ou para testes). Ele contém `stealthlingo.db` (SQLite, com migrações versionadas), `config.json` e `audio/` (arquivos de pronúncia baixados).

### Backups e listas de palavras

```bash
stealthlingo export backup.json   # tudo: cache, progresso, histórico de respostas
stealthlingo import backup.json   # mescla; reimportar o mesmo arquivo não muda nada
stealthlingo export words.csv     # word, note, status, due_at, definition
stealthlingo import examples/words.csv   # coluna "word" (ou a primeira); "note" é opcional
```

Quando a mesma palavra existe dos dois lados de uma importação JSON, vence o progresso revisado mais recentemente. A importação CSV pesquisa cada palavra que ainda não está em cache, por isso precisa de conexão.

## Funcionamento offline

- Estudar, revisar, `words`, `search` e `stats` nunca acessam a rede.
- `lookup` atualiza a partir do Wiktionary e mostra a cópia em cache se não conseguir se conectar.
- `add` e `audio` usam o cache sempre que possível e só acessam a internet para palavras novas.
- O áudio de pronúncia é baixado na primeira vez que é tocado e reutilizado depois.

## Sobre os dados do dicionário

- As entradas trazem a URL de origem e a licença, que o `lookup --all` exibe. O conteúdo do Wiktionary é licenciado sob CC BY-SA 4.0.
- A cobertura varia conforme a palavra: transcrição fonética, áudio, exemplos, sinônimos e antônimos podem faltar. O StealthLingo trata todos os campos como opcionais; palavras sem áudio são puladas na prática de audição.
- O Wiktionary não é um serviço de tradução. Dicas em português (ou em qualquer outro idioma) vêm da sua própria `--note`, nunca do dicionário.
- O serviço é gratuito e pode ficar lento ou fora do ar. Respeite os termos de uso da Wikimedia e não redistribua dados do cache em massa sem cumprir a licença.

## Desenvolvimento

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

O código é dividido em `dictionary` (cliente e parsers do Wiktionary mapeados para um modelo `Entry`, mais um decodificador para dados armazenados em cache por versões anteriores ao lançamento), `storage` (SQLite), `learning` (agendamento, correção de respostas, sessões), `audio`, `commands` e `tui` (a interface em tela cheia, feita com ratatui). As duas interfaces usam a mesma `learning::session::Session`. O agendamento é uma função pura, com testes próprios em `tests/scheduling_tests.rs`.

### Lançamentos

Os lançamentos são compilados pelo [dist](https://github.com/axodotdev/cargo-dist) (`dist-workspace.toml`, `.github/workflows/release.yml`). Aumente `version` no `Cargo.toml`, faça commit e depois envie uma tag correspondente:

```bash
git tag v0.2.0
git push origin v0.2.0
```

Em seguida, o GitHub Actions compila para todas as plataformas e publica os arquivos e instaladores como uma GitHub Release. `dist plan` mostra uma prévia do que será compilado; depois de alterar `dist-workspace.toml`, execute `dist generate` para atualizar o workflow.

## Licença

MIT, veja [LICENSE](../../LICENSE).
