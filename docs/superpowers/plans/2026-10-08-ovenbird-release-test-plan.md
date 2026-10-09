# Plano completo de testes para lançamento do Ovenbird

- **Data do plano:** 08/10/2026
- **Versão analisada:** 0.1.0
- **ID do aplicativo:** `io.github.diegopn.ovenbird`

## Objetivo

Validar as funções do Ovenbird, a preservação dos documentos e da biblioteca local, a instalação Flatpak e os fluxos de erro antes de publicar uma versão. Este roteiro cobre testes automatizados e testes manuais usando dados fictícios.

O Ovenbird é um aplicativo GNOME em Rust/GTK para editar projetos LaTeX e gerenciar uma biblioteca BibTeX local. Os projetos continuam sendo arquivos comuns no disco; a biblioteca local usa `library.bib`, com catálogos JSON para autores, perfis e tags. **A sincronização com Zotero está fora do escopo desta versão**: confirme que não restaram telas, credenciais ou ações de sincronização.

## Regras para executar os testes

- Use uma conta de sistema de teste ou um perfil de dados descartável. Não use sua biblioteca pessoal como massa de teste.
- Antes de testar instalação, atualização ou recuperação, faça uma cópia de segurança dos arquivos de teste. Nunca use `flatpak uninstall --delete-data` no perfil pessoal.
- Para cada caso, marque `[x]` somente depois de conferir o resultado esperado. Anote versão, sistema, arquitetura e método de instalação junto ao resultado.
- Registre falhas com: ID do caso, passos para reproduzir, resultado esperado, resultado observado, frequência, captura de tela/log sem dados pessoais e prioridade.
- Se um teste mostrar perda ou sobrescrita silenciosa de dados, pare os testes destrutivos e trate como bloqueador de lançamento.

### Prioridade de defeitos

| Prioridade | Significado | Exemplos |
|---|---|---|
| **P0 — bloqueador** | Risco de perda de dados, falha de segurança ou impossibilidade de iniciar/usar a função central. | Documento sobrescrito, biblioteca apagada, acesso fora do escopo do usuário, aplicativo não inicia. |
| **P1 — alta** | Fluxo central falha ou entrega resultado incorreto sem alternativa razoável. | Não salva/compila, importação corrompe referências, editor visual perde LaTeX desconhecido. |
| **P2 — normal** | Problema contornável que não destrói dados. | Ordenação ou filtro incorreto, mensagem de erro ruim, detalhe visual relevante. |
| **P3 — baixa** | Polimento sem impacto funcional importante. | Pequena inconsistência visual ou tradução secundária. |

## Preparação da massa de teste

Prepare tudo em uma pasta descartável, por exemplo `Ovenbird QA`, sem usar documentos reais:

1. Um projeto LaTeX válido com `main.tex`, uma imagem dentro de `figures/`, um PDF de exemplo, uma subpasta, um arquivo `.bib` do projeto e um arquivo de texto não suportado pelo editor.
2. Um `.tex` com título, seções, acentos, caracteres Unicode, negrito/itálico, lista, equação, tabela, link, imagem e citações existentes e inexistentes.
3. Um `.tex` deliberadamente inválido para conferir o diagnóstico de compilação.
4. Um `.bib` com pelo menos um exemplar de cada tipo oferecido: `article`, `book`, `incollection`, `inproceedings`, `phdthesis`, `mastersthesis`, `techreport`, `online` e `misc`. Inclua autores com acentos e sobrenomes compostos, tags, `keywords`, DOI, URL, campos adicionais desconhecidos, `@string` e comentários.
5. Uma biblioteca de 300 referências fictícias, com autores, anos e tags variados, para medir a experiência normal. Opcionalmente prepare 1.000–5.000 entradas para teste de estresse.
6. Uma imagem com o mesmo nome de outra já existente no projeto, para testar cópia sem sobrescrita.

## 0. Portão automatizado e metadados

Execute os comandos em uma cópia de trabalho limpa ou no diretório de build já configurado. Se ainda não houver `build/`, configure com `meson setup build`; não apague um diretório de build existente para contornar erros.

