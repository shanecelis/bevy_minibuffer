use asky::{style::{Style, Section, Flags}, utils::renderer::Renderer};
use std::io;
use text_style::AnsiColor::*;
use bevy::ecs::system::Resource;
// use text_style::{self, Color, Style, StyledString};
#[derive(Clone, Copy, Debug, Resource)]
pub struct MinibufferStyle {
    pub ascii: bool,
    pub newlines: bool,
}

impl Default for MinibufferStyle {
    fn default() -> Self {
        Self {
            ascii: true,
            newlines: false,
        }
    }
}

impl Style for MinibufferStyle {
    fn begin(&self, r: &mut dyn Renderer, section: Section) -> io::Result<()> {
        use Section::*;
        match section {
            Query(_answered) => {
                // if answered {
                //     // r.set_foreground(Green.dark())?;
                //     // write!(r, "{}", if self.ascii { "[x]" } else { "■" })?;
                //     // r.reset_color()?;
                //     // write!(r, " ")?;
                // } else {
                //     // r.set_foreground(Blue.dark())?;
                //     // write!(r, "{}", if self.ascii { "[ ]" } else { "▣" })?;
                //     // r.reset_color()?;
                //     // write!(r, " ")?;
                // }
            }
            Answer(show) => {
                r.set_foreground(Magenta.dark())?;
                if !show {
                    write!(r, "{}", if self.ascii { "..." } else { "…" })?;
                }
            } // was purple
            Toggle(selected) => {
                if selected {
                    r.set_foreground(Black.dark())?;
                    r.set_background(Blue.dark())?;
                    write!(r, " ")?;
                } else {
                    r.set_foreground(White.dark())?;
                    r.set_background(Black.light())?;
                    write!(r, " ")?;
                }
            }
            OptionExclusive(flags) => {
                match (
                    flags.contains(Flags::Focused),
                    flags.contains(Flags::Disabled),
                ) {
                    (false, _) => {
                        r.set_foreground(Black.light())?;
                        write!(r, "{}", if self.ascii { "( )" } else { "○" })?;
                        r.reset_color()?;
                    }
                    (true, true) => {
                        r.set_foreground(Red.dark())?;
                        write!(r, "{}", if self.ascii { "( )" } else { "○" })?;
                        r.reset_color()?;
                    }
                    (true, false) => {
                        r.set_foreground(Blue.dark())?;
                        write!(r, "{}", if self.ascii { "(x)" } else { "●" })?;
                        r.reset_color()?;
                    }
                }
                write!(r, " ")?;
                match (
                    flags.contains(Flags::Focused),
                    flags.contains(Flags::Disabled),
                ) {
                    (_, true) => {
                        r.set_foreground(Black.light())?;
                        // SetAttribute(Attribute::OverLined).write_ansi(f)?;
                    }
                    (true, false) => {
                        r.set_foreground(Blue.dark())?;
                    }
                    (false, false) => {}
                }
            }
            Option(flags) => {
                let prefix = match (
                    flags.contains(Flags::Selected),
                    flags.contains(Flags::Focused),
                ) {
                    (true, true) => {
                        if self.ascii {
                            "[o]"
                        } else {
                            "▣"
                        }
                    }
                    (true, false) => {
                        if self.ascii {
                            "[x]"
                        } else {
                            "■"
                        }
                    }
                    _ => {
                        if self.ascii {
                            "[ ]"
                        } else {
                            "□"
                        }
                    }
                };

                match (
                    flags.contains(Flags::Focused),
                    flags.contains(Flags::Selected),
                    flags.contains(Flags::Disabled),
                ) {
                    (true, _, true) => r.set_foreground(Red.dark())?,
                    (true, _, false) => r.set_foreground(Blue.dark())?,
                    (false, true, _) => r.reset_color()?,
                    (false, false, _) => r.set_foreground(Black.light())?,
                };
                write!(r, "{}", prefix)?;
                r.reset_color()?;
                write!(r, " ")?;
                match (
                    flags.contains(Flags::Focused),
                    flags.contains(Flags::Disabled),
                ) {
                    (_, true) => {
                        r.set_foreground(Black.light())?;
                        // SetAttribute(Attribute::OverLined).write_ansi(f)?;
                    }
                    (true, false) => {
                        r.set_foreground(Blue.dark())?;
                    }
                    (false, false) => {}
                }
            }
            Message => {}
            Action => {
                write!(r, " ")?;
                r.set_foreground(Blue.dark())?;
            }
            Validator(valid) => {
                r.set_foreground(if valid { Blue.dark() } else { Red.dark() })?;
            }
            Placeholder => {
                r.set_foreground(Black.light())?;
                write!(r, "Default: ")?;
            }
            Input => {
                // r.set_foreground(Blue.dark())?;
                // write!(r, "{}", if self.ascii { ">" } else { "›" })?;
                // r.reset_color()?;
                // write!(r, " ")?;
            }
            List => write!(r, "[")?,
            ListItem(first) => {
                if !first {
                    write!(r, ", ")?;
                }
            }
            Page(i, count) => {
                if count != 1 {
                    let icon = if self.ascii { "*" } else { "•" };
                    writeln!(r)?;
                    write!(r, "{}", " ".repeat(if self.ascii { 4 } else { 2 }))?;
                    r.set_foreground(Black.light())?;
                    write!(r, "{}", icon.repeat(i as usize))?;
                    r.reset_color()?;
                    write!(r, "{}", icon)?;
                    r.set_foreground(Black.light())?;
                    write!(r, "{}", icon.repeat(count.saturating_sub(i + 1) as usize))?;
                    r.reset_color()?;
                    writeln!(r)?;
                }
            }
            x => todo!("{:?} not impl", x),
            // x => {},
        }
        Ok(())
    }
    fn end(&self, r: &mut dyn Renderer, section: Section) -> io::Result<()> {
        use Section::*;
        match section {
            Query(_answered) => {
                write!(r, " ")?;
                // if answered {
                //     write!(r, " ")?;
                // } else if self.newlines {
                //     writeln!(r)?;
                // }
            }
            Answer(_) => {
                r.reset_color()?;
                if self.newlines {
                    writeln!(r)?;
                }
            }
            Toggle(_) => {
                write!(r, " ")?;
                r.reset_color()?;
                write!(r, "  ")?;
            }
            OptionExclusive(_flags) | Option(_flags) => {
                writeln!(r)?;
                r.reset_color()?;
            }
            List => write!(r, "]")?,
            ListItem(_) => {}
            Message => writeln!(r)?,
            _ => r.reset_color()?,
        }
        Ok(())
    }
}
