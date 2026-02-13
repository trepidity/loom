use ratatui::style::{Color, Modifier, Style};
use serde::Deserialize;
use tracing::warn;

/// Application theme with styles for every UI element.
#[derive(Debug, Clone)]
pub struct Theme {
    pub border: Style,
    pub border_focused: Style,
    pub selected: Style,
    pub header: Style,
    pub normal: Style,
    pub dimmed: Style,
    pub error: Style,
    pub warning: Style,
    pub success: Style,
    pub status_bar: Style,
    pub tab_active: Style,
    pub tab_inactive: Style,
    pub tree_node: Style,
    pub tree_node_expanded: Style,
    pub tree_node_selected: Style,
    pub popup_border: Style,
    pub popup_title: Style,
    pub command_prompt: Style,
    pub attr_operational: Style,
}

impl Theme {
    /// Default dark theme based on the Catppuccin Mocha palette.
    /// Soft pastel accents on a dark blue-gray base for readability and eye comfort.
    pub fn dark() -> Self {
        // Catppuccin Mocha palette
        let base = Color::Rgb(30, 30, 46); // #1e1e2e  background
        let surface0 = Color::Rgb(49, 50, 68); // #313244  elevated surfaces
        let surface1 = Color::Rgb(69, 71, 90); // #45475a  selection/active bg
        let overlay0 = Color::Rgb(108, 112, 134); // #6c7086 muted/dim
        let subtext0 = Color::Rgb(166, 173, 200); // #a6adc8 secondary text
        let text = Color::Rgb(205, 214, 244); // #cdd6f4  primary text
        let blue = Color::Rgb(137, 180, 250); // #89b4fa  accent/focus
        let lavender = Color::Rgb(180, 190, 254); // #b4befe secondary accent
        let green = Color::Rgb(166, 227, 161); // #a6e3a1  success
        let red = Color::Rgb(243, 139, 168); // #f38ba8  error
        let peach = Color::Rgb(250, 179, 135); // #fab387  warning/orange
        let yellow = Color::Rgb(249, 226, 175); // #f9e2af  warning
        let mauve = Color::Rgb(203, 166, 247); // #cba6f7  purple accent

        Self {
            border: Style::default().fg(surface1),
            border_focused: Style::default().fg(blue).add_modifier(Modifier::BOLD),
            selected: Style::default().fg(base).bg(blue),
            header: Style::default().fg(lavender).add_modifier(Modifier::BOLD),
            normal: Style::default().fg(text),
            dimmed: Style::default().fg(overlay0),
            error: Style::default().fg(red).add_modifier(Modifier::BOLD),
            warning: Style::default().fg(yellow),
            success: Style::default().fg(green),
            status_bar: Style::default().fg(subtext0).bg(surface0),
            tab_active: Style::default().fg(blue).add_modifier(Modifier::BOLD),
            tab_inactive: Style::default().fg(overlay0),
            tree_node: Style::default().fg(text),
            tree_node_expanded: Style::default().fg(green),
            tree_node_selected: Style::default().fg(base).bg(blue),
            popup_border: Style::default().fg(mauve),
            popup_title: Style::default().fg(mauve).add_modifier(Modifier::BOLD),
            command_prompt: Style::default().fg(peach),
            attr_operational: Style::default().fg(overlay0),
        }
    }

