use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn check_reports_correctable_mojibake() {
    let dir = tempdir().unwrap();
    let resx = dir.path().join("Resources.fr.resx");
    fs::copy(fixture("corrupted.fr.resx"), &resx).unwrap();
    let report = dir.path().join("report.csv");

    Command::cargo_bin("resxguard")
        .unwrap()
        .args(["--no-color", "--no-emoji", "check"])
        .arg(&resx)
        .args(["--report"])
        .arg(&report)
        .assert()
        .success()
        .stdout(predicate::str::contains("REPORT report rows:"));

    let csv = fs::read_to_string(report).unwrap();
    assert!(csv.contains("correctable"));
    assert!(csv.contains("key"));
}

#[test]
fn repair_updates_values_and_keys() {
    let dir = tempdir().unwrap();
    let resx = dir.path().join("Resources.fr.resx");
    fs::copy(fixture("corrupted.fr.resx"), &resx).unwrap();

    let report = dir.path().join("repair.csv");

    Command::cargo_bin("resxguard")
        .unwrap()
        .args(["--no-color", "--no-emoji", "repair"])
        .arg(&resx)
        .args(["--mode", "safe", "--backup", "--report"])
        .arg(&report)
        .assert()
        .success();

    let updated = fs::read_to_string(&resx).unwrap();
    assert!(
        updated.contains("Créez un visuel avec l’éditeur"),
        "repaired file did not contain corrected value; content was:\n{updated}"
    );
    assert!(
        updated.contains("ImageStudio_Éditeur"),
        "repaired file did not contain corrected key; content was:\n{updated}"
    );
    let csv = fs::read_to_string(report).unwrap();
    assert!(csv.contains("key"));
    assert!(
        resx.with_extension("resx.bak").exists()
            || dir.path().join("Resources.fr.resx.bak").exists()
    );
}

#[test]
fn set_refuses_suspicious_text() {
    let dir = tempdir().unwrap();
    let resx = dir.path().join("Resources.fr.resx");
    fs::copy(fixture("clean.fr.resx"), &resx).unwrap();

    Command::cargo_bin("resxguard")
        .unwrap()
        .args(["--no-color", "--no-emoji", "set"])
        .arg(&resx)
        .args(["Test.Bad", "CrÃ©ez un visuel", "--create"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("refusing to set suspicious value"));
}

#[test]
fn import_can_continue_on_error() {
    let dir = tempdir().unwrap();
    let resx = dir.path().join("Resources.fr.resx");
    fs::copy(fixture("clean.fr.resx"), &resx).unwrap();
    let report = dir.path().join("import.csv");
    let input = dir.path().join("translations-mixed.fr.csv");
    fs::write(
        &input,
        "name,value,comment\nTest.Import.Valid,Valeur propre,OK\nTest.Import.Bad,CrÃ©ez un visuel,Must be rejected\n",
    )
    .unwrap();

    Command::cargo_bin("resxguard")
        .unwrap()
        .args(["--no-color", "--no-emoji", "import"])
        .arg(&resx)
        .arg(&input)
        .args(["--continue-on-error", "--report"])
        .arg(&report)
        .assert()
        .success();

    let csv = fs::read_to_string(report).unwrap();
    assert!(csv.contains("added"));
    assert!(csv.contains("rejected_suspicious"));
}
