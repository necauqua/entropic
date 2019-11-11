use std::error::Error;
use std::io::{Write, stdout, stdin};

use entropic::term::*;
use entropic::input::{Events, Event, Modifiers, MouseButton};
use std::thread;
use std::sync::mpsc;
use std::sync::mpsc::Sender;

fn main() -> Result<(), Box<dyn Error>> {
    let mut input = Events::new(stdin());

    let mut stdout = stdout()
        .raw()?
        .alt_screen()?
        .mouse_input()?
        .cursor_control();

    stdout.set_cursor_hidden()?;
    stdout.flush()?;

    let mut x = 1u16;
    let mut y = 1u16;

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let mut out = std::io::stdout();
        loop {
            rx.recv().expect("channel should not be closed from the signal hook side");
            match write!(out, "\x1b[18t").and_then(|()| out.flush()) {
                Err(_) => break,
                _ => {},
            }
        }
    });

    unsafe {
        // you can't share the sender between threads and
        // signal handler could be called from any thread afaiu
        // well for me this works, and I would definitely do no better in C if you ask me
        // as I am trying to understand this should actually be okay-ish, idk really

        struct UnsafeSender(Sender<()>);
        unsafe impl Sync for UnsafeSender {}
        unsafe impl Send for UnsafeSender {}
        let tx = UnsafeSender(tx);
        signal_hook::register(signal_hook::SIGWINCH, move || {
            // ignoring error result not to panic in the signal handler
            // so if the channel closes we do nothing
            let _ = tx.0.send(());
        }).unwrap();
    }

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
                        stdout.write_at((x - 1) / 2 * 2 + 1, y, "  ")?;
                        y -= 1;
                        stdout.write_at((x - 1) / 2 * 2 + 1, y, "╺╸")?;
                    }
                    Event::ArrowDown => {
                        stdout.write_at((x - 1) / 2 * 2 + 1, y, "  ")?;
                        y += 1;
                        stdout.write_at((x - 1) / 2 * 2 + 1, y, "╺╸")?;
                    }
                    Event::ArrowRight => {
                        stdout.write_at((x - 1) / 2 * 2 + 1, y, "  ")?;
                        x += 2;
                        stdout.write_at((x - 1) / 2 * 2 + 1, y, "╺╸")?;
                    }
                    Event::ArrowLeft => {
                        stdout.write_at((x - 1) / 2 * 2 + 1, y, "  ")?;
                        x -= 2;
                        stdout.write_at((x - 1) / 2 * 2 + 1, y, "╺╸")?;
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
                    _event => {
                        print!("{:?}\n\x1b[999D", _event)
                    }
                }
            }
            Err(e) => return Err(Box::new(e)),
        }
    }

    Ok(())
}