- [ ] **AUTO-01 — Formatação Rust:** `cargo fmt --all -- --check`. Esperado: termina sem diferenças.
- [ ] **AUTO-02 — Clippy:** `cargo clippy --locked --all-targets -- -D warnings`. Esperado: termina sem warnings ou erros.
- [ ] **AUTO-03 — Compilação Meson:** `meson compile -C build`. Esperado: binário e recursos instaláveis compilam sem erro.
- [ ] **AUTO-04 — Testes existentes:** `meson test -C build --print-errorlogs`. Esperado: todos os testes passam e não há teste ignorado inesperadamente.
- [ ] **AUTO-05 — AppStream:** se `appstreamcli` estiver instalado, execute `appstreamcli validate data/io.github.diegopn.ovenbird.metainfo.xml`. Esperado: metadados válidos e versão/data coerentes com o pacote.
- [ ] **AUTO-06 — Desktop entry:** se `desktop-file-validate` estiver instalado, valide `data/io.github.diegopn.ovenbird.desktop`. Esperado: nenhuma chave inválida; nome, ícone, comando e tipos de arquivo apontam para o ID `io.github.diegopn.ovenbird`.

## 1. Instalação e ciclo de vida do aplicativo

- [ ] **APP-01 — Primeiro início:** instalar em perfil de teste sem dados anteriores e iniciar pelo menu de aplicativos. Esperado: uma janela abre, sem erro no terminal e sem exigir configuração manual.
- [ ] **APP-02 — Abrir arquivo pelo sistema:** abrir um `.tex` pelo gerenciador de arquivos e, em seguida, um `.bib`. Esperado: o aplicativo existente recebe o arquivo correto ou abre uma janela; não cria janelas duplicadas nem perde o arquivo pedido.
- [ ] **APP-03 — Ativação repetida:** iniciar o aplicativo duas vezes pelo menu. Esperado: a janela existente é apresentada, sem processo/janela duplicados.
- [ ] **APP-04 — Fechar com alterações:** alterar um documento e tentar fechar projeto, abrir outro ou sair. Testar Salvar, Descartar e Cancelar. Esperado: cada escolha tem o efeito correto e Cancelar preserva a sessão atual.
- [ ] **APP-05 — Reabrir após saída normal:** salvar e fechar. Esperado: ao reabrir, o app inicia limpo, sem modal travado e sem estado de compilação enganoso.
- [ ] **APP-06 — Atalhos:** conferir os atalhos exibidos na tela de atalhos e testar salvar, localizar, compilar, inserir citação, alternar lateral, desfazer/refazer, abrir/fechar e sair. Esperado: cada atalho executa a ação indicada sem conflito ou ação inesperada.
- [ ] **APP-07 — Painel lateral:** recolher/reabrir com F9 e alternar Editor, Referências, Tags e Autores. Esperado: o conteúdo e os controles permanecem acessíveis, inclusive após redimensionar a janela.

## 2. Projetos, templates e arquivos

- [ ] **PROJ-01 — Projeto em branco:** criar projeto em branco em uma pasta de teste. Esperado: cria `main.tex`, abre o documento e mostra a árvore correta.
- [ ] **PROJ-02 — Templates:** criar projetos ABNT/abnTeX2, IEEE/IEEEtran, ACM/acmart, Elsevier/elsarticle e Springer Nature. Esperado: cada template cria os arquivos esperados, abre um `.tex` principal e mantém seus avisos/licenças.
- [ ] **PROJ-03 — Nome e destino:** criar com espaços, acentos e caracteres Unicode. Tentar nome vazio, `../`, separadores de caminho e pasta já existente. Esperado: nomes válidos funcionam; nomes inseguros/inválidos são recusados sem criar arquivos parciais ou sobrescrever a pasta existente.
- [ ] **PROJ-04 — Cancelar criação:** cancelar na escolha de template, pasta e nome. Esperado: nenhum projeto incompleto fica no disco.
- [ ] **PROJ-05 — Seleção do documento principal:** abrir pasta com zero, um e vários `.tex`. Esperado: o caso sem `.tex` apresenta explicação; o caso com vários permite selecionar o principal; o arquivo certo é usado na compilação.
- [ ] **PROJ-06 — Abrir formatos:** abrir `.tex`, `.bib`, `.txt`, `.bst`, `.cls` e `.sty`; visualizar PDF e imagens PNG/JPG/SVG/WebP suportadas; clicar em recurso desconhecido. Esperado: cada tipo abre no editor, visualizador ou aplicativo externo apropriado; formato desconhecido não é tratado como LaTeX.
- [ ] **PROJ-07 — Criar itens:** criar documento `.tex`, bibliografia `.bib`, arquivo `.sty` e pasta. Esperado: nomes são validados, itens aparecem na árvore e arquivos recém-criados podem ser abertos/salvos.
- [ ] **PROJ-08 — Renomear e mover:** renomear arquivo e pasta; mover arquivo entre pastas e arrastar para outra pasta. Esperado: a árvore atualiza, abas/arquivo aberto continuam apontando ao caminho correto e a operação não sai da raiz do projeto.
- [ ] **PROJ-09 — Movimentos inválidos:** tentar mover uma pasta para dentro dela mesma, para fora do projeto ou sobre destino existente. Esperado: operação recusada com estado original intacto.
- [ ] **PROJ-10 — Lixeira:** excluir arquivo e pasta; cancelar confirmação e confirmar exclusão. Tentar excluir pasta com documento aberto. Esperado: cancelamento não altera nada, confirmação envia à Lixeira conforme a plataforma e arquivos abertos não ficam silenciosamente órfãos.
- [ ] **PROJ-11 — Arquivos ocultos e links simbólicos:** colocar arquivos ocultos e links simbólicos na árvore. Esperado: o comportamento é consistente com a política atual; links não permitem escapar da raiz nem criar ciclos.
- [ ] **PROJ-12 — Caminhos difíceis:** abrir projeto com espaço, acentos e caminhos longos. Esperado: árvore, edição, imagens, salvamento e compilação usam os caminhos corretamente.

