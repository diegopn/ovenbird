# Contribuindo

Ovenbird está em desenvolvimento inicial. Mudanças pequenas e focadas são mais fáceis de revisar.

## Ambiente

Use o GNOME SDK/Platform 51 com GTK 4, Libadwaita, GtkSourceView 5, Secret Service e a extensão Rust stable. Para builds nativos, instale `rust`, `cargo`, Meson, Ninja e os pacotes `-devel` descritos no README. Mantenha o build nativo separado do build Flatpak/Builder.

Para criar projetos pelos modelos incluídos, o utilitário `unzip` precisa estar disponível no ambiente de execução. Para compilar LaTeX, instale `latexmk`, `tectonic` ou `pdflatex`; com `pdflatex`, instale BibTeX para documentos BibTeX e Biber para documentos `biblatex`.

## Antes de abrir um pull request

- Descreva o comportamento que mudou e como reproduzi-lo.
- Mantenha os documentos do usuário em LaTeX comum e a biblioteca local exportável em BibTeX.
- Evite permissões Flatpak amplas; use portais para selecionar arquivos.
- Não sobrescreva referências locais automaticamente quando a sincronização Zotero detectar mudanças concorrentes.
- Não inclua dados pessoais, chaves de API, referências reais ou PDFs privados nos commits.

## Licença

A licença do projeto ainda precisa ser escolhida antes de aceitar contribuições externas.
