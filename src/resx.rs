use crate::cli::Mode;
use crate::encoding::{decode_file, detect_encoding, encode_file, FileEncoding};
use crate::error::{AppError, Result};
use crate::mojibake::{
    looks_unsafe_for_set, mojibake_score, repair_mojibake_text, xml_escape, xml_unescape,
};
use crate::report::{report_row, ReportKind, ReportRow, ReportStatus};
use crate::ui::Ui;
use regex::Regex;
use serde::Deserialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use walkdir::WalkDir;

static CULTURE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\.([a-z]{2,3}(?:-[a-z0-9]{2,8})*)\.resx$").expect("valid culture regex")
});
static DATA_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<data\b(?P<attrs>[^>]*)>(?P<body>.*?)</data>"#).expect("valid data regex")
});
static VALUE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<value\b[^>]*>(?P<text>.*?)</value>"#).expect("valid value regex")
});
static COMMENT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<comment\b[^>]*>(?P<text>.*?)</comment>"#).expect("valid comment regex")
});
static VALUE_CLOSE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?is)</value>"#).expect("valid value close regex"));
static DATA_INDENT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?m)^([ \t]*)<data\b"#).expect("valid data indent regex"));

#[derive(Debug, Deserialize, Clone)]
pub struct SetEntry {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub comment: Option<String>,
}

#[derive(Copy, Clone, Debug)]
pub enum Operation {
    Check,
    Repair { backup: bool, dry_run: bool },
}

#[derive(Debug, Clone)]
pub struct DataBlockInfo {
    pub start: usize,
    pub end: usize,
    pub name: String,
}

pub fn collect_resx_files(path: &Path, recursive: bool) -> Result<Vec<PathBuf>> {
    if path.is_file() {
        if extension_eq(path, "resx") {
            return Ok(vec![path.to_path_buf()]);
        }
        return Err(AppError::Message(format!(
            "not a .resx file: {}",
            path.display()
        )));
    }
    if !path.is_dir() {
        return Err(AppError::Message(format!(
            "path not found: {}",
            path.display()
        )));
    }
    let mut files = Vec::new();
    if recursive {
        for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            let p = entry.path();
            if p.is_file() && extension_eq(p, "resx") {
                files.push(p.to_path_buf());
            }
        }
    } else {
        for entry in fs::read_dir(path)? {
            let p = entry?.path();
            if p.is_file() && extension_eq(&p, "resx") {
                files.push(p);
            }
        }
    }
    files.sort();
    Ok(files)
}

pub fn extension_eq(path: &Path, expected: &str) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case(expected))
        .unwrap_or(false)
}

