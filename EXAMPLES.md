# ResxGuard examples

## Audit a directory

```bash
resxguard check ./samples --recursive --report resx-check.csv
```

## Repair conservatively

```bash
resxguard repair ./samples --recursive --mode safe --backup --report resx-repair.csv
```

By default this checks and repairs mojibake in values, comments, and resource keys such as `<data name="ImageStudio_MetaTitle">`.

To restrict processing to values/comments only:

```bash
resxguard repair ./samples --recursive --mode safe --no-attributes --backup --report resx-repair.csv
```

## Add or update a single value

```bash
resxguard set ./samples/Resources.fr.resx Test.Title "Titre de test" --create --backup
```

## Import valid rows

```bash
resxguard import ./samples/Resources.fr.resx ./translations-valid.fr.csv --backup --report resx-import-valid.csv
```

## Continue on errors

```bash
resxguard import ./samples/Resources.fr.resx ./translations-mixed.fr.csv --continue-on-error --backup --report resx-import-mixed.csv
```

## Update only and skip missing

```bash
resxguard import ./samples/Resources.fr.resx ./translations-update-only.fr.csv --update-only --skip-missing --backup --report resx-import-update-only.csv
```

## CI-friendly check

```bash
resxguard --no-color --no-emoji check ./samples --recursive --report resx-check.csv --fail-on-suspicious
```


---

ResxGuard is published by ETHIC SEIDO: <https://ethicseido.com>.
