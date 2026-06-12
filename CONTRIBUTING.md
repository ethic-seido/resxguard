# Contributing

Thank you for considering a contribution to ResxGuard.

## Publisher

ResxGuard is published by **ETHIC SEIDO**.

Website: <https://ethicseido.com>

## Development checks

Run these before opening a pull request:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Design constraints

- Preserve `.resx` encoding where possible.
- Prefer minimal diffs over full XML rewrites.
- Do not process binary/base64 `.resx` entries as text.
- Add tests for any new encoding, parser, or report behavior.
- Keep CI output stable with `--no-color --no-emoji`.