pub fn culture_from_path(path: &Path) -> Option<String> {
    let file_name = path.file_name()?.to_string_lossy();
    CULTURE_RE
        .captures(&file_name)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

pub fn process_file(
    path: &Path,
    mode: Mode,
    max_passes: usize,
    decode_html_entities: bool,
    include_attributes: bool,
    operation: Operation,
    ui: &Ui,
) -> Result<Vec<ReportRow>> {
    let bytes = fs::read(path)?;
    let enc = detect_encoding(&bytes)?;
    let content = decode_file(&bytes, &enc)?;
    let culture = culture_from_path(path);

    println!(
        "{} {} {} {} {} {} {}",
        ui.scan(),
        ui.bold(path.display().to_string()),
        ui.dim("| encoding:"),
        ui.cyan(&enc.name),
        ui.dim(format!("[{}]", enc.source)),
        ui.dim("| culture:"),
        ui.blue(culture.clone().unwrap_or_else(|| "neutral".to_string()))
    );

    let (new_content, mut rows, changed) = process_resx_text(
        path,
        &content,
        &enc,
        culture.as_deref(),
        mode,
        max_passes,
        decode_html_entities,
        include_attributes,
        matches!(operation, Operation::Repair { .. }),
    )?;

    if let Operation::Repair { backup, dry_run } = operation {
        if changed {
            if dry_run {
                println!(
                    "  {} would repair {}",
                    ui.warn(),
                    ui.bold(path.display().to_string())
                );
            } else {
                if backup {
                    let backup_path = backup_path_for(path);
                    fs::copy(path, &backup_path)?;
                    println!(
                        "  {} {}",
                        ui.backup(),
                        ui.dim(backup_path.display().to_string())
                    );
                }
                let out = encode_file(&new_content, &enc)?;
                let mut file = fs::File::create(path)?;
                file.write_all(&out)?;
                println!(
                    "  {} repaired {}",
                    ui.ok(),
                    ui.bold(path.display().to_string())
                );
            }
        }
    }

    if matches!(operation, Operation::Check) {
        for row in rows.iter_mut() {
            if row.status == ReportStatus::Corrected {
                row.status = ReportStatus::Correctable;
            }
        }
    }
    Ok(rows)
}

#[allow(clippy::too_many_arguments)]
pub fn process_resx_text(
    path: &Path,
    content: &str,
    enc: &FileEncoding,
    culture: Option<&str>,
    mode: Mode,
    max_passes: usize,
    decode_html_entities: bool,
    include_attributes: bool,
    apply_changes: bool,
) -> Result<(String, Vec<ReportRow>, bool)> {
    let mut out = String::with_capacity(content.len());
    let mut rows = Vec::new();
    let mut last = 0usize;
    let mut changed_any = false;

    for caps in DATA_RE.captures_iter(content) {
        let m = caps.get(0).expect("whole match");
        out.push_str(&content[last..m.start()]);
        let block = m.as_str();
        let attrs = caps.name("attrs").map(|m| m.as_str()).unwrap_or("");

        if is_non_text_data(attrs, block) {
            out.push_str(block);
            last = m.end();
            continue;
        }
        let resource = attr_value(attrs, "name").unwrap_or_else(|| "(unknown)".to_string());
        let (new_block, block_rows, block_changed) = process_data_block(
            path,
            block,
            enc,
            culture,
            mode,
            max_passes,
            decode_html_entities,
            include_attributes,
            apply_changes,
            &resource,
        );
        rows.extend(block_rows);
        changed_any |= block_changed;
        out.push_str(&new_block);
        last = m.end();
    }
    out.push_str(&content[last..]);
    Ok((out, rows, changed_any))
}

#[allow(clippy::too_many_arguments)]
fn process_data_block(
    path: &Path,
    block: &str,
    enc: &FileEncoding,
    culture: Option<&str>,
    mode: Mode,
    max_passes: usize,
    decode_html_entities: bool,
    include_attributes: bool,
    apply_changes: bool,
    resource: &str,
) -> (String, Vec<ReportRow>, bool) {
    let mut targets: Vec<(usize, usize, ReportKind)> = Vec::new();
    for caps in VALUE_RE.captures_iter(block) {
        if let Some(t) = caps.name("text") {
            targets.push((t.start(), t.end(), ReportKind::Value));
        }
    }
    for caps in COMMENT_RE.captures_iter(block) {
        if let Some(t) = caps.name("text") {
            targets.push((t.start(), t.end(), ReportKind::Comment));
        }
    }
    if include_attributes {
        if let Some((s, e)) = attr_value_range(block, "name") {
            targets.push((s, e, ReportKind::Key));
        }
    }
    targets.sort_by_key(|t| t.0);

    let mut out = String::with_capacity(block.len());
    let mut last = 0usize;
    let mut rows = Vec::new();
    let mut changed_any = false;

    for (s, e, kind) in targets {
        out.push_str(&block[last..s]);
        let raw_text = &block[s..e];
        let decoded = xml_unescape(raw_text);
        let before_score = mojibake_score(&decoded);
        if before_score == 0 && !decode_html_entities && mode != Mode::Aggressive {
            out.push_str(raw_text);
            last = e;
            continue;
        }
        let outcome = repair_mojibake_text(
            &decoded,
            mode,
            culture,
            max_passes,
            decode_html_entities || mode == Mode::Aggressive,
        );
        if outcome.changed {
            changed_any = true;
            if apply_changes {
                out.push_str(&xml_escape(&outcome.text));
            } else {
                out.push_str(raw_text);
            }
            rows.push(report_row(
                path,
                culture,
                enc,
                resource,
                kind,
                ReportStatus::Corrected,
                outcome.before_score,
                outcome.after_score,
                outcome.encoding_used.as_deref().unwrap_or(""),
                &decoded,
                &outcome.text,
            ));
        } else {
            out.push_str(raw_text);
        }
        if outcome.still_suspicious {
            rows.push(report_row(
                path,
                culture,
                enc,
                resource,
                kind,
                ReportStatus::StillSuspicious,
                outcome.before_score,
                outcome.after_score,
                outcome.encoding_used.as_deref().unwrap_or(""),
                &decoded,
                &outcome.text,
            ));
        }
        last = e;
    }
    out.push_str(&block[last..]);
    (out, rows, changed_any)
}

pub fn read_set_entries(input: &Path) -> Result<Vec<SetEntry>> {
    let is_json = input
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("json"))
        .unwrap_or(false);
    let entries = if is_json {
        let file = fs::File::open(input)?;
        serde_json::from_reader(file)?
    } else {
        let mut reader = csv::Reader::from_path(input)?;
        let mut entries = Vec::new();
        for row in reader.deserialize() {
            entries.push(row?);
        }
        entries
    };
    if entries.is_empty() {
        return Err(AppError::Message(format!(
            "input contains no key/value rows: {}",
            input.display()
        )));
    }
    Ok(entries)
}

