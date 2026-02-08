//! Crossterm terminal output backend.
//!
//! The `Driver` wraps a buffered stdout writer and provides methods for entering/leaving
//! alternate screen, applying cell updates from the compositor, and controlling the cursor.
//! Color strings are parsed as named colors or `#rrggbb` hex values.

use std::io::{self, Write, BufWriter, Stdout};
use crossterm::{
    cursor, execute, queue,
    style::{SetForegroundColor, SetBackgroundColor, SetAttribute, ResetColor, Print, Color, Attribute},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};

use super::compositor::CellUpdate;
use super::strip::CellStyle;

// ---------------------------------------------------------------------------
// Driver
// ---------------------------------------------------------------------------

/// Terminal output backend using crossterm.
///
/// Wraps a `BufWriter<Stdout>` for efficient batched writes. The driver does NOT
/// automatically enter alternate screen on creation — call `enter_alt_screen` explicitly.
pub struct Driver {
    writer: BufWriter<Stdout>,
}

impl Driver {
    /// Create a new driver wrapping stdout.
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            writer: BufWriter::new(io::stdout()),
        })
    }

    /// Enter alternate screen and enable raw mode.
    pub fn enter_alt_screen(&mut self) -> io::Result<()> {
        execute!(self.writer, EnterAlternateScreen)?;
        terminal::enable_raw_mode()?;
        Ok(())
    }

    /// Leave alternate screen and disable raw mode.
    pub fn leave_alt_screen(&mut self) -> io::Result<()> {
        terminal::disable_raw_mode()?;
        execute!(self.writer, LeaveAlternateScreen)?;
        Ok(())
    }

    /// Apply a batch of cell updates to the terminal.
    ///
    /// For each update, the cursor is moved to the cell's position, the style
    /// is applied, and the character is printed. Uses `queue!` for batching;
    /// call `flush()` afterward to send to the terminal.
    pub fn apply_updates(&mut self, updates: &[CellUpdate]) -> io::Result<()> {
        for update in updates {
            queue!(
                self.writer,
                cursor::MoveTo(update.x, update.y)
            )?;
            self.apply_cell_style(&update.cell.style)?;
            queue!(self.writer, Print(update.cell.ch))?;
            queue!(self.writer, ResetColor)?;
        }
        Ok(())
    }

    /// Flush the internal write buffer to the terminal.
    pub fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }

    /// Get the terminal size (columns, rows) via crossterm.
    pub fn terminal_size() -> io::Result<(u16, u16)> {
        terminal::size()
    }

    /// Hide the cursor.
    pub fn hide_cursor(&mut self) -> io::Result<()> {
        execute!(self.writer, cursor::Hide)
    }

    /// Show the cursor.
    pub fn show_cursor(&mut self) -> io::Result<()> {
        execute!(self.writer, cursor::Show)
    }

    /// Queue crossterm style commands for a given `CellStyle`.
    fn apply_cell_style(&mut self, style: &CellStyle) -> io::Result<()> {
        if let Some(ref fg) = style.fg {
            if let Some(color) = parse_color(fg) {
                queue!(self.writer, SetForegroundColor(color))?;
            }
        }
        if let Some(ref bg) = style.bg {
            if let Some(color) = parse_color(bg) {
                queue!(self.writer, SetBackgroundColor(color))?;
            }
        }
        if style.bold {
            queue!(self.writer, SetAttribute(Attribute::Bold))?;
        }
        if style.dim {
            queue!(self.writer, SetAttribute(Attribute::Dim))?;
        }
        if style.italic {
            queue!(self.writer, SetAttribute(Attribute::Italic))?;
        }
        if style.underline {
            queue!(self.writer, SetAttribute(Attribute::Underlined))?;
        }
        if style.strikethrough {
            queue!(self.writer, SetAttribute(Attribute::CrossedOut))?;
        }
        if style.reverse {
            queue!(self.writer, SetAttribute(Attribute::Reverse))?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Color parsing
// ---------------------------------------------------------------------------

/// Parse a color string into a crossterm `Color`.
///
/// Supports:
/// - Hex colors: `#rrggbb` or `#rgb`
/// - Named colors: `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`,
///   `dark_red`, `dark_green`, `dark_yellow`, `dark_blue`, `dark_magenta`, `dark_cyan`, `dark_grey`/`dark_gray`,
///   `grey`/`gray`
///
/// Returns `None` if the color string cannot be parsed.
pub fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim();

    // Hex color
    if let Some(hex) = s.strip_prefix('#') {
        return parse_hex_color(hex);
    }

    // Named colors (case-insensitive)
    match s.to_ascii_lowercase().as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "white" => Some(Color::White),
        "dark_red" | "darkred" => Some(Color::DarkRed),
        "dark_green" | "darkgreen" => Some(Color::DarkGreen),
        "dark_yellow" | "darkyellow" => Some(Color::DarkYellow),
        "dark_blue" | "darkblue" => Some(Color::DarkBlue),
        "dark_magenta" | "darkmagenta" => Some(Color::DarkMagenta),
        "dark_cyan" | "darkcyan" => Some(Color::DarkCyan),
        "dark_grey" | "dark_gray" | "darkgrey" | "darkgray" => Some(Color::DarkGrey),
        "grey" | "gray" => Some(Color::Grey),
        _ => None,
    }
}

