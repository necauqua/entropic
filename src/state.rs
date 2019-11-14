#![allow(unused)]

use std::fmt::{Debug, Formatter, Error};

#[derive(Copy, Clone)]
pub struct Pixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

pub struct Layer {
    pixels: Box<[Pixel]>,
}

#[derive(Copy, Clone, Debug)]
pub struct Dimension {
    pub width: u16,
    pub height: u16
}

impl Dimension {

    pub fn number(&self) -> u32 {
        self.width as u32 * self.height as u32
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
}

pub struct Picture {
    dimension: Dimension,
    layers: Vec<Layer>,
}

impl Debug for Pixel {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "#{:02x?}{:02x?}{:02x?}{:02x?}", self.r, self.g, self.b, self.a)
    }
}

impl Debug for Picture {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "Picture {{ dimension: {:?}, layers.len(): {} }}", self.dimension, self.layers.len())
    }
}