## 3. Editor Code/Visual e preservação do LaTeX

- [ ] **EDIT-01 — Edição Code:** escrever, selecionar, substituir, colar texto Unicode, navegar por linha e salvar. Esperado: o arquivo salvo contém exatamente o conteúdo esperado.
- [ ] **EDIT-02 — Estado alterado:** editar um arquivo e observar título/indicador de alteração; salvar e conferir que o indicador limpa. Esperado: o estado acompanha as alterações reais.
- [ ] **EDIT-03 — Desfazer/refazer:** testar digitação contínua, uma ação de toolbar e várias ações. Esperado: histórico agrupa digitação de forma razoável e restaura texto sem duplicar/perder caracteres.
- [ ] **EDIT-04 — Localizar:** buscar texto normal, acentuado e inexistente; avançar/retroceder entre ocorrências e permitir que a busca dê a volta no documento. Esperado: ocorrências e contador correspondem ao texto; fechar a busca devolve foco/visibilidade corretamente.
- [ ] **EDIT-05 — Indentação:** aumentar/diminuir indentação com cursor em uma linha, seleção de várias linhas e linhas vazias. Esperado: somente o intervalo pretendido muda.
- [ ] **EDIT-06 — Alternar modos:** no `.tex`, alternar Code/Visual repetidamente, editar em ambos, desfazer/refazer e salvar. Esperado: conteúdo equivalente permanece sincronizado.
- [ ] **EDIT-07 — LaTeX não suportado:** usar macros, ambientes e comentários desconhecidos no documento e alternar Visual/Code. Esperado: texto desconhecido, preâmbulo e final do documento não desaparecem nem são reescritos silenciosamente.
- [ ] **EDIT-08 — Modo indisponível:** abrir `.bib`, `.txt`, `.cls` ou `.sty` e tentar Visual. Esperado: app impede a troca ou explica a limitação; os dados continuam acessíveis em Code.
- [ ] **EDIT-09 — Formatação:** aplicar normal, seção, subseção, subsubseção, parágrafo, subparágrafo, negrito, itálico, sublinhado e monoespaçado a texto selecionado e sem seleção. Esperado: comandos LaTeX corretos; seleção/cursor permanecem utilizáveis.
- [ ] **EDIT-10 — Inserções de toolbar:** inserir link, matemática inline/display, símbolos gregos/matemáticos, listas com marcadores/numeradas, citação em bloco, comentário, nota de rodapé, nota de revisão, label e referência cruzada. Esperado: snippet válido, cursor no ponto de edição e undo restaura a versão anterior.
- [ ] **EDIT-11 — Tabela:** inserir tabela com dimensões mínimas, usuais e limite permitido; tentar valores inválidos. Esperado: estrutura LaTeX válida, cursor em célula editável, limites respeitados e nenhum congelamento.
- [ ] **EDIT-12 — Imagem:** inserir imagem dentro e fora da pasta; repetir nome já existente e compilar. Esperado: imagem é copiada para o projeto, não sobrescreve arquivo homônimo, caminho relativo ao documento principal funciona e falhas de acesso são informadas.
- [ ] **EDIT-13 — Codificação:** abrir UTF-8 e Latin-1, salvar sem alterações e com acentos. Para Latin-1, inserir caractere fora do conjunto e salvar. Esperado: bytes originais são preservados quando possível; falha de conversão não destrói o conteúdo salvo anteriormente.
- [ ] **EDIT-14 — Salvamento externo:** abrir arquivo, alterar também por editor externo e salvar no Ovenbird. Esperado: conflito não causa sobrescrita silenciosa; o comportamento é compreensível e seguro.

