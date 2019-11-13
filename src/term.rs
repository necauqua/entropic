use std::io;
use std::io::{Write, Error};
use std::thread;

use signal_hook::iterator::Signals;
use termios::*;
use std::thread::JoinHandle;
use std::ops::{Deref, DerefMut};

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

    fn cursor_control(self) -> io::Result<CursorControl<Self>> {
        let cursor_control = CursorControl { peer: self };
        cursor_control.set_cursor_hidden()?;
        Ok(cursor_control)
    }

    fn mouse_input(self) -> io::Result<MouseInput<Self>> {
        let mouse_input = MouseInput { peer: self };
        mouse_input.listen_to_mouse()?;
        Ok(mouse_input)
    }

    fn terminal_resizes(self) -> io::Result<TerminalResizes<Self>> {
        let mut resizes = TerminalResizes { handle: None, peer: self };
        resizes.listen_to_resizes()?;
        Ok(resizes)
    }
}

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

pub struct CursorControl<T: Terminal> {
    peer: T,
}

impl<T: Terminal> CursorControl<T> {
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

    pub fn cursor_push(&self) -> io::Result<()> {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        handle.write_all(b"\x1b[s")?;
        handle.flush()
    }

    pub fn cursor_pop(&self) -> io::Result<()> {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        handle.write_all(b"\x1b[u")?;
        handle.flush()
    }

    pub fn cursor_move(&self, x: u16, y: u16) -> io::Result<()> {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        write!(handle, "\x1b[{1};{0}H", x, y)?;
        handle.flush()
    }

    pub fn write_at(&self, x: u16, y: u16, s: &str) -> io::Result<()> {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        write!(handle, "\x1b[s\x1b[{1};{0}H{2}\x1b[u", x, y, s)?;
        handle.flush()
    }
}

terminal_mixin!(CursorControl, drop(&mut self) { self.set_cursor_visible().unwrap() });

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
        handle.flush()
    }
}

terminal_mixin!(MouseInput, drop(&mut self) { self.dont_listen_to_mouse().unwrap() });

pub struct TerminalResizes<T: Terminal> {
    handle: Option<(Signals, JoinHandle<()>)>,
    peer: T,
}

pub fn send_size() -> io::Result<()> {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    write!(handle, "\x1b[18t")?;
    handle.flush()
}

impl<T: Terminal> TerminalResizes<T> {

    pub fn send_size(&self) -> io::Result<()> {
        send_size()
    }

    pub fn listen_to_resizes(&mut self) -> io::Result<()> {
        self.dont_listen_to_resizes(); // noop if not listening, need to call anyway if listening

        let signals = Signals::new(&[signal_hook::SIGWINCH])?;
        let signals_bg = signals.clone();
        let join_handle = thread::spawn(move || {
            for _ in &signals_bg {
                match send_size() {
                    Err(_) => break,
                    _ => {},
                }
            }
        });
        self.handle = Some((signals, join_handle));
        Ok(())
    }

    pub fn dont_listen_to_resizes(&mut self) {
        if let Some((signals, join_handle)) = self.handle.take() {
            signals.close();
            join_handle.join().expect("couldn't join on the listening thread");
        }
    }
}

terminal_mixin!(TerminalResizes, drop(&mut self) { self.dont_listen_to_resizes() });
