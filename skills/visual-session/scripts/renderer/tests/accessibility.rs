const TEMPLATE: &str = include_str!("../../../assets/dashboard.html");

fn stylesheet() -> &'static str {
    TEMPLATE
        .split_once("<style>")
        .expect("dashboard must include its stylesheet")
        .1
        .split_once("</style>")
        .expect("dashboard stylesheet must close")
        .0
}

fn palette_color(name: &str) -> [f64; 3] {
    let variables = stylesheet()
        .split_once(":root")
        .expect("dashboard must declare its palette")
        .1
        .split_once('{')
        .expect("palette declaration must open")
        .1
        .split_once('}')
        .expect("palette declaration must close")
        .0;
    let property = format!("--{name}");
    let value = variables
        .split(';')
        .filter_map(|declaration| declaration.trim().split_once(':'))
        .find_map(|(key, value)| (key.trim() == property).then_some(value.trim()))
        .unwrap_or_else(|| panic!("missing palette color {property}"));
    let hex = value
        .strip_prefix('#')
        .unwrap_or_else(|| panic!("{property} must be a hexadecimal color"));
    assert_eq!(hex.len(), 6, "{property} must have six hexadecimal digits");
    std::array::from_fn(|index| {
        let channel = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)
            .unwrap_or_else(|_| panic!("invalid hexadecimal color for {property}"));
        let srgb = f64::from(channel) / 255.0;
        if srgb <= 0.04045 {
            srgb / 12.92
        } else {
            ((srgb + 0.055) / 1.055).powf(2.4)
        }
    })
}

fn luminance(name: &str) -> f64 {
    let [red, green, blue] = palette_color(name);
    0.2126 * red + 0.7152 * green + 0.0722 * blue
}

fn assert_contrast(foreground: &str, background: &str, minimum: f64) {
    let foreground_luminance = luminance(foreground);
    let background_luminance = luminance(background);
    let ratio = (foreground_luminance.max(background_luminance) + 0.05)
        / (foreground_luminance.min(background_luminance) + 0.05);
    assert!(
        ratio >= minimum,
        "{foreground} on {background} has {ratio:.2}:1 contrast; requires {minimum}:1"
    );
}

#[test]
fn dashboard_text_colors_meet_normal_text_contrast() {
    for (foreground, background) in [
        ("ink", "paper"),
        ("muted", "paper"),
        ("muted", "map"),
        ("planned", "surface"),
        ("running", "surface"),
        ("passed", "surface"),
        ("failed", "surface"),
        ("skipped", "surface"),
        ("unknown", "surface"),
        ("leaf", "night"),
        ("light-muted", "night"),
    ] {
        assert_contrast(foreground, background, 4.5);
    }
}

#[test]
fn dark_next_move_panel_uses_a_contrasting_keyboard_focus_outline() {
    let compact_css: String = stylesheet()
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    let focus_declarations = compact_css
        .split_once(".next-movea:focus-visible{")
        .expect("dark next-move links need a local focus outline override")
        .1
        .split_once('}')
        .expect("next-move focus declaration must close")
        .0;
    assert!(
        focus_declarations
            .split(';')
            .any(|declaration| declaration == "outline-color:var(--leaf)"),
        "next-move keyboard focus must use the tested leaf color"
    );
    assert_contrast("leaf", "night", 3.0);
}
