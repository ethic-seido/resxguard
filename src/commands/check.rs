use crate::cli::Mode;
use crate::error::{AppError, Result};
use crate::report::{write_report_if_requested, ReportStatus};
use crate::resx::{collect_resx_files, process_file, Operation};
use crate::ui::Ui;
use std::path::Path;

#[allow(clippy::too_many_arguments)]
pub fn run(
    path: &Path,
    recursive: bool,
    mode: Mode,
    max_passes: usize,
    decode_html_entities: bool,
    include_attributes: bool,
    report: Option<&Path>,
    fail_on_suspicious: bool,
    ui: &Ui,
) -> Result<()> {
    let files = collect_resx_files(path, recursive)?;
    let mut rows = Vec::new();
    for file in files {
        let mut file_rows = process_file(
            &file,
            mode,
            max_passes,
            decode_html_entities,
            include_attributes,
            Operation::Check,
            ui,
        )?;
        rows.append(&mut file_rows);
    }
    let suspicious = rows.iter().any(|r| {
        matches!(
            r.status,
            ReportStatus::StillSuspicious | ReportStatus::Correctable
        )
    });
    write_report_if_requested(&rows, report, ui)?;
    println!(
        "{} {}",
        ui.report(),
        ui.bold(format!("report rows: {}", rows.len()))
    );
    if fail_on_suspicious && suspicious {
        return Err(AppError::Message(
            "suspicious or correctable mojibake found".to_string(),
        ));
    }
    Ok(())
}
