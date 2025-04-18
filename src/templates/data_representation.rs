use std::io::Read;

use byteorder::{BigEndian, ReadBytesExt};

use super::GribRead;
use crate::Result;

/// Template 5.200 (Run length packing with level values)
#[derive(Debug)]
pub struct DataRepresentationTemplate5_200 {
    pub number_of_bits: u8,
    pub mv: u16,
    pub mvl: u16,
    pub decimal_scale_factor: i8,
    pub mvl_scaled_representative_values: Vec<u16>,
}

impl DataRepresentationTemplate5_200 {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let mut tmpl = Self {
            number_of_bits: reader.read_grib_value()?,
            mv: reader.read_grib_value()?,
            mvl: reader.read_grib_value()?,
            decimal_scale_factor: reader.read_grib_value()?,
            mvl_scaled_representative_values: Vec::new(),
        };
        tmpl.mvl_scaled_representative_values
            .reserve(tmpl.mvl.into());
        for _ in 0..tmpl.mvl {
            tmpl.mvl_scaled_representative_values
                .push(reader.read_u16::<BigEndian>()?);
        }
        Ok(tmpl)
    }
}
