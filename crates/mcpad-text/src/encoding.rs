//! Character encodings supported by the editor.

use encoding_rs::{KOI8_R, WINDOWS_1251};

/// Encoding of a text file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EncodingKind {
    /// UTF-8 without a byte order mark.
    Utf8,
    /// UTF-8 with a byte order mark.
    Utf8Bom,
    /// UTF-16 little endian, written with a byte order mark.
    Utf16Le,
    /// UTF-16 big endian, written with a byte order mark.
    Utf16Be,
    /// Windows-1251, the classic Russian single byte encoding.
    Windows1251,
    /// KOI8-R.
    Koi8R,
    /// Plain US-ASCII.
    Ascii,
}

impl EncodingKind {
    /// Every supported encoding, in the order shown in the settings UI.
    pub const ALL: [EncodingKind; 7] = [
        EncodingKind::Utf8,
        EncodingKind::Utf8Bom,
        EncodingKind::Utf16Le,
        EncodingKind::Utf16Be,
        EncodingKind::Windows1251,
        EncodingKind::Koi8R,
        EncodingKind::Ascii,
    ];

    /// Human readable label.
    pub fn label(&self) -> &'static str {
        match self {
            EncodingKind::Utf8 => "UTF-8",
            EncodingKind::Utf8Bom => "UTF-8 with BOM",
            EncodingKind::Utf16Le => "UTF-16 LE",
            EncodingKind::Utf16Be => "UTF-16 BE",
            EncodingKind::Windows1251 => "Windows-1251",
            EncodingKind::Koi8R => "KOI8-R",
            EncodingKind::Ascii => "US-ASCII",
        }
    }

    /// Finds an encoding by its label, case insensitively.
    pub fn from_label(label: &str) -> Option<Self> {
        EncodingKind::ALL
            .into_iter()
            .find(|kind| kind.label().eq_ignore_ascii_case(label.trim()))
    }

    /// Detects the encoding of a byte buffer.
    ///
    /// Byte order marks win. UTF-16 without a BOM is detected heuristically from
    /// the distribution of zero bytes. Everything that is valid UTF-8 stays
    /// UTF-8; the two legacy single byte encodings are told apart by counting
    /// replacement characters.
    pub fn detect(bytes: &[u8]) -> Self {
        if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
            return EncodingKind::Utf8Bom;
        }
        if bytes.starts_with(&[0xFF, 0xFE]) {
            return EncodingKind::Utf16Le;
        }
        if bytes.starts_with(&[0xFE, 0xFF]) {
            return EncodingKind::Utf16Be;
        }
        if let Some(kind) = Self::detect_utf16_without_bom(bytes) {
            return kind;
        }
        if bytes.is_ascii() {
            return EncodingKind::Ascii;
        }
        match std::str::from_utf8(bytes) {
            Ok(_) => EncodingKind::Utf8,
            Err(_) => Self::detect_legacy(bytes),
        }
    }

    /// Heuristic for UTF-16 files saved without a byte order mark.
    ///
    /// Both byte orders are tried and the one that produces plausible text
    /// wins. "Plausible" means the decoded characters are mostly letters,
    /// digits and punctuation instead of replacement characters or control
    /// codes. A file without a BOM is genuinely ambiguous when its text is
    /// short, in that case the encoding is left to the user.
    fn detect_utf16_without_bom(bytes: &[u8]) -> Option<EncodingKind> {
        let sample_len = (bytes.len().min(8192) / 2) * 2;
        if sample_len < 16 {
            return None;
        }
        let sample = &bytes[..sample_len];

        let little = utf16_score(sample, true);
        let big = utf16_score(sample, false);

        match (little, big) {
            (l, b) if l >= 0.8 && b <= 0.4 => Some(EncodingKind::Utf16Le),
            (l, b) if b >= 0.8 && l <= 0.4 => Some(EncodingKind::Utf16Be),
            _ => None,
        }
    }

    /// Tells Windows-1251 and KOI8-R apart by counting decoding errors.
    fn detect_legacy(bytes: &[u8]) -> EncodingKind {
        let cp1251_errors = WINDOWS_1251.decode(bytes).0.matches('\u{FFFD}').count();
        let koi8_errors = KOI8_R.decode(bytes).0.matches('\u{FFFD}').count();
        if koi8_errors < cp1251_errors {
            EncodingKind::Koi8R
        } else {
            EncodingKind::Windows1251
        }
    }

    /// Decodes a byte buffer into a string. Invalid bytes become `U+FFFD`.
    pub fn decode(&self, bytes: &[u8]) -> String {
        match self {
            EncodingKind::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
            EncodingKind::Utf8Bom => {
                let body = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
                String::from_utf8_lossy(body).into_owned()
            }
            EncodingKind::Utf16Le => decode_utf16(bytes, true),
            EncodingKind::Utf16Be => decode_utf16(bytes, false),
            EncodingKind::Windows1251 => WINDOWS_1251.decode(bytes).0.into_owned(),
            EncodingKind::Koi8R => KOI8_R.decode(bytes).0.into_owned(),
            EncodingKind::Ascii => String::from_utf8_lossy(bytes).into_owned(),
        }
    }

    /// Encodes a string. UTF-16 variants always carry a byte order mark so the
    /// encoding stays detectable after a round trip.
    pub fn encode(&self, text: &str) -> Vec<u8> {
        match self {
            EncodingKind::Utf8 => text.as_bytes().to_vec(),
            EncodingKind::Utf8Bom => {
                let mut out = vec![0xEF, 0xBB, 0xBF];
                out.extend_from_slice(text.as_bytes());
                out
            }
            EncodingKind::Utf16Le => {
                let mut out = vec![0xFF, 0xFE];
                for unit in text.encode_utf16() {
                    out.extend_from_slice(&unit.to_le_bytes());
                }
                out
            }
            EncodingKind::Utf16Be => {
                let mut out = vec![0xFE, 0xFF];
                for unit in text.encode_utf16() {
                    out.extend_from_slice(&unit.to_be_bytes());
                }
                out
            }
            EncodingKind::Windows1251 => {
                let (bytes, _, _) = WINDOWS_1251.encode(text);
                bytes.into_owned()
            }
            EncodingKind::Koi8R => {
                let (bytes, _, _) = KOI8_R.encode(text);
                bytes.into_owned()
            }
            EncodingKind::Ascii => text.as_bytes().to_vec(),
        }
    }
}

