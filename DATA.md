# Ovenbird data and storage

Ovenbird keeps user projects in the folders they choose. Its bibliography, author metadata, and application settings live separately:

| Data | Location |
| --- | --- |
| BibTeX library | `$XDG_DATA_HOME/ovenbird/library.bib` |
| Author catalog | `$XDG_DATA_HOME/ovenbird/authors.json` |
| Author profiles | `$XDG_DATA_HOME/ovenbird/author_profiles.json` |
| App settings | `$XDG_CONFIG_HOME/ovenbird/settings.json` |
| Build PDFs and intermediate files | Ovenbird's private cache, outside the project folder |

The bibliography remains available as standard BibTeX. Tags and author metadata use JSON sidecar files. Imported `.bib` files are added to the local library; during compilation, Ovenbird combines that library with bibliography entries declared by the project in a temporary BibTeX file. Export the library to create a permanent `.bib` file to share.

In Flatpak, `$XDG_DATA_HOME` is private to the application ID. A development build with a different ID therefore uses a separate data directory.
