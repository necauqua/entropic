use std::error::Error;
use std::io::{Read, Write, stdout};

use entropic::term::*;

fn main() -> Result<(), Box<dyn Error>> {
    let mut stdin = std::io::stdin();

    let mut stdout = stdout()
        .raw()?
        .alt_screen()?
        .mouse_input()?
        .cursor_control();

    stdout.set_cursor_hidden()?;
    stdout.flush()?;

    let mut state = 0;

    let mut x = 1;
    let mut y = 1;

    loop {
        let mut buf = [0; 1];
        stdin.read_exact(&mut buf)?;
        let byte = buf[0];

        match byte {
            3 | 113 => break, // Ctrl+C or Q to exit
            12 => {           // Ctrl+L to clear the screen
                write!(stdout, "\x1b[s\x1b[2J\x1b[3J\x1b[u")?;
                stdout.flush()?;
                state = 0;
            }
            27 if state == 0 => state = 1, // \e
            91 if state == 1 => state = 2, // [
            65 if state == 2 => {
                stdout.write_at((x as u32 - 1) / 2 * 2 + 1, y as u32, "  ")?;
                y -= 1;
                stdout.write_at((x as u32 - 1) / 2 * 2 + 1, y as u32, "╺╸")?;

                state = 0;
            }
            66 if state == 2 => {
                stdout.write_at((x as u32 - 1) / 2 * 2 + 1, y as u32, "  ")?;
                y += 1;
                stdout.write_at((x as u32 - 1) / 2 * 2 + 1, y as u32, "╺╸")?;

                state = 0;
            }
            67 if state == 2 => {
                stdout.write_at((x as u32 - 1) / 2 * 2 + 1, y as u32, "  ")?;
                x += 2;
                stdout.write_at((x as u32 - 1) / 2 * 2 + 1, y as u32, "╺╸")?;

                state = 0;
            }
            68 if state == 2 => {
                stdout.write_at((x as u32 - 1) / 2 * 2 + 1, y as u32, "  ")?;
                x -= 2;
                stdout.write_at((x as u32 - 1) / 2 * 2 + 1, y as u32, "╺╸")?;

                state = 0;
            }
            77 if state == 2 => {          // M
                let mut buf = [0; 3];
                stdin.read_exact(&mut buf)?;
                let b = buf[0] - 32;
                let x = buf[1] - 32;
                let y = buf[2] - 32;

                if b == 0 || b == 32 {
                    let p = "██▒▒";

                    write!(stdout, "\x1b[s\x1b[{1};{0}H{2}\x1b[u", (x - 1) / 2 * 2 + 1, y, p)?;
                    stdout.flush()?;
                }

                if b == 2 || b == 34 {
                    let p = "  ";

                    stdout.write_at((x as u32 - 1) / 2 * 2 + 1, y as u32, p)?;
                }

                state = 0;
            }
            _ => state = 0,
        }
    }

    Ok(())
}