    /// Matrix green-on-black retro theme.
    pub fn matrix() -> Self {
        Self {
            border: Style::default().fg(Color::Rgb(0, 100, 0)),
            border_focused: Style::default()
                .fg(Color::Rgb(0, 255, 0))
                .add_modifier(Modifier::BOLD),
            selected: Style::default().fg(Color::Black).bg(Color::Rgb(0, 200, 0)),
            header: Style::default()
                .fg(Color::Rgb(0, 255, 0))
                .add_modifier(Modifier::BOLD),
            normal: Style::default().fg(Color::Rgb(0, 190, 0)),
            dimmed: Style::default().fg(Color::Rgb(0, 80, 0)),
            error: Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            warning: Style::default().fg(Color::Yellow),
            success: Style::default().fg(Color::Rgb(0, 255, 0)),
            status_bar: Style::default()
                .fg(Color::Rgb(0, 190, 0))
                .bg(Color::Rgb(0, 30, 0)),
            tab_active: Style::default()
                .fg(Color::Rgb(0, 255, 0))
                .add_modifier(Modifier::BOLD),
            tab_inactive: Style::default().fg(Color::Rgb(0, 100, 0)),
            tree_node: Style::default().fg(Color::Rgb(0, 190, 0)),
            tree_node_expanded: Style::default().fg(Color::Rgb(0, 255, 0)),
            tree_node_selected: Style::default().fg(Color::Black).bg(Color::Rgb(0, 200, 0)),
            popup_border: Style::default().fg(Color::Rgb(0, 255, 0)),
            popup_title: Style::default()
                .fg(Color::Rgb(0, 255, 0))
                .add_modifier(Modifier::BOLD),
            command_prompt: Style::default().fg(Color::Rgb(0, 255, 0)),
            attr_operational: Style::default().fg(Color::Rgb(0, 120, 0)),
        }
    }

    /// Light theme for light terminal backgrounds.
    pub fn light() -> Self {
        Self {
            border: Style::default().fg(Color::DarkGray),
            border_focused: Style::default().fg(Color::Blue),
            selected: Style::default().fg(Color::White).bg(Color::Blue),
            header: Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            normal: Style::default().fg(Color::Black),
            dimmed: Style::default().fg(Color::Gray),
            error: Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            warning: Style::default().fg(Color::Yellow),
            success: Style::default().fg(Color::Green),
            status_bar: Style::default().fg(Color::Black).bg(Color::Gray),
            tab_active: Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            tab_inactive: Style::default().fg(Color::DarkGray),
            tree_node: Style::default().fg(Color::Black),
            tree_node_expanded: Style::default().fg(Color::Blue),
            tree_node_selected: Style::default().fg(Color::White).bg(Color::Blue),
            popup_border: Style::default().fg(Color::Blue),
            popup_title: Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            command_prompt: Style::default().fg(Color::Blue),
            attr_operational: Style::default().fg(Color::Gray),
        }
    }

    /// Solarized dark theme.
    pub fn solarized() -> Self {
        let base03 = Color::Rgb(0, 43, 54);
        let base0 = Color::Rgb(131, 148, 150);
        let base1 = Color::Rgb(147, 161, 161);
        let base01 = Color::Rgb(88, 110, 117);
        let yellow = Color::Rgb(181, 137, 0);
        let orange = Color::Rgb(203, 75, 22);
        let red = Color::Rgb(220, 50, 47);
        let cyan = Color::Rgb(42, 161, 152);
        let green = Color::Rgb(133, 153, 0);
        let blue = Color::Rgb(38, 139, 210);

        Self {
            border: Style::default().fg(base01),
            border_focused: Style::default().fg(cyan),
            selected: Style::default().fg(base03).bg(cyan),
            header: Style::default().fg(yellow).add_modifier(Modifier::BOLD),
            normal: Style::default().fg(base0),
            dimmed: Style::default().fg(base01),
            error: Style::default().fg(red).add_modifier(Modifier::BOLD),
            warning: Style::default().fg(orange),
            success: Style::default().fg(green),
            status_bar: Style::default().fg(base1).bg(base03),
            tab_active: Style::default().fg(blue).add_modifier(Modifier::BOLD),
            tab_inactive: Style::default().fg(base01),
            tree_node: Style::default().fg(base0),
            tree_node_expanded: Style::default().fg(green),
            tree_node_selected: Style::default().fg(base03).bg(cyan),
            popup_border: Style::default().fg(yellow),
            popup_title: Style::default().fg(yellow).add_modifier(Modifier::BOLD),
            command_prompt: Style::default().fg(cyan),
            attr_operational: Style::default().fg(base01),
        }
    }