/// Parse a hex color string (without the leading `#`).
///
/// Supports 6-digit (`rrggbb`) and 3-digit (`rgb`) formats.
fn parse_hex_color(hex: &str) -> Option<Color> {
    match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Color::Rgb { r, g, b })
        }
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
            // Expand: 0xA -> 0xAA
            Some(Color::Rgb {
                r: r * 16 + r,
                g: g * 16 + g,
                b: b * 16 + b,
            })
        }
        _ => None,
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::style::Color;

    // -----------------------------------------------------------------------
    // Color parsing — hex
    // -----------------------------------------------------------------------

    #[test]
    fn parse_hex_6digit() {
        assert_eq!(
            parse_color("#ff0000"),
            Some(Color::Rgb { r: 255, g: 0, b: 0 })
        );
    }

    #[test]
    fn parse_hex_mixed_case() {
        assert_eq!(
            parse_color("#FF8800"),
            Some(Color::Rgb {
                r: 255,
                g: 136,
                b: 0
            })
        );
    }

    #[test]
    fn parse_hex_3digit() {
        // #f00 -> #ff0000
        assert_eq!(
            parse_color("#f00"),
            Some(Color::Rgb { r: 255, g: 0, b: 0 })
        );
    }

    #[test]
    fn parse_hex_3digit_expanded() {
        // #abc -> #aabbcc
        assert_eq!(
            parse_color("#abc"),
            Some(Color::Rgb {
                r: 0xaa,
                g: 0xbb,
                b: 0xcc
            })
        );
    }

    #[test]
    fn parse_hex_invalid_length() {
        assert_eq!(parse_color("#ff00"), None);
        assert_eq!(parse_color("#ff00000"), None);
    }

    #[test]
    fn parse_hex_invalid_chars() {
        assert_eq!(parse_color("#gghhii"), None);
    }

    // -----------------------------------------------------------------------
    // Color parsing — named
    // -----------------------------------------------------------------------

    #[test]
    fn parse_named_colors() {
        assert_eq!(parse_color("red"), Some(Color::Red));
        assert_eq!(parse_color("green"), Some(Color::Green));
        assert_eq!(parse_color("blue"), Some(Color::Blue));
        assert_eq!(parse_color("black"), Some(Color::Black));
        assert_eq!(parse_color("white"), Some(Color::White));
        assert_eq!(parse_color("yellow"), Some(Color::Yellow));
        assert_eq!(parse_color("magenta"), Some(Color::Magenta));
        assert_eq!(parse_color("cyan"), Some(Color::Cyan));
    }

    #[test]
    fn parse_named_case_insensitive() {
        assert_eq!(parse_color("Red"), Some(Color::Red));
        assert_eq!(parse_color("RED"), Some(Color::Red));
        assert_eq!(parse_color("rEd"), Some(Color::Red));
    }

    #[test]
    fn parse_named_dark_variants() {
        assert_eq!(parse_color("dark_red"), Some(Color::DarkRed));
        assert_eq!(parse_color("darkred"), Some(Color::DarkRed));
        assert_eq!(parse_color("dark_grey"), Some(Color::DarkGrey));
        assert_eq!(parse_color("dark_gray"), Some(Color::DarkGrey));
    }

    #[test]
    fn parse_grey_variants() {
        assert_eq!(parse_color("grey"), Some(Color::Grey));
        assert_eq!(parse_color("gray"), Some(Color::Grey));
    }

    #[test]
    fn parse_unknown_color() {
        assert_eq!(parse_color("rainbow"), None);
        assert_eq!(parse_color(""), None);
    }

    #[test]
    fn parse_color_with_whitespace() {
        assert_eq!(parse_color("  red  "), Some(Color::Red));
        assert_eq!(parse_color(" #ff0000 "), Some(Color::Rgb { r: 255, g: 0, b: 0 }));
    }

    // -----------------------------------------------------------------------
    // Driver — structural tests
    // -----------------------------------------------------------------------

    #[test]
    fn driver_terminal_size_returns_nonzero() {
        // This may fail in CI without a terminal, but should not panic.
        // We just ensure it doesn't panic.
        let _ = Driver::terminal_size();
    }

    #[test]
    fn driver_new_succeeds() {
        // Verify we can construct a driver without error.
        let driver = Driver::new();
        assert!(driver.is_ok());
    }
}
