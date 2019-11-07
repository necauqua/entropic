use std::{io, ops};
use std::io::Write;
use std::mem::MaybeUninit;

#[inline]
fn check(x: libc::c_int) -> io::Result<libc::c_int> {
    match x {
        -1 => Err(io::Error::last_os_error()),
        _ => Ok(x)
    }
}

pub fn tcgetattr() -> io::Result<libc::termios> {
    unsafe {
        let mut termios = MaybeUninit::uninit();
        check(libc::tcgetattr(1, termios.as_mut_ptr()))?;
        Ok(termios.assume_init())
    }
}

pub fn tcsetattr(termios: &libc::termios) -> io::Result<()> {
    check(unsafe { libc::tcsetattr(1, 0, termios) }).and(Ok(()))
}

pub fn cfmakeraw(termios: &mut libc::termios) {
    extern "C" {
        pub fn cfmakeraw(termios: *mut libc::termios);
    }
    unsafe { cfmakeraw(termios) }
}


pub struct Raw<W: Write> {
    prev_ios: libc::termios,
    output: W,
}

pub fn raw<W: Write>(output: W) -> io::Result<Raw<W>> {
    let mut ios = tcgetattr()?;
    let prev_ios = ios;

    cfmakeraw(&mut ios);
    tcsetattr(&ios)?;

    Ok(Raw { prev_ios, output })
}

impl<W: Write> Raw<W> {

    #[inline]
    pub fn suspend_raw_mode(&self) -> io::Result<()> {
        tcsetattr(&self.prev_ios)?;
        Ok(())
    }

    #[inline]
    pub fn activate_raw_mode(&self) -> io::Result<()> {
        let mut ios = tcgetattr()?;
        cfmakeraw(&mut ios);
        tcsetattr(&ios)?;
        Ok(())
    }
}

impl<W: Write> Write for Raw<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.output.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.output.flush()
    }
}

impl<W: Write> ops::Deref for Raw<W> {
    type Target = W;

    fn deref(&self) -> &W {
        &self.output
    }
}

impl<W: Write> ops::DerefMut for Raw<W> {
    fn deref_mut(&mut self) -> &mut W {
        &mut self.output
    }
}

impl<W: Write> Drop for Raw<W> {
    fn drop(&mut self) {
        self.suspend_raw_mode().unwrap();
    }
}