## 4. Compilação e visualização

- [ ] **BUILD-01 — Compilação válida:** compilar o projeto de exemplo com o mecanismo disponível. Esperado: status de sucesso, PDF correto no preview e saída/intermediários fora da pasta do projeto, exceto arquivos de origem necessários.
- [ ] **BUILD-02 — Documento alterado:** compilar com alterações não salvas. Esperado: o app salva antes de compilar ou avisa para salvar; o PDF corresponde à versão mais recente.
- [ ] **BUILD-03 — Mecanismos:** testar `latexmk`, `pdflatex` e Tectonic, quando instalados; simular ausência de mecanismos em ambiente de teste. Esperado: escolha/fallback documentados e mensagem clara quando nenhum estiver disponível.
- [ ] **BUILD-04 — Tectonic Flatpak:** em instalação limpa, testar a primeira compilação com rede disponível e a recompilação offline. Esperado: dependências iniciais são baixadas quando necessário, o uso offline posterior é entendido e a interface não congela durante a compilação.
- [ ] **BUILD-05 — Erro de LaTeX:** compilar documento inválido. Esperado: status de erro, mensagem útil e linha de origem correta quando disponível; PDF antigo não aparece como se fosse o novo resultado.
- [ ] **BUILD-06 — Citação ausente:** usar citação sem entrada em nenhuma bibliografia configurada. Esperado: erro lista cada chave/linha; adicionar a entrada elimina o diagnóstico.
- [ ] **BUILD-07 — Bibliografias do projeto:** usar `\addbibresource` e `\bibliography`, caminhos relativos e múltiplos `.bib`. Esperado: entradas do projeto e da biblioteca local são combinadas sem modificar os `.bib` originais.
- [ ] **BUILD-08 — Imagem ausente:** compilar com caminho de imagem inválido. Esperado: erro explica o problema e não mascara erros anteriores.
- [ ] **BUILD-09 — Exportar PDF:** exportar para caminho novo, caminho com espaço e arquivo existente; cancelar o seletor e testar destino sem permissão. Esperado: PDF exportado é válido; cancelamento não cria arquivo; falhas não apagam um PDF anterior.
- [ ] **BUILD-10 — Preview PDF:** testar abrir, zoom +/−, página anterior/próxima, número de página e PDF com muitas páginas. Esperado: controles sincronizam, limites são respeitados e troca/fechamento do projeto não deixa preview obsoleto.
- [ ] **BUILD-11 — Poppler opcional:** executar uma build com preview embutido e, se suportado pelo build nativo, outra sem Poppler-GLib. Esperado: compilação/edição continua disponível e limitação de preview é clara.

## 5. Biblioteca de referências e BibTeX

