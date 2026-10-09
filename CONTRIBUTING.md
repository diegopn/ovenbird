# Contributing

Ovenbird is in early development. Small, focused changes are easiest to review. Before opening an issue or pull request, check the project status and build instructions in the [README](README.md).

## Development setup

For a native Fedora build, install the packages listed in the README, then run:

```sh
meson setup build
meson compile -C build
meson test -C build --print-errorlogs
```

The repository pins Rust 1.92.0 in `rust-toolchain.toml`; rustup selects it when working in this directory. The pinned toolchain includes `rustfmt` and Clippy. Keep `dtolnay/rust-toolchain` in GitHub Actions aligned with this version; Dependabot skips it because its action reference also selects the Rust version. Before submitting Rust changes, run:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
meson compile -C build
meson test -C build --print-errorlogs
```

GitHub Actions runs the same quality checks on pushes and pull requests. Meson is the integration build and runs Cargo offline with the repository's dependency cache.

For a local Flatpak build, use the GNOME SDK and Rust extension described in the README. Keep native and Flatpak build directories separate.

## Security checks

The `Ovenbird checks` workflow runs build, test, and security checks on every
push and pull request. Dependabot checks Cargo dependencies and GitHub Actions
every two months and proposes updates as pull requests; security updates remain
separate from this version update schedule. Updates are not merged automatically.

- `cargo-audit` checks `Cargo.lock` against the RustSec advisory database.
- Gitleaks scans Git history for exposed credentials, with secret values redacted.
  `.gitleaksignore` lists seven reviewed false positives from public `ring` test
  vectors and documentation in previously tracked Cargo sources. Each exception
  identifies a specific commit, file, rule, and line; new findings remain visible.
- CodeQL analyzes Rust with the `security-extended` query suite and publishes
  findings to the repository's Security tab. Its build mode is `none`; the build
  and test job remains responsible for compiling and testing the application.

These checks require no paid account or additional API secrets for this public,
personally owned repository. Actions are pinned to commit hashes.

When a dependency update changes `Cargo.lock`, refresh the checked-in
`cargo-cache/registry/index` and `cargo-cache/registry/cache` for the new locked
dependencies before merging. Dependabot does not refresh this cache. Keep the
Meson build offline and run the usual quality checks against the updated cache.
Do not ignore advisories or secret findings just to make a check pass. A leaked
credential must be revoked or rotated, even if it has been removed from the code.

The dependency audit covers Rust crates, not system GTK/Poppler libraries or
installed LaTeX engines. These checks complement focused tests for document
parsing, file access, and compiler execution.

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
