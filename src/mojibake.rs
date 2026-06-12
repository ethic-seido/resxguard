use crate::cli::Mode;
use encoding_rs::{Encoding, UTF_8};
use regex::Regex;
use std::sync::LazyLock;

static MOJIBAKE_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"Ã.",
        r"Â.",
        r"â€.",
        r"â..",
        r"ä[\u{00A0}-\u{00BF}]",
        r"æ[\u{00A0}-\u{00BF}\u{2010}-\u{2027}]",
        r"å[\u{00A0}-\u{00BF}]",
        r"Ø[\u{0080}-\u{00FF}]",
        r"Ù[\u{0080}-\u{00FF}]",
        r"Ð[\u{0080}-\u{00FF}]",
        r"Ñ[\u{0080}-\u{00FF}]",
        r"[\u{0080}-\u{009F}]",
        r"�",
    ]
    .into_iter()
    .map(|p| Regex::new(p).expect("valid mojibake regex"))
    .collect()
});

static STRONG_MOJIBAKE_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"Ã[\u{0080}-\u{00BF}]",
        r"Â[\u{0080}-\u{00BF}]",
        r"â[\u{0080}-\u{00BF}\u{2010}-\u{2122}]",
        r"ä[\u{0080}-\u{00BF}]",
        r"æ[\u{0080}-\u{00BF}\u{2010}-\u{2027}]",
        r"å[\u{0080}-\u{00BF}]",
        r"Ø[\u{0080}-\u{00FF}]",
        r"Ù[\u{0080}-\u{00FF}]",
        r"Ð[\u{0080}-\u{00FF}]",
        r"Ñ[\u{0080}-\u{00FF}]",
    ]
    .into_iter()
    .map(|p| Regex::new(p).expect("valid strong mojibake regex"))
    .collect()
});

static HEX_ENTITY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"&#x([0-9A-Fa-f]+);").expect("valid hex entity regex"));
static DEC_ENTITY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"&#([0-9]+);").expect("valid dec entity regex"));

#[derive(Debug, Clone)]
pub struct RepairOutcome {
    pub text: String,
    pub changed: bool,
    pub before_score: i32,
    pub after_score: i32,
    pub encoding_used: Option<String>,
    pub still_suspicious: bool,
}

pub fn repair_mojibake_text(
    text: &str,
    mode: Mode,
    culture: Option<&str>,
    max_passes: usize,
    decode_html_entities: bool,
) -> RepairOutcome {
    let original = text.to_string();
    let mut current = if decode_html_entities {
        decode_numeric_and_basic_entities(text)
    } else {
        text.to_string()
    };
    let before = mojibake_score(&current);
    let candidates = wrong_encoding_candidates(mode);
    let mut encoding_used = None;

    for _ in 0..max_passes {
        let current_moji = mojibake_score(&current);
        if current_moji == 0 {
            break;
        }
        let current_quality = quality_score(&current, culture);
        let mut best_text = current.clone();
        let mut best_quality = current_quality;
        let mut best_moji = current_moji;
        let mut best_encoding = None;

        for enc in &candidates {
            let (bytes, _, encode_errors) = enc.encode(&current);
            if encode_errors {
                continue;
            }
            let (candidate, decode_errors) = UTF_8.decode_without_bom_handling(&bytes);
            if decode_errors {
                continue;
            }
            let candidate = candidate.into_owned();
            let candidate_moji = mojibake_score(&candidate);
            let candidate_quality = quality_score(&candidate, culture);
            let accept = match mode {
                Mode::Safe => candidate_moji < best_moji && candidate_quality < best_quality,
                Mode::Broad => {
                    candidate_moji < best_moji
                        || (candidate_quality < best_quality && candidate_moji <= best_moji)
                }
                Mode::Aggressive => candidate_quality < best_quality,
            };
            if accept {
                best_moji = candidate_moji;
                best_quality = candidate_quality;
                best_encoding = Some(enc.name().to_string());
                best_text = candidate;
            }
        }
        if best_text == current {
            break;
        }
        if best_encoding.is_some() {
            encoding_used = best_encoding;
        }
        current = best_text;
    }

    if has_strong_mojibake_marker(&current) {
        if let Some(segment_repaired) = repair_mojibake_segments(&current, mode) {
            current = segment_repaired;
            encoding_used.get_or_insert_with(|| "segment-repair".to_string());
        }
    }

    let after = mojibake_score(&current);
    RepairOutcome {
        text: current.clone(),
        changed: current != original,
        before_score: before,
        after_score: after,
        encoding_used,
        still_suspicious: after > 0,
    }
}

