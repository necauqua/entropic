use std::error::Error;
use std::io::Write;

use entropic::{
    term::*,
    input::*,
    state::*,
};
use std::io;
use std::cmp::{min, max};

type FullTerminal = AltScreen<TerminalResizes<CursorControl<MouseInput<Raw<TerminalBase>>>>>;

fn draw_gui(term: &mut FullTerminal, size: Dimension, mouse: Cursor) -> io::Result<()> {
    let Dimension { width, height } = size;

    term.write_all(b"\x1b[2J")?;
    term.cursor_move(1, 1)?;

    let mx = mouse.x.max(2).min(width - 2);
    let my = mouse.y.max(2).min(height);

    write!(term, "\x1b[48;2;120;120;120m  \x1b[0m")?;
    for i in 1..min(width / 2, 100) {
        if i % 2 == 0 {
            term.write_all(b"\x1b[48;2;120;120;120m")?;
        } else {
            term.write_all(b"\x1b[48;2;100;100;100m")?;
        }
        let x_off = i * 2 + 1;
        write!(term, "{:0>2}\x1b[{};{}H\x1b[48;2;80;80;80m  \x1b[0m\x1b[1;{}H", i, my, x_off, x_off + 2)?;
    }

    for i in 1..min(height, 100) {
        write!(term, "\x1b[{};1H", i + 1)?;
        if i % 2 == 0 {
            term.write_all(b"\x1b[48;2;120;120;120m")?;
        } else {
            term.write_all(b"\x1b[48;2;100;100;100m")?;
        }
        let y_off = i;
        write!(term, "{:0>2}\x1b[0m\x1b[{};{}H\x1b[48;2;80;80;80m  \x1b[0m\x1b[{};1H", i, y_off+1, mx / 2 * 2 + 1, y_off + 4)?;
    }
    term.write_all(b"\x1b[0m")?;
    term.flush()
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut input = Events::new(std::io::stdin());

    let mut term = TerminalBase
        .raw()?
        .mouse_input()?
        .cursor_control()?
        .terminal_resizes()?
        .alt_screen()?; // so that the switch back to primary buffer happens last

    let mut cursor = Cursor { x: 2, y: 2 };
    let mut size = Dimension { width: 80, height: 24 };

    term.send_size()?;

    loop {
        match input.next() {
            Ok(event) => {
                match event {
                    Event::Press('c', Modifiers::Ctrl) => break,
                    Event::Press('q', Modifiers::None) => break,
                    Event::Press('l', Modifiers::Ctrl) => {
                        write!(term, "\x1b[2J\x1b[1;1H")?;
                        term.flush()?;
                    }
                    Event::ArrowUp => {
                        term.write_at((cursor.x - 1) / 2 * 2 + 1, cursor.y, "  ")?;
                        cursor.y -= 1;
                        term.write_at((cursor.x - 1) / 2 * 2 + 1, cursor.y, "╺╸")?;
                    }
                    Event::ArrowDown => {
                        term.write_at((cursor.x - 1) / 2 * 2 + 1, cursor.y, "  ")?;
                        cursor.y += 1;
                        term.write_at((cursor.x - 1) / 2 * 2 + 1, cursor.y, "╺╸")?;
                    }
                    Event::ArrowRight => {
                        term.write_at((cursor.x - 1) / 2 * 2 + 1, cursor.y, "  ")?;
                        cursor.x += 2;
                        term.write_at((cursor.x - 1) / 2 * 2 + 1, cursor.y, "╺╸")?;
                    }
                    Event::ArrowLeft => {
                        term.write_at((cursor.x - 1) / 2 * 2 + 1, cursor.y, "  ")?;
                        cursor.x -= 2;
                        term.write_at((cursor.x - 1) / 2 * 2 + 1, cursor.y, "╺╸")?;
                    }
                    Event::Mouse(_, _, pos, _) => {
                        cursor = pos;
                        draw_gui(&mut term, size, cursor)?
                    }
                    Event::MouseMotion(pos, _) => {
                        cursor = pos;
                        draw_gui(&mut term, size, cursor)?
                    }
                    Event::TerminalSize(terminal_size) => {
                        size = terminal_size;
                        draw_gui(&mut term, size, cursor)?
                    }
                    _event => {
                        write!(term, "{:?}\n\x1b[999D", _event)?;
                        term.flush()?;
                    }
                }
            }
            Err(e) => return Err(Box::new(e)),
        }
    }

    Ok(())
}

