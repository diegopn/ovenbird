# Redução geral de memória do Ovenbird

## Intenção

Reduzir o pico e a memória privada retida pelo Ovenbird em fluxos reais de edição, busca, biblioteca e prévia PDF. A troca do alocador será avaliada como uma hipótese; o objetivo não é incluir uma dependência sem demonstrar ganho no app.

## Evidências atuais

- O editor mantém buffers de código e visual com o documento. Alterações no código copiam o texto, renderizam novamente o buffer visual e gravam um estado completo no histórico. O histórico retém até 200 estados.
- O editor também mantém uma cópia completa do texto salvo em `Runtime.saved_text` para calcular o estado de alterações pendentes.
- A busca transforma o documento inteiro em vários vetores e também guarda todas as posições encontradas antes de escolher uma.
- A lista da biblioteca clona cada referência filtrada para o modelo GTK, embora os registros originais já permaneçam na biblioteca local.
- O alocador global Rust cobre alocações da biblioteca padrão Rust. O caminho padrão da GLib usa `malloc` do sistema; o mimalloc oferece uma via separada para interpor `malloc`, cuja resolução de símbolos ainda precisa ser confirmada no binário e no Flatpak do Ovenbird.
- A prévia PDF já renderiza apenas páginas visíveis, mas mantém o documento Poppler enquanto a prévia está aberta.

## Objetivos e critérios

- Medir a versão atual e variantes do mimalloc com builds release e a mesma carga de trabalho.
- Usar PSS e memória privada de `/proc/<pid>/smaps_rollup` como métricas principais; registrar `VmHWM` como pico RSS e o pico amostrado como fallback.
- Fazer três execuções baseline por cenário; depois comparar cada variante do alocador pelo menos três vezes em todos os cenários e cinco vezes nos dois cenários de maior consumo e em repouso. Iniciar um processo novo em cada execução e comparar medianas, registrando máquina, versão, build e formato de distribuição.
- Considerar um modo de alocador candidato a padrão somente se reduzir PSS mediano em pelo menos 5% em dois ou mais cenários, sem regressão superior a 5% nos demais e sem regressões funcionais ou perceptíveis de resposta.
- Considerar a iniciativa bem-sucedida se as correções confirmadas pelo perfil reduzirem em pelo menos 10% o pico ou a PSS estabilizada do cenário de maior consumo, preservando busca, desfazer/refazer, modo visual e abertura de PDF.

## Fora de escopo

- Alterar o formato portátil dos documentos TeX ou o formato exportável BibTeX.
- Remover o modo visual, o histórico de desfazer/refazer ou a prévia PDF para reduzir memória.
- Tratar a memória de `pdflatex` como memória do processo Ovenbird; registrar o subprocesso separadamente.
- Habilitar mimalloc como padrão antes de medir os modos disponíveis.
- Alterar o alocador padrão dos builds nativos antes de uma medição própria desses builds; a primeira decisão de distribuição se limita ao Flatpak release.

## Abordagens consideradas

1. **Trocar somente o alocador global Rust.** É a opção mais simples, mas não cobre diretamente as alocações feitas por GTK, GLib, GtkSourceView, Cairo ou Poppler.
2. **Interpor o `malloc` do processo.** Pode alcançar as bibliotecas nativas em Linux, mas depende de resolução correta dos símbolos, empacotamento e compatibilidade com a combinação nativa distribuída.
3. **Sequência combinada, selecionada.** Registrar um baseline; comparar o alocador atual, o global Rust e a interposição do processo; otimizar os caminhos de texto confirmados pelos dados; manter mimalloc como padrão somente se atingir os critérios acima.

## Direção técnica proposta

