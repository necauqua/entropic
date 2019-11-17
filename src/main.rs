use std::error::Error;
use std::io::Write;

use entropic::{
    term::*,
    input::*,
    state::*,
    draw::*,
};
use std::io;

struct GuiState {
    terminal: Dimension,
    mouse: Position,
    buffer: TerminalState,
    picture: Picture,
}

impl GuiState {

    fn draw(&mut self) -> io::Result<()> {
        let Dimension { width, height } = self.terminal;

        let mx = self.mouse.x.max(3).min(width);
        let my = self.mouse.y.max(2).min(height);

        self.buffer.clear(self.terminal);

        self.buffer.put_text(Position::default(), CellColor::default().bg(Color::gray(120)), "  ");

        for i in 1..(width / 2).min(100) {
            let bg = if i == (mx - 1) / 2 {
                Color::gray(40)
            } else {
                Color::gray(120 - ((i % 2) as u8 * 20))
            };
            self.buffer.put_text(Position { x: i * 2, y: 0 }, CellColor::default().bg(bg), format!("{:0>2}", i));
        }

        for i in 1..height.min(100) {
            let bg = if i == my - 1 {
                Color::gray(40)
            } else {
                Color::gray(120 - ((i % 2) as u8 * 20))
            };
            self.buffer.put_text(Position { x: 0, y: i }, CellColor::default().bg(bg), format!("{:0>2}", i));
        }

        for (offset, pos) in self.picture.size.into_iter().enumerate() {
            let mut pixel = Pixel::default();
            for layer in self.picture.layers.iter() {
                pixel = Pixel::blend(pixel, layer.pixels[offset]);
            }
            self.buffer.put_text(Position { x: pos.x * 2 + 2, y: pos.y + 1 }, CellColor::default().bg(pixel.into()), "  ")
        }

        self.buffer.draw(self.terminal)
    }

    fn redraw(&mut self) -> io::Result<()> {
        self.buffer.redraw(self.terminal)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut input = Events::new(std::io::stdin());

    let mut term = TerminalBase
        .raw()?
        .mouse_input()?
        .cursor_control()?
        .terminal_resizes()?
        .alt_screen()?; // so that the switch back to primary buffer happens last

    let mut gui = GuiState {
        terminal: Dimension { width: 80, height: 24 },
        mouse: Position::default(),
        buffer: TerminalState::new(Dimension { width: 80 * 4, height: 24 * 4 }),
        picture: Picture {
            size: Dimension { width: 32, height: 32 },
            layers: vec![Layer { pixels: vec![Pixel { r: 0x3f, g: 0x3f, b: 0x3f, a: 0xff }; 32*32].into_boxed_slice() }]
        }
    };

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
//                    Event::Arrow(arrow, Modifiers::None) => {
//                        let old_cursor = cursor;
//                        match arrow {
//                            Arrow::Up => cursor.y -= 1,
//                            Arrow::Down => cursor.y += 1,
//                            Arrow::Right => cursor.x += 1,
//                            Arrow::Left => cursor.x -= 1,
//                        }
//                        term.write_at((old_cursor.x - 1) / 2 * 2 + 1, old_cursor.y, "  ")?;
//                        term.write_at((cursor.x - 1) / 2 * 2 + 1, cursor.y, "╺╸")?;
//                    }
                    Event::Press('r', Modifiers::Ctrl) => {
                        gui.buffer.redraw(gui.terminal)?;
                    }
                    Event::Mouse(_, MouseButton::Left, pos, _) => {
                        gui.mouse = pos;
                        let x = (pos.x - 3) / 2;
                        let y = pos.y - 2;
                        let pixels = &mut gui.picture.layers[0].pixels;
                        pixels[gui.picture.size.offset(Position { x, y })] = Pixel {
                            r: 255, g: 255, b: 255, a: 255
                        };
                        gui.draw()?;
                    }
                    Event::Mouse(_, _, pos, _) => {
                        gui.mouse = pos;
                        gui.draw()?;
                    }
                    Event::MouseMotion(pos, _) => {
                        gui.mouse = pos;
                        gui.draw()?;
                    }
                    Event::TerminalSize(terminal_size) => {
                        gui.terminal = terminal_size;
                        gui.redraw()?;
                        gui.draw()?;
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

