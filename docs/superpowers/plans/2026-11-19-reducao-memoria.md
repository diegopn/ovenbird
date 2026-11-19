# Plano de redução do uso de memória do Ovenbird

> **Execução:** faça as tarefas na ordem. Cada etapa termina com uma verificação e uma revisão curta antes da próxima.

**Goal:** Reduzir em pelo menos 10% a memória privada ou o pico do fluxo de maior consumo, preservando a edição, a busca, o histórico, o modo visual e a prévia PDF.

**Architecture:** Primeiro criar dados e medição reproduzíveis; depois comparar o alocador atual, o alocador global Rust e a interposição de `malloc`. Em seguida, reduzir alocações temporárias da busca e cópias retidas pelo editor. O alocador do Flatpak só muda se os dados confirmarem ganho.

**Tech Stack:** Rust 1.92, GTK 4, Libadwaita, GtkSourceView, Meson, Flatpak e Python 3 da biblioteca padrão.

**Spec:** `docs/superpowers/specs/2026-11-19-memory-reduction-design.md`

## Restrições globais

- Manter o campo `rust-version` de `Cargo.toml` alinhado com `rust-toolchain.toml`.
- Usar Meson como build de integração; ele compila Cargo offline com o cache versionado em `cargo-cache/`.
- Preservar permissões mínimas do Flatpak e o uso de portais para arquivos selecionados pelo usuário.
- Não sobrescrever `build/`; criar diretórios separados para cada variante release.
- Manter o alocador atual como padrão até os critérios de medição serem atingidos.
- Preservar documentos TeX como arquivos portáteis e a biblioteca como BibTeX exportável.
- Executar cada cenário em diretórios temporários separados para XDG data/config/cache; nunca importar fixtures para a biblioteca real do usuário nem editar seus documentos.

## Pontos de revisão

- Busca com acentos, caracteres suplementares, conversão de minúsculas que expande caracteres, consultas vazias e retorno circular; cobrir em `src/tests.rs` na Tarefa 3.
- Histórico após substituição grande, edição Unicode, coalescência de digitação e nova edição após desfazer; cobrir em `src/tests.rs` na Tarefa 4.
- Alternância entre código e visual com comandos LaTeX que o modo visual não representa, posição do cursor e desfazer/refazer; cobrir em `src/ui/editor.rs` na Tarefa 5.
- Estado dirty após editar, salvar uma versão anterior e desfazer até o conteúdo salvo; conferir resumo, comprimento e latência de digitação na Tarefa 6.
- Referências filtradas e ordenadas continuam corretas quando o modelo armazena índices; resolver seleções contra a biblioteca que originou o modelo na Tarefa 6.
- Interposição de `malloc` alcançando GLib/GTK dentro do Flatpak sem incompatibilidade entre alocação e liberação; verificar no smoke test da Tarefa 2 e novamente na Tarefa 7.
- PDF longo, zoom, troca de arquivo e memória do processo compilador separada da memória do Ovenbird; medir na Tarefa 1 e repetir na Tarefa 7.

---

## Tarefas

### Tarefa 1: Fixtures e medidor de memória

**Arquivos:**

- Criar: `scripts/generate-memory-fixtures.py`
- Criar: `scripts/sample-process-memory.py`
- Criar: `tests/test_memory_fixtures.py`
- Criar: `docs/performance/memory-baseline.md`
- Atualizar após a medição: `docs/validacao-completa-ovenbird.md` (item PER-014)

**Interfaces:**

- `generate-memory-fixtures.py --output DIR` gera `main-long.tex`, `library-300.bib`, `library-1000.bib`, `pdf-100-pages.pdf` e `SHA256SUMS`.
- `sample-process-memory.py PID --scenario NAME --interval-ms 250 --duration-sec N --output CSV` registra timestamp monotônico e RSS, PSS, memória privada limpa/suja e `VmHWM` em KiB; também registra o pico RSS amostrado como fallback.

