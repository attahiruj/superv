# Security Policy

superv handles clipboard contents, which often include sensitive data, so security reports are taken seriously.

## Reporting a vulnerability

Please do not open a public issue. Use GitHub's **Report a vulnerability** (private security advisory) option on this repository's Security tab.

Include what you found, how to reproduce it, and the potential impact. You can expect an acknowledgement within a week.

## Scope

Examples of issues in scope: reading or writing history from other users or processes unexpectedly, unsafe handling of the `paste_command` setting, D-Bus actions that can be abused by other local processes.

## Good to know

- superv records everything copied, including passwords from password managers (by design).
- History is stored unencrypted in `~/.local/share/superv/history.db`. Protect it with filesystem permissions or full-disk encryption.
- On GNOME the app installs its Shell extension into `~/.local/share/gnome-shell/extensions` and adds it to `enabled-extensions`.
- `paste_command` from `config.toml` is executed as your user; treat that file as trusted.
