use std::io::Write;
use std::io;

pub struct Ansi<W: Write> {
    output: W
}

pub fn ansi<W: Write>(output: W) -> Ansi<W> {
    Ansi { output }
}

impl<W: Write> Ansi<W> {

    fn xterm_toggle(&mut self, code: u16, enable: bool) -> io::Result<()>  {
        write!(self, "\x1b[?{}{}", code, if enable { 'h' } else { 'l' })
    }

    pub fn alt_screen(&mut self, show: bool) -> io::Result<()>  {
        self.xterm_toggle(1049, show)
    }

    pub fn cursor(&mut self, show: bool) -> io::Result<()>  {
        self.xterm_toggle(25, show)
    }

    pub fn mouse_tracking(&mut self, enable: bool) -> io::Result<()>  {
        self.xterm_toggle(1002, enable)
    }

    pub fn cursor_push(&mut self) -> io::Result<()> {
        write!(self, "\x1b[s")
    }

    pub fn cursor_pop(&mut self) -> io::Result<()> {
        write!(self, "\x1b[u")
    }

    pub fn cursor_move(&mut self, x: u32, y: u32) -> io::Result<()>  {
        write!(self, "\x1b[{1};{0}H", x, y)
    }

    pub fn write_at(&mut self, x: u32, y: u32, s: &str) -> io::Result<()>  {
        self.cursor_push()?;
        self.cursor_move(x, y)?;
        write!(self, "{}", s)?;
        self.cursor_pop()?;
        self.flush()
    }
}

impl<W: Write> Write for Ansi<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.output.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.output.flush()
    }
}

impl<W: Write> Drop for Ansi<W> {
    fn drop(&mut self) {
        self.mouse_tracking(false).unwrap();
        self.alt_screen(false).unwrap();
        self.cursor(true).unwrap();
        self.flush().unwrap();
    }
}

impl<W: Write> std::ops::Deref for Ansi<W> {
    type Target = W;

    fn deref(&self) -> &W {
        &self.output
    }
}

impl<W: Write> std::ops::DerefMut for Ansi<W> {
    fn deref_mut(&mut self) -> &mut W {
        &mut self.output
    }
}
