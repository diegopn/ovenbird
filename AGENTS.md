# Ovenbird project guidance

Ovenbird is a native GNOME workspace for writing LaTeX documents and managing a local BibTeX library. It is written in Rust with GTK 4, Libadwaita, and GtkSourceView, and Meson integrates the native dependencies with the Rust build.

The application edits ordinary `.tex` files and keeps project documents in folders chosen by the user. It also provides Code and Visual editing modes, project file management, LaTeX compilation, and PDF or image previews. The BibTeX library is local and independent of any one project; it can import and export `.bib` files. Projects can start from bundled academic templates.

## Product boundaries

- Keep LaTeX documents as ordinary, portable files. Do not introduce a proprietary project format or move user documents into application data.
- Visual mode supports a conservative subset of LaTeX. Preserve unknown commands and environments as source text so editing does not discard content the visual editor cannot represent.
- Keep the reference library exportable as BibTeX. The library is stored in `library.bib`; tags and author metadata use JSON sidecar files. Do not assume the current application uses SQLite.
- Compilation output and intermediate files belong in Ovenbird's private cache, outside the user's project folder.
- Use file chooser portals for user-selected files where available. Keep Flatpak permissions limited to what the application needs.

## Repository map

- `rust/src/app.rs`, `window.rs`, and `application_menu.rs` contain application startup, the main window, and application actions.
- `rust/src/editor.rs`, `project.rs`, and `history.rs` implement document editing, project file operations, and edit history.
- `rust/src/bibtex.rs`, `reference_exchange.rs`, `reference_guides.rs`, and `storage.rs` handle reference data, import/export, and local persistence.
- `rust/src/latex.rs`, `build.rs`, `latex_diagnostics.rs`, `pdf_preview.rs`, and `templates.rs` support LaTeX workflows, previews, and bundled templates.
- `rust/src/tests.rs` contains Rust unit tests. Other Rust modules cover commands, search, internationalization, and UI integration.
- `data/` contains application metadata, icons, and styles. `po/` contains gettext catalogs. `src/templates/` contains template files and their license notices.
- `meson.build` and `src/meson.build` define the integration build. Cargo dependencies are pinned in `Cargo.lock`, and `cargo-cache/` supplies the checked-in registry cache used by Meson for offline builds.

Keep Rust tooling and project-specific guidance scoped to this repository.

## Before finishing Rust changes

- Format with `cargo fmt --all -- --check`.
- Run Clippy with `cargo clippy --locked --all-targets -- -D warnings`.
- Build and run tests through Meson: `meson compile -C build` and `meson test -C build --print-errorlogs`.
- If the Meson build directory does not exist, configure it with `meson setup build` first. Do not remove or overwrite an existing build directory to work around configuration issues.
- Report checks that could not run and why. Do not claim a check passed unless its command completed successfully.

Meson is the integration build: it checks native GTK dependencies and runs Cargo offline using the repository's `cargo-cache`. Cargo-only commands may need the build directory's Cargo environment or network access to resolve dependencies.

## Project constraints

- Keep `Cargo.toml`'s `rust-version` and `rust-toolchain.toml` aligned.
- Preserve user LaTeX documents as ordinary files and BibTeX data as exportable `.bib` files.
- Preserve Flatpak's least-privilege permissions and use portals for user-selected files where available.
- Keep translations in sync with the English source strings and gettext catalogs under `po/`.
- Preserve third-party template licenses and notices documented in `src/templates/README.md`.
