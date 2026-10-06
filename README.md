# Ovenbird

Editor LaTeX nativo para GNOME, com uma biblioteca bibliográfica local e sincronização opcional com Zotero.

Os documentos de trabalho continuam sendo arquivos `.tex`: não há conversão para Markdown nem formato proprietário. O editor oferece modos **Código** e **Visual** para o mesmo conteúdo; a biblioteca local funciona sem conta Zotero e sem conexão com a internet. O Markdown deste repositório é apenas documentação para o GitHub.

> **Estado:** protótipo inicial em desenvolvimento. Ainda não é uma versão pronta para publicação no Flathub.

## O que já está nesta base

- Interface no idioma do sistema: português do Brasil e espanhol, com inglês como idioma padrão.
- Aplicativo GJS com GTK 4, Libadwaita e GtkSourceView.
- Janela adaptável inspirada nos padrões de navegação do GNOME, com áreas de Escrita e Biblioteca local.
- Abrir um `.tex` isolado ou uma pasta de projeto, criar documentos dentro de uma pasta autorizada e salvar `.tex`.
- Alternância entre Código e Visual no mesmo documento.
- Uma faixa horizontal de edição presente em Código e Visual, com estilos de parágrafo, formatação, equações, símbolos, links, listas, recuo, notas, imagens e tabelas. Ações sem representação visual no subconjunto atual inserem LaTeX no documento.
- Desfazer e refazer compartilhados entre Código e Visual.
- Pré-visualização embutida quando Poppler está disponível; caso contrário, o PDF abre no leitor padrão.
- Cadastro, busca, edição e exclusão de referências localmente.
- Importação e exportação de BibTeX; diretivas `@string`, `@preamble` e `@comment` são mantidas.
- Busca rápida de citações por título, autor ou ano; selecionar um resultado ou pressionar Enter insere `\cite{chave}` no cursor. Com a biblioteca vazia, o mesmo fluxo oferece adicionar ou importar referências.
- Ao compilar, mescla referências locais no `.bib` declarado por `\addbibresource` ou `\bibliography`. Chaves já existentes no projeto são preservadas e conflitos são informados.
- Sincronização manual bidirecional básica com a biblioteca pessoal do Zotero por meio da API Web v3.
- Chaves Zotero armazenadas pelo Secret Service do sistema; arquivos de projetos acessados pelo seletor de arquivos do desktop.

## Limites conhecidos

- O modo Visual cobre um subconjunto de LaTeX. Classes, macros, ambientes e comandos personalizados podem aparecer como código para preservar o texto original.
- A edição do preâmbulo é feita no modo Código.
- A sincronização ainda não transfere PDFs/anexos nem propaga exclusões. Conflitos entre alterações locais e remotas são preservados localmente e informados, sem sobrescrita automática. A sincronização é completa, não incremental.
- No Flatpak, o Tectonic 0.17.0 é incluído para compilar documentos LaTeX e processar bibliografias BibTeX. A primeira compilação pode baixar arquivos de suporte do Tectonic; depois eles ficam no cache local. Projetos `biblatex` que exigem Biber precisam de um ambiente TeX que inclua Biber, como TeX Live com `latexmk`.
- PDFs e arquivos intermediários da compilação ficam no cache privado do Ovenbird, fora da pasta do projeto. A interface informa quando a primeira execução do Tectonic pode estar baixando seus arquivos de suporte; cada processo de compilação tem limite de cinco minutos.
- A validação do editor visual e da sincronização ainda precisa crescer; a licença e a URL do repositório também precisam ser escolhidas antes da publicação.

## Requisitos para executar a versão de desenvolvimento

- GJS com introspecção de GTK 4 e Libadwaita.
- GtkSourceView 5 disponível para GObject Introspection.
- Ferramentas GNU gettext (`msgfmt` e `xgettext`) para compilar os catálogos durante o desenvolvimento; o GNOME SDK do Flatpak já as fornece.
- Poppler-GLib para a pré-visualização embutida (opcional; sem ela, o PDF abre no leitor padrão).
- Para executar fora do Flatpak, instale `latexmk`, `pdflatex` com BibTeX, ou Tectonic. Projetos `biblatex` precisam também do Biber; `latexmk` automatiza as passagens quando os processadores correspondentes estão instalados.
- Para usar Zotero: conexão de rede, uma chave da API com leitura e escrita, e um serviço Secret Service (por exemplo, GNOME Keyring).

O Flatpak usa GNOME Platform/SDK 51 e solicita rede para a integração Zotero e para baixar arquivos de suporte do Tectonic na primeira compilação. A biblioteca bibliográfica local e os arquivos `.bib` do projeto continuam armazenados localmente. O aplicativo não pede acesso irrestrito ao diretório pessoal: escolha a pasta do projeto pelo seletor para permitir acesso aos arquivos relacionados, como imagens e `.bib`.

## Executar localmente

```sh
meson setup build-dir
meson compile -C build-dir
OVENBIRD_LOCALEDIR="$PWD/build-dir/po" gjs -m src/main.js
```

## Validar o núcleo

```sh
meson setup build-dir
meson test -C build-dir --print-errorlogs
```

## Construir o Flatpak

Com `flatpak-builder`, o GNOME SDK 51 e o runtime correspondentes instalados:

```sh
flatpak-builder --user --force-clean build-dir org.ovenbird.Ovenbird.json
```

Para instalar localmente depois da construção:

```sh
flatpak-builder --user --install --force-clean build-dir org.ovenbird.Ovenbird.json
```

## Dados locais e sincronização

- Biblioteca BibTeX: `$XDG_DATA_HOME/ovenbird/library.bib`.
- Estado de sincronização Zotero: `$XDG_DATA_HOME/ovenbird/zotero-sync.json`.
- ID da biblioteca Zotero: `$XDG_CONFIG_HOME/ovenbird/settings.json`.
- A chave Zotero fica no Secret Service, separada dos arquivos acima.
- Projetos LaTeX permanecem em suas pastas originais; o Ovenbird não os converte para um formato proprietário.

A sincronização é acionada pela pessoa usuária. Para conectar, informe o ID da biblioteca pessoal e uma chave Zotero com permissão de escrita. A chave pode ser criada nas [configurações de API do Zotero](https://www.zotero.org/settings/keys). O projeto também mantém uma cópia BibTeX padrão por documento; referências da biblioteca local entram nela quando o documento é compilado.

## Estrutura

```text
.
├── data/                  # Ícone, desktop entry e metadados AppStream
├── .github/               # Formulários de issues e modelo de pull request
├── tests/                 # Verificações do parser, BibTeX, comandos e histórico do editor
├── src/
│   ├── core/              # Biblioteca BibTeX, comandos e histórico do editor, compilação e Zotero
│   ├── application.js
│   ├── main.js
│   ├── ovenbird           # Launcher instalado no Flatpak
│   └── window.js
├── meson.build
├── org.ovenbird.Ovenbird.json
└── README.md
```

## Contribuições

Issues e pull requests no GitHub são bem-vindos. Antes de uma primeira publicação no Flathub, ainda é necessário incluir a URL do repositório nos metadados AppStream e escolher uma licença.
