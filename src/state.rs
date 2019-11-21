#![allow(unused)]

use std::fmt::{Debug, Formatter, Error};
use crate::draw::Drawable;
use std::io::{Write, StdoutLock};
use std::io;
use std::ops::{Add, Sub};

#[derive(Copy, Clone, Default)]
pub struct Pixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Pixel {
    fn from_f32(r: f32, g: f32, b: f32, a: f32) -> Pixel {
        Pixel { r: (r * 255f32) as u8, g: (g * 255f32) as u8, b: (b * 255f32) as u8, a: (a * 255f32) as u8 }
    }

    fn to_f32(self) -> (f32, f32, f32, f32) {
        (self.r as f32 / 255f32, self.g as f32 / 255f32, self.b as f32 / 255f32, self.a as f32 / 255f32)
    }

    pub fn blend(bg: Pixel, fg: Pixel) -> Pixel {
        let (br, bg, bb, ba) = bg.to_f32();
        let (fr, fg, fb, fa) = fg.to_f32();
        let a = 1f32 - (1f32 - fa) * (1f32 - ba);
        let r = fr * fa / a + br * ba * (1f32 - fa) / a;
        let g = fg * fa / a + bg * ba * (1f32 - fa) / a;
        let b = fb * fa / a + bb * ba * (1f32 - fa) / a;
        Self::from_f32(r, g, b, a)
    }
}

pub struct Layer {
    pub pixels: Box<[Pixel]>,
}

#[derive(Copy, Clone, Debug)]
pub struct Dimension {
    pub width: u16,
    pub height: u16,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}

impl Add for Position {
    type Output = Position;

    fn add(self, rhs: Self) -> Self::Output {
        Position { x: self.x + rhs.x, y: self.x + rhs.y }
    }
}

impl Sub for Position {
    type Output = Position;

    fn sub(self, rhs: Self) -> Self::Output {
        Position { x: self.x - rhs.x, y: self.x - rhs.y }
    }
}

impl Drawable for Position {
    fn draw(&self, handle: &mut StdoutLock) -> io::Result<()> {
        write!(handle, "\x1b[{};{}H", self.y + 1, self.x + 1)
    }
}

impl Dimension {
    pub fn number(&self) -> usize {
        self.width as usize * self.height as usize
    }

    pub fn min(self, other: Dimension) -> Dimension {
        Dimension {
            width: self.width.min(other.width),
            height: self.height.min(other.height),
        }
    }

    pub fn max(self, other: Dimension) -> Dimension {
        Dimension {
            width: self.width.max(other.width),
            height: self.height.max(other.height),
        }
    }

    #[inline]
    pub fn offset(self, pos: Position) -> usize {
        pos.y as usize * self.width as usize + pos.x as usize
    }
}

pub struct DimensionIter {
    width: u16,
    height: u16,
    x: u16,
    y: u16,
}

impl Iterator for DimensionIter {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        if self.x == self.width {
            self.y += 1;
            self.x = 0;
        }
        if self.y == self.height {
            return None;
        }
        let res = Some(Position { x: self.x, y: self.y });
        self.x += 1;
        res
    }
}

impl IntoIterator for Dimension {
    type Item = Position;
    type IntoIter = DimensionIter;

    fn into_iter(self) -> Self::IntoIter {
        DimensionIter { width: self.width, height: self.height, x: 0, y: 0 }
    }
}

pub struct Picture {
    pub size: Dimension,
    pub layers: Vec<Layer>,
}

impl Debug for Pixel {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "#{:02x?}{:02x?}{:02x?}{:02x?}", self.r, self.g, self.b, self.a)
    }
}

impl Debug for Picture {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "Picture {{ dimension: {:?}, layers.len(): {} }}", self.size, self.layers.len())
    }
}