- [ ] **Passo 1: Escrever testes para os fixtures.** Verificar 5.000 linhas no TeX, aproximadamente 2,5 MiB, 300 e 1.000 entradas BibTeX com os mesmos campos e resumos de 1 KiB, 100 páginas PDF com uma imagem RGB de 512×512 compartilhada, e hashes que correspondem aos arquivos gerados.
- [ ] **Passo 2: Rodar os testes e confirmar a falha.** Executar `python3 -m unittest discover -s tests -p 'test_memory_fixtures.py'`.
- [ ] **Passo 3: Implementar o gerador com Python padrão.** Escrever o PDF mínimo diretamente com objetos, fluxos e tabela `xref`, sem dependências Python externas; gravar fixtures no diretório solicitado, nunca no diretório-fonte por padrão.
- [ ] **Passo 4: Implementar o amostrador de `/proc`.** Ler `smaps_rollup` e `status`, falhar com mensagem clara quando o PID terminar ou `/proc` não estiver disponível, e fechar o CSV ao encerrar.
- [ ] **Passo 5: Rodar a suíte Python e validar o amostrador.** Repetir o comando de testes; executar o amostrador por 1 segundo sobre o próprio processo e confirmar cabeçalho e amostras numéricas no CSV.
- [ ] **Passo 6: Registrar o protocolo baseline.** Medir em builds release e processos novos: repouso após 60 s; abrir o TeX e fazer 20 edições separadas por 800 ms; buscar `a` e `citação`; importar e pesquisar/filtrar as bibliotecas temporárias; percorrer o PDF e usar zoom 100% e 300%; abrir outro arquivo para liberar a prévia; medir novamente após 30 s. Fazer três execuções por cenário com diretórios temporários XDG separados e anotar máquina, sistema, versão, formato do pacote e medianas.
- [ ] **Passo 7: Revisar e registrar o baseline.** Usar PSS e memória privada como métricas principais, `VmHWM` como pico RSS e máximo amostrado como fallback; separar o processo de compilação dos números do app e registrar a medição concluída em PER-014.

### Tarefa 2: Variantes do mimalloc para comparação

**Arquivos:**

- Modificar: `Cargo.toml`, `Cargo.lock`, `src/main.rs`, `src/meson.build`
- Criar: `meson_options.txt`
- Atualizar: fontes, arquivos e índice do cache Cargo em `cargo-cache/`
- Não alterar nesta etapa: padrão dos manifests Flatpak

**Interfaces:**

- Cargo: features mutuamente exclusivas `mimalloc-global` e `mimalloc-override`; ambas selecionam `mimalloc::MiMalloc`, e a segunda também habilita o recurso `override` da dependência.
- Meson: opção `memory-allocator` com valores `system`, `mimalloc-global` e `mimalloc-override`; padrão `system`.

- [ ] **Passo 1: Adicionar configuração de builds separados.** Criar a opção Meson e mapear seu valor para features Cargo, preservando a seleção existente de `poppler-preview` em builds e testes.
- [ ] **Passo 2: Adicionar a dependência opcional e o alocador condicional.** Fixar a versão em `Cargo.toml`/`Cargo.lock`, definir `#[global_allocator]` somente nas duas variantes mimalloc e emitir erro de compilação caso as duas features sejam ativadas juntas.
- [ ] **Passo 3: Atualizar o cache Cargo offline.** Incluir o pacote mimalloc e todos os transitivos requeridos pelo build; confirmar que a compilação funciona com `CARGO_NET_OFFLINE=true`.
- [ ] **Passo 4: Preparar três builds release isolados.** Configurar `build-memory-system`, `build-memory-rust` e `build-memory-override` com `meson setup DIR --buildtype=release -Dmemory-allocator=VALUE`; não reutilizar nem remover `build/`.
- [ ] **Passo 5: Compilar as três variantes.** Executar `meson compile -C DIR` para cada diretório e registrar tamanho do binário e falhas de ligação.
- [ ] **Passo 6: Confirmar a interposição no Flatpak experimental.** Criar uma cópia temporária do manifesto release ao lado do original, adicionar `-Dmemory-allocator=mimalloc-override` em `config-opts` do módulo Ovenbird e manter `path: ..`; não alterar o manifesto acompanhado pelo Git. Construir em `build-memory-flatpak` e executar com `flatpak-builder --run build-memory-flatpak MANIFESTO_TEMPORARIO env LD_DEBUG=bindings /app/bin/ovenbird`. Confirmar nos bindings que chamadas `malloc` de GLib/GTK resolvem para mimalloc. Se não resolverem, marcar a variante como não funcional para memória nativa; não inferir cobertura pelo uso de `#[global_allocator]`.
- [ ] **Passo 7: Comparar os alocadores.** Baseline: três repetições por cenário. Depois comparar cada variante pelo menos três vezes em todos os cenários para verificar regressões; fazer cinco repetições em repouso e nos dois cenários de maior consumo. Guardar CSVs e medianas em `docs/performance/memory-baseline.md`.
- [ ] **Passo 8: Revisar o resultado antes de escolher padrão.** Um modo só é candidato se reduzir PSS mediano em pelo menos 5% em dois cenários e não elevar PSS em mais de 5% nos demais, sem regressão observável em abertura, digitação, busca ou prévia.

