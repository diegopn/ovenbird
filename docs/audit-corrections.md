# Auditoria geral e correções — Ovenbird

**Auditoria concluída:** 2026-10-08 17:07 -03:00

**Escopo:** módulos Rust, construção e compilação LaTeX, armazenamento local, gestão de projetos e modelos, integração GTK, cache offline, manifesto Flatpak, metadados AppStream, arquivos desktop, traduções e documentação.

## Correções feitas

### 2026-10-08 16:58 -03:00 — Citações e compilação

- Removida a inclusão temporária automática de referências da biblioteca local durante a compilação. A compilação passa a considerar as referências presentes no documento ou nos arquivos `.bib` configurados nele; a pessoa deve escolher explicitamente quando inserir uma referência.
- Removidos os tipos, as rotinas e as mensagens de interface que só existiam para essa inclusão automática.
- Adicionado um teste de regressão para garantir que uma chave existente apenas na biblioteca local não satisfaça a validação de citações do documento.

### 2026-10-08 16:58 -03:00 — Leitura de pastas

- A enumeração de arquivos do projeto e dos arquivos extraídos de modelos agora propaga erros de leitura. Antes, erros individuais eram descartados silenciosamente e podiam deixar arquivos fora da interface ou da cópia do projeto.

### 2026-10-08 16:59 -03:00 — Dependência opcional de Poppler

- Poppler agora é opcional na configuração nativa do Meson, em concordância com a documentação e com o código que ativa a prévia de PDF apenas quando a dependência está disponível.

### 2026-10-08 17:01 -03:00 — Identidade do aplicativo

- Substituído `org.ovenbird.Ovenbird` por `io.github.diegopn.ovenbird` no identificador da aplicação, no manifesto Flatpak, no desktop file, no ícone, no AppStream e nos comandos do README. O identificador agora corresponde ao repositório GitHub e evita anunciar um domínio que o mantenedor não usa.
- Adicionado o link `vcs-browser` ao AppStream.

### 2026-10-08 17:03 -03:00 — Modelo de traduções

- Removido do `ovenbird.pot` o campo `Plural-Forms` com valores de exemplo (`INTEGER`/`EXPRESSION`), que fazia `msgfmt --check` reportar erro fatal. O cabeçalho de plural continua definido nos catálogos de cada idioma.

### 2026-10-08 17:05 -03:00 — Cache de dependências offline

- Removidos 109 MiB de fontes de crates já descompactadas que não eram copiadas nem usadas pelo build, e 92 arquivos `.crate` de pacotes ausentes do `Cargo.lock`. O cache do projeto passou de aproximadamente 141 MiB para 26 MiB.
- Mantidos os cinco diretórios de crates `toml` necessários que não têm arquivo `.crate` no cache. Os registros `.cargo-checksum.json` correspondem aos checksums do `Cargo.lock`, e o script do Meson agora copia esses diretórios para o `CARGO_HOME` temporário.
- Incrementada a versão do cache para forçar uma cópia limpa em builds existentes. O `.gitignore` também evita que cópias redundantes de fontes de crates voltem a entrar no repositório.

### 2026-10-08 17:41 -03:00 — Reparo do cache offline do Cargo

- Restaurados os cinco arquivos `.crate` de `toml`, `toml_datetime`, `toml_edit`, `toml_parser` e `toml_writer`, que estavam ausentes apesar de serem exigidos pelo `Cargo.lock`.
- Os SHA-256 dos cinco arquivos foram conferidos com os checksums do `Cargo.lock`.
- Atualizada a versão do cache de `3` para `4`, para que o próximo build recopie os pacotes restaurados ao `CARGO_HOME` do Meson, que ainda estava marcado como versão `3`.

### 2026-10-08 17:44 -03:00 — Erros de compilação Rust

- Ajustados os braços do `match` que alternam o foco da busca para todos retornarem `()`, evitando conflito com o braço vazio.
- Especificado `Vec<_>` na coleta dos autores disponíveis para remover a ambiguidade de tipo apontada pelo compilador.
- O log confirmou que o cache offline foi resolvido e que a compilação agora chega ao código da aplicação; não foi possível executar uma nova compilação neste ambiente porque `cargo` e `rustc` não estão instalados.

### 2026-10-08 17:50 -03:00 — Licença e preparação para o GitHub

- Adicionada a cópia integral da GNU GPL versão 3, identificada a licença do projeto como `GPL-3.0-or-later` no Cargo, Meson e AppStream, e incluída a licença na instalação do aplicativo.
- Atualizadas as seções de licença do README e do guia de contribuição. Os arquivos de modelos de terceiros continuam com seus próprios avisos.
- Atualizado o `.gitignore` para ignorar diretórios locais de build, além dos arquivos `settings` e `jsconfig.json` do GNOME Workbench presentes neste checkout.

## Verificações executadas

- AppStream validado em modo estrito, desktop file validado e XML analisado sem erros.
- Manifesto Flatpak analisado por `flatpak-builder --show-manifest`.
- JSON do Flatpak, `Cargo.toml`, `Cargo.lock`, dependências diretas e caminhos de instalação verificados por script.
- Catálogos `pt_BR` e `es` passaram por `msgfmt --check`; o `.pot` também foi verificado, com avisos esperados dos campos de modelo ainda não preenchidos.
- Arquivos ZIP dos cinco modelos conferidos quanto a caminhos inseguros e links simbólicos; os documentos iniciais `main.tex` estão nos overlays de cada modelo.
- Busca por referências remanescentes a Zotero, libsecret, `org.ovenbird` e `ovenbird.org` não encontrou ocorrências no código, nos metadados ou na documentação do produto (excluindo este relatório de auditoria).
- `git diff --check` passou.

## Pendências e limites

- **Licenças de terceiros:** os arquivos de modelos mantêm licenças e avisos próprios, separados da licença do Ovenbird; preserve-os e confira os termos de cada pacote antes de alterar sua redistribuição.
- **Build e testes Rust:** não foi possível executar `cargo test --offline --all-targets` nem concluir `meson setup`, porque este ambiente não tem `cargo` nem `rustc`. Portanto, as verificações estáticas acima não substituem uma compilação e a execução da suíte em um ambiente com o toolchain Rust e as dependências GTK requeridas.
- **Desempenho:** o cache redundante foi reduzido; não foi feito profiling da interface ou da compilação, então não há alegação de que cada caminho esteja no máximo de desempenho possível.
