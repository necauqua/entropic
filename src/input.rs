use std::{io, thread};
use std::io::{ErrorKind, Read};
use crossbeam_channel;
use crossbeam_channel::{Receiver, Sender};

use crate::state::{Dimension, Position};

#[derive(Debug)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
}

#[derive(Copy, Clone, Debug)]
pub enum MouseWheelDirection {
    Up,
    Down,
}

#[derive(Copy, Clone, Debug)]
pub enum Modifiers {
    // made as an enum for simpler matching
    None,
    Shift,
    Alt,
    AltShift,
    Ctrl,
    CtrlShift,
    CtrlAlt,
    CtrlShiftAlt,
}

#[derive(Copy, Clone, Debug)]
pub enum MouseAction {
    Press,
    Release,
    Drag,
}

#[derive(Copy, Clone, Debug)]
pub enum Arrow {
    Up,
    Down,
    Right,
    Left,
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

    Arrow(Arrow, Modifiers),

    Press(char, Modifiers),

    Mouse(MouseAction, MouseButton, Position, Modifiers),
    MouseMotion(Position, Modifiers),
    MouseWheel(MouseWheelDirection, Position, Modifiers),

    TerminalSize(Dimension),

    UnknownByteSequence(Vec<u8>),
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

// this is so stupid, ikr
// idk for what reason I fear allocations in low-level parsing,
// or why I do this lowest-level stupid parsing at all
// this stupid rust newcomer disease I guess ¯\_(ツ)_/¯
fn read_params(bytes: &[u8], until: u8, until2: u8) -> ([u16; 3], usize) {
    let mut res = [0; 3];
    let mut buf = [0; 3];
    let mut buf_idx = 0;
    let mut res_idx = 0;
    let mut read = 0;
    for &byte in bytes {
        read += 1;
        if byte == until || byte == until2 {
            if res_idx > 3 {
                return ([1000; 3], read);
            }
            res[res_idx] = parse_decimal(&buf[..buf_idx]);
            return (res, read);
        }
        if byte == 59 {
            if res_idx > 3 {
                return ([1000; 3], read);
            }
            res[res_idx] = parse_decimal(&buf[..buf_idx]);
            buf_idx = 0;
            res_idx += 1;
        } else {
            if buf_idx > 3 {
                return ([1000; 3], read);
            }
            buf[buf_idx] = byte;
            buf_idx += 1;
        }
    }
    ([1000; 3], read)
}

fn parse_arrow(byte: u8) -> Option<Arrow> {
    match byte {
        65 => Some(Arrow::Up),
        66 => Some(Arrow::Down),
        67 => Some(Arrow::Right),
        68 => Some(Arrow::Left),
        _ => None,
    }
}

fn parse_mods(byte: u8) -> Option<Modifiers> {
    match byte {
        50 => Some(Modifiers::Shift),
        51 => Some(Modifiers::Alt),
        52 => Some(Modifiers::AltShift),
        53 => Some(Modifiers::Ctrl),
        54 => Some(Modifiers::CtrlShift),
        55 => Some(Modifiers::CtrlAlt),
        56 => Some(Modifiers::CtrlShiftAlt),
        _ => None,
    }
}

macro_rules! fail {
    ($bytes:ident) => {
        return (Event::UnknownByteSequence($bytes.to_vec()), $bytes.len())
    };
}

/// This method tries to parse a sequence of bytes into an event.
/// It returns an event, and a number of consumed bytes.
/// If it fails to parse the event, it returns `Event::UnknownByteSequence`
/// with all the bytes received, and the length of the given slice
fn parse_input_sequence(bytes: &[u8]) -> (Event, usize) {
    if bytes.len() == 1 {
        return (match bytes[0] {
            27 => Event::Escape,
            9 => Event::Tab,
            32 => Event::Space,
            127 => Event::Backspace,
            b if b < 32 => Event::Press(char::from(b + 96), Modifiers::Ctrl),
            b => {
                let ch = char::from(b);
                if ch.is_ascii_uppercase() {
                    Event::Press(ch.to_ascii_lowercase(), Modifiers::Shift)
                } else {
                    Event::Press(ch, Modifiers::None)
                }
            }
        }, 1);
    }
    // all next patterns start with ESC, just handle it at the beginning
    if bytes[0] != 27 {
        fail!(bytes);
    }
    if bytes.len() == 2 {
        let b = bytes[1];
        return (if b < 32 {
            Event::Press(char::from(b + 96), Modifiers::CtrlAlt)
        } else {
            let ch = char::from(b);
            if ch.is_ascii_uppercase() {
                Event::Press(ch.to_ascii_lowercase(), Modifiers::AltShift)
            } else {
                Event::Press(ch, Modifiers::Alt)
            }
        }, 2);
    }
    // all other ones start with CSI (ESC+[)
    if bytes[1] != 91 {
        fail!(bytes);
    }
    let code = &bytes[2..];
    match code {
        [b @ 65..=68] => (Event::Arrow(parse_arrow(*b).unwrap(), Modifiers::None), 3),

        [70] => (Event::End, 3),
        [72] => (Event::Home, 3),

        [50, 126] => (Event::Insert, 4),
        [51, 126] => (Event::Delete, 4),
        [53, 126] => (Event::PgUp, 4),
        [54, 126] => (Event::PgDown, 4),

        [49, 59, mods @ 50..=56, arrow] => {
            match parse_arrow(*arrow) {
                Some(arrow) => (Event::Arrow(arrow, parse_mods(*mods).unwrap()), 6),
                _ => fail!(bytes)
            }
        }

        // CSI '8' ';' height ';' width 't'
        _ if code.len() > 2 && code[0] == 56 && code[1] == 59 => {
            let ([height, width, extra], read) = read_params(&code[2..], 116, 116);
            if height == 1000 || width == 1000 || extra != 0 {
                fail!(bytes);
            }
            (Event::TerminalSize(Dimension { width, height }), read + 4)
        }

        _ if code.len() > 1 && code[0] == 60 => {
            let ([b, x, y], read) = read_params(&code[1..], 109, 77);
            if b == 1000 || x == 1000 || y == 1000 {
                fail!(bytes);
            }
            let pos = Position { x, y };
            let mods = match (b & 0b11100) >> 2 {
                0b000 => Modifiers::None,
                0b001 => Modifiers::Shift,
                0b010 => Modifiers::Alt,
                0b011 => Modifiers::AltShift,
                0b100 => Modifiers::Ctrl,
                0b101 => Modifiers::CtrlShift,
                0b110 => Modifiers::CtrlAlt,
                0b111 => Modifiers::CtrlShiftAlt,
                _ => unreachable!(),
            };
            if b & 0b1000000 != 0 { // wheel bit
                let dir = if b & 0b1 == 0 { MouseWheelDirection::Up } else { MouseWheelDirection::Down };
                return (Event::MouseWheel(dir, pos, mods), read + 3);
            }
            let action = if code[read] == 109 {
                MouseAction::Release
            } else if b & 0b100000 != 0 { // drag bit
                MouseAction::Drag
            } else {
                MouseAction::Press
            };
            let button = match b & 0b11 {
                0b00 => MouseButton::Left,
                0b01 => MouseButton::Middle,
                0b10 => MouseButton::Right,
                0b11 => return (Event::MouseMotion(pos, mods), read + 3),
                _ => unreachable!(),
            };
            return (Event::Mouse(action, button, pos, mods), read + 3);
        }
        _ => fail!(bytes)
    }
}

/// Creates a new instance of the receiver of the console events
/// that might be read from the given input.
///
/// It spins up a thread that will read the input forever, with no
/// means of stopping it other than closing the input or the receiver.
/// How the input closes depends on its implementation, but you can
/// simply drop the receiver.
/// Note that the thread would only close after it goes out of being
/// blocked by receiving some data from the input.
///
/// The algorithm relies on the fact that it reads individual key presses
/// as separate chunks, and when it receives a lot of bytes at once
/// it tries to parse sequences of them, checking how many bytes they consumed and
/// storing the excesses (when receiving events rapidly enough) for the next iterations
///
pub fn create_event_receiver<R: Read + Send + 'static>(mut input: R) -> Receiver<io::Result<Event>> {
    let (tx, rx) = crossbeam_channel::unbounded();
    thread::spawn(move || {
        let mut offset = 0;
        // mouse event with both coords >100 takes 13 bytes, max found yet
        let mut buf = [0; 16];
        loop {
            let res = match input.read(&mut buf[offset..]) {
                Ok(bytes_read) => {
                    match bytes_read {
                        0 => break, // 0 bytes read means EOF
                        _ => {
                            let size = bytes_read + offset;
                            let (event, read) = parse_input_sequence(&buf[0..size]);
                            if read < size {
                                // store the unconsumed part as part of the next buffer
                                offset = size - read;
                                buf.copy_within(read..size, 0);
                            } else {
                                offset = 0;
                            }
                            tx.send(Ok(event))
                        }
                    }
                }
                Err(e) if e.kind() == ErrorKind::UnexpectedEof => break,
                Err(e) => tx.send(Err(e)),
            };
            if res.is_err() {
                // SendError only occurs when you close the channel, so we just shut down
                break;
            }
        }
    });
    rx
}