pub fn validate_set_entry(entry: &SetEntry, allow_suspicious: bool) -> Result<()> {
    if entry.name.trim().is_empty() {
        return Err(AppError::Message(
            "resource name cannot be empty".to_string(),
        ));
    }
    if !allow_suspicious {
        let suspicious_field = if looks_unsafe_for_set(&entry.name) {
            Some("name")
        } else if looks_unsafe_for_set(&entry.value) {
            Some("value")
        } else if entry
            .comment
            .as_deref()
            .map(looks_unsafe_for_set)
            .unwrap_or(false)
        {
            Some("comment")
        } else {
            None
        };
        if let Some(field) = suspicious_field {
            return Err(AppError::Message(format!("refusing to set suspicious {field} for resource '{}'; run check/repair first or pass --allow-suspicious", entry.name)));
        }
    }
    Ok(())
}

fn invalid_import_row(
    path: &Path,
    culture: Option<&str>,
    enc: &FileEncoding,
    entry: &SetEntry,
    message: &str,
) -> ReportRow {
    let (kind, text) = if entry.name.trim().is_empty() || looks_unsafe_for_set(&entry.name) {
        (ReportKind::Name, entry.name.as_str())
    } else if looks_unsafe_for_set(&entry.value) {
        (ReportKind::Value, entry.value.as_str())
    } else if entry
        .comment
        .as_deref()
        .map(looks_unsafe_for_set)
        .unwrap_or(false)
    {
        (ReportKind::Comment, entry.comment.as_deref().unwrap_or(""))
    } else {
        (ReportKind::Value, message)
    };
    let status = if message.contains("suspicious") {
        ReportStatus::RejectedSuspicious
    } else {
        ReportStatus::RejectedInvalid
    };
    report_row(
        path,
        culture,
        enc,
        &entry.name,
        kind,
        status,
        mojibake_score(text),
        mojibake_score(text),
        "",
        text,
        message,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn upsert_resx_values_file(
    path: &Path,
    entries: &[SetEntry],
    create_missing: bool,
    allow_suspicious: bool,
    backup: bool,
    dry_run: bool,
    skip_missing: bool,
    continue_on_error: bool,
    ui: &Ui,
) -> Result<Vec<ReportRow>> {
    if !extension_eq(path, "resx") {
        return Err(AppError::Message(format!(
            "not a .resx file: {}",
            path.display()
        )));
    }
    let bytes = fs::read(path)?;
    let enc = detect_encoding(&bytes)?;
    let mut content = decode_file(&bytes, &enc)?;
    let culture = culture_from_path(path);
    let mut rows = Vec::new();
    let mut changed_any = false;
    let mut applied_count = 0usize;

    for entry in entries {
        if let Err(err) = validate_set_entry(entry, allow_suspicious) {
            let row = invalid_import_row(path, culture.as_deref(), &enc, entry, &err.to_string());
            if continue_on_error {
                println!(
                    "{} {} {}",
                    ui.warn(),
                    ui.yellow(ReportStatus::RejectedSuspicious.to_string()),
                    err
                );
                rows.push(row);
                continue;
            }
            return Err(err);
        }

        match upsert_resx_value_in_content(
            path,
            &content,
            &enc,
            culture.as_deref(),
            entry,
            create_missing,
        ) {
            Ok((new_content, row, changed)) => {
                content = new_content;
                changed_any |= changed;
                if changed {
                    applied_count += 1;
                }
                rows.push(row);
            }
            Err(AppError::Message(message)) if message.starts_with("resource not found:") => {
                let status = if skip_missing {
                    ReportStatus::SkippedMissing
                } else {
                    ReportStatus::Missing
                };
                let row = report_row(
                    path,
                    culture.as_deref(),
                    &enc,
                    &entry.name,
                    ReportKind::Value,
                    status,
                    0,
                    mojibake_score(&entry.value),
                    "",
                    "",
                    &entry.value,
                );
                if skip_missing || continue_on_error {
                    println!(
                        "{} {} {}",
                        ui.warn(),
                        ui.yellow(row.status.to_string()),
                        entry.name
                    );
                    rows.push(row);
                    continue;
                }
                return Err(AppError::Message(message));
            }
            Err(err) => {
                if continue_on_error {
                    let row = report_row(
                        path,
                        culture.as_deref(),
                        &enc,
                        &entry.name,
                        ReportKind::Value,
                        ReportStatus::Failed,
                        0,
                        mojibake_score(&entry.value),
                        "",
                        "",
                        &err.to_string(),
                    );
                    println!(
                        "{} {} {}",
                        ui.err(),
                        ui.red(ReportStatus::Failed.to_string()),
                        err
                    );
                    rows.push(row);
                    continue;
                }
                return Err(err);
            }
        }
    }

    if dry_run {
        if changed_any {
            println!(
                "{} would update {} resource(s) in {}",
                ui.warn(),
                applied_count,
                ui.bold(path.display().to_string())
            );
        } else {
            println!(
                "{} no changes needed in {}",
                ui.ok(),
                ui.dim(path.display().to_string())
            );
        }
    } else if changed_any {
        if backup {
            let backup_path = backup_path_for(path);
            fs::copy(path, &backup_path)?;
            println!(
                "{} {}",
                ui.backup(),
                ui.dim(backup_path.display().to_string())
            );
        }
        let out = encode_file(&content, &enc)?;
        let mut file = fs::File::create(path)?;
        file.write_all(&out)?;
        println!(
            "{} updated {} resource(s) in {}",
            ui.ok(),
            applied_count,
            ui.bold(path.display().to_string())
        );
    } else {
        println!(
            "{} no changes needed in {}",
            ui.ok(),
            ui.dim(path.display().to_string())
        );
    }
    Ok(rows)
}

pub fn upsert_resx_value_in_content(
    path: &Path,
    content: &str,
    enc: &FileEncoding,
    culture: Option<&str>,
    entry: &SetEntry,
    create_missing: bool,
) -> Result<(String, ReportRow, bool)> {
    if let Some(block_info) = find_data_block_by_name(content, &entry.name) {
        return update_existing_data_block(path, content, enc, culture, entry, block_info);
    }
    if !create_missing {
        return Err(AppError::Message(format!(
            "resource not found: {}",
            entry.name
        )));
    }
    add_new_data_block(path, content, enc, culture, entry)
}

fn find_data_block_by_name(content: &str, name: &str) -> Option<DataBlockInfo> {
    for caps in DATA_RE.captures_iter(content) {
        let m = caps.get(0).expect("whole match");
        let attrs = caps.name("attrs").map(|m| m.as_str()).unwrap_or("");
        if let Some(decoded_name) = attr_value(attrs, "name") {
            if decoded_name == name {
                return Some(DataBlockInfo {
                    start: m.start(),
                    end: m.end(),
                    name: decoded_name,
                });
            }
        }
    }
    None
}

fn update_existing_data_block(
    path: &Path,
    content: &str,
    enc: &FileEncoding,
    culture: Option<&str>,
    entry: &SetEntry,
    info: DataBlockInfo,
) -> Result<(String, ReportRow, bool)> {
    let block = &content[info.start..info.end];
    let value_caps = VALUE_RE
        .captures(block)
        .ok_or_else(|| AppError::Message(format!("resource has no <value>: {}", entry.name)))?;
    let text_match = value_caps.name("text").expect("text capture");
    let old_decoded = xml_unescape(text_match.as_str());
    let new_escaped = xml_escape(&entry.value);
    let mut new_block = String::with_capacity(block.len() + new_escaped.len());
    new_block.push_str(&block[..text_match.start()]);
    new_block.push_str(&new_escaped);
    new_block.push_str(&block[text_match.end()..]);
    if let Some(comment) = &entry.comment {
        new_block = upsert_comment_in_block(&new_block, comment);
    }
    let changed = new_block != block;
    let mut new_content = String::with_capacity(content.len() + new_block.len());
    new_content.push_str(&content[..info.start]);
    new_content.push_str(&new_block);
    new_content.push_str(&content[info.end..]);
    let status = if changed {
        ReportStatus::Updated
    } else {
        ReportStatus::Unchanged
    };
    let row = report_row(
        path,
        culture,
        enc,
        &info.name,
        ReportKind::Value,
        status,
        mojibake_score(&old_decoded),
        mojibake_score(&entry.value),
        "",
        &old_decoded,
        &entry.value,
    );
    Ok((new_content, row, changed))
}

fn upsert_comment_in_block(block: &str, comment: &str) -> String {
    let escaped_comment = xml_escape(comment);
    if let Some(caps) = COMMENT_RE.captures(block) {
        let text_match = caps.name("text").expect("text capture");
        let mut out = String::with_capacity(block.len() + escaped_comment.len());
        out.push_str(&block[..text_match.start()]);
        out.push_str(&escaped_comment);
        out.push_str(&block[text_match.end()..]);
        return out;
    }
    let Some(m) = VALUE_CLOSE_RE.find(block) else {
        return block.to_string();
    };
    let newline = detect_newline(block);
    let data_indent = detect_current_block_indent(block);
    let child_indent = format!("{}  ", data_indent);
    let comment_fragment = format!(
        "{}{}<comment>{}</comment>",
        newline, child_indent, escaped_comment
    );
    let mut out = String::with_capacity(block.len() + comment_fragment.len());
    out.push_str(&block[..m.end()]);
    out.push_str(&comment_fragment);
    out.push_str(&block[m.end()..]);
    out
}

fn add_new_data_block(
    path: &Path,
    content: &str,
    enc: &FileEncoding,
    culture: Option<&str>,
    entry: &SetEntry,
) -> Result<(String, ReportRow, bool)> {
    let root_end = content.rfind("</root>").ok_or_else(|| {
        AppError::Message(format!(
            "cannot add resource because </root> was not found: {}",
            path.display()
        ))
    })?;
    let newline = detect_newline(content);
    let data_indent = detect_data_indent(content).unwrap_or_else(|| "  ".to_string());
    let child_indent = format!("{}  ", data_indent);
    let mut block = String::new();
    block.push_str(&data_indent);
    block.push_str("<data name=\"");
    block.push_str(&xml_escape(&entry.name));
    block.push_str("\" xml:space=\"preserve\">");
    block.push_str(newline);
    block.push_str(&child_indent);
    block.push_str("<value>");
    block.push_str(&xml_escape(&entry.value));
    block.push_str("</value>");
    if let Some(comment) = &entry.comment {
        block.push_str(newline);
        block.push_str(&child_indent);
        block.push_str("<comment>");
        block.push_str(&xml_escape(comment));
        block.push_str("</comment>");
    }
    block.push_str(newline);
    block.push_str(&data_indent);
    block.push_str("</data>");
    block.push_str(newline);

    let mut new_content = String::with_capacity(content.len() + block.len());
    new_content.push_str(&content[..root_end]);
    if !content[..root_end].ends_with(newline) {
        new_content.push_str(newline);
    }
    new_content.push_str(&block);
    new_content.push_str(&content[root_end..]);
    let row = report_row(
        path,
        culture,
        enc,
        &entry.name,
        ReportKind::Value,
        ReportStatus::Added,
        0,
        mojibake_score(&entry.value),
        "",
        "",
        &entry.value,
    );
    Ok((new_content, row, true))
}

fn detect_newline(content: &str) -> &'static str {
    if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}
fn detect_data_indent(content: &str) -> Option<String> {
    DATA_INDENT_RE
        .captures(content)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}
fn detect_current_block_indent(block: &str) -> String {
    detect_data_indent(block).unwrap_or_else(|| "  ".to_string())
}

fn is_non_text_data(attrs: &str, block: &str) -> bool {
    let attrs_lower = attrs.to_ascii_lowercase();
    if attrs_lower.contains("resxfileref")
        || attrs_lower.contains("system.drawing")
        || attrs_lower.contains("system.byte[]")
        || attrs_lower.contains("application/x-microsoft.net.object")
        || attrs_lower.contains("application/octet-stream")
        || attrs_lower.contains("base64")
    {
        return true;
    }
    if let Some(caps) = VALUE_RE.captures(block) {
        let text = caps.name("text").map(|m| m.as_str()).unwrap_or("");
        return is_base64_like(text);
    }
    false
}

fn is_base64_like(text: &str) -> bool {
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    compact.len() >= 128
        && compact.len().is_multiple_of(4)
        && compact
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
}

pub fn attr_value(attrs: &str, name: &str) -> Option<String> {
    let n = regex::escape(name);
    let re = Regex::new(&format!(
        r#"(?is)\b{}\s*=\s*"(?P<v1>[^"]*)"|\b{}\s*=\s*'(?P<v2>[^']*)'"#,
        n, n
    ))
    .ok()?;
    let caps = re.captures(attrs)?;
    let value = caps
        .name("v1")
        .or_else(|| caps.name("v2"))
        .map(|m| m.as_str())?;
    Some(xml_unescape(value))
}

fn attr_value_range(block: &str, name: &str) -> Option<(usize, usize)> {
    let n = regex::escape(name);
    let re = Regex::new(&format!(
        r#"(?is)<data\b[^>]*\b{}\s*=\s*"(?P<v1>[^"]*)"|<data\b[^>]*\b{}\s*=\s*'(?P<v2>[^']*)'"#,
        n, n
    ))
    .ok()?;
    let caps = re.captures(block)?;
    let m = caps.name("v1").or_else(|| caps.name("v2"))?;
    Some((m.start(), m.end()))
}

pub fn backup_path_for(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_os_string();
    s.push(".bak");
    PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use encoding_rs::UTF_8;

    fn enc() -> FileEncoding {
        FileEncoding {
            encoding: UTF_8,
            name: "utf-8 without BOM".to_string(),
            bom: b"",
            source: "test".to_string(),
        }
    }

    #[test]
    fn repairs_mojibake_in_data_name_attribute_when_enabled() {
        let xml = r#"<root><data name="ImageStudio_Ã‰diteur"><value>Hello</value></data></root>"#;
        let (new_xml, rows, changed) = process_resx_text(
            Path::new("Resources.en.resx"),
            xml,
            &enc(),
            Some("en"),
            Mode::Safe,
            2,
            false,
            true,
            true,
        )
        .unwrap();
        assert!(changed);
        assert!(new_xml.contains("ImageStudio_Éditeur"));
        assert!(rows.iter().any(|r| r.kind == ReportKind::Key));
    }

    #[test]
    fn ignores_mojibake_in_data_name_attribute_when_disabled() {
        let xml = r#"<root><data name="ImageStudio_Ã‰diteur"><value>Hello</value></data></root>"#;
        let (new_xml, rows, changed) = process_resx_text(
            Path::new("Resources.en.resx"),
            xml,
            &enc(),
            Some("en"),
            Mode::Safe,
            2,
            false,
            false,
            true,
        )
        .unwrap();
        assert!(!changed);
        assert_eq!(new_xml, xml);
        assert!(rows.is_empty());
    }

    #[test]
    fn adds_missing_resource_when_create_enabled() {
        let xml = "<root>\n</root>";
        let entry = SetEntry {
            name: "Login.Button".to_string(),
            value: "Connexion".to_string(),
            comment: Some("Bouton de connexion".to_string()),
        };
        let (new_xml, row, changed) = upsert_resx_value_in_content(
            Path::new("Resources.fr.resx"),
            xml,
            &enc(),
            Some("fr"),
            &entry,
            true,
        )
        .unwrap();
        assert!(changed);
        assert_eq!(row.status, ReportStatus::Added);
        assert!(new_xml.contains(r#"<data name="Login.Button" xml:space="preserve">"#));
        assert!(new_xml.contains("<value>Connexion</value>"));
        assert!(new_xml.contains("<comment>Bouton de connexion</comment>"));
    }

    #[test]
    fn updates_existing_resource_value_without_rewriting_full_xml() {
        let xml = r#"<root><data name="Login.Button" xml:space="preserve"><value>Login</value></data></root>"#;
        let entry = SetEntry {
            name: "Login.Button".to_string(),
            value: "Connexion".to_string(),
            comment: None,
        };
        let (new_xml, row, changed) = upsert_resx_value_in_content(
            Path::new("Resources.fr.resx"),
            xml,
            &enc(),
            Some("fr"),
            &entry,
            true,
        )
        .unwrap();
        assert!(changed);
        assert_eq!(row.status, ReportStatus::Updated);
        assert_eq!(
            new_xml,
            r#"<root><data name="Login.Button" xml:space="preserve"><value>Connexion</value></data></root>"#
        );
    }

    #[test]
    fn validates_set_entries() {
        let ok = SetEntry {
            name: "Test.Key".to_string(),
            value: "Valeur accentuée éàç".to_string(),
            comment: None,
        };
        assert!(validate_set_entry(&ok, false).is_ok());
        let bad = SetEntry {
            name: "Test.Key".to_string(),
            value: "CrÃ©ez un visuel avec lâ€™éditeur".to_string(),
            comment: None,
        };
        assert!(validate_set_entry(&bad, false).is_err());
    }
}
