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
pub struct TerminalCell {
    pub color: CellColor,
    pub char: char,
}

impl Default for TerminalCell {
    fn default() -> Self {
        Self::new(' ')
    }
}

impl TerminalCell {
    pub fn new(char: char) -> TerminalCell {
        TerminalCell { color: CellColor::default(), char }
    }

    pub fn color(mut self, color: CellColor) -> TerminalCell {
        self.color = color;
        self
    }

    pub fn bg(mut self, rgb: Color) -> TerminalCell {
        self.color = self.color.bg(rgb);
        self
    }

    pub fn fg(mut self, rgb: Color) -> TerminalCell {
        self.color = self.color.fg(rgb);
        self
    }

    pub fn clear(&mut self) {
        self.char = ' ';
        self.color.clear();
    }
}

impl Drawable for TerminalCell {
    fn draw(&self, handle: &mut StdoutLock) -> io::Result<()> {
        self.color.draw(handle)?;
        write!(handle, "{}\x1b[0m", self.char)
    }
}

#[derive(Clone)]
struct DisplayBuffer {
    size: Dimension,
    cells: Box<[TerminalCell]>,
}

impl DisplayBuffer {
    fn new(size: Dimension) -> DisplayBuffer {
        let cells = vec![Default::default(); size.number()].into_boxed_slice();
        DisplayBuffer { size, cells }
    }

    fn clear(&mut self, rect: Dimension) {
        for pos in rect {
            self.cells[self.size.offset(pos)].clear();
        }
    }

    fn fully_draw(&self, part: Dimension) -> io::Result<()> {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();

        let Dimension { width, height } = part.min(self.size);

        for y in 0..height {
            write!(handle, "\x1b[{};1H", y + 1)?;
            for x in 0..width {
                self.cells[self.size.offset(Position { x, y })].draw(&mut handle)?;
            }
        }
        write!(handle, "\x1b[0m")?;
        handle.flush()
    }
}

pub struct TerminalState {
    drawn: Option<DisplayBuffer>,
    buffer: DisplayBuffer,
}

impl TerminalState {
    pub fn new(max_size: Dimension) -> TerminalState {
        TerminalState { drawn: None, buffer: DisplayBuffer::new(max_size) }
    }

    #[inline]
    fn offset(&self, pos: Position) -> usize {
        self.buffer.size.offset(pos)
    }

    pub fn clear(&mut self, rect: Dimension) {
        self.buffer.clear(rect);
    }

    pub fn put(&mut self, pos: Position, cell: TerminalCell) {
        self.buffer.cells[self.offset(pos)] = cell.clone();
    }

    pub fn put_text(&mut self, pos: Position, color: CellColor, text: impl AsRef<str>) {
        let pos = self.offset(pos);
        let mut offset = 0;
        for ch in text.as_ref().chars() {
            let cell = TerminalCell::new(ch).color(color.clone());
            self.buffer.cells[pos + offset] = cell;
            offset += 1;
        }
    }

    pub fn redraw(&mut self, part: Dimension) -> io::Result<()> {
        self.buffer.fully_draw(part)?;
        self.drawn = Some(self.buffer.clone());
        Ok(())
    }

    pub fn draw(&mut self, part: Dimension) -> io::Result<()> {
        match &mut self.drawn {
            None => self.redraw(part),
            Some(drawn) => {
                let stdout = std::io::stdout();
                let mut handle = stdout.lock();

                for pos in part.min(self.buffer.size) {
                    let offset = drawn.size.offset(pos);

                    let old = &mut drawn.cells[offset];
                    let new = &self.buffer.cells[offset];

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


