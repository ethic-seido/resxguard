use crate::error::Result;
use crate::report::write_report_if_requested;
use crate::resx::{upsert_resx_values_file, SetEntry};
use crate::ui::Ui;
use std::path::Path;

#[allow(clippy::too_many_arguments)]
pub fn run(
    path: &Path,
    name: String,
    value: String,
    comment: Option<String>,
    create: bool,
    allow_suspicious: bool,
    backup: bool,
    dry_run: bool,
    report: Option<&Path>,
    ui: &Ui,
) -> Result<()> {
    let entry = SetEntry {
        name,
        value,
        comment,
    };
    let rows = upsert_resx_values_file(
        path,
        &[entry],
        create,
        allow_suspicious,
        backup,
        dry_run,
        false,
        false,
        ui,
    )?;
    write_report_if_requested(&rows, report, ui)?;
    println!(
        "{} {}",
        ui.report(),
        ui.bold(format!("report rows: {}", rows.len()))
    );
    Ok(())
}