### Tarefa 3: Busca com memória auxiliar limitada à consulta

**Gate:** aplicar se a busca for um dos dois cenários de maior pico/PSS ou se a busca isolada elevar em pelo menos 5% o pico/PSS do cenário de edição. Caso contrário, registrar a decisão e seguir para a próxima etapa.

**Arquivos:**

- Modificar: `src/search.rs`
- Modificar: `src/tests.rs`

**Interfaces:**

- Preservar `find_document_match(text: &str, query: &str, from_offset: usize, forward: bool) -> Option<MatchRange>` e `MatchRange { start, end }`.
- A implementação deve usar memória auxiliar proporcional ao tamanho da consulta, sem vetores que armazenem o documento inteiro ou todas as posições encontradas.

- [ ] **Passo 1: Escrever testes para as regras atuais.** Manter `search_uses_unicode_character_offsets_and_wraps`; acrescentar casos de busca para trás com retorno circular, consulta vazia, ausência de resultado, sobreposição e `İ` minúsculo expandido em `i` mais ponto combinante.
- [ ] **Passo 2: Rodar os testes e confirmar a falha de qualquer caso novo.** Executar `CARGO_HOME=build-memory-system/cargo-home CARGO_TARGET_DIR=build-memory-system/cargo-target CARGO_NET_OFFLINE=true cargo test --manifest-path Cargo.toml --locked --offline --lib search_`.
- [ ] **Passo 3: Implementar a busca incremental.** Aplicar KMP sobre os caracteres minúsculos da consulta; percorrer o texto uma vez, mapear a janela de caracteres transformados para offsets de caracteres originais e guardar somente o primeiro/último resultado necessário para direção e retorno circular.
- [ ] **Passo 4: Reexecutar os testes de busca.** Confirmar ranges em offsets Unicode do GtkTextBuffer e paridade de comportamento com a busca anterior.
- [ ] **Passo 5: Medir o pico da busca.** Repetir buscas `a` e `citação` no fixture de 2,5 MiB; comparar pico RSS e PSS com o baseline, além do tempo de resposta.

### Tarefa 4: Histórico por alterações reversíveis

**Gate:** aplicar se o cenário de edição prolongada mostrar crescimento atribuível ao histórico de pelo menos 5% da PSS estabilizada ou do pico. Estimar o limite inferior pelas cópias completas retidas e registrar a decisão.

**Arquivos:**

- Modificar: `src/history.rs`
- Modificar: `src/tests.rs`

**Interfaces:**

- Preservar `EditorHistory::new`, `record`, `undo`, `redo`, `can_undo` e `can_redo`.
- Guardar um estado-base, o estado corrente, metadados de cursor/modo e até 199 alterações reversíveis; manter até 200 estados totais e a janela de coalescência atual de 700 ms.
- Representar cada alteração por offset de byte UTF-8 válido, texto removido e texto inserido; nunca cortar uma string dentro de um caractere.

