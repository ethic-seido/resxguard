use crate::error::{AppError, Result};
use encoding_rs::{Encoding, UTF_16BE, UTF_16LE, UTF_8, WINDOWS_1252};
use regex::Regex;
use std::borrow::Cow;
use std::sync::LazyLock;

static XML_ENCODING_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)<\?xml\s+[^>]*encoding\s*=\s*["']([^"']+)["']"#)
        .expect("valid XML encoding regex")
});

#[derive(Debug, Clone)]
pub struct FileEncoding {
    pub encoding: &'static Encoding,
    pub name: String,
    pub bom: &'static [u8],
    pub source: String,
}

pub fn detect_encoding(bytes: &[u8]) -> Result<FileEncoding> {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return Ok(FileEncoding {
            encoding: UTF_8,
            name: "UTF-8 with BOM".to_string(),
            bom: &[0xEF, 0xBB, 0xBF],
            source: "BOM".to_string(),
        });
    }
    if bytes.starts_with(&[0xFF, 0xFE, 0x00, 0x00]) || bytes.starts_with(&[0x00, 0x00, 0xFE, 0xFF])
    {
        return Err(AppError::UnsupportedEncoding(
            "UTF-32 is not supported".to_string(),
        ));
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return Ok(FileEncoding {
            encoding: UTF_16LE,
            name: "UTF-16 LE with BOM".to_string(),
            bom: &[0xFF, 0xFE],
            source: "BOM".to_string(),
        });
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        return Ok(FileEncoding {
            encoding: UTF_16BE,
            name: "UTF-16 BE with BOM".to_string(),
            bom: &[0xFE, 0xFF],
            source: "BOM".to_string(),
        });
    }

    if let Some(label) = xml_declared_encoding(bytes) {
        if let Some(enc) = Encoding::for_label(label.as_bytes()) {
            return Ok(FileEncoding {
                encoding: enc,
                name: format!("{} without BOM", label),
                bom: &[],
                source: "XML declaration".to_string(),
            });
        }
    }

    if std::str::from_utf8(bytes).is_ok() {
        return Ok(FileEncoding {
            encoding: UTF_8,
            name: "UTF-8 without BOM".to_string(),
            bom: &[],
            source: "UTF-8 strict validation".to_string(),
        });
    }

    Ok(FileEncoding {
        encoding: WINDOWS_1252,
        name: "Windows-1252".to_string(),
        bom: &[],
        source: "fallback".to_string(),
    })
}

fn xml_declared_encoding(bytes: &[u8]) -> Option<String> {
    let prefix_len = bytes.len().min(512);
    let prefix = String::from_utf8_lossy(&bytes[..prefix_len]);
    XML_ENCODING_RE
        .captures(&prefix)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

pub fn decode_file(bytes: &[u8], enc: &FileEncoding) -> Result<String> {
    let body = if !enc.bom.is_empty() && bytes.starts_with(enc.bom) {
        &bytes[enc.bom.len()..]
    } else {
        bytes
    };
    let (cow, had_errors) = enc.encoding.decode_without_bom_handling(body);
    if had_errors {
        return Err(AppError::Message(format!(
            "decoding errors using {}",
            enc.name
        )));
    }
    Ok(cow.into_owned())
}

pub fn encode_file(text: &str, enc: &FileEncoding) -> Result<Vec<u8>> {
    let (cow, _, had_errors) = enc.encoding.encode(text);
    if had_errors {
        return Err(AppError::Message(format!(
            "encoding errors using {}",
            enc.name
        )));
    }
    let mut out = Vec::new();
    out.extend_from_slice(enc.bom);
    match cow {
        Cow::Borrowed(b) => out.extend_from_slice(b),
        Cow::Owned(b) => out.extend_from_slice(&b),
    }
    Ok(out)
}
