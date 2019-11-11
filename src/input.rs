use std::{io, thread};
use std::io::{ErrorKind, Read};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

#[derive(Debug)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    Extra(u8),
}

#[derive(Debug)]
pub enum MouseWheelDirection {
    Up,
    Down,
}

#[derive(Debug)]
pub enum Modifiers {
    // made as an enum for simpler matching
    None,
    Alt,
    Ctrl,
    CtrlAlt,
}

#[derive(Debug)]
pub struct MousePosition {
    pub x: u16,
    pub y: u16,
}

#[derive(Debug)]
pub enum MouseAction {
    Press,
    Release,
    Drag,
}

#[derive(Debug)]
pub enum Event {
    Enter,
    Space,
    Tab,
    Escape,
    Backspace,

    Home,
    End,
    Insert,
    Delete,
    PgUp,
    PgDown,

    ArrowUp,
    ArrowDown,
    ArrowRight,
    ArrowLeft,

    Press(char, Modifiers),

    Mouse(MouseAction, MouseButton, MousePosition, Modifiers),
    MouseWheel(MouseWheelDirection, MousePosition, Modifiers),

    UnknownByteSequence(Vec<u8>),
}

pub struct Events {
    rx: Receiver<io::Result<Event>>
}

fn parse_decimal(bytes: &[u8]) -> u16 {
    if bytes.len() > 3 || bytes.len() == 0 {
        return 1000;
    }
    let mut res = 0;
    for &byte in bytes { // this should unroll I guess
        if byte > 47 && byte < 58 {
            res = res * 10 + (byte - 48) as u16;
        } else {
            return 1000;
        };
    }
    res
}

fn parse_input_sequence(bytes: &[u8]) -> Event {
    if bytes.len() == 1 {
        return match bytes[0] {
            27 => Event::Escape,
            9 => Event::Tab,
            32 => Event::Space,
            127 => Event::Backspace,
            b if b < 32 => Event::Press(char::from(b + 96), Modifiers::Ctrl),
            b => Event::Press(char::from(b), Modifiers::None),
        }
    }
    // all next patterns start with ESC, just handle it at the beginning
    if bytes[0] != 27 {
        return Event::UnknownByteSequence(bytes.to_vec());
    }
    if bytes.len() == 2 {
        let b = bytes[1];
        return if b < 32 {
            Event::Press(char::from(b + 96), Modifiers::CtrlAlt)
        } else {
            Event::Press(char::from(b), Modifiers::Alt)
        };
    }
    // all other ones start with CSI (ESC+[)
    if bytes[1] != 91 {
        return Event::UnknownByteSequence(bytes.to_vec());
    }
    let code = &bytes[2..];
    match code {
        [65] => Event::ArrowUp,
        [66] => Event::ArrowDown,
        [67] => Event::ArrowRight,
        [68] => Event::ArrowLeft,

        [70] => Event::End,
        [72] => Event::Home,

        [50, 126] => Event::Insert,
        [51, 126] => Event::Delete,
        [53, 126] => Event::PgUp,
        [54, 126] => Event::PgDown,

        _ if code.len() > 1 && code[0] == 60 => {
            let mut split = (&code[1..]).split(|&b| b == 59);

            let b = split.next();
            let x = split.next();
            let y = split.next();

            if split.next().is_none() && y.is_some() {
                let b = parse_decimal(b.unwrap());
                let x = parse_decimal(x.unwrap());

                let y = y.unwrap();
                if let Some((&s, y)) = y.split_last() {
                    let y = parse_decimal(y);
                    if b != 1000 && x != 1000 && y != 1000 {
                        let pos = MousePosition { x, y };
                        let mods = match (b & 0b11000) >> 3 {
                            0 => Modifiers::None,
                            1 => Modifiers::Alt,
                            2 => Modifiers::Ctrl,
                            3 => Modifiers::CtrlAlt,
                            _ => unreachable!(),
                        };
                        if b & 0b1000000 != 0 { // wheel bit
                            let dir = if b & 0b1 == 0 { MouseWheelDirection::Up } else { MouseWheelDirection::Down };
                            return Event::MouseWheel(dir, pos, mods);
                        }
                        let button = match b & 0b11 {
                            0 => MouseButton::Left,
                            1 => MouseButton::Middle,
                            2 => MouseButton::Right,
                            3 => MouseButton::Extra(3), // idk actually
                            _ => unreachable!(),
                        };
                        let action = if s == 109 {
                            MouseAction::Release
                        } else if b & 0b100000 != 0 { // drag bit
                            MouseAction::Drag
                        } else {
                            MouseAction::Press
                        };
                        return Event::Mouse(action, button, pos, mods);
                    }
                }
            }
            Event::UnknownByteSequence(bytes.to_vec())
        }
        _ => Event::UnknownByteSequence(bytes.to_vec())
    }
}

fn spawn_reader<R: Read + Send + 'static>(mut input: R, tx: Sender<io::Result<Event>>) {

    // if you like super-rapidly click, then this would send some UnknownByteSequence's and
    // miss a couple of clicks but this is more than enough for everything to work fine

    thread::spawn(move || loop {
        // mouse event with both coords >100 takes 13 bytes, max found yet
        let mut buf = [0; 13];

        let res = match input.read(&mut buf) {
            Ok(bytes_read) => {
                match bytes_read {
                    0 => break, // 0 bytes read means EOF
                    _ => tx.send(Ok(parse_input_sequence(&buf[0..bytes_read])))
                }
            }
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => break,
            Err(e) => tx.send(Err(e)),
        };
        if res.is_err() {
            // SendError only occurs when you close the channel, so we just shut down
            break;
        }
    });
}

impl Events {
    // this should be called once per a program lifetime
    // there is no mechanism for stopping the thread,
    // we rely just on Rust closing everything on program termination
    pub fn new<R: Read + Send + 'static>(input: R) -> Events {
        let (tx, rx) = mpsc::channel();
        spawn_reader(input, tx);
        Events { rx }
    }

    pub fn next(&mut self) -> io::Result<Event> {
        // SendError never occurs unless the stdin is closed, and this should not happen
        return self.rx.recv().unwrap();
    }
}