- [ ] **REF-01 — Cadastro por tipo:** criar uma referência para cada tipo listado na preparação. Conferir campos próprios, autores/tags, salvar, fechar e reabrir. Esperado: tipo/campos persistem e nenhum campo desconhecido é descartado.
- [ ] **REF-02 — Validação do cadastro:** testar título/chave vazios, chaves repetidas, valores longos, acentos, chaves BibTeX especiais e Cancelar. Esperado: erros são claros, chave existente não é sobrescrita e Cancelar não salva mudanças.
- [ ] **REF-03 — Edição e exclusão:** alterar cada tipo de campo, autor, ano/data e tag; testar excluir e cancelar confirmação. Esperado: apenas a referência selecionada muda ou é removida.
- [ ] **REF-04 — Detalhes:** abrir detalhes e conferir campos longos, DOI/URL, tags e citação formatada. Esperado: copiar continua disponível para os campos comuns e para a citação formatada; a linha de tags é chip estático, sem copiar/remover.
- [ ] **REF-05 — Formatos de citação:** validar ABNT, MLA, AMS, APA 7, Chicago, Harvard, Vancouver e IEEE com entradas de artigo, livro, conferência, tese e site. Esperado: nomes/ordem dos autores, ano, título e dados bibliográficos aparecem sem conteúdo `BibTeX` cru.
- [ ] **REF-06 — Importar BibTeX válido:** importar arquivo com todos os tipos, acentos, `@string`, comentários, campos desconhecidos, tags, macros e chaves já existentes. Esperado: contagem correta, entrada existente não é corrompida e conteúdo suportado permanece.
- [ ] **REF-07 — Importar arquivo inválido:** testar arquivo vazio, truncado, texto que não é BibTeX e arquivo sem permissão. Esperado: erro visível; biblioteca anterior permanece intacta.
- [ ] **REF-08 — Exportar e reimportar:** exportar a biblioteca, abrir o arquivo `.bib`, importar numa biblioteca de teste vazia e comparar chaves/campos. Esperado: round-trip sem perda de referências, autores, macros e campos extras suportados.
- [ ] **REF-09 — Pesquisa e filtros:** buscar por título, chave, autor, ano, tag e tipo; combinar busca com filtros de autor/ano/tag/tipo e limpar cada filtro. Esperado: resultados corretos e estado vazio compreensível.
- [ ] **REF-10 — Ordenação:** ordenar por título, autor, ano e tag, em ordem crescente/decrescente. Esperado: ordem correta, incluindo entradas sem ano/tag e mais de uma tag.
- [ ] **REF-11 — Seletor de citação:** inserir citação LaTeX e referência bibliográfica formatada a partir da biblioteca; testar busca e fechar/cancelar. Esperado: comando/chave correto no `.tex`, estilo escolhido aplicado e nenhuma alteração na biblioteca.
- [ ] **REF-12 — Inserir em `.bib`:** usar o fluxo de inserir referência em arquivo BibTeX. Esperado: entrada válida adicionada uma vez e o documento `.bib` continua parseável.
- [ ] **REF-13 — Importação repetida:** importar o mesmo `.bib` duas vezes. Esperado: duplicatas são tratadas conforme a política do app, sem dobrar entradas silenciosamente nem perder a existente.

## 6. Autores, tags e listagens

- [ ] **META-01 — Autor novo:** cadastrar nome completo, conferir nome usado em referências gerado, editar esse nome e testar botão de geração. Esperado: sobrenomes compostos e partículas (`de`, `van`, `von`, `dos`) são preservados.
- [ ] **META-02 — Perfil do autor:** salvar ORCID, e-mail e instituição; abrir detalhes, copiar campos, editar e reabrir. Esperado: valores persistem e os vínculos às referências continuam corretos.
- [ ] **META-03 — Autor ligado a referência:** criar/editar referência com vários autores, reordenar, remover um autor e salvar. Esperado: nome não é removido ao clicar na linha, controles explícitos funcionam e a ordem da citação é mantida.
- [ ] **META-04 — Renomear autor:** alterar nome usado em referências de autor associado. Esperado: entradas BibTeX relacionadas são atualizadas sem alterar autores homônimos indevidos.
- [ ] **META-05 — Excluir autor associado:** testar Cancelar e confirmar exclusão com referências vinculadas. Esperado: a ação mostra impacto/confirmacão explícita e não remove referências por acidente.
- [ ] **META-06 — Busca/ordenação de autores:** buscar por nome completo, nome de citação e instituição; ordenar nome/instituição ascendente e descendente. Esperado: campos e direção correspondem à escolha.
- [ ] **META-07 — Tag nova e edição:** criar, renomear e excluir uma tag; associar/desassociar em referências. Esperado: referências não recebem tags duplicadas e os vínculos após renomear/excluir são consistentes.
- [ ] **META-08 — Tag duplicada:** adicionar nomes com caixa, espaços e separadores diferentes (vírgula/ponto e vírgula) em cadastro/importação. Esperado: itens equivalentes não aparecem duplicados indevidamente.
- [ ] **META-09 — Busca/ordenação de tags:** pesquisar e ordenar nome ascendente/descendente. Esperado: lista e direção corretas, ações editar/excluir não abrem detalhes de tag.
- [ ] **META-10 — Filtro de autor:** no filtro de referências, verificar nomes completos e busca visual com autores de nome composto. Esperado: autor selecionado corresponde ao perfil e filtra as referências corretas.

## 7. Persistência, recuperação e migração de dados

