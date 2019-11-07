use std::io;
use std::io::Write;
use std::os::unix::io::AsRawFd;

use termios::*;

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

    pub fn cursor_move(&mut self, x: u32, y: u32) -> io::Result<()> {
        write!(self, "\x1b[{1};{0}H", x, y)
    }

    pub fn write_at(&mut self, x: u32, y: u32, s: &str) -> io::Result<()> {
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

pub struct MouseInput<W: Write> {
    output: W
}

impl<W: Write> MouseInput<W> {

    pub fn listen_to_mouse(&mut self) -> io::Result<()> {
        write!(self, "\x1b[?1002h")
    }
    pub fn dont_listen_to_mouse(&mut self) -> io::Result<()> {
        write!(self, "\x1b[?1002l")
    }
}

impl_output_mixin! {
    mixin AsMouseInput: MouseInput<W> {
        apply => listen_to_mouse;
        reset => dont_listen_to_mouse;

        fn mouse_input(self) -> io::Result<MouseInput<W>>;
    }
}