    /// Nord theme.
    pub fn nord() -> Self {
        let polar0 = Color::Rgb(46, 52, 64);
        let snow0 = Color::Rgb(216, 222, 233);
        let snow2 = Color::Rgb(236, 239, 244);
        let frost0 = Color::Rgb(143, 188, 187);
        let frost2 = Color::Rgb(129, 161, 193);
        let frost3 = Color::Rgb(94, 129, 172);
        let aurora_red = Color::Rgb(191, 97, 106);
        let aurora_orange = Color::Rgb(208, 135, 112);
        let aurora_yellow = Color::Rgb(235, 203, 139);
        let aurora_green = Color::Rgb(163, 190, 140);

        Self {
            border: Style::default().fg(frost0),
            border_focused: Style::default().fg(frost2),
            selected: Style::default().fg(polar0).bg(frost0),
            header: Style::default().fg(frost2).add_modifier(Modifier::BOLD),
            normal: Style::default().fg(snow0),
            dimmed: Style::default().fg(frost0),
            error: Style::default().fg(aurora_red).add_modifier(Modifier::BOLD),
            warning: Style::default().fg(aurora_orange),
            success: Style::default().fg(aurora_green),
            status_bar: Style::default().fg(snow2).bg(polar0),
            tab_active: Style::default().fg(frost3).add_modifier(Modifier::BOLD),
            tab_inactive: Style::default().fg(frost0),
            tree_node: Style::default().fg(snow0),
            tree_node_expanded: Style::default().fg(aurora_green),
            tree_node_selected: Style::default().fg(polar0).bg(frost0),
            popup_border: Style::default().fg(aurora_yellow),
            popup_title: Style::default()
                .fg(aurora_yellow)
                .add_modifier(Modifier::BOLD),
            command_prompt: Style::default().fg(frost2),
            attr_operational: Style::default().fg(frost0),
        }
    }

