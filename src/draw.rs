use std::io::{Write, StdoutLock};
use std::io;
use crate::state::{Dimension, Position, Pixel};

pub trait Drawable {

    fn draw(&self, handle: &mut StdoutLock) -> io::Result<()>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Color {
        Color { r, g, b }
    }

    pub fn gray(gray: u8) -> Color {
        Color { r: gray, g: gray, b: gray }
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct CellColor {
    pub bg: Option<Color>,
    pub fg: Option<Color>,
}

impl CellColor {
    pub fn none() -> CellColor {
        Self::default()
    }

    pub fn bg(mut self, rgb: Color) -> CellColor {
        self.bg = Some(rgb);
        self
    }

    pub fn fg(mut self, rgb: Color) -> CellColor {
        self.fg = Some(rgb);
        self
    }

    pub fn clear(&mut self) {
        self.bg = None;
        self.fg = None;
    }
}

impl Drawable for CellColor {
    fn draw(&self, handle: &mut StdoutLock) -> io::Result<()> {
        if let Some(Color { r, g, b }) = self.bg {
            write!(handle, "\x1b[48;2;{};{};{}m", r, g, b)?;
        }
        if let Some(Color { r, g, b }) = self.fg {
            write!(handle, "\x1b[38;2;{};{};{}m", r, g, b)?;
        }
        Ok(())
    }
}

impl From<Pixel> for Color {
    fn from(pixel: Pixel) -> Self {
        Color { r: pixel.r, g: pixel.g, b: pixel.b }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CharCell {
    pub color: CellColor,
    pub char: char,
}

impl Default for CharCell {
    fn default() -> Self {
        Self::new(' ')
    }
}

impl CharCell {
    pub fn new(char: char) -> CharCell {
        CharCell { color: CellColor::default(), char }
    }

    pub fn color(mut self, color: CellColor) -> CharCell {
        self.color = color;
        self
    }

    pub fn bg(mut self, rgb: Color) -> CharCell {
        self.color = self.color.bg(rgb);
        self
    }

    pub fn fg(mut self, rgb: Color) -> CharCell {
        self.color = self.color.fg(rgb);
        self
    }

    pub fn clear(&mut self) {
        self.char = ' ';
        self.color.clear();
    }
}

impl Drawable for CharCell {
    fn draw(&self, handle: &mut StdoutLock) -> io::Result<()> {
        self.color.draw(handle)?;
        write!(handle, "{}\x1b[0m", self.char)
    }
}

pub struct TerminalState {
    size: Dimension,
    drawn: Option<Box<[CharCell]>>,
    buffer: Box<[CharCell]>,
}

impl TerminalState {
    pub fn new(size: Dimension) -> TerminalState {
        let buffer = vec![CharCell::default(); size.number()].into_boxed_slice();
        TerminalState { size, buffer, drawn: None }
    }

    #[inline]
    fn offset(&self, pos: Position) -> usize {
        self.size.offset(pos)
    }

    pub fn clear(&mut self, rect: Dimension) {
        for pos in rect {
            self.buffer[self.size.offset(pos)].clear();
        }
    }

    pub fn put(&mut self, pos: Position, cell: CharCell) {
        self.buffer[self.offset(pos)] = cell.clone();
    }

    pub fn put_text(&mut self, pos: Position, color: CellColor, text: impl AsRef<str>) {
        let pos = self.offset(pos);
        let mut offset = 0;
        for ch in text.as_ref().chars() {
            let cell = CharCell::new(ch).color(color.clone());
            self.buffer[pos + offset] = cell;
            offset += 1;
        }
    }

    pub fn redraw(&mut self, part: Dimension) -> io::Result<()> {
        let Dimension { width, height } = part.min(self.size);
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        write!(handle, "\x1b[3J")?;
        for y in 0..height {
            write!(handle, "\x1b[{};1H", y + 1)?;
            for x in 0..width {
                self.buffer[self.size.offset(Position { x, y })].draw(&mut handle)?;
            }
        }
        write!(handle, "\x1b[0m\x1b[1;1H")?;
        handle.flush()?;
        self.drawn = Some(self.buffer.clone());
        Ok(())
    }

    pub fn draw(&mut self, part: Dimension) -> io::Result<()> {
        match &mut self.drawn {
            None => self.redraw(part),
            Some(drawn) => {
                let stdout = std::io::stdout();
                let mut handle = stdout.lock();

                for pos in part.min(self.size) {
                    let offset = self.size.offset(pos);

                    let old = &mut drawn[offset];
                    let new = &self.buffer[offset];

                    if old != new {
                        pos.draw(&mut handle)?;
                        new.draw(&mut handle)?;
                        *old = new.clone();
                    }
                }
                handle.flush()
            }
        }
    }
}


