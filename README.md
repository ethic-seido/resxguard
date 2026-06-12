# ResxGuard

ResxGuard is a portable CLI for protecting `.resx` files from mojibake, encoding damage, and unsafe manual XML edits.

It is intentionally slice-oriented: it modifies only targeted `.resx` text ranges instead of rewriting the full XML document, which keeps Git diffs small and helps preserve original formatting and encoding.

## Publisher

ResxGuard is published by **ETHIC SEIDO**.

Website: <https://ethicseido.com>


## Features

- Detect mojibake in `.resx` values, comments, and `data name` resource keys.
- Repair common UTF-8-as-Windows-1252 / ISO-8859-1 damage.
- Preserve file encoding where possible: UTF-8, UTF-8 BOM, UTF-16 LE/BE BOM, XML-declared encodings, and Windows-1252 fallback.
- Exclude binary/base64/resource-file entries.
- Safely set one key/value without direct XML editing.
- Import many key/value rows from CSV or JSON.
- Human-friendly CLI with colors and emoji.
- CI-friendly mode with `--no-color --no-emoji` and CSV/JSON reports.

## Install from source

```bash
cargo build --release
```

The binary is generated at:

```text
target/release/resxguard
```

On Windows:

```text
target\release\resxguard.exe
```

## Commands

### Check

```bash
resxguard check ./src --recursive --report resx-check.csv --fail-on-suspicious
```

By default, `check` includes resource keys in `<data name="...">`. Disable key checks with:

```bash
resxguard check ./src --recursive --no-attributes
```

### Repair

```bash
resxguard repair ./src --recursive --mode safe --backup --report resx-repair.csv
```

Available modes:

- `safe`: conservative default.
- `broad`: tests more likely wrong encodings.
- `aggressive`: tests wider encoding candidates and HTML/numeric entity decoding when enabled.

### Set a single value

```bash
resxguard set ./Resources.fr.resx Login.Button "Connexion" --create --backup
```

With comment:

```bash
resxguard set ./Resources.fr.resx Login.Button "Connexion" --comment "Button label" --create --backup
```

`set` refuses suspicious/mojibaked text by default. Override only when intentional:

```bash
resxguard set ./Resources.fr.resx Test.Bad "CrÃ©ez" --create --allow-suspicious
```

### Import many values

CSV format:

```csv
name,value,comment
Login.Button,Connexion,Button label
Logout.Button,Déconnexion,
```

Import and create missing keys:

```bash
resxguard import ./Resources.fr.resx ./translations.fr.csv --backup --report resx-import.csv
```

Update only existing keys:

```bash
resxguard import ./Resources.fr.resx ./translations.fr.csv --update-only --backup
```

Skip missing keys instead of failing:

```bash
resxguard import ./Resources.fr.resx ./translations.fr.csv --update-only --skip-missing --backup --report resx-import.csv
```

Continue on invalid/suspicious rows and report them:

```bash
resxguard import ./Resources.fr.resx ./translations.fr.csv --continue-on-error --backup --report resx-import.csv
```

### CI output

```bash
resxguard --no-color --no-emoji check ./src --recursive --report resx-check.csv
```

`NO_COLOR` is also respected.

## Report statuses

Reports use stable `snake_case` statuses:

- `correctable`
- `corrected`
- `still_suspicious`
- `added`
- `updated`
- `unchanged`
- `skipped_missing`
- `rejected_suspicious`
- `rejected_invalid`
- `missing`
- `failed`

## Design note

ResxGuard is not a general-purpose XML formatter or parser. It is a pragmatic `.resx` maintenance tool that targets standard `.resx` structures and intentionally avoids full XML rewrites to preserve formatting and minimize diffs.

Always review diffs before committing repairs.
