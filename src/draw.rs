use std::io::{Write, StdoutLock};
use std::io;
use crate::state::Dimension;

#[derive(Debug, Clone, PartialEq)]
pub struct TerminalRgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl TerminalRgb {
    pub fn new(r: u8, g: u8, b: u8) -> TerminalRgb {
        TerminalRgb { r, g, b }
    }

    pub fn gray(gray: u8) -> TerminalRgb {
        TerminalRgb { r: gray, g: gray, b: gray }
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct CellColor {
    pub bg: Option<TerminalRgb>,
    pub fg: Option<TerminalRgb>,
}

impl CellColor {
    #[inline]
    pub fn none() -> CellColor {
        Default::default()
    }

    pub fn bg(mut self, rgb: TerminalRgb) -> CellColor {
        self.bg = Some(rgb);
        self
    }

    pub fn fg(mut self, rgb: TerminalRgb) -> CellColor {
        self.fg = Some(rgb);
        self
    }

    pub fn draw(&self, handle: &mut StdoutLock) -> io::Result<()> {
        if let Some(TerminalRgb { r, g, b }) = self.bg {
            write!(handle, "\x1b[48;2;{};{};{}m", r, g, b)?;
        }
        if let Some(TerminalRgb { r, g, b }) = self.fg {
            write!(handle, "\x1b[38;2;{};{};{}m", r, g, b)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TerminalCell {
    pub color: CellColor,
    pub char: char,
}

impl Default for TerminalCell {
    fn default() -> Self {
        TerminalCell::new(' ')
    }
}

impl TerminalCell {
    pub fn new(char: char) -> TerminalCell {
        TerminalCell { color: Default::default(), char }
    }

    pub fn color(mut self, color: CellColor) -> TerminalCell {
        self.color = color;
        self
    }

    pub fn bg(mut self, rgb: TerminalRgb) -> TerminalCell {
        self.color = self.color.bg(rgb);
        self
    }

    pub fn fg(mut self, rgb: TerminalRgb) -> TerminalCell {
        self.color = self.color.fg(rgb);
        self
    }

    pub fn draw(&self, handle: &mut StdoutLock) -> io::Result<()> {
        self.color.draw(handle)?;
        write!(handle, "{}\x1b[0m", self.char)
    }
}

#[derive(Clone)]
struct TerminalBuffer {
    max_size: Dimension,
    cells: Box<[TerminalCell]>,
}

impl TerminalBuffer {
    #[inline]
    fn offset(&self, x: u16, y: u16) -> usize {
        y as usize * self.max_size.width as usize + x as usize
    }

    fn fully_draw(&self, part: Dimension) -> io::Result<()> {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();

        let Dimension { width, height } = part.min(self.max_size);

        for y in 0..height {
            write!(handle, "\x1b[{};1H", y + 1)?;
            for x in 0..width {
                self.cells[self.offset(x, y)].draw(&mut handle)?;
            };
        }
        write!(handle, "\x1b[0m")?;
        handle.flush()
    }
}

pub struct TerminalState {
    drawn: Option<TerminalBuffer>,
    buffer: TerminalBuffer,
}

impl TerminalState {
    pub fn new(max_size: Dimension) -> TerminalState {
        let cells = vec![Default::default(); max_size.number() as usize].into_boxed_slice();
        let buffer = TerminalBuffer { max_size, cells };
        TerminalState { drawn: None, buffer }
    }

    #[inline]
    fn offset(&self, x: u16, y: u16) -> usize {
        self.buffer.offset(x, y)
    }

    pub fn put(&mut self, x: u16, y: u16, cell: TerminalCell) {
        self.buffer.cells[self.offset(x, y)] = cell.clone();
    }

    pub fn put_text(&mut self, x: u16, y: u16, color: CellColor, text: impl AsRef<str>) {
        let pos = self.offset(x, y);
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
                let Dimension { width, height } = part.min(self.buffer.max_size);

                for y in 0..height {
                    for x in 0..width {
                        let offset = drawn.offset(x, y);

                        let old = &mut drawn.cells[offset];
                        let new = &self.buffer.cells[offset];

                        if old != new {
                            write!(handle, "\x1b[{};{}H", y + 1, x + 1)?;
                            new.draw(&mut handle)?;
                            *old = new.clone();
                        }
                    };
                }
                handle.flush()
            }
        }
    }
}


