//! Line ending detection and conversion.

/// Line ending style of a text file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LineEnding {
    /// Unix style, `\n`.
    Lf,
    /// Windows style, `\r\n`.
    Crlf,
}

impl LineEnding {
    /// The raw characters of the line ending.
    pub fn as_str(&self) -> &'static str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::Crlf => "\r\n",
        }
    }

    /// Human readable label.
    pub fn label(&self) -> &'static str {
        match self {
            LineEnding::Lf => "LF",
            LineEnding::Crlf => "CRLF",
        }
    }

    /// Finds a line ending by its label, case insensitively.
    pub fn from_label(label: &str) -> Option<Self> {
        match label.trim().to_ascii_uppercase().as_str() {
            "LF" | "UNIX" => Some(LineEnding::Lf),
            "CRLF" | "WINDOWS" => Some(LineEnding::Crlf),
            _ => None,
        }
    }

    /// Detects the dominant line ending of a text.
    pub fn detect(text: &str) -> Self {
        let crlf = text.matches("\r\n").count();
        let total_lf = text.matches('\n').count();
        let lone_lf = total_lf.saturating_sub(crlf);
        if crlf > lone_lf {
            LineEnding::Crlf
        } else {
            LineEnding::Lf
        }
    }

    /// Converts a text to the requested line ending style.
    pub fn convert(text: &str, target: Self) -> String {
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        match target {
            LineEnding::Lf => normalized,
            LineEnding::Crlf => normalized.replace('\n', "\r\n"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_crlf() {
        assert_eq!(LineEnding::detect("a\r\nb\r\n"), LineEnding::Crlf);
    }

    #[test]
    fn detects_lf() {
        assert_eq!(LineEnding::detect("a\nb\n"), LineEnding::Lf);
    }

    #[test]
    fn mixed_content_follows_the_majority() {
        assert_eq!(LineEnding::detect("a\r\nb\r\nc\n"), LineEnding::Crlf);
        assert_eq!(LineEnding::detect("a\nb\nc\r\n"), LineEnding::Lf);
    }

    #[test]
    fn converts_between_styles() {
        assert_eq!(LineEnding::convert("a\r\nb", LineEnding::Lf), "a\nb");
        assert_eq!(LineEnding::convert("a\nb", LineEnding::Crlf), "a\r\nb");
        assert_eq!(LineEnding::convert("a\r\nb", LineEnding::Crlf), "a\r\nb");
    }

    #[test]
    fn lone_carriage_returns_are_normalised() {
        assert_eq!(LineEnding::convert("a\rb", LineEnding::Lf), "a\nb");
    }

    #[test]
    fn labels_are_reversible() {
        assert_eq!(LineEnding::from_label("lf"), Some(LineEnding::Lf));
        assert_eq!(LineEnding::from_label("CRLF"), Some(LineEnding::Crlf));
        assert_eq!(LineEnding::from_label("nope"), None);
    }
}
