use std::cell::RefCell;
use std::error::Error;
use std::io;
use std::io::Write;
use std::rc::Rc;

use crossbeam_channel::select;

use entropic::{
    draw::*,
    input::*,
    state::*,
    term::*,
};

trait Widget {
    fn draw(&self, gui: &mut GuiState) -> io::Result<()>;

    fn get_bounds(&self) -> (Position, Dimension);

    fn on_mouse_input(&mut self, _: MouseAction, _: MouseButton, _: Position, _: Modifiers) -> io::Result<()> {
        Ok(())
    }
}

struct TheField {
    picture: Picture
}

impl Widget for TheField {
    fn draw(&self, gui: &mut GuiState) -> io::Result<()> {
        let Dimension { width, height } = self.picture.size.min(gui.terminal);

        let mx = gui.mouse.x.max(3).min(width * 2);
        let my = gui.mouse.y.max(2).min(height + 1);

        gui.buffer.clear(gui.terminal);

        gui.buffer.put_text(Position::default(), CellColor::default().bg(Color::gray(100)), "--");

        for i in 0..width.min(100) {
            let bg = if i == (mx - 2) / 2 {
                Color::gray(40)
            } else {
                Color::gray(120 - ((i % 2) as u8 * 20))
            };
            gui.buffer.put_text(Position { x: i * 2 + 2, y: 0 }, CellColor::default().bg(bg), format!("{:0>2}", i));
        }

        for i in 0..height.min(100) {
            let bg = if i == my - 2 {
                Color::gray(40)
            } else {
                Color::gray(120 - ((i % 2) as u8 * 20))
            };
            gui.buffer.put_text(Position { x: 0, y: i + 1 }, CellColor::default().bg(bg), format!("{:0>2}", i));
        }

        for (offset, pos) in self.picture.size.into_iter().enumerate() {
            let mut pixel = Pixel::default();
            for layer in self.picture.layers.iter() {
                pixel = Pixel::blend(pixel, layer.pixels[offset]);
            }
            gui.buffer.put_text(Position { x: pos.x * 2 + 2, y: pos.y + 1 }, CellColor::default().bg(pixel.into()), "  ")
        }

        gui.buffer.draw(gui.terminal)?;

        Ok(())
    }

    fn get_bounds(&self) -> (Position, Dimension) {
        let Dimension { width, height } = self.picture.size;
        (Position { x: 0, y: 0 }, Dimension { width: width * 2 + 1, height: height + 1 })
    }

    fn on_mouse_input(&mut self, _: MouseAction, button: MouseButton, pos: Position, _: Modifiers) -> io::Result<()> {
        let pic = &mut self.picture;
        let size = pic.size;
        let pos = Position { x: pos.x / 2, ..pos };
        if pos.x >= 1 && pos.y >= 1 {
            let pos = pos - Position { x: 1, y: 1 };
            let pixels = &mut pic.layers[0].pixels;
            pixels[size.offset(pos)] = Pixel {
                r: 255,
                g: match button { MouseButton::Left => 255, _ => 0 },
                b: 255,
                a: 255,
            };
        }
        Ok(())
    }
}

struct GuiState {
    terminal: Dimension,
    mouse: Position,
    buffer: TerminalState,
    widgets: Rc<RefCell<Vec<Box<dyn Widget>>>>,
}

impl GuiState {
    pub fn for_each_widget(&mut self, mut f: impl FnMut(&mut Self, &mut dyn Widget) -> io::Result<()>) -> io::Result<()> {
        let rc = self.widgets.clone();
        let mut widgets = (*rc).borrow_mut();
        for widget in widgets.iter_mut() {
            f(self, &mut **widget)?;
        }
        Ok(())
    }

    fn draw(&mut self) -> io::Result<()> {
        self.for_each_widget(|gui, widget| widget.draw(gui))
    }

    fn redraw(&mut self) -> io::Result<()> {
        self.buffer.redraw(self.terminal)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let base_term = TerminalBase
        .raw()?
        .mouse_input()?
        .hide_cursor()?
        .no_wrap()?
        .alt_screen()?;

    let term_hook = base_term.clone();

    let mut term = base_term.terminal_resizes()?;
    let resizes_rx = term.get_resize_event_receiver().clone();

    let hook = color_backtrace::create_panic_handler(color_backtrace::Settings::default());
    std::panic::set_hook(Box::new(move |panic_info| {
        // apparently panic info prints before unwinding,
        // so drop calls happen only after its already printed
        let _ = term_hook.dont_listen_to_mouse();
        let _ = term_hook.switch_to_normal();
        let _ = term_hook.normal_mode();

        hook(panic_info);
        // set stored cursor pos so that switch_to_normal call from the actual drop
        // does not restore the cursor to original position but rather
        // to the position where the panic output left it
        print!("\x1b[s");
    }));

    let (w, h) = term_size::dimensions_stdout().expect("can't get terminal dimensions, todo handle this");

    let mut gui = GuiState {
        terminal: Dimension { width: w as u16, height: h as u16 },
        mouse: Position::default(),
        buffer: TerminalState::new(Dimension { width: 80 * 4, height: 24 * 4 }),
        widgets: Rc::new(RefCell::new(vec![
            Box::new(TheField {
                picture: Picture {
                    size: Dimension { width: 32, height: 32 },
                    layers: vec![Layer { pixels: vec![Pixel { r: 0x3f, g: 0x3f, b: 0x3f, a: 0xff }; 32 * 32].into_boxed_slice() }],
                }
            })
        ])),
    };

    gui.draw()?;

    let events = create_event_receiver(std::io::stdin());

    loop {
        select! {
            recv(events) -> event => {
                match event {
                    Ok(Ok(event)) => {
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
                                gui.redraw()?;
                            }
                            Event::Mouse(action, button, mouse, modifiers) => {
                                gui.mouse = mouse;
                                gui.for_each_widget(|_, widget| {
                                    let (pos, size) = widget.get_bounds();

                                    if mouse.x > pos.x &&
                                        mouse.y > pos.y &&
                                        mouse.x <= pos.x + size.width &&
                                        mouse.y <= pos.y + size.height
                                    {
                                        widget.on_mouse_input(action, button, mouse - pos, modifiers)?;
                                    }

                                    Ok(())
                                })?;
                                gui.draw()?;
                            }
                            Event::MouseMotion(pos, _) => {
                                gui.mouse = pos;
                                gui.draw()?;
                            }
                            Event::TerminalSize(terminal_size) => {
                                gui.terminal = terminal_size;
                                gui.draw()?;
                                gui.redraw()?;
                            }
                            _event => {
                                write!(term, "{:?}\n\x1b[999D", _event)?;
                                term.flush()?;
                            }
                        }
                    }
                    Ok(Err(e)) => return Err(Box::new(e)),
                    Err(e) => return Err(Box::new(e)),
                }
            }
            recv(resizes_rx) -> _ => {
                let (w, h) = term_size::dimensions_stdout().expect("can't get terminal dimensions, todo handle this");
                gui.terminal = Dimension { width: w as u16, height: h as u16 };
                gui.draw()?;
                gui.redraw()?;
            }
        }
        ;
    }

    Ok(())
}

