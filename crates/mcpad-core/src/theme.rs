//! Theme documents on the Rust side.
//!
//! The structs below mirror `theme/schema/theme.schema.json` and the TypeScript
//! definitions in `apps/desktop/src/theme/types.ts`. A theme file can therefore
//! be written once and read by both sides.
//!
//! Loading is forgiving by design: a partial user theme is merged over the
//! built-in default, every missing or malformed token falls back to the
//! default value, and the problems are reported instead of thrown. A broken
//! theme must never stop the editor from starting.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{Error, Result};

/// Colour mode of a theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeMode {
    /// Light theme.
    Light,
    /// Dark theme.
    Dark,
    /// Maximum contrast theme.
    HighContrast,
}

impl ThemeMode {
    /// The value used in CSS and in the `data-theme-mode` attribute.
    pub fn as_str(&self) -> &'static str {
        match self {
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
            ThemeMode::HighContrast => "high-contrast",
        }
    }
}

/// A complete theme document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    /// Display name.
    pub name: String,
    /// Schema version of the document.
    pub version: u32,
    /// Colour mode.
    pub mode: ThemeMode,
    /// Optional author.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Optional description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Icon name to relative path of an SVG file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icons: Option<BTreeMap<String, String>>,
    /// The tokens themselves.
    pub tokens: ThemeTokens,
}

/// The token tree of a theme.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeTokens {
    /// Colours.
    pub color: ColorTokens,
    /// Typography.
    pub typography: TypographyTokens,
    /// Corner radii.
    #[serde(default)]
    pub radius: BTreeMap<String, String>,
    /// Shadows.
    #[serde(default)]
    pub shadow: BTreeMap<String, String>,
    /// Interface density.
    pub density: DensityTokens,
    /// Icon settings.
    pub icon: IconTokens,
}

/// Colours of a theme.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorTokens {
    /// Raw palette, keyed by hue and step.
    #[serde(default)]
    pub primitive: BTreeMap<String, BTreeMap<String, String>>,
    /// Semantic colours, the contract every component uses.
    pub semantic: SemanticColorTokens,
    /// Component level overrides.
    #[serde(default)]
    pub component: BTreeMap<String, BTreeMap<String, String>>,
}

/// Semantic colours.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticColorTokens {
    /// Backgrounds.
    pub surface: SurfaceTokens,
    /// Text.
    pub text: TextTokens,
    /// Borders.
    pub border: BorderTokens,
    /// Accent colour.
    pub accent: AccentTokens,
    /// Status colours.
    pub status: StatusTokens,
    /// Code editor.
    pub editor: EditorTokens,
    /// Infinite canvas.
    pub canvas: CanvasTokens,
    /// Diff view.
    pub diff: DiffTokens,
}

/// Background tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceTokens {
    /// Main background.
    pub base: String,
    /// Raised surfaces.
    pub raised: String,
    /// Sunken surfaces.
    pub sunken: String,
    /// Overlays.
    pub overlay: String,
    /// Hover state.
    pub hover: String,
    /// Active state.
    pub active: String,
    /// Selection.
    pub selected: String,
}

/// Text tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextTokens {
    /// Primary text.
    pub primary: String,
    /// Secondary text.
    pub secondary: String,
    /// Muted text.
    pub muted: String,
    /// Text on an inverted surface.
    pub inverse: String,
    /// Links.
    pub link: String,
    /// Inline code.
    pub code: String,
}

/// Border tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BorderTokens {
    /// Default border.
    pub default: String,
    /// Strong border.
    pub strong: String,
    /// Subtle border.
    pub subtle: String,
    /// Focus ring.
    pub focus: String,
}

/// Accent tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccentTokens {
    /// Accent colour.
    pub default: String,
    /// Hover state.
    pub hover: String,
    /// Active state.
    pub active: String,
    /// Tinted background.
    pub subtle: String,
    /// Text on the accent colour.
    pub on: String,
}

/// Status tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusTokens {
    /// Success.
    pub success: String,
    /// Warning.
    pub warning: String,
    /// Danger.
    pub danger: String,
    /// Information.
    pub info: String,
}

/// Editor tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorTokens {
    /// Editor background.
    pub bg: String,
    /// Gutter background.
    pub gutter_bg: String,
    /// Gutter text.
    pub gutter_text: String,
    /// Active line.
    pub active_line: String,
    /// Selection.
    pub selection: String,
    /// Selection when the editor is not focused.
    pub selection_inactive: String,
    /// Caret.
    pub cursor: String,
    /// Matching bracket.
    pub matching_bracket: String,
    /// Indent guide.
    pub indent_guide: String,
    /// Search match.
    pub find_match: String,
    /// Current search match.
    pub find_match_current: String,
}

