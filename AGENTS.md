Purpose

This repository may be edited by AI coding agents such as Codex or Claude Code.

This file defines the mandatory rules for working with .resx localization files.

The most important rule is:

Do not manually edit .resx files. Use resxguard.

Project context

This is a .NET solution maintained by ETHIC SEIDO.

The repository uses .resx files for localization. These files are sensitive because they combine:

XML structure;
file encoding;
Unicode localized text;
resource keys used by application code.

Manual edits by AI agents can corrupt .resx files, especially by introducing mojibake such as:

CrÃ©ez
lâ€™utilisateur
Ã‰diteur
Ù…Ø±Ø­Ø¨Ø§
ä¸­æ–‡

To prevent this, this repository uses resxguard.

Mandatory .resx rules

AI agents must not manually edit .resx XML files unless explicitly instructed by a maintainer.

Agents must not:

rewrite a full .resx file;
reformat .resx XML;
change .resx file encoding manually;
paste or regenerate complete .resx contents;
use generic text replacement over entire .resx files;
bypass resxguard safety checks unless explicitly instructed;
use --allow-suspicious unless explicitly instructed by a maintainer.

Agents must:

use resxguard set for one key/value change;
use resxguard import for batch localization updates;
run resxguard check after every .resx change;
run resxguard repair --mode safe if check reports correctable mojibake;
run resxguard check again after any repair;
review the Git diff after any .resx repair or import.
resxguard location

The expected executable path is:

.\Tools\resxguard\resxguard.exe

If this path does not exist, stop and report the issue instead of editing .resx files manually.

Single resource update

Use set for a single resource key.

.\Tools\resxguard\resxguard.exe set .\src\MyApp\Resources.fr.resx Login.Button "Connexion" --create --backup

Rules:

Use --create only when the key may be missing.
Use --backup unless explicitly told not to.
Do not modify unrelated keys.
Do not rename keys unless the task explicitly requires it.
Batch localization update

For multiple translations, create a CSV and use import.

CSV format:

name,value,comment
Login.Button,Connexion,Bouton de connexion
Logout.Button,Déconnexion,Bouton de déconnexion

Import command:

.\Tools\resxguard\resxguard.exe import .\src\MyApp\Resources.fr.resx .\translations.fr.csv --backup --report .\artifacts\resx-import.fr.csv

For large imports, use:

.\Tools\resxguard\resxguard.exe import .\src\MyApp\Resources.fr.resx .\translations.fr.csv --continue-on-error --backup --report .\artifacts\resx-import.fr.csv

If updating only existing keys:

.\Tools\resxguard\resxguard.exe import .\src\MyApp\Resources.fr.resx .\translations.fr.csv --update-only --skip-missing --backup --report .\artifacts\resx-import.fr.csv
Check .resx files

After every .resx change, run:

.\Tools\resxguard\resxguard.exe --no-color --no-emoji check .\src --recursive --report .\artifacts\resx-check.csv

Use --no-color --no-emoji in automation and agent logs.

If the report contains correctable, run safe repair.

Repair mojibake

Only use safe mode by default:

.\Tools\resxguard\resxguard.exe --no-color --no-emoji repair .\src --recursive --mode safe --backup --report .\artifacts\resx-repair.csv

Then run check again:

.\Tools\resxguard\resxguard.exe --no-color --no-emoji check .\src --recursive --report .\artifacts\resx-check-after-repair.csv

Then review the diff:

git diff -- "*.resx"

If repair --mode safe still leaves suspicious entries, stop and report the affected keys. Do not attempt manual bulk edits.

Do not use the following modes unless explicitly requested by a maintainer:

--mode broad
--mode aggressive
Suspicious text handling

If set or import refuses a value with a message like:

refusing to set suspicious value

do not bypass the warning.

Instead:

inspect the value;
correct the source text;
retry the command;
if the text is intentionally suspicious, ask a maintainer before using --allow-suspicious.

Do not use:

--allow-suspicious

unless explicitly requested.

Resource keys

Resource keys may also contain mojibake. resxguard check and resxguard repair include .resx resource keys by default.

Be careful: changing a resource key may require updating code references.

If keys are repaired, search for old mojibake references in the repository before finishing the task.

Do not rename valid keys for stylistic reasons.

Backups and generated files

resxguard may create .bak files and CSV reports.

These are working artifacts and should normally not be committed unless explicitly requested.

Typical ignored files:

*.resx.bak
artifacts/
resx-*.csv
Recommended workflow for agents

For a single translation:

.\Tools\resxguard\resxguard.exe set <file.resx> <key> "<value>" --create --backup
.\Tools\resxguard\resxguard.exe --no-color --no-emoji check .\src --recursive --report .\artifacts\resx-check.csv
git diff -- "*.resx"

For batch translations:

.\Tools\resxguard\resxguard.exe import <file.resx> <translations.csv> --continue-on-error --backup --report .\artifacts\resx-import.csv
.\Tools\resxguard\resxguard.exe --no-color --no-emoji check .\src --recursive --report .\artifacts\resx-check.csv
git diff -- "*.resx"

For mojibake repair:

.\Tools\resxguard\resxguard.exe --no-color --no-emoji check .\src --recursive --report .\artifacts\resx-check.csv
.\Tools\resxguard\resxguard.exe --no-color --no-emoji repair .\src --recursive --mode safe --backup --report .\artifacts\resx-repair.csv
.\Tools\resxguard\resxguard.exe --no-color --no-emoji check .\src --recursive --report .\artifacts\resx-check-after-repair.csv
git diff -- "*.resx"
Final checklist for .resx tasks

Before finishing a task involving .resx files, verify:

resxguard was used for all .resx modifications;
resxguard check was run after modifications;
no suspicious mojibake remains unless explicitly reported;
Git diff only contains expected .resx changes;
no .bak or temporary report files are accidentally included;
no full .resx file was reformatted or regenerated.