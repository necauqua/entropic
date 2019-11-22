use std::io;
use std::io::{Error, Write};
use std::ops::{Deref, DerefMut};
use std::thread;
use std::thread::JoinHandle;

use crossbeam_channel::{Receiver, Sender};
use signal_hook::iterator::Signals;
use termios::*;

macro_rules! terminal_mixin {
    ($name:ident, drop(&mut $self:ident) { $($code:tt)* }) => {
        impl<T: Terminal> Terminal for $name<T> {}

        impl<T: Terminal> Deref for $name<T> {
            type Target = T;

            fn deref(&self) -> &Self::Target {
                &self.peer
            }
        }

        impl<T: Terminal> DerefMut for $name<T> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.peer
            }
        }

        impl<T: Terminal> Drop for $name<T> {
            fn drop(&mut $self) {
                $($code)*
            }
        }
    };
}

pub trait Terminal where Self: Sized {
    fn raw(self) -> io::Result<Raw<Self>> {
        let raw = Raw { prev_ios: Termios::from_fd(libc::STDOUT_FILENO)?, peer: self };
        raw.raw_mode()?;
        Ok(raw)
    }

    fn alt_screen(self) -> io::Result<AltScreen<Self>> {
        let alt_screen = AltScreen { peer: self };
        alt_screen.switch_to_alt()?;
        Ok(alt_screen)
    }

    fn hide_cursor(self) -> io::Result<HideCursor<Self>> {
        let cursor_control = HideCursor { peer: self };
        cursor_control.set_cursor_hidden()?;
        Ok(cursor_control)
    }

    fn mouse_input(self) -> io::Result<MouseInput<Self>> {
        let mouse_input = MouseInput { peer: self };
        mouse_input.listen_to_mouse()?;
        Ok(mouse_input)
    }

    fn terminal_resizes(self) -> io::Result<TerminalResizes<Self>> {
        let (tx, rx) = crossbeam_channel::bounded(0);
        let mut resizes = TerminalResizes {
            resizes_process: None,
            tx,
            rx,
            peer: self
        };
        resizes.listen_to_resizes()?;
        Ok(resizes)
    }

    fn no_wrap(self) -> io::Result<NoWrap<Self>> {
        let no_wrap = NoWrap { peer: self };
        no_wrap.no_wrap_mode()?;
        Ok(no_wrap)
    }
}

#[derive(Clone)]
pub struct TerminalBase;

impl Terminal for TerminalBase {}

impl Write for TerminalBase {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        std::io::stdout().write(buf)
    }

    fn flush(&mut self) -> Result<(), Error> {
        std::io::stdout().flush()
    }
}

#[derive(Clone)]
pub struct Raw<T: Terminal> {
    prev_ios: Termios,
    peer: T,
}

impl<T: Terminal> Raw<T> {
    pub fn raw_mode(&self) -> io::Result<()> {
        let mut ios = Termios::from_fd(libc::STDOUT_FILENO)?;
        cfmakeraw(&mut ios);
        tcsetattr(libc::STDOUT_FILENO, termios::TCSAFLUSH, &ios)?;
        Ok(())
    }

    pub fn normal_mode(&self) -> io::Result<()> {
        tcsetattr(libc::STDOUT_FILENO, termios::TCSAFLUSH, &self.prev_ios)
    }
}

terminal_mixin!(Raw, drop(&mut self) { self.normal_mode().unwrap() });

#[derive(Clone)]
pub struct AltScreen<T: Terminal> {
    peer: T,
}

impl<T: Terminal> AltScreen<T> {
    pub fn switch_to_alt(&self) -> io::Result<()> {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        handle.write_all(b"\x1b[?1049h")?;
        handle.flush()
    }

    pub fn switch_to_normal(&self) -> io::Result<()> {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        handle.write_all(b"\x1b[?1049l")?;
        handle.flush()
    }
}

terminal_mixin!(AltScreen, drop(&mut self) { self.switch_to_normal().unwrap() });

#[derive(Clone)]
pub struct HideCursor<T: Terminal> {
    peer: T,
}

impl<T: Terminal> HideCursor<T> {
    pub fn set_cursor_hidden(&self) -> io::Result<()> {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        handle.write_all(b"\x1b[?25l")?;
        handle.flush()
    }

    pub fn set_cursor_visible(&self) -> io::Result<()> {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        handle.write_all(b"\x1b[?25h")?;
        handle.flush()
    }
}

terminal_mixin!(HideCursor, drop(&mut self) { self.set_cursor_visible().unwrap() });

#[derive(Clone)]
pub struct MouseInput<T: Terminal> {
    peer: T,
}

impl<T: Terminal> MouseInput<T> {
    pub fn listen_to_mouse(&self) -> io::Result<()> {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        handle.write_all(b"\x1b[?1003h\x1b[?1006h")?;
        handle.flush()
    }

    pub fn dont_listen_to_mouse(&self) -> io::Result<()> {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        handle.write_all(b"\x1b[?1003l\x1b[?1006l")?;
        handle.flush()?;
        std::thread::sleep(std::time::Duration::from_millis(30));
        // ↑ this is needed so that the terminal has time to actually disable mouse input
        Ok(())
    }
}

terminal_mixin!(MouseInput, drop(&mut self) { self.dont_listen_to_mouse().unwrap(); });

pub struct TerminalResizes<T: Terminal> {
    resizes_process: Option<(Signals, JoinHandle<()>)>,
    tx: Sender<()>,
    rx: Receiver<()>,
    peer: T,
}

impl<T: Terminal> TerminalResizes<T> {
    pub fn listen_to_resizes(&mut self) -> io::Result<()> {
        self.dont_listen_to_resizes(); // noop if not listening, need to call anyway if listening

        let signals = Signals::new(&[signal_hook::SIGWINCH])?;

        let tx_bg = self.tx.clone();
        let signals_bg = signals.clone();

        let join_handle = thread::spawn(move ||
            while !signals_bg.is_closed() {
                if signals_bg.wait().count() > 0 {
                    match tx_bg.send(()) {
                        Err(_) => break,
                        _ => {}
                    }
                }
            });

        self.resizes_process = Some((signals, join_handle));
        Ok(())
    }

    pub fn get_resize_event_receiver(&self) -> &Receiver<()> {
        &self.rx
    }

    pub fn dont_listen_to_resizes(&mut self) {
        if let Some((signals, join_handle)) = self.resizes_process.take() {
            signals.close();
            join_handle.join().expect("couldn't join on the listening thread");
        }
    }
}

terminal_mixin!(TerminalResizes, drop(&mut self) { self.dont_listen_to_resizes() });

#[derive(Clone)]
pub struct NoWrap<T: Terminal> {
    peer: T,
}

impl<T: Terminal> NoWrap<T> {
    pub fn no_wrap_mode(&self) -> io::Result<()> {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        handle.write_all(b"\x1b[?7l")?;
        handle.flush()
    }

    pub fn wrap_mode(&self) -> io::Result<()> {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        handle.write_all(b"\x1b[?7h")?;
        handle.flush()
    }
}

terminal_mixin!(NoWrap, drop(&mut self) { self.wrap_mode().unwrap() });
