# stealthlingo

[English](../../README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Español](README.es.md) | **Português (Brasil)** | [Русский](README.ru.md) | [Tiếng Việt](README.vi.md)

Uma ferramenta de aprendizado de idiomas feita para o terminal, para estudar com discrição.

O StealthLingo é uma pequena CLI em Rust para praticar vocabulário nos minutos livres: pesquise uma palavra, salve-a e pratique com flashcards, ortografia, letras faltando, ditado por áudio ou uma mistura de tudo, rumo a uma meta diária de palavras. Esc esconde a tela inteira de uma vez. Os dados do dicionário vêm do [Wiktionary](https://en.wiktionary.org/) em inglês pela API oficial da Wikimedia, sem precisar de chave; o agendamento das revisões, a correção das respostas e o seu progresso ficam em um banco de dados SQLite local, então as palavras salvas podem ser revisadas offline. Um agente de IA também pode adicionar as palavras que você talvez não conheça enquanto você lê ou programa (veja [Agentes de IA](#agentes-de-ia-mcp)).

Na v0.1, só é possível estudar inglês.

![Tela inicial do StealthLingo: meta de hoje, palavras pendentes e menu de prática](../images/home.png)

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

Em um terminal, `stealthlingo`, `study` e `review` abrem uma interface em tela cheia. A tela inicial mostra o que está pendente hoje, o seu progresso na meta diária e um menu (`r` revisar, `s` flashcards, `p` ortografia, `m` letras faltando, `l` audição, `x` prática mista, `e` refazer erros, `/` pesquisar, `w` lista de palavras, `t` estatísticas, `c` configurações, `q` sair); `↑`/`↓` (ou `k`/`j`) e `Enter` também funcionam. Listas e páginas longas rolam com as mesmas teclas.

![Esc esconde o app e mostra o terminal; outro Esc traz o app de volta](../images/boss-key.webp)

- **Esc esconde tudo na hora** e mostra o terminal de onde você abriu o programa; pressione Esc de novo para voltar. O cronômetro da sessão fica pausado enquanto a interface está escondida.
- Os **flashcards** não precisam de Enter: `Space` revela a resposta, `1`–`4` avaliam, `a` toca a palavra, `s` pula e `q` encerra a sessão.
- **Sotaques**: `a` toca primeiro o seu sotaque padrão (a configuração `accent`); pressionar de novo na mesma palavra toca o outro sotaque gravado, e assim por diante. O rodapé mostra qual sotaque vem a seguir, e o cabeçalho, qual está tocando.
- **Ortografia e audição**: digite a palavra e pressione `Enter`. `Tab` mostra a resposta e `Ctrl+N` pula. Na prática de audição, `Enter` em uma linha vazia repete a gravação e `Ctrl+R` toca o outro sotaque. Uma resposta errada mostra quais letras sobraram ou faltaram. As perguntas de letras faltando mostram a palavra com algumas letras ocultas (`e _ h e _ e r _ l`); digite a palavra inteira.
- **Lista de palavras**: `/` filtra por texto, `s` alterna entre os status, `p` entre as classes gramaticais e `u` mostra só as palavras pendentes. `Enter` abre a entrada salva (offline), `a` toca a palavra, `n` edita a nota e `d` seguido de `y` remove a palavra (salvá-la de novo restaura o progresso). `PgUp`/`PgDn` e `g`/`G` navegam por listas longas. As palavras adicionadas por um agente de IA aparecem marcadas com `agent`, e ao selecioná-las os detalhes abaixo mostram o motivo dado pelo agente.
- **Configurações** (`c`): `←`/`→` alteram o valor selecionado, `PgUp`/`PgDn` de 10 em 10, `Enter` permite digitar um número, `r` restaura o padrão e `R` restaura todas as configurações. Cada alteração é salva na hora; valores fora do intervalo permitido não são salvos.
- **Pesquisar**: digite uma palavra e pressione `Enter`; depois, `s` salva ou remove a palavra, `a` toca a palavra, `Tab` alterna entre a entrada resumida e a completa, `/` inicia uma nova pesquisa e `q` volta.
- `Ctrl+C` encerra a sessão atual, cancela um filtro ou uma nota e, nos outros casos, fecha o programa. Cada resposta é salva assim que é dada.
- A interface precisa de um terminal com pelo menos 40×12 caracteres. Em janelas baixas, os quadros encolhem, o resumo da tela inicial cabe em uma linha e o menu se divide em duas colunas (janelas largas) ou rola, para que o item selecionado e o campo de resposta continuem visíveis.

![Um flashcard com o significado revelado, aguardando a avaliação](../images/flashcard.png)

![Uma resposta de ortografia errada: a letra que faltou aparece marcada na resposta](../images/spelling.png)

![Pesquisa de uma palavra: pronúncias britânica e americana e as principais definições](../images/lookup.png)

`--plain` (ou qualquer entrada/saída que não seja um terminal, como um pipe) usa os prompts linha a linha descritos abaixo. `NO_COLOR` desativa as cores.

## Comandos

| Comando | O que faz |
|---|---|
| `stealthlingo` | Abre o app em tela cheia. Com `--plain`: revisa as palavras pendentes; se não houver nenhuma, estuda palavras novas; com a lista vazia, explica como começar |
| `lookup <WORD> [--all]` | Consulta o Wiktionary (atualiza o cache; sem conexão, usa a cópia em cache) |
| `add <WORD> [--note TEXT]` | Salva uma palavra. Salvar duas vezes não causa problema; `--note` atualiza a nota |
| `remove <WORD>` | Remove uma palavra da sua lista (os dados do dicionário em cache e o histórico são mantidos) |
| `words [--status S] [--pos P] [--due]` | Lista as palavras salvas com status, data da próxima revisão e quem as adicionou, com filtros opcionais |
| `search <QUERY> [--status S] [--pos P] [--due]` | Pesquisa nas palavras salvas e nas notas |
| `study [--mode memory\|spelling\|letters\|listening\|mixed] [--mistakes] [--minutes N] [--count N]` | Sessão de estudo |
| `review [--count N] [--minutes N]` | Revisão com flashcards apenas das palavras pendentes |
| `stats` | Palavras salvas por status, pendentes, meta diária e últimos 7 dias, respostas de hoje e de todo o período com a taxa de acerto, próxima revisão |
| `audio <WORD> [--accent uk\|us]` | Toca a pronúncia (padrão: a configuração `accent`) |
| `config` / `config set <KEY> <VALUE>` / `config reset [KEY]` | Mostra, altera ou restaura as configurações; mostra os locais dos dados |
| `links [status\|install\|uninstall]` | Ctrl+clique no `lookup` para tocar pronúncias e adicionar ou remover palavras (Windows) |
| `export <PATH>` / `import <PATH>` | `.json` backup completo, ou `.csv` lista de palavras |
| `mcp` | Atende agentes de IA via MCP em stdin/stdout; é iniciado pelo cliente do agente (veja Agentes de IA abaixo) |

Sem `--minutes` ou `--count`, uma sessão dura `default_minutes` (5). Sessões iniciadas pela tela inicial, ou com `stealthlingo --plain`, duram até a meta diária ser atingida (veja [Meta diária](#meta-diária)).

Os filtros da lista de palavras podem ser combinados: `--status` é `new`, `learning`, `review` ou `mastered`; `--pos` corresponde ao início de uma classe gramatical (`adj`, `n`, `verb`); `--due` mantém só as palavras cuja revisão já chegou.

```bash
stealthlingo words --status learning --pos adj
stealthlingo words --due
```

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
- **letters (letras faltando)**: como ortografia, mas cerca de metade das letras aparece (`e _ h e _ e r _ l`). As lacunas são sempre as mesmas para cada palavra. As respostas contam como prática de ortografia.
- **listening (audição)**: ouça a pronúncia e digite a palavra. `r` repete. Só são usadas palavras com áudio de pronúncia.
- **mixed (misto)**: palavras nunca estudadas começam como flashcards; as outras se revezam entre flashcards, ortografia e (quando há gravação) audição.

`--mistakes` pratica as palavras que você errou nos últimos 30 dias, ou que esqueceu depois de aprender, começando pelos erros mais recentes, estejam pendentes ou não. Funciona com qualquer modo; na tela inicial, `e` usa a prática mista.

Em todos os modos, `s` pula a palavra atual sem registrar resposta e `q` encerra a sessão. Ctrl+C também encerra a sessão; um segundo Ctrl+C sai imediatamente. Cada resposta enviada é salva na hora, então sair nunca faz perder o que já foi feito, e uma pergunta não respondida nunca é registrada como errada.

A correção ignora maiúsculas e minúsculas e espaços no início e no fim, mas apóstrofos e hifens contam (`well-being` não é `wellbeing`).

### Agendamento

Um algoritmo SM-2 simplificado decide quando uma palavra volta:

- **Again**: volta em 10 minutos; o progresso é reiniciado.
- **Hard**: intervalo mais curto que o normal.
- **Good**: 1 dia, depois 3 dias, e então o intervalo cresce pelo fator de facilidade (ease factor) da palavra.
- **Easy**: intervalo mais longo, e a palavra passa a ser considerada mais fácil daí em diante.

Em ortografia e audição, respostas certas contam como Good e erradas como Again. Uma palavra errada volta mais uma vez no fim da mesma sessão. Revisões pendentes sempre vêm antes de palavras novas, e no máximo `daily_new_limit` (padrão 10) palavras nunca estudadas são introduzidas por dia. Palavras com intervalo de 21 dias ou mais aparecem como `mastered` (dominadas).

### Meta diária

`daily_goal` (padrão 10) é o número de palavras diferentes a praticar por dia; responder de novo a mesma palavra não conta duas vezes. A tela inicial, a página de estatísticas e `stats` mostram o progresso de hoje, e `stats` também mostra os últimos 7 dias. Até a meta ser atingida, as sessões iniciadas pela tela inicial (ou com `stealthlingo --plain`) terminam quando ela é atingida, em vez de depois de `default_minutes`; se as palavras acabarem antes, a sessão informa quantas faltaram. Um `--minutes` ou `--count` explícito sempre tem prioridade. `0` desativa a meta.

## Configuração

Altere as configurações na tela de configurações (`c` na tela inicial) ou com `config`:

```bash
stealthlingo config set daily_goal 20
stealthlingo config set daily_new_limit 15
stealthlingo config set default_minutes 3
stealthlingo config set accent us        # tocar primeiro as gravações americanas (padrão: uk)
stealthlingo config reset daily_goal     # volta ao padrão
stealthlingo config reset                # todas as configurações, após confirmar (--yes pula a confirmação)
```

| Chave | Padrão | Intervalo | Significado |
|---|---|---|---|
| `daily_goal` | 10 | 0–500, 0 = desativado | Palavras diferentes a praticar por dia |
| `daily_new_limit` | 10 | 0–200, 0 = só revisão | Palavras nunca estudadas introduzidas por dia |
| `default_minutes` | 5 | 1–120 | Duração de uma sessão sem `--minutes` ou `--count` |
| `accent` | uk | uk / us | Sotaque tocado primeiro |
| `mcp_daily_add_limit` | 30 | 0–200, 0 = desativado | Palavras que agentes de IA podem adicionar por dia via MCP |
| `http_timeout_secs` | 10 | 1–120 | Tempo limite de rede |

`config` lista cada valor ao lado do padrão e do intervalo. Se o `config.json` tiver um valor fora do intervalo, o StealthLingo avisa e usa o valor permitido mais próximo, sem reescrever o arquivo. Palavras gravadas em um só sotaque sempre tocam essa gravação.

### Dicionário

O StealthLingo usa a API oficial do Wiktionary, da Wikimedia: definições e exemplos vêm da API REST, e o IPA (identificado por sotaque), sinônimos, antônimos e gravações do Wikimedia Commons (OGG) vêm do código-fonte da página. Quando a forma digitada e o verbete do dicionário são diferentes (`Ephemeral` e `ephemeral`), a forma digitada é lembrada, de modo que `add` e `remove` se referem à mesma palavra salva.

Versões de desenvolvimento anteriores ao lançamento da v0.1.0 também podiam usar a Free Dictionary API ou o Merriam-Webster. Palavras armazenadas em cache a partir dessas fontes continuam legíveis offline e são substituídas por dados do Wiktionary na próxima vez que forem pesquisadas (o progresso de estudo e as notas são mantidos). As configurações antigas dessas fontes em `config.json` são ignoradas.

## Agentes de IA (MCP)

`stealthlingo mcp` executa um servidor [Model Context Protocol](https://modelcontextprotocol.io) em stdin/stdout, para que um agente de IA (Cursor, Claude Desktop ou outro cliente MCP) possa pesquisar palavras e adicionar à sua lista de estudo as que você talvez não conheça enquanto lê ou programa. O StealthLingo em si nunca chama um modelo de IA. Adicione-o à configuração MCP do seu cliente (no Cursor, `~/.cursor/mcp.json`, ou `.cursor/mcp.json` dentro de um projeto):

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

Se `stealthlingo` não estiver no seu `PATH`, use o caminho completo do executável em `command`. Depois peça ao agente, por exemplo, que adicione as palavras desta página que você talvez não conheça.

| Ferramenta | O que faz |
|---|---|
| `get_study_status` | Palavras salvas, pendentes e ainda não iniciadas, o progresso e a precisão de hoje em relação à meta, e quantas palavras os agentes ainda podem adicionar hoje |
| `list_words` | Palavras salvas com status, data de revisão, áudio, esquecimentos e uma definição curta; `query`, `status` e `limit` restringem a lista. As notas nunca são compartilhadas |
| `lookup_word` | A entrada do Wiktionary de uma palavra (cache primeiro), sem salvá-la |
| `add_words` | Salva até 20 palavras por chamada, cada uma com um `reason` opcional; informa o resultado palavra por palavra |

Os agentes só escolhem palavras:

- Definições, exemplos e pronúncias vêm apenas do Wiktionary. Um agente pode dizer onde a palavra apareceu (`reason`), mas não pode escrever definições, traduções nem as suas notas. Palavras que o Wiktionary não tem não são adicionadas.
- Não existe ferramenta para remover palavras, mudar configurações ou registrar respostas.
- Palavras que você removeu (com `remove`, `d` na lista de palavras ou um link) nunca são adicionadas de volta por um agente; se você mesmo salvar uma delas, ela volta a ficar disponível.
- Os agentes adicionam no máximo `mcp_daily_add_limit` (padrão 30) palavras por dia, e remover uma delas não devolve a vaga. `0` desativa a adição. As mudanças valem a partir da próxima chamada, sem reiniciar o cliente.
- Palavras adicionadas por um agente são estudadas como qualquer outra. A lista de palavras as marca com `agent` e mostra o motivo do agente; `words` mostra quem adicionou cada palavra na coluna `BY`. Os backups JSON guardam as duas informações e as palavras que você removeu.

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
- No `mcp`, `get_study_status` e `list_words` nunca acessam a rede; `lookup_word` e `add_words` só acessam a internet para palavras que não estão no cache.
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
