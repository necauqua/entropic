use std::{io, thread};
use std::io::{ErrorKind, Read};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

use crate::state::Dimension;

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

    TerminalResize(Dimension),

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
            b => Event::Press(char::from(b), Modifiers::None),
        }, 1)
    }
    // all next patterns start with ESC, just handle it at the beginning
    if bytes[0] != 27 {
        return (Event::UnknownByteSequence(bytes.to_vec()), bytes.len());
    }
    if bytes.len() == 2 {
        let b = bytes[1];
        return (if b < 32 {
            Event::Press(char::from(b + 96), Modifiers::CtrlAlt)
        } else {
            Event::Press(char::from(b), Modifiers::Alt)
        }, 2);
    }
    // all other ones start with CSI (ESC+[)
    if bytes[1] != 91 {
        return (Event::UnknownByteSequence(bytes.to_vec()), bytes.len());
    }
    let code = &bytes[2..];
    match code {
        [65] => (Event::ArrowUp, 3),
        [66] => (Event::ArrowDown, 3),
        [67] => (Event::ArrowRight, 3),
        [68] => (Event::ArrowLeft, 3),

        [70] => (Event::End, 3),
        [72] => (Event::Home, 3),

        [50, 126] => (Event::Insert, 4),
        [51, 126] => (Event::Delete, 4),
        [53, 126] => (Event::PgUp, 4),
        [54, 126] => (Event::PgDown, 4),

        // CSI '8' ';' height ';' width 't'
        _ if code.len() > 2 && code[0] == 56 && code[1] == 59 => {
            let ([height, width, extra], read) = read_params(&code[2..], 116, 116);
            if height != 1000 && width != 1000 && extra == 0 {
                (Event::TerminalResize(Dimension { width, height }), read + 4)
            } else {
                (Event::UnknownByteSequence(bytes.to_vec()), bytes.len())
            }
        }

        _ if code.len() > 1 && code[0] == 60 => {
            let ([b, x, y], read) = read_params(&code[1..], 109, 77);
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
                    return (Event::MouseWheel(dir, pos, mods), read + 3);
                }
                let button = match b & 0b11 {
                    0 => MouseButton::Left,
                    1 => MouseButton::Middle,
                    2 => MouseButton::Right,
                    3 => MouseButton::Extra(3), // idk actually
                    _ => unreachable!(),
                };
                let action = if code[read] == 109 {
                    MouseAction::Release
                } else if b & 0b100000 != 0 { // drag bit
                    MouseAction::Drag
                } else {
                    MouseAction::Press
                };
                return (Event::Mouse(action, button, pos, mods), read + 3);
            }
            (Event::UnknownByteSequence(bytes.to_vec()), bytes.len())
        }
        _ => (Event::UnknownByteSequence(bytes.to_vec()), bytes.len())
    }
}

/// Spawn a thread that will read the data from given input forever
/// and will try to parse it into a stream of events.
/// It sends those events into the given sender.
///
/// The algorithm relies on the fact that it reads individual key presses
/// as separate chunks, and when it receives a lot of bytes at once
/// it tries to parse sequences of them, checking how many bytes they consumed and
/// storing the excesses (when receiving events rapidly enough) for the next iterations
fn spawn_reader<R: Read + Send + 'static>(mut input: R, tx: Sender<io::Result<Event>>) {
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
                            let bytes_read = bytes_read + offset;
                            let (event, read) = parse_input_sequence(&buf[0..bytes_read]);
                            if read < bytes_read {
                                offset = bytes_read - read;
                                buf.copy_within(read..bytes_read, 0);
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
}

impl Events {
    /// Creates a new instance of the receiver of the console events
    /// that might be read from the given input.
    ///
    /// It spins up a thread that will read the input forever, with no
    /// means of stopping it other than closing the input or the receiver.
    /// How the input closes depends on its implementation, but you can
    /// simply drop this object to close the underlying receiver.
    /// Note that the thread would only close after it goes out of being
    /// blocked by receiving some data from the input.
    ///
    /// This receiver is basically a wrapper over the Receiver type,
    /// and the spun up thread uses its connected Sender.
    pub fn new<R: Read + Send + 'static>(input: R) -> Events {
        let (tx, rx) = mpsc::channel();
        spawn_reader(input, tx);
        Events { rx }
    }

    /// Blocks (or not) until a console event occurs and returns it.
    /// The returned type might be an error if some low-level IO error occurs.
    pub fn next(&mut self) -> io::Result<Event> {
        // SendError never occurs unless the stdin is closed, and this should not happen
        return self.rx.recv().unwrap();
    }
}
