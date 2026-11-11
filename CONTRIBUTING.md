# Contributing to superv

Thanks for helping out. Bug reports, testing on different desktops, docs and code are all welcome.

## Reporting bugs

Open an issue using the bug template. Please include:

- Distro, desktop environment, and session type (`echo $XDG_CURRENT_DESKTOP $XDG_SESSION_TYPE`)
- How you installed superv and its version
- Steps to reproduce, and what you expected
- Daemon output: `journalctl --user -u superv` or run `superv --daemon` in a terminal

Never paste your clipboard history or `history.db` into an issue; it may contain secrets.

## Suggesting features

Open an issue first for anything non-trivial so we can agree on scope before you write code.

## Development setup

1. Install Rust (stable) and the GTK4 + gtk4-layer-shell development packages (see the README).
2. Fork https://github.com/attahiruj/superv, `git clone` your fork and create a branch from `main`.
3. Build and test:

   ```sh
   cargo build
   cargo test -p superv-core
   ```

To try the app without touching your installed copy, stop the service (`systemctl --user stop superv`) and run `cargo run -- --daemon`, then `cargo run -- --toggle` from another terminal.

## Code guidelines

- Run `cargo fmt --all` and `cargo clippy --workspace --all-targets` before pushing. CI checks both.
- Keep GUI-free logic (storage, filtering, models) in `superv-core` and cover it with unit tests. The `superv` crate should stay thin.
- New recording rules (what is or isn't saved) need a test in `superv-core`.
- Don't add network access or telemetry. Clipboard data must stay on the user's machine.
- Be conservative with new dependencies and justify them in the PR.
- GNOME extension: keep `shell-version` in `metadata.json` accurate for the versions you tested.

## Pull requests

- Keep PRs focused; one change per PR.
- Describe what changed and why, and which desktop/session you tested on.
- Update the README and CHANGELOG for user-visible changes.
- Be ready to rebase; history should be tidy.

By contributing you agree that your contributions are licensed under the project's [MIT license](LICENSE).

## Conduct

Participation is governed by the [Code of Conduct](CODE_OF_CONDUCT.md).