- [ ] **DATA-01 — Persistir biblioteca:** adicionar/editar referências, autores e tags; fechar normalmente e reabrir. Esperado: biblioteca, sidecars de autores/perfis/tags e relações ficam iguais.
- [ ] **DATA-02 — Persistir configurações:** selecionar tema automático/claro/escuro, fechar e reabrir. Esperado: preferência permanece; modo automático acompanha tema do sistema.
- [ ] **DATA-03 — Salvar depois de interrupção:** em perfil descartável, encerrar o processo após salvar e abrir novamente. Esperado: último estado salvo não se perde e a inicialização não cria dados duplicados.
- [ ] **DATA-04 — JSON/BibTeX danificado:** em cópia de teste, corromper cada arquivo de dados separadamente (`library.bib`, `authors.json`, `author_profiles.json`, `tags.json`, `settings.json`). Esperado: app apresenta erro recuperável, preserva o arquivo danificado para recuperação e não substitui silenciosamente com biblioteca vazia.
- [ ] **DATA-05 — Leitura/gravação negada:** tornar pasta de dados de teste somente leitura e salvar uma mudança. Esperado: erro claro; cópia persistida anterior continua íntegra.
- [ ] **DATA-06 — Caminhos XDG:** verificar nativo e Flatpak. Esperado: biblioteca em `$XDG_DATA_HOME/ovenbird/`, configurações em `$XDG_CONFIG_HOME/ovenbird/settings.json` e cache de compilação em `$XDG_CACHE_HOME/ovenbird/build/`; nenhum dado é salvo em local inesperado.
- [ ] **DATA-07 — Atualização normal:** instalar uma build, criar dados sintéticos, atualizar para a build candidata com o mesmo ID. Esperado: a atualização mantém a biblioteca e configurações.
- [ ] **DATA-08 — ID antigo do Flatpak:** se a instalação de teste anterior usava `org.ovenbird.Ovenbird`, testar separadamente a instalação nova `io.github.diegopn.ovenbird`. Esperado: confirmar e documentar a estratégia de migração/recuperação; o Flatpak isola os diretórios por ID, então não presumir que os dados antigos aparecem automaticamente. Não remover o ID/diretório antigo até confirmar a cópia/exportação.
- [ ] **DATA-09 — Backup/restauração:** exportar `library.bib` e guardar uma cópia dos sidecars; iniciar perfil limpo, importar e comparar. Esperado: método de backup funciona sem depender do diretório privado do Flatpak.

## 8. Desempenho, estabilidade e interface

- [ ] **PERF-01 — 300 referências:** abrir biblioteca de teste, rolar até o fim, buscar, filtrar, ordenar, abrir detalhes e editar. Esperado: nenhuma referência some, a janela permanece interativa e não há crescimento contínuo de memória.
- [ ] **PERF-02 — Estresse opcional:** repetir com 1.000–5.000 referências, autores e tags. Registrar tempo de abertura, busca, filtro, ordenação e pico aproximado de memória; não deve ocorrer crash ou congelamento permanente.
- [ ] **PERF-03 — Projeto grande:** abrir documento longo (ex.: dezenas de páginas de fonte) e árvore com centenas de arquivos. Esperado: navegar/editar/salvar sem travamento prolongado.
- [ ] **PERF-04 — Compilação demorada:** compilar projeto que leve tempo e tentar usar a janela/acompanhar status. Esperado: interface continua responsiva e os estados Building/Built/Error são inequívocos.
- [ ] **UI-01 — Tamanhos de janela:** testar janela pequena, maximizada e redimensionada; diálogos longos (edição de referência e autores) com rolagem. Esperado: botões não ficam ocultos e conteúdo acessível sem sobreposição.
- [ ] **UI-02 — Escala e tema:** testar escala 100%, 150–200%, tema claro/escuro e tema automático do GNOME. Esperado: controles, chips, seleções, textos e scrollbar mantêm legibilidade e alvo de clique.
- [ ] **UI-03 — Teclado:** percorrer telas e diálogos com Tab/Shift+Tab, Enter, Escape e setas. Esperado: foco visível, ações previsíveis e confirmação/cancelamento acessível.
- [ ] **UI-04 — Leitor de tela:** percorrer controles principais com Orca, conferindo rótulos de ícones, campos, listas, botões de excluir/copiar e mensagens de erro. Registrar controles sem nome acessível.
- [ ] **UI-05 — Traduções:** testar interface em inglês, português brasileiro e espanhol. Conferir menus, formulários, erros, vazio de listas, diálogos, pluralização e ausência de texto cortado.
- [ ] **UI-06 — Sem rede:** desligar rede e testar abrir projetos locais, editar, biblioteca, import/export de arquivos locais, templates e compilação com mecanismo local. Esperado: operações locais funcionam; dependência de rede é explicada quando necessária.