- [ ] **Passo 1: Escrever testes para desfazer/refazer por alteração.** Manter `history_coalesces_typing_but_keeps_separate_actions`; acrescentar edição Unicode, substituição de documento inteiro, truncamento do redo após novo ramo e limite de 200 estados.
- [ ] **Passo 2: Rodar os testes e confirmar as falhas.** Executar `CARGO_HOME=build-memory-system/cargo-home CARGO_TARGET_DIR=build-memory-system/cargo-target CARGO_NET_OFFLINE=true cargo test --manifest-path Cargo.toml --locked --offline --lib history_`.
- [ ] **Passo 3: Implementar diferenças entre estados.** Calcular prefixo e sufixo comuns apenas em limites UTF-8 e guardar apenas o segmento substituído e o novo segmento. Coalescer digitação recalculando a alteração em relação ao estado anterior ao grupo; depois de desfazer e gravar novo estado, eliminar o ramo de redo.
- [ ] **Passo 4: Implementar undo/redo sobre o estado corrente.** Aplicar o segmento inverso ou direto, restaurar cursor e modo, e avançar o estado-base quando o limite de 200 estados remover a alteração mais antiga.
- [ ] **Passo 5: Rodar os testes de histórico.** Confirmar texto, cursor, modo, coalescência, limite e ramo após undo/redo.
- [ ] **Passo 6: Medir edição prolongada.** Fazer 20 pausas de digitação separadas por 800 ms no fixture longo; comparar memória estabilizada e pico com o baseline, sem reduzir a profundidade máxima de undo.

### Tarefa 5: Materializar o buffer visual somente quando usado

**Gate:** aplicar se a diferença entre os cenários do mesmo documento em modo de código e modo visual for de pelo menos 2% da PSS mediana. Caso contrário, registrar a decisão e manter a implementação atual.

**Arquivos:**

- Modificar: `src/ui/editor/construction.rs`
- Modificar: `src/ui/editor/document.rs`
- Modificar: `src/ui/editor/history.rs`
- Modificar: `src/ui/editor.rs`

**Interfaces:**

- O texto do buffer de código continua sendo a fonte canônica.
- Ao entrar no modo visual, renderizar o documento atual antes de calcular posição de cursor. Ao sair, sincronizar texto/posição e então limpar o buffer visual. Undo/redo deve preencher o buffer somente quando o estado restaurado estiver em modo visual.

- [ ] **Passo 1: Escrever testes GTK para o ciclo dos buffers.** Acrescentar um teste que confirme buffer visual vazio após `load_text`, preenchido ao entrar no modo visual e vazio ao voltar ao modo de código; acrescentar um round trip com comando desconhecido e undo/redo.
- [ ] **Passo 2: Rodar os testes e confirmar as falhas.** Executar a suíte GTK pelo Meson em sessão gráfica.
- [ ] **Passo 3: Remover renderizações desnecessárias.** Não renderizar o buffer visual em `load_text` nem em cada alteração no modo de código; materializá-lo em `set_mode(Visual)` e limpá-lo após a saída do modo visual.
- [ ] **Passo 4: Ajustar restauração de histórico.** Em `restore_history`, renderizar somente se o modo restaurado for visual; caso contrário, limpar o buffer visual e posicionar o cursor do código.
- [ ] **Passo 5: Reexecutar os testes GTK.** Confirmar preservação de comandos desconhecidos, cursor, modo, dirty state e histórico na alternância.
- [ ] **Passo 6: Medir as duas situações.** Comparar documento aberto em modo de código e em modo visual, antes e depois das alterações, usando o mesmo fixture.

### Tarefa 6: Remover cópias redundantes confirmadas pelo perfil

**Gate:** executar cada subparte somente se os dados da Tarefa 1 estimarem que a cópia evitável representa pelo menos 2% da PSS mediana de seu cenário. Para o texto, comparar o tamanho UTF-8 da cópia salva com o cenário de edição; para a biblioteca, somar o tamanho UTF-8 dos campos e valores clonados como limite inferior da cópia do modelo. Registrar a estimativa e a decisão, inclusive quando o gate não for atingido.

**Arquivos:**

- Condicional: `src/ui/editor.rs`, `src/ui/editor/construction.rs`, `src/ui/editor/document.rs`, `src/ui/editor/history_helpers.rs`
- Condicional: `src/app/window/mod.rs`, `src/app/window/references/filters/matching.rs`, `src/app/window/references/library.rs`, `src/app/window/references/list/view.rs`

