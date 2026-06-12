use crate::error::Result;
use crate::report::write_report_if_requested;
use crate::resx::{read_set_entries, upsert_resx_values_file};
use crate::ui::Ui;
use std::path::Path;

#[allow(clippy::too_many_arguments)]
pub fn run(
    path: &Path,
    input: &Path,
    update_only: bool,
    skip_missing: bool,
    continue_on_error: bool,
    allow_suspicious: bool,
    backup: bool,
    dry_run: bool,
    report: Option<&Path>,
    ui: &Ui,
) -> Result<()> {
    let entries = read_set_entries(input)?;
    let rows = upsert_resx_values_file(
        path,
        &entries,
        !update_only,
        allow_suspicious,
        backup,
        dry_run,
        skip_missing,
        continue_on_error,
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