    /// Load a theme by name. Supports built-in names and custom TOML paths.
    pub fn load(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "dark" => Self::dark(),
            "light" => Self::light(),
            "solarized" => Self::solarized(),
            "nord" => Self::nord(),
            "matrix" => Self::matrix(),
            _ => {
                // Try loading from config themes directory
                if let Some(config_dir) = dirs::config_dir() {
                    let theme_path = config_dir
                        .join("loom")
                        .join("themes")
                        .join(format!("{}.toml", name));
                    if theme_path.exists() {
                        if let Ok(content) = std::fs::read_to_string(&theme_path) {
                            if let Ok(theme_def) = toml::from_str::<ThemeDefinition>(&content) {
                                return theme_def.to_theme();
                            } else {
                                warn!("Failed to parse theme file: {:?}", theme_path);
                            }
                        }
                    }
                }
                warn!("Unknown theme '{}', using dark", name);
                Self::dark()
            }
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

// --- TOML deserialization types ---

#[derive(Debug, Deserialize)]
struct ThemeDefinition {
    colors: ThemeColors,
}

#[derive(Debug, Deserialize)]
struct ThemeColors {
    #[serde(default)]
    border: StyleDef,
    #[serde(default)]
    border_focused: StyleDef,
    #[serde(default)]
    selected: StyleDef,
    #[serde(default)]
    header: StyleDef,
    #[serde(default)]
    normal: StyleDef,
    #[serde(default)]
    dimmed: StyleDef,
    #[serde(default)]
    error: StyleDef,
    #[serde(default)]
    warning: StyleDef,
    #[serde(default)]
    success: StyleDef,
    #[serde(default = "default_white")]
    status_bar_fg: String,
    #[serde(default = "default_dark_gray")]
    status_bar_bg: String,
    #[serde(default)]
    tab_active: StyleDef,
    #[serde(default)]
    tab_inactive: StyleDef,
    #[serde(default)]
    tree_node: StyleDef,
    #[serde(default)]
    tree_node_expanded: StyleDef,
    #[serde(default)]
    tree_node_selected: StyleDef,
    #[serde(default)]
    popup_border: StyleDef,
    #[serde(default)]
    popup_title: StyleDef,
    #[serde(default)]
    command_prompt: StyleDef,
    #[serde(default)]
    attr_operational: StyleDef,
}

fn default_white() -> String {
    "white".to_string()
}
fn default_dark_gray() -> String {
    "dark_gray".to_string()
}

#[derive(Debug, Default, Deserialize)]
struct StyleDef {
    #[serde(default)]
    fg: Option<String>,
    #[serde(default)]
    bg: Option<String>,
    #[serde(default)]
    modifiers: Option<String>,
}

impl StyleDef {
    fn to_style(&self) -> Style {
        let mut style = Style::default();
        if let Some(ref fg) = self.fg {
            style = style.fg(parse_color(fg));
        }
        if let Some(ref bg) = self.bg {
            style = style.bg(parse_color(bg));
        }
        if let Some(ref mods) = self.modifiers {
            for m in mods.split('|') {
                match m.trim().to_uppercase().as_str() {
                    "BOLD" => style = style.add_modifier(Modifier::BOLD),
                    "ITALIC" => style = style.add_modifier(Modifier::ITALIC),
                    "UNDERLINED" => style = style.add_modifier(Modifier::UNDERLINED),
                    "DIM" => style = style.add_modifier(Modifier::DIM),
                    _ => {}
                }
            }
        }
        style
    }
}

impl ThemeDefinition {
    fn to_theme(&self) -> Theme {
        let c = &self.colors;
        Theme {
            border: c.border.to_style(),
            border_focused: c.border_focused.to_style(),
            selected: c.selected.to_style(),
            header: c.header.to_style(),
            normal: c.normal.to_style(),
            dimmed: c.dimmed.to_style(),
            error: c.error.to_style(),
            warning: c.warning.to_style(),
            success: c.success.to_style(),
            status_bar: Style::default()
                .fg(parse_color(&c.status_bar_fg))
                .bg(parse_color(&c.status_bar_bg)),
            tab_active: c.tab_active.to_style(),
            tab_inactive: c.tab_inactive.to_style(),
            tree_node: c.tree_node.to_style(),
            tree_node_expanded: c.tree_node_expanded.to_style(),
            tree_node_selected: c.tree_node_selected.to_style(),
            popup_border: c.popup_border.to_style(),
            popup_title: c.popup_title.to_style(),
            command_prompt: c.command_prompt.to_style(),
            attr_operational: c.attr_operational.to_style(),
        }
    }
}

fn parse_color(s: &str) -> Color {
    let s = s.trim().to_lowercase();
    match s.as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "dark_gray" | "dark_grey" | "darkgray" => Color::DarkGray,
        "light_red" | "lightred" => Color::LightRed,
        "light_green" | "lightgreen" => Color::LightGreen,
        "light_yellow" | "lightyellow" => Color::LightYellow,
        "light_blue" | "lightblue" => Color::LightBlue,
        "light_magenta" | "lightmagenta" => Color::LightMagenta,
        "light_cyan" | "lightcyan" => Color::LightCyan,
        "white" => Color::White,
        _ => {
            // Try #RRGGBB or RRGGBB
            let hex = s.strip_prefix('#').unwrap_or(&s);
            if hex.len() == 6 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    u8::from_str_radix(&hex[0..2], 16),
                    u8::from_str_radix(&hex[2..4], 16),
                    u8::from_str_radix(&hex[4..6], 16),
                ) {
                    return Color::Rgb(r, g, b);
                }
            }
            Color::White
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_color_names() {
        assert_eq!(parse_color("red"), Color::Red);
        assert_eq!(parse_color("dark_gray"), Color::DarkGray);
        assert_eq!(parse_color("DarkGray"), Color::DarkGray);
    }

    #[test]
    fn test_parse_color_hex() {
        assert_eq!(parse_color("#FF0000"), Color::Rgb(255, 0, 0));
        assert_eq!(parse_color("00FF00"), Color::Rgb(0, 255, 0));
    }

    #[test]
    fn test_load_builtin_themes() {
        let _dark = Theme::load("dark");
        let _light = Theme::load("light");
        let _solarized = Theme::load("solarized");
        let _nord = Theme::load("nord");
        let _matrix = Theme::load("matrix");
    }
}