/// Canvas tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasTokens {
    /// Canvas background.
    pub bg: String,
    /// Grid dots.
    pub grid: String,
    /// Major grid lines.
    pub grid_major: String,
    /// Node background.
    pub node_bg: String,
    /// Node border.
    pub node_border: String,
    /// Selected node.
    pub node_selected: String,
    /// Node text.
    pub node_text: String,
    /// Edge colour.
    pub edge: String,
    /// Edge label.
    pub edge_label: String,
    /// Group background.
    pub group_bg: String,
    /// Group border.
    pub group_border: String,
    /// Group label.
    pub group_label: String,
}

/// Diff tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffTokens {
    /// Added lines.
    pub added: String,
    /// Removed lines.
    pub removed: String,
    /// Modified lines.
    pub modified: String,
}

/// Typography tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypographyTokens {
    /// Font families.
    #[serde(default)]
    pub font_family: BTreeMap<String, String>,
    /// Font sizes.
    #[serde(default)]
    pub font_size: BTreeMap<String, String>,
    /// Font weights.
    #[serde(default)]
    pub font_weight: BTreeMap<String, u16>,
    /// Line heights.
    #[serde(default)]
    pub line_height: BTreeMap<String, f64>,
    /// Letter spacing.
    #[serde(default)]
    pub letter_spacing: BTreeMap<String, String>,
}

/// Density tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DensityTokens {
    /// Global scale factor.
    pub scale: f64,
    /// Base spacing unit.
    pub spacing_unit: String,
}

/// Icon tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IconTokens {
    /// Name of the icon set.
    pub set: String,
    /// Icon sizes.
    #[serde(default)]
    pub size: BTreeMap<String, String>,
    /// Stroke widths.
    #[serde(default)]
    pub stroke_width: BTreeMap<String, f64>,
}

/// Paths that accept additional keys when a partial theme is merged.
const EXTENSIBLE_PATHS: [&str; 7] = [
    "color.primitive",
    "color.component",
    "typography",
    "radius",
    "shadow",
    "icon.size",
    "icon.strokeWidth",
];

fn is_extensible(path: &str) -> bool {
    let normalized = path.strip_prefix("tokens.").unwrap_or(path);
    EXTENSIBLE_PATHS.iter().any(|candidate| {
        normalized == *candidate || normalized.starts_with(&format!("{candidate}."))
    })
}

/// Whether a string is a valid hex colour.
pub fn is_hex_colour(value: &str) -> bool {
    let Some(digits) = value.strip_prefix('#') else {
        return false;
    };
    if !matches!(digits.len(), 3 | 4 | 6 | 8) {
        return false;
    }
    digits.chars().all(|c| c.is_ascii_hexdigit())
}

/// Merges `over` over `base`, collecting the problems that were found.
fn merge_json(base: &Value, over: &Value, path: &str, errors: &mut Vec<String>) -> Value {
    match base {
        Value::Object(base_map) => match over {
            Value::Object(over_map) => {
                let mut result = base_map.clone();
                for (key, value) in over_map {
                    let child_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    match base_map.get(key) {
                        Some(base_value) => {
                            let merged = merge_json(base_value, value, &child_path, errors);
                            result.insert(key.clone(), merged);
                        }
                        None => {
                            if is_extensible(path) {
                                result.insert(key.clone(), value.clone());
                            } else {
                                errors.push(format!("{child_path}: unknown token"));
                            }
                        }
                    }
                }
                Value::Object(result)
            }
            _ => {
                errors.push(format!("{path}: expected an object"));
                base.clone()
            }
        },
        Value::String(base_string) => match over {
            Value::String(over_string) => {
                if base_string.starts_with('#') && !is_hex_colour(over_string) {
                    errors.push(format!("{path}: \"{over_string}\" is not a hex colour"));
                    base.clone()
                } else {
                    Value::String(over_string.clone())
                }
            }
            _ => {
                errors.push(format!("{path}: expected a string"));
                base.clone()
            }
        },
        Value::Number(_) => match over {
            Value::Number(number) => Value::Number(number.clone()),
            _ => {
                errors.push(format!("{path}: expected a number"));
                base.clone()
            }
        },
        _ => base.clone(),
    }
}

impl Theme {
    /// Parses a theme document.
    pub fn from_json(raw: &str) -> Result<Self> {
        serde_json::from_str(raw).map_err(Error::from)
    }

