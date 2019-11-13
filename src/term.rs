use std::io;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use signal_hook::iterator::Signals;
use termios::*;

#[derive(Clone)]
pub struct Raw<W: Write + AsRawFd> {
    prev_ios: Termios,
    output: W,
}

impl_output_mixin! {
    mixin AsRaw: Raw<W + AsRawFd> {
        reset => normal_mode;

        fn raw(self) -> io::Result<Raw<W>> {
            let mut ios = Termios::from_fd(self.as_raw_fd())?;
            let prev_ios = ios.clone();

            cfmakeraw(&mut ios);
            tcsetattr(self.as_raw_fd(), termios::TCSAFLUSH, &ios)?;

            Ok(Raw { prev_ios, output: self })
        }
    }
}

impl<W: Write + AsRawFd> Raw<W> {
    pub fn raw_mode(&self) -> io::Result<()> {
        let mut ios = Termios::from_fd(self.output.as_raw_fd())?;
        cfmakeraw(&mut ios);
        tcsetattr(self.output.as_raw_fd(), termios::TCSAFLUSH, &ios)?;
        Ok(())
    }

    pub fn normal_mode(&self) -> io::Result<()> {
        tcsetattr(self.output.as_raw_fd(), termios::TCSAFLUSH, &self.prev_ios)
    }
}

#[derive(Clone)]
pub struct AltScreen<W: Write> {
    output: W
}

impl<W: Write> AltScreen<W> {
    pub fn switch_to_alt(&mut self) -> io::Result<()> {
        write!(self, "\x1b[?1049h")
    }

    pub fn switch_to_normal(&mut self) -> io::Result<()> {
        write!(self, "\x1b[?1049l")
    }
}

impl_output_mixin! {
    mixin AsAltScreen: AltScreen<W> {
        apply => switch_to_alt;
        reset => switch_to_normal;
        fn alt_screen(self) -> io::Result<AltScreen<W>>;
    }
}

#[derive(Clone)]
pub struct CursorControl<W: Write> {
    output: W
}

impl<W: Write> CursorControl<W> {
    pub fn set_cursor_hidden(&mut self) -> io::Result<()> {
        write!(self, "\x1b[?25l")
    }
    pub fn set_cursor_visible(&mut self) -> io::Result<()> {
        write!(self, "\x1b[?25h")
    }

    pub fn cursor_push(&mut self) -> io::Result<()> {
        write!(self, "\x1b[s")
    }

    pub fn cursor_pop(&mut self) -> io::Result<()> {
        write!(self, "\x1b[u")
    }

    pub fn cursor_move(&mut self, x: u16, y: u16) -> io::Result<()> {
        write!(self, "\x1b[{1};{0}H", x, y)
    }

    pub fn write_at(&mut self, x: u16, y: u16, s: &str) -> io::Result<()> {
        self.cursor_push()?;
        self.cursor_move(x, y)?;
        write!(self, "{}", s)?;
        self.cursor_pop()?;
        self.flush()
    }
}

impl_output_mixin! {
    mixin AsCursorControl: CursorControl<W> {
        reset => set_cursor_visible;

        fn cursor_control(self) -> CursorControl<W>;
    }
}

#[derive(Clone)]
pub struct MouseInput<W: Write> {
    output: W
}

impl<W: Write> MouseInput<W> {

    pub fn listen_to_mouse(&mut self) -> io::Result<()> {
        write!(self, "\x1b[?1002h\x1b[?1006h")
    }

    pub fn dont_listen_to_mouse(&mut self) -> io::Result<()> {
        write!(self, "\x1b[?1002l\x1b[?1006l")
    }
}

impl_output_mixin! {
    mixin AsMouseInput: MouseInput<W> {
        apply => listen_to_mouse;
        reset => dont_listen_to_mouse;

        fn mouse_input(self) -> io::Result<MouseInput<W>>;
    }
}

pub struct TerminalResizes<W: Write> {
    output: W,
    enabled: AtomicBool
}

impl<W: Write> TerminalResizes<W> {

    pub fn listen_to_resizes(&mut self) -> io::Result<()> {
        self.enabled.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub fn dont_listen_to_resizes(&mut self) -> io::Result<()> {
        self.enabled.store(false, Ordering::SeqCst);
        Ok(())
    }
}

impl_output_mixin! {
    mixin AsTerminalResizes: TerminalResizes<W> {
        reset => dont_listen_to_resizes;

        fn terminal_resizes(self) -> io::Result<TerminalResizes<W>> {
            let signals = Signals::new(&[signal_hook::SIGWINCH])?;
            thread::spawn(move || {
                let mut out = std::io::stdout();
                for _ in &signals {
                    match write!(out, "\x1b[18t").and_then(|()| out.flush()) {
                        Err(_) => break,
                        _ => {},
                    }
                }
            });
            Ok(TerminalResizes { output: self, enabled: AtomicBool::new(true) })
        }
    }
}
