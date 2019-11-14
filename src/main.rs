use std::error::Error;
use std::io::Write;

use entropic::{
    term::*,
    input::*,
    state::*,
    draw::*,
};
use std::io;

type FullTerminal = AltScreen<TerminalResizes<CursorControl<MouseInput<Raw<TerminalBase>>>>>;

fn draw_gui(_term: &mut FullTerminal, size: Dimension, _mouse: Cursor, state: &mut TerminalState) -> io::Result<()> {
    let Dimension { width, height } = size;

//    let mx = mouse.x.max(3).min(width);
//    let my = mouse.y.max(2).min(height);

    state.put_text(0, 0, CellColor::default().bg(TerminalRgb::gray(120)), "  ");

    for i in 1..(width / 2 - 1).min(100) {
        let bg = TerminalRgb::gray(120 - ((i % 2) as u8 * 20));
        state.put_text(i * 2, 0, CellColor::default().bg(bg), format!("{:0>2}", i));
    }

    for i in 1..(height - 1).min(100) {
        let bg = TerminalRgb::gray(120 - ((i % 2) as u8 * 20));
        state.put_text(0, i, CellColor::default().bg(bg), format!("{:0>2}", i));

    }
    state.draw(size)
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

    let mut state = TerminalState::new(Dimension { width: 80 * 8, height: 24 * 8 });

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
                    Event::Arrow(arrow, Modifiers::None) => {
                        let old_cursor = cursor;
                        match arrow {
                            Arrow::Up => cursor.y -= 1,
                            Arrow::Down => cursor.y += 1,
                            Arrow::Right => cursor.x += 1,
                            Arrow::Left => cursor.x -= 1,
                        }
                        term.write_at((old_cursor.x - 1) / 2 * 2 + 1, old_cursor.y, "  ")?;
                        term.write_at((cursor.x - 1) / 2 * 2 + 1, cursor.y, "╺╸")?;
                    }
                    Event::Press('r', Modifiers::Ctrl) => {
                        state.redraw(size)?;
                    }
                    Event::Mouse(_, _, pos, _) => {
                        cursor = pos;
                        draw_gui(&mut term, size, cursor, &mut state)?
                    }
                    Event::MouseMotion(pos, _) => {
                        cursor = pos;
                        draw_gui(&mut term, size, cursor, &mut state)?
                    }
                    Event::TerminalSize(terminal_size) => {
                        size = terminal_size;
                        draw_gui(&mut term, size, cursor, &mut state)?
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

