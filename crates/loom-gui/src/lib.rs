slint::include_modules!();

use loom_core::config::AppConfig;

pub fn run() -> Result<(), slint::PlatformError> {
    let config = AppConfig::load();
    let main_window = MainWindow::new()?;

    apply_theme(&main_window, &config.general.theme);

    main_window.set_status_message("Ready".into());

    main_window.run()
}

fn apply_theme(window: &MainWindow, theme_name: &str) {
    let theme = window.global::<AppTheme>();
    match theme_name {
        "light" => {
            theme.set_bg_primary(slint::Color::from_rgb_u8(0xfa, 0xfa, 0xfa));
            theme.set_bg_secondary(slint::Color::from_rgb_u8(0xf0, 0xf0, 0xf0));
            theme.set_bg_tertiary(slint::Color::from_rgb_u8(0xe8, 0xe8, 0xe8));
            theme.set_bg_hover(slint::Color::from_rgb_u8(0xe0, 0xe0, 0xe0));
            theme.set_bg_selected(slint::Color::from_rgb_u8(0xd8, 0xd8, 0xd8));
            theme.set_fg_primary(slint::Color::from_rgb_u8(0x1a, 0x1a, 0x1a));
            theme.set_fg_secondary(slint::Color::from_rgb_u8(0x55, 0x55, 0x55));
            theme.set_fg_muted(slint::Color::from_rgb_u8(0x99, 0x99, 0x99));
            theme.set_accent(slint::Color::from_rgb_u8(0x22, 0x7c, 0xe6));
            theme.set_border(slint::Color::from_rgb_u8(0xd0, 0xd0, 0xd0));
            theme.set_border_focus(slint::Color::from_rgb_u8(0x22, 0x7c, 0xe6));
        }
        "solarized" => {
            theme.set_bg_primary(slint::Color::from_rgb_u8(0x00, 0x2b, 0x36));
            theme.set_bg_secondary(slint::Color::from_rgb_u8(0x07, 0x36, 0x42));
            theme.set_bg_tertiary(slint::Color::from_rgb_u8(0x0a, 0x40, 0x4d));
            theme.set_fg_primary(slint::Color::from_rgb_u8(0x83, 0x94, 0x96));
            theme.set_fg_secondary(slint::Color::from_rgb_u8(0x65, 0x7b, 0x83));
            theme.set_accent(slint::Color::from_rgb_u8(0x26, 0x8b, 0xd2));
            theme.set_border(slint::Color::from_rgb_u8(0x58, 0x6e, 0x75));
        }
        "nord" => {
            theme.set_bg_primary(slint::Color::from_rgb_u8(0x2e, 0x34, 0x40));
            theme.set_bg_secondary(slint::Color::from_rgb_u8(0x3b, 0x42, 0x52));
            theme.set_bg_tertiary(slint::Color::from_rgb_u8(0x43, 0x4c, 0x5e));
            theme.set_fg_primary(slint::Color::from_rgb_u8(0xec, 0xef, 0xf4));
            theme.set_fg_secondary(slint::Color::from_rgb_u8(0xd8, 0xde, 0xe9));
            theme.set_accent(slint::Color::from_rgb_u8(0x88, 0xc0, 0xd0));
            theme.set_border(slint::Color::from_rgb_u8(0x4c, 0x56, 0x6a));
        }
        "matrix" => {
            theme.set_bg_primary(slint::Color::from_rgb_u8(0x0a, 0x0a, 0x0a));
            theme.set_bg_secondary(slint::Color::from_rgb_u8(0x12, 0x12, 0x12));
            theme.set_bg_tertiary(slint::Color::from_rgb_u8(0x1a, 0x1a, 0x1a));
            theme.set_fg_primary(slint::Color::from_rgb_u8(0x00, 0xff, 0x00));
            theme.set_fg_secondary(slint::Color::from_rgb_u8(0x00, 0xcc, 0x00));
            theme.set_fg_muted(slint::Color::from_rgb_u8(0x00, 0x66, 0x00));
            theme.set_accent(slint::Color::from_rgb_u8(0x00, 0xcc, 0x00));
            theme.set_border(slint::Color::from_rgb_u8(0x00, 0x33, 0x00));
        }
        _ => {} // "dark" is the default from theme.slint
    }
}
