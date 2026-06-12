use crate::encoding::FileEncoding;
use crate::error::Result;
use crate::ui::Ui;
use serde::Serialize;
use std::fmt;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportKind {
    Value,
    Comment,
    Key,
    Name,
}

impl fmt::Display for ReportKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Value => "value",
            Self::Comment => "comment",
            Self::Key => "key",
            Self::Name => "name",
        };
        f.write_str(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportStatus {
    Correctable,
    Corrected,
    StillSuspicious,
    Added,
    Updated,
    Unchanged,
    SkippedMissing,
    RejectedSuspicious,
    RejectedInvalid,
    Missing,
    Failed,
}

impl fmt::Display for ReportStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Correctable => "correctable",
            Self::Corrected => "corrected",
            Self::StillSuspicious => "still_suspicious",
            Self::Added => "added",
            Self::Updated => "updated",
            Self::Unchanged => "unchanged",
            Self::SkippedMissing => "skipped_missing",
            Self::RejectedSuspicious => "rejected_suspicious",
            Self::RejectedInvalid => "rejected_invalid",
            Self::Missing => "missing",
            Self::Failed => "failed",
        };
        f.write_str(value)
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct ReportRow {
    pub file: String,
    pub culture: String,
    pub encoding: String,
    pub resource: String,
    pub kind: ReportKind,
    pub status: ReportStatus,
    pub before_score: i32,
    pub after_score: i32,
    pub encoding_used: String,
    pub old_preview: String,
    pub new_preview: String,
}

#[allow(clippy::too_many_arguments)]
pub fn report_row(
    path: &Path,
    culture: Option<&str>,
    enc: &FileEncoding,
    resource: &str,
    kind: ReportKind,
    status: ReportStatus,
    before_score: i32,
    after_score: i32,
    encoding_used: &str,
    old_text: &str,
    new_text: &str,
) -> ReportRow {
    ReportRow {
        file: path.display().to_string(),
        culture: culture.unwrap_or("").to_string(),
        encoding: enc.name.clone(),
        resource: resource.to_string(),
        kind,
        status,
        before_score,
        after_score,
        encoding_used: encoding_used.to_string(),
        old_preview: preview(old_text, 160),
        new_preview: preview(new_text, 160),
    }
}

pub fn preview(s: &str, max: usize) -> String {
    let one_line = s.replace('\r', "\\r").replace('\n', "\\n");
    if one_line.chars().count() <= max {
        one_line
    } else {
        let mut out = one_line.chars().take(max).collect::<String>();
        out.push_str("...");
        out
    }
}

pub fn write_report_if_requested(rows: &[ReportRow], report: Option<&Path>, ui: &Ui) -> Result<()> {
    let Some(path) = report else {
        return Ok(());
    };
    let is_json = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("json"))
        .unwrap_or(false);
    if is_json {
        let file = std::fs::File::create(path)?;
        serde_json::to_writer_pretty(file, rows)?;
    } else {
        let mut writer = csv::Writer::from_path(path)?;
        for row in rows {
            writer.serialize(row)?;
        }
        writer.flush()?;
    }
    println!("{} {}", ui.report(), ui.bold(path.display().to_string()));
    Ok(())
}
