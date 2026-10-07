# Ovenbird Rust migration

## Inventory

The original application was a GJS/ES-module GNOME app. Its behavior and the
uncommitted product changes were inventoried before the port. The application
entry point, window, core modules, and tests now live in Rust; their replaced
JavaScript files have been removed. Meson still installs the desktop entry,
icon, AppStream data, offline template packages, and gettext catalogs. Flatpak
uses GNOME SDK/Platform 51 and includes Tectonic.

| Product area | Existing behavior to retain |
| --- | --- |
| Editor | GtkSourceView LaTeX/BibTeX code editor; visual subset for paragraphs, headings and inline formatting; source-preserving raw LaTeX; mode switching; shared undo/redo; search; image/link/table/math/list/citation insertion. |
| Project | Open/save `.tex`, `.bib`, `.sty`; recursive project file tree; create files/folders; rename, trash and drag/drop files; choose a main `.tex`; compile while a bibliography or style file is active. |
| Build | Prefer latexmk, pdflatex, then Tectonic; BibTeX/Biber passes; temporary local-library bibliography; build outputs in the existing user cache; PDF preview with Poppler when available and external-viewer fallback. |
| References | Local `library.bib` with field-specific reference forms; BibTeX import/export; citation search/insertion; preserve directives, macros, and unchanged raw expressions. |
| Zotero | Two-way Zotero Web API v3 synchronization; Secret Service key under schema `org.ovenbird.Ovenbird`, attribute `application=ovenbird`; settings JSON and sync-state JSON retain their current XDG paths and formats; detect concurrent edits rather than overwrite. |
| Templates | Offline project creation from the bundled ABNT, IEEE, ACM, Elsevier and Springer Nature archives and overlays; validate archive paths; no template-server dependency. |
| GNOME | Application ID, adaptive sidebar, menus/shortcuts, dark style, Portuguese (Brazil) system-language default, Spanish, English fallback, desktop entry, icon, AppStream, Flatpak permissions, and app data. |

## Architecture

- `rust/src/main.rs`, `app.rs`, and `window.rs`: GTK application lifecycle,
  adaptive window, sidebar, actions, dialogs, notifications, and navigation.
- `editor.rs` and `latex.rs`: GtkSourceView code mode, GTK text-buffer visual
  mode, conservative LaTeX tokenization/serialization, formatting, search,
  and history. GTK text positions are character offsets, not UTF-8 byte offsets.
- `project.rs`, `build.rs`, and `templates.rs`: portal-based file selection,
  project tree operations, compiler subprocesses, bibliography staging, PDF
  preview, and safe offline template extraction.
- `bibtex.rs`, `storage.rs`, and `zotero.rs`: BibTeX parsing/serialization,
  XDG-compatible storage, citation operations, Secret Service access, and
  asynchronous Zotero API synchronization.
- `i18n.rs`: bind the existing `ovenbird` gettext domain and keep the `.po`
  catalogs as the translation source of truth.
- Meson remains the install, translation, and GNOME integration entry point;
  Cargo owns Rust dependency resolution and puts its home/target directories
  under the Meson build directory.

## Migration result

The GTK application, editor, LaTeX transformations, history, search, project
tree, build and PDF preview, local BibTeX library, Zotero synchronization,
offline templates, menu actions, translations, and GNOME packaging are in Rust
or declarative resources. Cargo uses the checked-in lockfile and local crate
cache; Meson invokes Cargo with network access disabled. The old GJS launcher,
application/window modules, core modules, and JS tests are removed.

## Verification and remaining limits

Verified in this workspace on 2026-10-07:

- A fresh Meson setup and compile completed in GNOME SDK 51 at
  `.tooling/meson-audit-2026-10-07`; the Meson test suite passed. The Cargo
  suite contains 44 library tests and 3 application/window tests, all passing.
  This clean build directory did not reproduce the earlier Meson 1.11.2 versus
  1.12.0 install error.
- The latest Flatpak build completed successfully. It installs the Rust
  application, translations, metadata, Tectonic, and bundled templates. The
  Flatpak module recreates its Meson directory for each build, keeping its
  Meson version consistent.
- The five bundled project templates (ABNT, IEEE, ACM, Elsevier, and Springer
  Nature) were created and compiled to PDF with bundled Tectonic 0.17.0 in the
  previous integration pass on 2026-10-06. Missing TeX support files were
  downloaded during compilation; creating template projects itself works
  offline, while the first compilation may need network access.
- The final packaged UI was opened through Broadway and checked in Portuguese
  (Brazil). Screenshots under `.tooling/ui-audit/` cover the editor/sidebar,
  sidebar-hidden layout, search open/closed states, main and editor menus,
  and article/book reference forms. The toolbar and menus display their icons,
  and the reference form changes fields when its type changes.
- Coverage includes project-tree listing, file move/rename path handling,
  BibTeX parsing and merging, editor commands/history/search, per-type
  reference fields, template creation, and Zotero field conversion/conflict
  logic. Zotero tests use fixtures and do not contact the service.
- Both Portuguese (Brazil) and Spanish gettext catalogs pass
  `msgfmt --check --check-format`. `cargo fmt --check` and `git diff --check`
  pass. No JavaScript source or tests remain under `src/` or `tests/`.

The Zotero API was not exercised with a real account, and drag-and-drop plus
context-menu actions were not manually exercised in the GUI; their underlying
file operations have unit coverage. Poppler-GLib was not available in the SDK
build, so PDFs open in the system viewer there; builds where Poppler-GLib is
available enable the embedded preview. Clippy exits successfully but reports
Rust style warnings and GTK deprecation warnings for older dialog, chooser,
and CSS APIs. They do not prevent compilation or tests.

Keep native and Flatpak/Meson build directories separate and inside the
repository. The verified Flatpak build uses the GNOME SDK's Meson and removes
its own generated Meson directory before setup, avoiding reuse across Meson
versions.