/// Decodes UTF-16 bytes, ignoring a leading byte order mark.
///
/// The decoder is implemented here instead of using `encoding_rs` so that the
/// byte order mark stays under our control: what we write is what we read
/// back, byte for byte.
fn decode_utf16(bytes: &[u8], little_endian: bool) -> String {
    let body = match (little_endian, bytes) {
        (true, [0xFF, 0xFE, rest @ ..]) => rest,
        (false, [0xFE, 0xFF, rest @ ..]) => rest,
        _ => bytes,
    };

    let units: Vec<u16> = body
        .chunks_exact(2)
        .map(|pair| {
            if little_endian {
                u16::from_le_bytes([pair[0], pair[1]])
            } else {
                u16::from_be_bytes([pair[0], pair[1]])
            }
        })
        .collect();

    String::from_utf16_lossy(&units)
}

/// Whether a character is plausible for a text file.
fn is_common_text_char(value: char) -> bool {
    matches!(value,
        '\u{0009}'..='\u{000D}'          // tab, line feed, carriage return
        | ' '..='~'                     // ASCII printable
        | '\u{00A0}'..='\u{00FF}'        // Latin-1 supplement
        | '\u{0370}'..='\u{03FF}'        // Greek
        | '\u{0400}'..='\u{04FF}'        // Cyrillic
        | '\u{2010}'..='\u{206F}'        // general punctuation
    )
}

/// Scores how much a byte sample looks like text when read as UTF-16.
///
/// The score is the share of plausible characters minus the share of
/// replacement characters, so binary noise scores low.
fn utf16_score(sample: &[u8], little_endian: bool) -> f64 {
    let units: Vec<u16> = sample
        .chunks_exact(2)
        .map(|pair| {
            if little_endian {
                u16::from_le_bytes([pair[0], pair[1]])
            } else {
                u16::from_be_bytes([pair[0], pair[1]])
            }
        })
        .collect();

    let text = String::from_utf16_lossy(&units);
    let total = text.chars().count();
    if total == 0 {
        return 0.0;
    }

    let common = text.chars().filter(|c| is_common_text_char(*c)).count();
    let broken = text.chars().filter(|c| *c == '\u{FFFD}').count();

    (common as f64 / total as f64) - (broken as f64 / total as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_utf8_without_bom() {
        assert_eq!(
            EncodingKind::detect("привет".as_bytes()),
            EncodingKind::Utf8
        );
    }

    #[test]
    fn detects_utf8_with_bom() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice("hello".as_bytes());
        assert_eq!(EncodingKind::detect(&bytes), EncodingKind::Utf8Bom);
    }

    #[test]
    fn detects_utf16_with_bom() {
        let le = EncodingKind::Utf16Le.encode("hello");
        let be = EncodingKind::Utf16Be.encode("hello");
        assert_eq!(EncodingKind::detect(&le), EncodingKind::Utf16Le);
        assert_eq!(EncodingKind::detect(&be), EncodingKind::Utf16Be);
    }

    #[test]
    fn round_trips_every_encoding() {
        for kind in EncodingKind::ALL {
            let text = "Привет, мир! 42";
            let bytes = kind.encode(text);
            assert_eq!(kind.decode(&bytes), text, "round trip failed for {kind:?}");
        }
    }

    #[test]
    fn detects_ascii() {
        assert_eq!(EncodingKind::detect(b"plain text"), EncodingKind::Ascii);
    }

    #[test]
    fn legacy_encodings_do_not_produce_replacement_characters() {
        let text = "Привет мир";
        let cp1251 = WINDOWS_1251.encode(text).0.into_owned();
        assert_eq!(EncodingKind::detect(&cp1251), EncodingKind::Windows1251);
        assert_eq!(EncodingKind::Windows1251.decode(&cp1251), text);
    }

    #[test]
    fn utf16_without_bom_is_detected_heuristically() {
        let text = "MCPad тест построчно".repeat(20);

        let mut little: Vec<u8> = Vec::new();
        for unit in text.encode_utf16() {
            little.extend_from_slice(&unit.to_le_bytes());
        }
        assert_eq!(EncodingKind::detect(&little), EncodingKind::Utf16Le);

        let mut big: Vec<u8> = Vec::new();
        for unit in text.encode_utf16() {
            big.extend_from_slice(&unit.to_be_bytes());
        }
        assert_eq!(EncodingKind::detect(&big), EncodingKind::Utf16Be);
    }

    #[test]
    fn plain_utf8_is_never_mistaken_for_utf16() {
        let text = "обычный текст без BOM, много букв\n".repeat(20);
        assert_eq!(EncodingKind::detect(text.as_bytes()), EncodingKind::Utf8);
    }

    #[test]
    fn labels_are_reversible() {
        for kind in EncodingKind::ALL {
            assert_eq!(EncodingKind::from_label(kind.label()), Some(kind));
        }
        assert_eq!(EncodingKind::from_label("utf-8"), Some(EncodingKind::Utf8));
    }
}