    /// Reads a theme file.
    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path).map_err(|e| Error::io(path, e))?;
        Self::from_json(&raw)
    }

    /// Merges a partial theme document over this theme.
    ///
    /// Returns the merged theme and the list of problems. The merged theme
    /// always stays valid: invalid tokens keep the default value.
    pub fn merged_with(&self, over: &Value) -> (Self, Vec<String>) {
        let mut errors = Vec::new();
        let base = serde_json::to_value(self).unwrap_or(Value::Null);
        let merged = merge_json(&base, over, "", &mut errors);

        match serde_json::from_value::<Theme>(merged) {
            Ok(theme) => (theme, errors),
            Err(error) => {
                errors.push(format!("theme: {error}"));
                (self.clone(), errors)
            }
        }
    }

    /// Reads a partial theme file and merges it over this theme.
    pub fn load_partial(&self, path: &Path) -> (Self, Vec<String>) {
        let raw = match std::fs::read_to_string(path) {
            Ok(raw) => raw,
            Err(error) => {
                return (
                    self.clone(),
                    vec![format!("theme: cannot read {} ({error})", path.display())],
                )
            }
        };

        match serde_json::from_str::<Value>(&raw) {
            Ok(value) => self.merged_with(&value),
            Err(error) => (self.clone(), vec![format!("theme: invalid JSON ({error})")]),
        }
    }

    /// Checks the colours of the theme.
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if let Ok(value) = serde_json::to_value(self) {
            if let Some(color) = value.get("tokens").and_then(|tokens| tokens.get("color")) {
                validate_colors(color, "tokens.color", &mut errors);
            }
        }
        errors
    }
}

fn validate_colors(node: &Value, path: &str, errors: &mut Vec<String>) {
    match node {
        Value::Object(map) => {
            for (key, value) in map {
                validate_colors(value, &format!("{path}.{key}"), errors);
            }
        }
        Value::String(text) if text.starts_with('#') && !is_hex_colour(text) => {
            errors.push(format!("{path}: \"{text}\" is not a hex colour"));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn builtin_theme(name: &str) -> Theme {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../theme/themes")
            .join(format!("{name}.json"));
        Theme::load(&path).expect("built-in theme must be readable")
    }

    #[test]
    fn built_in_themes_parse_and_validate() {
        for name in ["dark", "light", "high-contrast"] {
            let theme = builtin_theme(name);
            assert_eq!(
                theme.validate(),
                Vec::<String>::new(),
                "{name} must be valid"
            );
        }
    }

    #[test]
    fn modes_round_trip() {
        let theme = builtin_theme("high-contrast");
        assert_eq!(theme.mode, ThemeMode::HighContrast);
        assert_eq!(theme.mode.as_str(), "high-contrast");
        let json = serde_json::to_string(&theme.mode).unwrap();
        assert_eq!(json, "\"high-contrast\"");
    }

    #[test]
    fn a_partial_theme_inherits_the_missing_tokens() {
        let base = builtin_theme("dark");
        let over = serde_json::json!({
            "name": "My theme",
            "tokens": { "color": { "semantic": { "surface": { "base": "#101010" } } } }
        });

        let (merged, errors) = base.merged_with(&over);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(merged.name, "My theme");
        assert_eq!(merged.tokens.color.semantic.surface.base, "#101010");
        assert_eq!(merged.tokens.color.semantic.surface.raised, "#232833");
    }

    #[test]
    fn an_invalid_colour_keeps_the_default() {
        let base = builtin_theme("dark");
        let over = serde_json::json!({
            "tokens": { "color": { "semantic": { "surface": { "base": "nope" } } } }
        });

        let (merged, errors) = base.merged_with(&over);
        assert_eq!(errors.len(), 1);
        assert_eq!(merged.tokens.color.semantic.surface.base, "#1c2027");
    }

    #[test]
    fn unknown_tokens_of_a_fixed_section_are_reported() {
        let base = builtin_theme("dark");
        let over = serde_json::json!({
            "tokens": { "color": { "semantic": { "surface": { "unknown": "#ffffff" } } } }
        });

        let (_, errors) = base.merged_with(&over);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("unknown token"), "{errors:?}");
    }

    #[test]
    fn extensible_sections_accept_new_keys() {
        let base = builtin_theme("dark");
        let over = serde_json::json!({
            "tokens": {
                "color": {
                    "primitive": { "teal": { "500": "#008080" } },
                    "component": { "toolbar": { "bg": "#000000" } }
                }
            }
        });

        let (merged, errors) = base.merged_with(&over);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(
            merged.tokens.color.primitive.get("teal").unwrap()["500"],
            "#008080"
        );
        assert_eq!(merged.tokens.color.component["toolbar"]["bg"], "#000000");
    }

    #[test]
    fn a_wrong_type_keeps_the_default() {
        let base = builtin_theme("dark");
        let over = serde_json::json!({
            "tokens": { "typography": { "fontWeight": { "regular": "bold" } } }
        });

        let (merged, errors) = base.merged_with(&over);
        assert_eq!(errors.len(), 1);
        assert_eq!(
            merged.tokens.typography.font_weight.get("regular").copied(),
            Some(400)
        );
    }

    #[test]
    fn hex_colours_are_checked() {
        assert!(is_hex_colour("#fff"));
        assert!(is_hex_colour("#ffff"));
        assert!(is_hex_colour("#ffffff"));
        assert!(is_hex_colour("#ffffffaa"));
        assert!(!is_hex_colour("#fffff"));
        assert!(!is_hex_colour("ffffff"));
        assert!(!is_hex_colour("#gggggg"));
    }
}
