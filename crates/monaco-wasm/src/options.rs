use crate::EditorError;
use crate::sys::OnOff;

/// Settings used once, when an [`Editor`](crate::Editor) is created.
///
/// The struct is non-exhaustive: start from [`Default::default`] and assign
/// fields.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct EditorConfig {
    /// Initial document text. Default: empty.
    pub initial_value: String,
    /// Monaco language ID. Default: `"plaintext"`. Must not be blank.
    pub language: String,
    pub options: EditorOptions,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            initial_value: String::new(),
            language: "plaintext".into(),
            options: EditorOptions::default(),
        }
    }
}

impl EditorConfig {
    pub(crate) fn validate(&self) -> Result<(), EditorError> {
        validate_language(&self.language)?;
        self.options.validate()
    }
}

/// Editor options that can be changed after creation.
///
/// [`EditorHandle::update_options`](crate::EditorHandle::update_options)
/// applies all fields, so returning a field to its default resets it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct EditorOptions {
    /// Default: `false`.
    pub read_only: bool,
    /// Default: `14.0`. Must be finite and positive; Monaco may clamp it.
    pub font_size: f64,
    /// Default: [`WordWrap::Off`].
    pub word_wrap: WordWrap,
    /// Default: `false`.
    pub minimap_enabled: bool,
    /// Default: [`LineNumbers::On`].
    pub line_numbers: LineNumbers,
    /// Default: `"Code editor"`. Must not be blank.
    pub aria_label: String,
}

impl Default for EditorOptions {
    fn default() -> Self {
        Self {
            read_only: false,
            font_size: 14.0,
            word_wrap: WordWrap::Off,
            minimap_enabled: false,
            line_numbers: LineNumbers::On,
            aria_label: "Code editor".into(),
        }
    }
}

impl EditorOptions {
    pub(crate) fn validate(&self) -> Result<(), EditorError> {
        if !self.font_size.is_finite() || self.font_size <= 0.0 {
            return Err(EditorError::InvalidOptions(format!(
                "font_size must be finite and positive, got {}",
                self.font_size
            )));
        }
        if self.aria_label.trim().is_empty() {
            return Err(EditorError::InvalidOptions(
                "aria_label must not be blank".into(),
            ));
        }
        Ok(())
    }
}

pub(crate) fn validate_language(language: &str) -> Result<(), EditorError> {
    if language.trim().is_empty() {
        return Err(EditorError::InvalidOptions(
            "language must not be blank".into(),
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WordWrap {
    #[default]
    Off,
    On,
}

impl From<WordWrap> for OnOff {
    fn from(value: WordWrap) -> Self {
        match value {
            WordWrap::Off => OnOff::Off,
            WordWrap::On => OnOff::On,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LineNumbers {
    Off,
    #[default]
    On,
}

impl From<LineNumbers> for OnOff {
    fn from(value: LineNumbers) -> Self {
        match value {
            LineNumbers::Off => OnOff::Off,
            LineNumbers::On => OnOff::On,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid() {
        EditorConfig::default().validate().unwrap();
    }

    #[test]
    fn rejects_invalid_values() {
        let mut options = EditorOptions::default();
        options.font_size = f64::NAN;
        assert!(matches!(
            options.validate(),
            Err(EditorError::InvalidOptions(_))
        ));

        let mut options = EditorOptions::default();
        options.font_size = 0.0;
        assert!(options.validate().is_err());

        let mut options = EditorOptions::default();
        options.aria_label = "  ".into();
        assert!(options.validate().is_err());

        let mut config = EditorConfig::default();
        config.language = String::new();
        assert!(config.validate().is_err());
    }
}
