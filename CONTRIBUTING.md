# Contributing

Ovenbird is in early development. Small, focused changes are easiest to review. Before opening an issue or pull request, check the project status and build instructions in the [README](README.md).

## Development setup

For a native Fedora build, install the packages listed in the README, then run:

```sh
meson setup build
meson compile -C build
meson test -C build --print-errorlogs
```

The repository pins Rust 1.92.0 in `rust-toolchain.toml`; rustup selects it when working in this directory. The pinned toolchain includes `rustfmt` and Clippy. Before submitting Rust changes, run:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
meson compile -C build
meson test -C build --print-errorlogs
```

GitHub Actions runs the same quality checks on pushes and pull requests. Meson is the integration build and runs Cargo offline with the repository's dependency cache.

For a local Flatpak build, use the GNOME SDK and Rust extension described in the README. Keep native and Flatpak build directories separate.

## Pull requests

- Explain the user-visible change and how to reproduce or review it.
- Include focused tests for behavior changes when practical.
- Keep user documents as standard LaTeX and the reference library exportable as BibTeX.
- Use XDG portals for file access where available; avoid adding broad Flatpak permissions.
- Do not include private documents, real personal references, PDFs, API keys, or other credentials.
- Keep documentation, issue forms, and pull request templates in English.

## Translations

The interface uses gettext catalogs in `po/`. English is the source language; Brazilian Portuguese and Spanish translations are maintained in `po/pt_BR.po` and `po/es.po`.

## License

Ovenbird is licensed under the GNU General Public License, version 3 or later; see [LICENSE](LICENSE). Bundled LaTeX templates have separate licenses and notices documented in [src/templates/README.md](src/templates/README.md). Preserve those notices and check compatibility before adding third-party files.
