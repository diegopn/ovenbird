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
- Use the interface in English, Brazilian Portuguese, Spanish, French, or Italian.

Visual mode supports a conservative subset of LaTeX. Unknown commands and environments remain source text so the editor does not discard document content.

## Building from source on Linux

Install the native build dependencies for your distribution. A native build requires GTK 4 4.20 or newer, libadwaita 1.8 or newer, GtkSourceView 5.18 or newer, and libpanel 1.10.4 or newer. Package versions vary by release; for example, Debian 13 (Trixie) does not meet these minimums. Meson reports when an installed library is too old.

### Fedora-based and compatible RPM distributions

On Fedora, use DNF:

```sh
sudo dnf install gcc rustup pkgconf-pkg-config meson ninja-build gettext-devel gtk4-devel libadwaita-devel gtksourceview5-devel libpanel-devel desktop-file-utils unzip
```

If your compatible RPM-based distribution uses YUM, use:

```sh
sudo yum install gcc rustup pkgconf-pkg-config meson ninja-build gettext-devel gtk4-devel libadwaita-devel gtksourceview5-devel libpanel-devel desktop-file-utils unzip
```

On Fedora, run `rustup-init` once after installing `rustup`.

### Debian, Ubuntu, and derivatives

```sh
sudo apt-get install build-essential rustup pkg-config meson ninja-build gettext libgtk-4-dev libadwaita-1-dev libgtksourceview-5-dev libpanel-dev desktop-file-utils unzip
```

Use `apt` instead of `apt-get` if you prefer. Choose a release whose packages meet the version minimums above; Debian 13 stable is currently too old for a native build.

### Arch-based distributions

```sh
sudo pacman -S --needed base-devel rustup pkgconf meson ninja gettext gtk4 libadwaita gtksourceview5 libpanel desktop-file-utils unzip
```

### openSUSE Tumbleweed

```sh
sudo zypper install gcc rustup pkgconf-pkg-config meson ninja gettext-tools gtk4-devel libadwaita-devel gtksourceview5-devel libpanel-devel desktop-file-utils unzip
```

For openSUSE Leap, check that the packages in your release meet the version minimums above before building.

### Optional PDF preview

Poppler-GLib enables the embedded PDF preview. Install its development package with your distribution's package manager:

- Fedora/RPM: `sudo dnf install poppler-glib-devel` (or `sudo yum install poppler-glib-devel`)
- Debian/Ubuntu: `sudo apt-get install libpoppler-glib-dev`
- Arch: `sudo pacman -S --needed poppler-glib`
- openSUSE: `sudo zypper install libpoppler-glib-devel`

### Optional LaTeX tools

LaTeX engines are installed separately for native builds. `biblatex` projects may also need Biber:

- Fedora/RPM: `sudo dnf install latexmk texlive biber` (or `sudo yum install latexmk texlive biber`)
- Debian/Ubuntu: `sudo apt-get install latexmk texlive-latex-base biber`
- Arch: `sudo pacman -S --needed texlive-basic texlive-binextra texlive-latex texlive-bibtexextra biber`
- openSUSE Tumbleweed: `sudo zypper install texlive-scheme-basic texlive-latexmk-bin texlive-biber-bin`

### Build and run

These commands are the same on each distribution:

```sh
meson setup build
meson compile -C build
meson test -C build --print-errorlogs
```

The repository pins Rust 1.92.0, including `rustfmt` and Clippy, in `rust-toolchain.toml`; `rustup` selects that toolchain for the project. Do not install the distribution's `rust` package alongside `rustup`. The initial Rust toolchain download requires network access.

Meson invokes Cargo with `Cargo.lock` and network access disabled. Build artifacts and the Cargo home stay inside the build directory. If you already have a build directory from another Meson version, configure a new directory instead of deleting existing project files. See [CONTRIBUTING.md](CONTRIBUTING.md) for the checks to run before submitting Rust changes; GitHub Actions runs them on pushes and pull requests.

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