pub fn wrong_encoding_candidates(mode: Mode) -> Vec<&'static Encoding> {
    let labels: &[&[u8]] = match mode {
        Mode::Safe => &[b"windows-1252", b"iso-8859-1"],
        Mode::Broad => &[
            b"windows-1252",
            b"iso-8859-1",
            b"iso-8859-15",
            b"macintosh",
            b"windows-1250",
            b"windows-1251",
            b"windows-1256",
        ],
        Mode::Aggressive => &[
            b"windows-1252",
            b"iso-8859-1",
            b"iso-8859-15",
            b"macintosh",
            b"windows-1250",
            b"windows-1251",
            b"windows-1256",
            b"windows-1253",
            b"windows-1255",
            b"shift_jis",
            b"gbk",
            b"big5",
            b"euc-kr",
        ],
    };
    labels
        .iter()
        .filter_map(|l| Encoding::for_label(l))
        .collect()
}

pub fn mojibake_score(text: &str) -> i32 {
    if text.is_empty() {
        return 0;
    }
    MOJIBAKE_PATTERNS
        .iter()
        .map(|re| re.find_iter(text).count() as i32)
        .sum()
}

pub fn quality_score(text: &str, culture: Option<&str>) -> i32 {
    mojibake_score(text) * 100 - expected_script_score(text, culture) * 2
}

pub fn expected_script_score(text: &str, culture: Option<&str>) -> i32 {
    let Some(culture) = culture else {
        return 0;
    };
    let lang = culture.split('-').next().unwrap_or("").to_ascii_lowercase();
    text.chars()
        .filter(|&ch| match lang.as_str() {
            "ar" | "fa" | "ur" => in_range(ch, 0x0600, 0x06FF) || in_range(ch, 0x0750, 0x077F),
            "he" => in_range(ch, 0x0590, 0x05FF),
            "ru" | "uk" | "bg" => in_range(ch, 0x0400, 0x04FF),
            "el" => in_range(ch, 0x0370, 0x03FF),
            "zh" => in_range(ch, 0x4E00, 0x9FFF),
            "ja" => {
                in_range(ch, 0x4E00, 0x9FFF)
                    || in_range(ch, 0x3040, 0x309F)
                    || in_range(ch, 0x30A0, 0x30FF)
            }
            "ko" => in_range(ch, 0xAC00, 0xD7AF) || in_range(ch, 0x1100, 0x11FF),
            "th" => in_range(ch, 0x0E00, 0x0E7F),
            _ => ch.is_ascii() || in_range(ch, 0x00C0, 0x024F),
        })
        .count() as i32
}

fn in_range(ch: char, start: u32, end: u32) -> bool {
    let c = ch as u32;
    c >= start && c <= end
}

pub fn looks_unsafe_for_set(text: &str) -> bool {
    has_strong_mojibake_marker(text)
}

pub fn has_strong_mojibake_marker(text: &str) -> bool {
    text.contains('�') || STRONG_MOJIBAKE_PATTERNS.iter().any(|re| re.is_match(text))
}

pub fn decode_numeric_and_basic_entities(s: &str) -> String {
    let mut out = s
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&");
    out = HEX_ENTITY_RE
        .replace_all(&out, |caps: &regex::Captures| {
            u32::from_str_radix(&caps[1], 16)
                .ok()
                .and_then(char::from_u32)
                .map(|c| c.to_string())
                .unwrap_or_else(|| caps[0].to_string())
        })
        .into_owned();
    DEC_ENTITY_RE
        .replace_all(&out, |caps: &regex::Captures| {
            caps[1]
                .parse::<u32>()
                .ok()
                .and_then(char::from_u32)
                .map(|c| c.to_string())
                .unwrap_or_else(|| caps[0].to_string())
        })
        .into_owned()
}

