# Ovenbird development instructions

Ovenbird is a Rust 2021 GTK 4 and Libadwaita application built with Meson. Keep Rust tooling and project-specific guidance scoped to this repository.

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
