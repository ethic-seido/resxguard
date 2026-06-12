# Agent instructions for ResxGuard

When working on a repository that uses `.resx` files:

1. Do not edit `.resx` XML manually unless explicitly requested.
2. Use `resxguard set` for single key/value updates.
3. Use `resxguard import` for batch updates from CSV or JSON.
4. Run `resxguard check` after any `.resx` change.
5. `check` and `repair` include resource keys in `<data name="...">` by default. Use `--no-attributes` only when keys must never be changed.
6. Use `--mode broad` or `--mode aggressive` only after reviewing a report.
7. Prefer CI-stable output with `--no-color --no-emoji`.

Recommended command after changes:

```bash
resxguard --no-color --no-emoji check ./src --recursive --report ./artifacts/resx-check.csv --fail-on-suspicious
```

For safe updates:

```bash
resxguard set ./Resources.fr.resx Login.Button "Connexion" --create --backup
```


## Publisher

ResxGuard is published by ETHIC SEIDO: <https://ethicseido.com>.
