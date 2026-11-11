//! Emoji and symbol data for the picker tab.

pub struct Entry {
    pub glyph: String,
    /// Lowercase searchable text (name + shortcodes).
    pub search: String,
    pub name: String,
}

const SYMBOLS: &[(&str, &str)] = &[
    ("©", "copyright"),
    ("®", "registered"),
    ("™", "trademark"),
    ("°", "degree"),
    ("±", "plus minus"),
    ("×", "multiply times"),
    ("÷", "divide"),
    ("≠", "not equal"),
    ("≈", "approximately"),
    ("≤", "less or equal"),
    ("≥", "greater or equal"),
    ("∞", "infinity"),
    ("√", "square root"),
    ("π", "pi"),
    ("Σ", "sigma sum"),
    ("Δ", "delta"),
    ("µ", "micro mu"),
    ("Ω", "omega ohm"),
    ("€", "euro"),
    ("£", "pound"),
    ("¥", "yen"),
    ("¢", "cent"),
    ("₦", "naira"),
    ("₹", "rupee"),
    ("₿", "bitcoin"),
    ("←", "left arrow"),
    ("→", "right arrow"),
    ("↑", "up arrow"),
    ("↓", "down arrow"),
    ("↔", "left right arrow"),
    ("⇒", "implies double arrow"),
    ("•", "bullet"),
    ("…", "ellipsis"),
    ("—", "em dash"),
    ("–", "en dash"),
    ("§", "section"),
    ("¶", "paragraph"),
    ("†", "dagger"),
    ("‡", "double dagger"),
    ("«", "left guillemet"),
    ("»", "right guillemet"),
    ("“", "left double quote"),
    ("”", "right double quote"),
    ("¡", "inverted exclamation"),
    ("¿", "inverted question"),
    ("½", "one half"),
    ("¼", "one quarter"),
    ("¾", "three quarters"),
    ("²", "squared superscript two"),
    ("³", "cubed superscript three"),
    ("✓", "check mark"),
    ("✗", "ballot x cross"),
    ("★", "black star"),
    ("☆", "white star"),
    ("♥", "heart suit"),
    ("♪", "musical note"),
    ("№", "numero number"),
];

pub fn emojis() -> Vec<Entry> {
    emojis::iter()
        .filter(|e| e.skin_tone().is_none_or(|t| t == emojis::SkinTone::Default))
        .map(|e| {
            let mut search = e.name().to_lowercase();
            for sc in e.shortcodes() {
                search.push(' ');
                search.push_str(sc);
            }
            Entry {
                glyph: e.as_str().to_string(),
                search,
                name: e.name().to_string(),
            }
        })
        .collect()
}

pub fn symbols() -> Vec<Entry> {
    SYMBOLS
        .iter()
        .map(|(g, n)| Entry {
            glyph: g.to_string(),
            search: n.to_lowercase(),
            name: n.to_string(),
        })
        .collect()
}