fn repair_mojibake_segments(text: &str, mode: Mode) -> Option<String> {
    let candidates = wrong_encoding_candidates(mode);
    let mut out = String::with_capacity(text.len());
    let mut segment = String::new();
    let mut segment_has_marker = false;
    let mut changed = false;

    fn flush_segment(
        out: &mut String,
        segment: &mut String,
        segment_has_marker: &mut bool,
        changed: &mut bool,
        candidates: &[&'static Encoding],
    ) {
        if segment.is_empty() {
            return;
        }

        if *segment_has_marker {
            for enc in candidates {
                let (bytes, _, encode_errors) = enc.encode(segment.as_str());
                if encode_errors {
                    continue;
                }
                let (candidate, decode_errors) = UTF_8.decode_without_bom_handling(&bytes);
                if decode_errors {
                    continue;
                }
                let candidate = candidate.into_owned();
                if candidate != *segment && mojibake_score(&candidate) < mojibake_score(segment) {
                    out.push_str(&candidate);
                    *changed = true;
                    segment.clear();
                    *segment_has_marker = false;
                    return;
                }
            }
        }

        out.push_str(segment);
        segment.clear();
        *segment_has_marker = false;
    }

    for ch in text.chars() {
        let marker = is_mojibake_marker_char(ch);
        let continuation = ch.is_ascii() || marker || is_mojibake_continuation_char(ch);

        if continuation {
            segment.push(ch);
            segment_has_marker |= marker;
        } else {
            flush_segment(
                &mut out,
                &mut segment,
                &mut segment_has_marker,
                &mut changed,
                &candidates,
            );
            out.push(ch);
        }
    }

    flush_segment(
        &mut out,
        &mut segment,
        &mut segment_has_marker,
        &mut changed,
        &candidates,
    );

    changed.then_some(out)
}

fn is_mojibake_marker_char(ch: char) -> bool {
    matches!(
        ch,
        'Ã' | 'Â' | 'â' | 'ä' | 'æ' | 'å' | 'Ø' | 'Ù' | 'Ð' | 'Ñ'
    )
}

fn is_mojibake_continuation_char(ch: char) -> bool {
    let c = ch as u32;
    (0x0080..=0x00BF).contains(&c) || (0x2010..=0x2122).contains(&c)
}

pub fn xml_unescape(s: &str) -> String {
    decode_numeric_and_basic_entities(s)
}

pub fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repairs_french_mojibake() {
        let r = repair_mojibake_text("FranÃ§ais", Mode::Safe, Some("fr"), 2, false);
        assert_eq!(r.text, "Français");
    }

    #[test]
    fn repairs_arabic_mojibake() {
        let r = repair_mojibake_text(
            "\u{00D9}\u{2026}\u{00D8}\u{00B1}\u{00D8}\u{00AD}\u{00D8}\u{00A8}\u{00D8}\u{00A7}",
            Mode::Safe,
            Some("ar"),
            2,
            false,
        );
        assert_eq!(r.text, "مرحبا");
    }

    #[test]
    fn repairs_cjk_mojibake() {
        let r = repair_mojibake_text(
            "\u{00E4}\u{00B8}\u{00AD}\u{00E6}\u{2013}\u{2021}",
            Mode::Safe,
            Some("zh-CN"),
            2,
            false,
        );
        assert_eq!(r.text, "中文");
    }

    #[test]
    fn keeps_correct_unicode() {
        let r = repair_mojibake_text("中文 العربية Français", Mode::Safe, Some("zh-CN"), 2, false);
        assert_eq!(r.text, "中文 العربية Français");
        assert!(!r.changed);
    }

    #[test]
    fn flags_mixed_mojibake_for_set() {
        assert!(looks_unsafe_for_set("CrÃ©ez un visuel avec lâ€™éditeur"));
    }

    #[test]
    fn repairs_mixed_mojibake_without_touching_valid_accents() {
        let r = repair_mojibake_text(
            "CrÃ©ez un visuel avec lâ€™éditeur",
            Mode::Safe,
            Some("fr"),
            2,
            false,
        );
        assert_eq!(r.text, "Créez un visuel avec l’éditeur");
        assert!(r.changed);
    }

    #[test]
    fn allows_valid_accents_for_set() {
        assert!(!looks_unsafe_for_set("Valeur accentuée éàç"));
        assert!(!looks_unsafe_for_set("Âge de l’utilisateur"));
    }
}