**Interfaces:**

- Se o texto salvo passar pelo gate, trocar `Runtime.saved_text: String` por comprimento em bytes e SHA-256 do conteúdo, calculados ao carregar/salvar. `update_dirty` calcula o resumo do texto atual; medir latência antes de manter a troca.
- Se a lista BibTeX passar pelo gate, trocar `ReferenceListItem::Entry(BibEntry)` por `Entry(usize)`, fazer `filtered_reference_entries` retornar índices ordenados e resolver o índice contra `State.library` para exibição e ações. O modelo deve ser reconstruído sempre que a biblioteca mudar; ações de detalhes/edição/exclusão clonam somente a referência selecionada.

- [ ] **Passo 1: Conferir o gate de memória.** Registrar tamanhos estimados das cópias evitáveis, PSS mediana e decisão para cada subparte em `docs/performance/memory-baseline.md`.
- [ ] **Passo 2: Escrever testes do estado dirty.** Cobrir edição, `mark_saved` enquanto o buffer contém uma versão mais nova, retorno ao conteúdo salvo por undo e callback sem notificações redundantes; adicionar em `src/ui/editor.rs`.
- [ ] **Passo 3: Implementar resumo do texto salvo, se aprovado pelo gate.** Usar a dependência `sha2` existente; comparar comprimento e resumo nos caminhos de carregamento, salvamento, alteração e restauração de histórico.
- [ ] **Passo 4: Escrever testes de índices da biblioteca.** Cobrir filtro, ordenação, ordem estável em empates, substituição do modelo após atualização da biblioteca e resolução de uma linha selecionada; adicionar aos testes de referência em `src/app/window/mod.rs`.
- [ ] **Passo 5: Implementar o modelo por índices, se aprovado pelo gate.** Ordenar e filtrar índices consultando os registros originais, armazenar somente cada índice no modelo GTK e atualizar binding/callbacks para resolver pela biblioteca corrente.
- [ ] **Passo 6: Rodar os testes afetados e medir.** Verificar semântica dirty, detalhes/edição/exclusão da referência e latência de digitação; comparar PSS e RSS com o baseline. Se houver regressão perceptível na digitação ou ações inválidas por índice obsoleto, manter a cópia original.

### Tarefa 7: Verificação final e decisão de distribuição

**Arquivos:**

- Atualizar: `docs/performance/memory-baseline.md`
- Condicional: `flatpak/io.github.diegopn.ovenbird.release.json`

- [ ] **Passo 1: Executar as verificações Rust exigidas pelo projeto.** Rodar `cargo fmt --all -- --check` e `CARGO_HOME=build-memory-system/cargo-home CARGO_TARGET_DIR=build-memory-system/cargo-target CARGO_NET_OFFLINE=true cargo clippy --locked --all-targets -- -D warnings`.
- [ ] **Passo 2: Compilar e testar cada configuração aceita.** Executar `meson compile -C DIR` e `meson test -C DIR --print-errorlogs` em builds separados, sem substituir `build/` existente.
- [ ] **Passo 3: Repetir a medição release Flatpak.** Comparar sistema e o melhor modo mimalloc em três processos novos para todos os cenários e em cinco processos para repouso e os dois cenários de maior consumo; confirmar também abertura, busca, desfazer/refazer, prévia PDF e fechamento do projeto.
- [ ] **Passo 4: Aplicar o critério de sucesso.** A melhoria total exige queda de pelo menos 10% no pico ou PSS estabilizada do cenário mais pesado. Ativar mimalloc no manifest release somente se o próprio Flatpak cumprir também o critério de alocador da Tarefa 2; caso contrário, manter `system`.
- [ ] **Passo 5: Registrar a decisão e os dados.** Incluir medianas, dispersão, tamanhos dos fixtures, ambiente, modo de alocador e limitações, incluindo qualquer modo que não tenha interposto `malloc` de bibliotecas nativas.
- [ ] **Passo 6: Revisar o diff final.** Confirmar que nenhum documento de usuário, permissão Flatpak, licença de terceiro ou catálogo de tradução foi alterado sem necessidade.
