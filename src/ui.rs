use std::io::IsTerminal;

#[derive(Debug, Clone)]
pub struct Ui {
    color: bool,
    emoji: bool,
}

impl Ui {
    pub fn new(no_color: bool, no_emoji: bool) -> Self {
        let color =
            !no_color && std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal();
        Self {
            color,
            emoji: !no_emoji,
        }
    }

    fn paint(&self, text: impl AsRef<str>, code: &str) -> String {
        let text = text.as_ref();
        if self.color {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    pub fn bold(&self, text: impl AsRef<str>) -> String {
        self.paint(text, "1")
    }
    pub fn dim(&self, text: impl AsRef<str>) -> String {
        self.paint(text, "2")
    }
    pub fn yellow(&self, text: impl AsRef<str>) -> String {
        self.paint(text, "33")
    }
    pub fn blue(&self, text: impl AsRef<str>) -> String {
        self.paint(text, "34")
    }
    pub fn cyan(&self, text: impl AsRef<str>) -> String {
        self.paint(text, "36")
    }
    pub fn red(&self, text: impl AsRef<str>) -> String {
        self.paint(text, "31")
    }

    fn icon(&self, emoji: &'static str, plain: &'static str) -> &'static str {
        if self.emoji {
            emoji
        } else {
            plain
        }
    }

    pub fn ok(&self) -> &'static str {
        self.icon("✅", "OK")
    }
    pub fn warn(&self) -> &'static str {
        self.icon("⚠️", "WARN")
    }
    pub fn err(&self) -> &'static str {
        self.icon("❌", "ERR")
    }
    pub fn scan(&self) -> &'static str {
        self.icon("🔎", "CHECK")
    }
    pub fn backup(&self) -> &'static str {
        self.icon("💾", "BACKUP")
    }
    pub fn report(&self) -> &'static str {
        self.icon("📄", "REPORT")
    }
}
