<p align="center">
  <a href="https://diegopn.github.io/ovenbird-site/">
    <img src="data/io.github.diegopn.ovenbird.svg" alt="Ovenbird icon" width="112" />
  </a>
</p>

<h1 align="center">Ovenbird</h1>

<p align="center">A native GNOME workspace for writing and compiling LaTeX projects, previewing PDFs, and managing a local BibTeX library.</p>

<p align="center">
  <a href="https://github.com/diegopn/ovenbird/releases/latest">Download</a> ·
  <a href="https://diegopn.github.io/ovenbird-site/">Website</a> ·
  <a href="https://diegopn.github.io/ovenbird-site/manual/">User manual</a>
</p>

<p align="center">
  <a href="https://www.rust-lang.org/"><img alt="Rust" src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white"></a>
  <a href="https://www.gtk.org/"><img alt="GTK 4" src="https://img.shields.io/badge/GTK-4A86CF?style=for-the-badge&logo=gnome&logoColor=white"></a>
  <a href="https://gnome.pages.gitlab.gnome.org/libadwaita/"><img alt="Libadwaita" src="https://img.shields.io/badge/Libadwaita-3584E4?style=for-the-badge&logo=gnome&logoColor=white"></a>
  <a href="https://gnome.pages.gitlab.gnome.org/gtksourceview/gtksourceview5/5.0/gtkdoc/"><img alt="GtkSourceView" src="https://img.shields.io/badge/GtkSourceView-5B5B5B?style=for-the-badge"></a>
  <a href="https://mesonbuild.com/"><img alt="Meson" src="https://img.shields.io/badge/Meson-0078D4?style=for-the-badge"></a>
</p>

Ovenbird keeps LaTeX projects as ordinary files while bringing editing, compilation, PDF preview, and reference management into one desktop workspace.

## Highlights

| Workflow | What Ovenbird provides |
| --- | --- |
| Write | Code and visual editing for `.tex`, with shared undo and redo. |
| Organize | Project files and folders, plus tables, equations, images, links, notes, and citations. |
| Cite | A local BibTeX library with search, tags, import, and export. |
| Compile | `latexmk`, `pdflatex`, or Tectonic, with PDF and image previews. The Flatpak includes Tectonic. |
| Start a document | Bundled ABNT, IEEE, ACM, Elsevier, and Springer Nature templates. |

Visual mode supports a focused subset of LaTeX. Unknown commands and environments remain source text, so unsupported content is preserved.
The interface is available in English, Brazilian Portuguese, Spanish, French, and Italian.

## Screenshots

<table>
  <tr>
    <td align="center"><strong>LaTeX editor and PDF preview</strong></td>
    <td align="center"><strong>Local reference library</strong></td>
  </tr>
  <tr>
    <td><img src="data/screenshots/ovenbird-editor-compiled.png" alt="Ovenbird LaTeX editor with a compiled PDF preview" width="100%"></td>
    <td><img src="data/screenshots/ovenbird-references-list.png" alt="Ovenbird local library showing searchable BibTeX references" width="100%"></td>
  </tr>
</table>

## Build from source

See [BUILDING.md](BUILDING.md) for native Linux dependencies, optional LaTeX tools, and local Flatpak development.

## Data and documents

Your project files stay in the folders you choose. Ovenbird stores the local BibTeX library and app settings separately; see [DATA.md](DATA.md) for locations and Flatpak behavior.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development and pull request guidance. Bundled template licenses and notices are listed in [src/templates/README.md](src/templates/README.md).

## Support

Sponsorship helps with Ovenbird development, maintenance, documentation, and releases.

<p align="center">
  <a href="https://github.com/sponsors/diegopn"><img alt="Become a sponsor" src="https://img.shields.io/badge/Become_a_sponsor-EA4AAA?style=for-the-badge&logo=githubsponsors&logoColor=white"></a>
</p>

## License

Ovenbird is licensed under the GNU General Public License, version 3 or later. See [LICENSE](LICENSE) for the complete terms. Bundled LaTeX templates retain their own licenses and notices; see [src/templates/README.md](src/templates/README.md).
