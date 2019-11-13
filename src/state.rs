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
