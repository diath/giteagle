//! What the window itself needs to know about the frontend's themes.

use tauri::WebviewWindow;
use tauri::window::Color;

/// The window's background in each theme, the same as the `bgApp` role in
/// `ui/src/theme/palette.ts`. It shows before the page is first drawn and
/// wherever the page lags behind, e.g. while the window grows.
fn window_background(theme: &str) -> Color {
    match theme {
        "darkBrown" => Color(0x14, 0x12, 0x10, 0xff),
        "lightBeige" => Color(0xf3, 0xed, 0xe2, 0xff),
        "lightWhite" => Color(0xf6, 0xf6, 0xf8, 0xff),
        // Dark (Black), the default, which the frontend also draws for a
        // name it doesn't know.
        _ => Color(0x0f, 0x0f, 0x11, 0xff),
    }
}

/// Paints the window in the background of `theme`. Failing leaves only the
/// edges of the window in the old color, so it is ignored.
pub fn apply(window: &WebviewWindow, theme: &str) {
    let _ = window.set_background_color(Some(window_background(theme)));
}
