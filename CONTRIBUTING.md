# Contribuindo

Ovenbird está em desenvolvimento inicial. Mudanças pequenas e focadas são mais fáceis de revisar.

## Ambiente

Use o GNOME SDK/Platform 51 com GTK 4, Libadwaita, GJS, GtkSourceView 5 e Poppler-GLib. Para compilar LaTeX, instale `latexmk`, `tectonic` ou `pdflatex`; com `pdflatex`, instale BibTeX para documentos BibTeX e Biber para documentos `biblatex`.

## Antes de abrir um pull request

- Descreva o comportamento que mudou e como reproduzi-lo.
- Mantenha os documentos do usuário em LaTeX comum e a biblioteca local exportável em BibTeX.
- Evite permissões Flatpak amplas; use portais para selecionar arquivos.
- Não sobrescreva referências locais automaticamente quando a sincronização Zotero detectar mudanças concorrentes.
- Não inclua dados pessoais, chaves de API, referências reais ou PDFs privados nos commits.

## Licença

A licença do projeto ainda precisa ser escolhida antes de aceitar contribuições externas.
