use std::error::Error;
use std::io::Write;

use entropic::{
    term::*,
    input::*
};
use entropic::state::Dimension;

fn main() -> Result<(), Box<dyn Error>> {
    let mut input = Events::new(std::io::stdin());

    let mut stdout = TerminalBase
        .raw()?
        .alt_screen()?
        .mouse_input()?
        .cursor_control()?
        .terminal_resizes()?;

    let mut cursor = Cursor { x: 1, y: 1 };

    loop {
        match input.next() {
            Ok(event) => {
                match event {
                    Event::Press('c', Modifiers::Ctrl) => break,
                    Event::Press('q', Modifiers::None) => break,
                    Event::Press('l', Modifiers::Ctrl) => {
                        write!(stdout, "\x1b[2J\x1b[1;1H")?;
                        stdout.flush()?;
                    }
                    Event::ArrowUp => {
                        stdout.write_at((cursor.x - 1) / 2 * 2 + 1, cursor.y, "  ")?;
                        cursor.y -= 1;
                        stdout.write_at((cursor.x - 1) / 2 * 2 + 1, cursor.y, "╺╸")?;
                    }
                    Event::ArrowDown => {
                        stdout.write_at((cursor.x - 1) / 2 * 2 + 1, cursor.y, "  ")?;
                        cursor.y += 1;
                        stdout.write_at((cursor.x - 1) / 2 * 2 + 1, cursor.y, "╺╸")?;
                    }
                    Event::ArrowRight => {
                        stdout.write_at((cursor.x - 1) / 2 * 2 + 1, cursor.y, "  ")?;
                        cursor.x += 2;
                        stdout.write_at((cursor.x - 1) / 2 * 2 + 1, cursor.y, "╺╸")?;
                    }
                    Event::ArrowLeft => {
                        stdout.write_at((cursor.x - 1) / 2 * 2 + 1, cursor.y, "  ")?;
                        cursor.x -= 2;
                        stdout.write_at((cursor.x - 1) / 2 * 2 + 1, cursor.y, "╺╸")?;
                    }
                    Event::Mouse(_, button, pos, _) => {
                        match button {
                            MouseButton::Left => {
                                let p = "██▒▒";
                                write!(stdout, "\x1b[s\x1b[{1};{0}H{2}\x1b[u", (pos.x - 1) / 2 * 2 + 1, pos.y, p)?;
                                stdout.flush()?;
                            }
                            MouseButton::Right => {
                                let p = "  ";
                                stdout.write_at((pos.x - 1) / 2 * 2 + 1, pos.y, p)?;
                            }
                            _ => {}
                        }
                    }
                    Event::TerminalSize(Dimension { width, height }) => {
                        write!(stdout, "\x1b[2J\x1b[1;1H")?;
                        write!(stdout, "┏{}┓", "━".repeat((width - 2) as usize))?;
                        for i in 0..height - 2 {
                            write!(stdout, "\x1b[{};1H", i+2)?;
                            write!(stdout, "┃{}┃", " ".repeat((width - 2) as usize))?;
                            write!(stdout, "┃{}┃", " ".repeat((width - 2) as usize))?;
                        }
                        write!(stdout, "\x1b[{};1H", height)?;
                        write!(stdout, "┗{}┛", "━".repeat((width - 2) as usize))?;
                        stdout.cursor_move(2, 2)?;
                        write!(stdout, "w: {}, h: {}", width, height)?;
                        stdout.flush()?;
                    }
                    _event => {
                        write!(stdout, "{:?}\n\x1b[999D", _event)?;
                        stdout.flush()?;
                    }
                }
            }
            Err(e) => return Err(Box::new(e)),
        }
    }

    Ok(())
}

