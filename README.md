# Ovenbird

Editor LaTeX para GNOME, escrito em Rust com GTK 4, Libadwaita e GtkSourceView. Os documentos continuam em arquivos `.tex`; o projeto não converte o conteúdo para Markdown nem para um formato proprietário.

## Recursos

- Modos Código e Visual para o mesmo documento, com histórico compartilhado de desfazer e refazer.
- Edição de `.tex`, `.bib` e `.sty`; os demais arquivos do projeto aparecem como recursos.
- Navegador recursivo de arquivos do projeto, criação de arquivos e pastas, arrastar para mover, renomear e enviar à lixeira.
- Busca no documento, inserção de links, imagens, tabelas, equações, listas, notas e citações.
- Biblioteca BibTeX local independente dos projetos, com formulários por tipo de referência e importação e exportação `.bib`.
- Sincronização bidirecional manual com Zotero Web API v3. Credenciais ficam no Secret Service do sistema.
- Projetos em branco, ABNT, IEEE, ACM, Elsevier e Springer Nature criados a partir dos modelos incluídos, sem baixar modelos da internet.
- Compilação por `latexmk`, `pdflatex` ou Tectonic, com referências locais combinadas em um `.bib` temporário.
- Prévia de PDF dentro do editor quando Poppler-GLib está disponível no build; sem ele, o PDF abre no leitor padrão.
- Interface traduzida para português do Brasil e espanhol, respeitando o idioma do sistema e usando inglês como fallback.

O modo Visual cobre um subconjunto conservador de LaTeX. Comandos e ambientes desconhecidos permanecem como texto-fonte para evitar perda de conteúdo.

## Requisitos no Fedora

Para compilar e executar fora do Flatpak:

```sh
sudo dnf install gcc rust cargo pkgconf-pkg-config meson ninja-build gettext-devel gtk4-devel libadwaita-devel gtksourceview5-devel libsecret-devel unzip
```

O `rust` e o `cargo` são necessários para o build nativo pelo Meson. A extensão Rust do Flatpak é usada pelo GNOME Builder e não instala o Cargo no Fedora fora do SDK.

O Poppler é opcional e habilita a prévia embutida:

```sh
sudo dnf install poppler-glib-devel
```

Para compilar LaTeX fora do Flatpak no Fedora:

```sh
sudo dnf install latexmk texlive
```

Isso fornece `latexmk`, `pdflatex` e BibTeX. Projetos que usam `biblatex` podem precisar do Biber:

```sh
sudo dnf install biber
```

Para sincronizar com Zotero, é necessária uma sessão Secret Service, como GNOME Keyring, e uma chave Zotero com acesso de leitura e escrita.

## GNOME Builder e Flatpak

O manifesto usa GNOME Platform/SDK 51 e a extensão `org.freedesktop.Sdk.Extension.rust-stable`. Com Builder, compile pelo perfil Flatpak do projeto. No Fedora, o runtime Flatpak e a extensão Rust devem estar instalados na mesma instalação do Flatpak usada pelo Builder. A extensão que acompanha este ambiente é a branch 26.08; o manifesto inclui a extensão para que o Builder a monte no SDK.

Para construir pelo terminal com `flatpak-builder` (instalado no Fedora como `flatpak-builder`):

```sh
flatpak-builder --user --force-clean .flatpak-build org.ovenbird.Ovenbird.json
```

O módulo do aplicativo limpa o diretório Meson interno antes de configurar o build. Assim o Flatpak Builder não reutiliza `coredata.dat` criado por outra versão do Meson. Para recuperar um checkout Flatpak antigo, use o comando acima da raiz do repositório; mantenha `.flatpak-build` dentro da pasta do projeto.

Para instalar localmente depois do build:

```sh
flatpak-builder --user --install --force-clean .flatpak-build org.ovenbird.Ovenbird.json
flatpak run org.ovenbird.Ovenbird
```

No Flatpak, Tectonic é incluído. A criação de projetos pelos modelos funciona offline; a primeira compilação com Tectonic pode baixar os arquivos de suporte do LaTeX. A prévia embutida de PDF depende de Poppler-GLib no ambiente de build; caso contrário, o aplicativo usa o visualizador padrão do sistema.

## Build com Meson e Cargo

Execute a partir da raiz do repositório e mantenha o diretório de build dentro dele. Para o build nativo:

```sh
rm -rf rust-build-fedora
meson setup rust-build-fedora
meson compile -C rust-build-fedora
meson test -C rust-build-fedora --print-errorlogs
```

Meson é a entrada do build GNOME. Ele chama Cargo usando `Cargo.lock`, sem rede, e mantém o `CARGO_HOME` e os artefatos Cargo dentro do diretório de build Meson. Se aparecer a incompatibilidade entre Meson 1.11.2 e 1.12.0, apague apenas o diretório de build nativo e configure novamente. O comando de build Flatpak acima limpa seu próprio diretório interno automaticamente.

## Dados locais

- Biblioteca: `$XDG_DATA_HOME/ovenbird/library.bib`.
- Estado de sincronização Zotero: `$XDG_DATA_HOME/ovenbird/zotero-sync.json`.
- ID da biblioteca Zotero: `$XDG_CONFIG_HOME/ovenbird/settings.json`.
- Chave Zotero: Secret Service, schema `org.ovenbird.Ovenbird` e atributo `application=ovenbird`.
- PDFs e arquivos intermediários: cache privado do Ovenbird, fora da pasta do projeto.
- Arquivos do projeto: permanecem nas pastas escolhidas pela pessoa usuária.

Os `.bib` importados são incorporados à biblioteca local. Durante a compilação, o Ovenbird cria um BibTeX temporário com a biblioteca e as referências declaradas no projeto; exporte a biblioteca quando quiser compartilhar um `.bib` permanente.

## Estrutura

```text
.
├── data/                  # Ícone, desktop entry e metadados AppStream
├── docs/                  # Inventário e plano da migração Rust
├── po/                    # Catálogos gettext pt_BR e es
├── rust/src/              # Aplicação, editor, projetos, referências e serviços
├── src/templates/         # Modelos LaTeX incluídos
├── cargo-cache/           # Cache local de crates para builds offline
├── Cargo.toml
├── Cargo.lock
├── meson.build
└── org.ovenbird.Ovenbird.json
```