- Gerar entradas determinísticas: `.tex` válido com 5.000 linhas de texto seguro e cerca de 2,5 MiB; bibliotecas BibTeX com 300 e 1.000 entradas, cada uma com os mesmos campos e resumo de 1 KiB; PDF de 100 páginas usando a mesma imagem 512×512 em cada página. Guardar os arquivos gerados fora do diretório-fonte e registrar os hashes.
- Medir repouso, edição e busca no `.tex`, pesquisa/filtros nas duas bibliotecas, prévia PDF com navegação/zoom e memória 30 segundos após fechar projeto/prévia. Usar a build release Flatpak como alvo principal.
- Usar diretórios temporários isolados para XDG data/config/cache em cada execução; importar a biblioteca e abrir o projeto de teste nesses diretórios, sem tocar na biblioteca ou nos documentos reais do usuário.
- Expor a variante de alocador por uma opção de build, sem mudar o padrão, e garantir que a dependência e seus transitivos estejam no cache Cargo offline usado pelo Meson.
- Se o perfil confirmar os candidatos: reduzir a memória temporária da busca para depender do tamanho da consulta; materializar o buffer visual ao entrar no modo visual e limpá-lo ao voltar ao modo código; guardar histórico como alterações incrementais em vez de até 200 cópias completas.
- Se o tamanho da cópia de texto salvo representar pelo menos 2% da PSS do cenário de edição, substituir a cópia por comprimento e resumo SHA-256 usando a dependência `sha2` existente. Medir latência de digitação e preservar os casos de texto sujo, salvo e restaurado por undo; manter a cópia se a medição mostrar regressão de resposta.
- Se as referências clonadas representarem pelo menos 2% da PSS no cenário de biblioteca, guardar índices no modelo GTK e resolver os registros na biblioteca canônica. A ordem só é válida até a próxima atualização da biblioteca, que já deve reconstruir o modelo. Clonar a referência somente ao abrir detalhes, editar ou excluir.
- Manter o limite de 200 passos, coalescência de digitação em 700 ms e semântica de desfazer/refazer. Uma edição que substitua uma região muito grande ainda poderá gerar uma alteração histórica grande; o cenário será medido sem descartar estados antigos por um orçamento arbitrário.
- Preservar a semântica atual de conversão para minúsculas, offsets Unicode em caracteres, retorno circular da busca e preservação de comandos LaTeX desconhecidos.
- Manter o alocador atual como padrão nos builds nativos. Só selecionar um modo mimalloc no Flatpak release se ele cumprir os critérios no próprio pacote; qualquer mudança de padrão para builds nativos exige medição equivalente neles.
- Repetir a mesma matriz de medição após cada grupo de mudanças. Se a interposição não alcançar as bibliotecas nativas no Flatpak ou não atingir os critérios, removê-la do caminho de distribuição e manter apenas as otimizações justificadas.

## Restrições do projeto

- Manter `rust-version` e `rust-toolchain.toml` alinhados.
- O build de integração é Meson e usa Cargo em modo offline com `cargo-cache`.
- Preservar permissões mínimas do Flatpak e seleção de arquivos por portais.
- Não sobrescrever o diretório `build/` existente; usar diretórios de build separados para cada variante.
- Para mudanças Rust, seguir os comandos de formatação, Clippy, compilação Meson e testes Meson definidos no `AGENTS.md` do repositório.
- Gerar fixtures e dados da biblioteca dentro de diretórios temporários; não usar a biblioteca nem os documentos reais do usuário nas medições.

## Referências técnicas

- A documentação Rust descreve `#[global_allocator]` como o alocador das funções de alocação da biblioteca padrão; a distinção em relação a chamadas C nativas é uma inferência que precisa ser validada no processo real: <https://doc.rust-lang.org/std/alloc/index.html>.
- A documentação GLib afirma que `g_malloc()` usa a implementação de `malloc` do sistema desde GLib 2.46: <https://docs.gtk.org/glib/memory.html>.
- O wrapper Rust do mimalloc documenta o alocador global e o requisito de compilador C; a configuração de `override` será verificada na versão fixada e no Flatpak do projeto: <https://github.com/purpleprotocol/mimalloc_rust> e <https://docs.rs/crate/mimalloc/latest/features>.