## 9. Flatpak, permissões, privacidade e distribuição

Use diretório de build temporário e o manifesto atual:

```sh
flatpak-builder --user --install --force-clean /tmp/ovenbird-flatpak-build io.github.diegopn.ovenbird.json
flatpak run io.github.diegopn.ovenbird
```

- [ ] **PKG-01 — Build Flatpak limpo:** criar e instalar a partir do manifesto atual, sem GNOME Builder/cache antigo. Esperado: ID `io.github.diegopn.ovenbird`, binário `ovenbird` e recursos coincidem; não aparece `org.ovenbird.Ovenbird`.
- [ ] **PKG-02 — Sandboxing e portal:** abrir/salvar projeto fora da pasta home usando os seletores do sistema. Esperado: acesso somente após seleção/autorização, sem exigir acesso amplo ao sistema de arquivos.
- [ ] **PKG-03 — MIME e launcher:** iniciar pelo launcher e abrir `.tex`/`.bib` pelo gerenciador de arquivos. Conferir nome, ícone, categoria, arquivo associado e janela sem terminal.
- [ ] **PKG-04 — Arquitetura suportada:** repetir instalação/build nas arquiteturas anunciadas para a distribuição (manifesto traz Tectonic x86_64 e aarch64). Esperado: módulos, binários e dependências existem para cada arquitetura divulgada.
- [ ] **PKG-05 — Rede e Zotero:** confirmar que tarefas locais não dependem de rede, que não há opção/credencial/endpoint de Zotero ativo e que bibliotecas não são enviadas para fora do dispositivo. Documentar que a permissão de rede existente é necessária para o fluxo de download de recursos do Tectonic, se aplicável.
- [ ] **PKG-06 — Licença e templates:** confirmar GPL-3.0-or-later no pacote, avisos/licenças dos arquivos de template e que a tela/README não atribui ao Ovenbird uma licença incompatível.
- [ ] **PKG-07 — Instalação/reinício/remoção:** reiniciar sessão e iniciar de novo; testar atualização mantendo dados. Em perfil descartável, conferir o comportamento de desinstalação comum e deixar `--delete-data` restrito ao teste isolado.

## 10. Critérios de aprovação para lançar

Marque o release como aprovado somente se todos estes pontos forem verdadeiros:

- [ ] AUTO-01 a AUTO-06 passaram, ou a ferramenta opcional ausente foi anotada sem esconder erro.
- [ ] Todos os casos P0 e P1 passaram; nenhum defeito P0/P1 está aberto.
- [ ] Criar, editar, salvar, reabrir, compilar e exportar PDF funciona no build nativo e no Flatpak suportado.
- [ ] `.bib` mantém referências/campos no ciclo importar → salvar → exportar → reimportar.
- [ ] Dados do usuário permanecem após reinício e atualização; a migração do ID antigo está resolvida ou claramente documentada antes da divulgação.
- [ ] Os fluxos de autores, tags, filtros, ordenação, citações e referências funcionam com 300 entradas fictícias.
- [ ] Inglês, português brasileiro e espanhol foram revisados; nenhum bloqueio de teclado, janela pequena, escala alta ou leitor de tela ficou sem decisão.
- [ ] Não há sincronização Zotero residual ou transferência inesperada de documentos/biblioteca.
- [ ] Página/manifesto/launcher exibem o ID, licença, versão, ícone e requisitos corretos.
- [ ] A biblioteca e os documentos de teste foram exportados/guardados, e o perfil descartável foi identificado para limpeza segura.

## Registro de execução

Preencha uma cópia desta tabela para cada build candidato:

| Campo | Valor |
|---|---|
| Versão / commit | |
| Data e hora | |
| Sistema / GNOME | |
| Arquitetura | |
| Instalação (nativa/Flatpak) | |
| Idioma / tema / escala | |
| Casos aprovados / total | |
| Defeitos P0 / P1 / P2 / P3 | |
| Decisão (aprovar/reprovar) | |
| Responsável / observações | |
