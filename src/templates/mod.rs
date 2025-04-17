pub mod data;
pub mod data_representation;
pub mod grid_definition;
pub mod product_definition;

use byteorder::{BigEndian, ReadBytesExt};
use std::io::Read;

use crate::Result;
pub use data::*;
pub use data_representation::*;
pub use grid_definition::*;
pub use product_definition::*;

pub trait FromGribValue: Sized {
    fn from_grib_reader(reader: impl ReadBytesExt) -> Result<Self>;
}

impl FromGribValue for u8 {
    fn from_grib_reader(mut reader: impl ReadBytesExt) -> Result<Self> {
        Ok(reader.read_u8()?)
    }
}

impl FromGribValue for i8 {
    fn from_grib_reader(mut reader: impl ReadBytesExt) -> Result<Self> {
        Ok(match reader.read_u8()? {
            u if u < 0x80 => u as i8,
            u => -((u & 0x7F) as i8),
        })
    }
}

impl FromGribValue for u16 {
    fn from_grib_reader(mut reader: impl ReadBytesExt) -> Result<Self> {
        Ok(reader.read_u16::<BigEndian>()?)
    }
}

impl FromGribValue for i16 {
    fn from_grib_reader(mut reader: impl ReadBytesExt) -> Result<Self> {
        Ok(match reader.read_u16::<BigEndian>()? {
            u if u < 0x8000 => u as i16,
            u => -((u & 0x7FF) as i16),
        })
    }
}

impl FromGribValue for u32 {
    fn from_grib_reader(mut reader: impl ReadBytesExt) -> Result<Self> {
        Ok(reader.read_u32::<BigEndian>()?)
    }
}

impl FromGribValue for i32 {
    fn from_grib_reader(mut reader: impl ReadBytesExt) -> Result<Self> {
        Ok(match reader.read_u32::<BigEndian>()? {
            u if u < 0x80000000 => u as i32,
            u => -((u & 0x7FFFFFFF) as i32),
        })
    }
}

pub trait GribRead: ReadBytesExt {
    fn read_grib_value<T: FromGribValue>(&mut self) -> Result<T> {
        T::from_grib_reader(self)
    }
}

impl<T: Read> GribRead for T {}
