# Ovenbird

Ovenbird is a native GNOME workspace for writing LaTeX documents and managing a local BibTeX library. It is written in Rust with GTK 4, Libadwaita, and GtkSourceView. Project documents remain ordinary files; Ovenbird does not convert them to a proprietary format.

Created by Diego Pereira do Nascimento.

## Features

- Edit `.tex` documents in Code or Visual mode, with shared undo and redo history.
- Browse project files, create folders, move and rename items, and send files to Trash.
- Insert and format tables, equations, lists, images, links, notes, and citations.
- Manage a project-independent BibTeX library with reference types, tags, search, and `.bib` import and export.
- Compile with `latexmk`, `pdflatex`, or Tectonic. The local Flatpak manifest includes Tectonic.
- Preview PDFs and images in the editor. Embedded PDF preview requires Poppler-GLib at build time.
- Start projects from bundled blank, ABNT, IEEE, ACM, Elsevier, and Springer Nature templates.
- Use the interface in English, Brazilian Portuguese, or Spanish.

Visual mode supports a conservative subset of LaTeX. Unknown commands and environments remain source text so the editor does not discard document content.

## Building on Fedora

Install the native build dependencies:

```sh
sudo dnf install gcc rust cargo pkgconf-pkg-config meson ninja-build gettext-devel gtk4-devel libadwaita-devel gtksourceview5-devel libpanel-devel desktop-file-utils unzip
```

Poppler-GLib is optional and enables embedded PDF preview:

```sh
sudo dnf install poppler-glib-devel
```

To build and run the app:

```sh
meson setup build
meson compile -C build
meson test -C build --print-errorlogs
```

Meson invokes Cargo with `Cargo.lock` and network access disabled. Build artifacts and the Cargo home stay inside the build directory. If you already have a build directory from another Meson version, configure a new directory instead of deleting existing project files.

The repository pins Rust 1.92.0, including `rustfmt` and Clippy, in `rust-toolchain.toml`. See [CONTRIBUTING.md](CONTRIBUTING.md) for the checks to run before submitting Rust changes; GitHub Actions runs them on pushes and pull requests.

LaTeX engines are installed separately for native builds. For example, Fedora's `latexmk` and TeX Live packages provide `latexmk`, `pdflatex`, and BibTeX; `biblatex` projects may also need Biber:

```sh
sudo dnf install latexmk texlive biber
```

## Local Flatpak development

The in-repository manifest is intended for local builds while the packaging is being prepared. It is not yet the Flathub submission manifest.

```sh
flatpak-builder --user --force-clean .flatpak-build io.github.diegopn.ovenbird.json
flatpak-builder --user --install --force-clean .flatpak-build io.github.diegopn.ovenbird.json
flatpak run io.github.diegopn.ovenbird
```

The manifest uses GNOME Platform/SDK 51 and the stable Rust SDK extension. The first Tectonic build may download LaTeX support files at runtime. Creating a project from a bundled template does not require a network connection.

In GNOME Builder, use `io.github.diegopn.ovenbird.json` as the Flatpak manifest. If the build still reports `org.ovenbird.Ovenbird`, recreate the build configuration from the current manifest; Builder is using the old application ID.

## Local data

- Bibliography: `$XDG_DATA_HOME/ovenbird/library.bib`
- Author catalog: `$XDG_DATA_HOME/ovenbird/authors.json`
- Author profiles: `$XDG_DATA_HOME/ovenbird/author_profiles.json`
- App settings: `$XDG_CONFIG_HOME/ovenbird/settings.json`
- Build PDFs and intermediate files: Ovenbird's private cache, outside the project folder
- Project documents: remain in the folders selected by the user

The local reference library is stored as BibTeX in `library.bib`; the current application does not use SQLite. Tags and author metadata are kept in JSON sidecar files. In Flatpak, `$XDG_DATA_HOME` is private to the application ID, so changing the ID creates a different data directory.

Imported `.bib` files are added to the local library. During compilation, Ovenbird combines the local library with bibliography entries declared by the project in a temporary BibTeX file. Export the library when you need a permanent `.bib` file to share.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development and pull request guidance. Details and upstream notices for bundled template archives are in [src/templates/README.md](src/templates/README.md).

## License

Ovenbird is licensed under the GNU General Public License, version 3 or later. See [LICENSE](LICENSE) for the complete terms. Bundled LaTeX templates retain their own licenses and notices; see [src/templates/README.md](src/templates/README.md).
