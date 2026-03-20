use oni_core::palette;
use ratatui::style::Style;

/// Format text as ALL_CAPS for UI chrome labels
pub fn label(text: &str) -> String {
    text.to_uppercase().replace(' ', "_")
}

/// Styles re-exported from palette for convenience
pub fn data() -> Style {
    palette::data_style()
}

pub fn system() -> Style {
    palette::system_style()
}

pub fn alert() -> Style {
    palette::alert_style()
}

pub fn state() -> Style {
    palette::state_style()
}

pub fn dim() -> Style {
    palette::dim_style()
}

pub fn input() -> Style {
    palette::input_style()
}
