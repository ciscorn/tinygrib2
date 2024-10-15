use std::io::BufRead;

use byteorder::{BigEndian, ReadBytesExt};

use crate::Result;

/// Template 5.200 (Run length packing with level values)
///
/// https://github.com/wmo-im/GRIB2/blob/master/GRIB2_Template_5_200_DataRepresentationTemplate_en.csv
#[derive(Debug)]
pub struct DataRepresentationTemplate5_200 {
    pub number_of_bits: u8,
    pub mv: u16,
    pub mvl: u16,
    pub decimal_scale_factor: u8,
    pub mvl_scaled_representative_values: Vec<u16>,
}

impl DataRepresentationTemplate5_200 {
    pub fn read<R: BufRead>(reader: &mut R) -> Result<Self> {
        let mut template = Self {
            number_of_bits: reader.read_u8()?,
            mv: reader.read_u16::<BigEndian>()?,
            mvl: reader.read_u16::<BigEndian>()?,
            decimal_scale_factor: reader.read_u8()?,
            mvl_scaled_representative_values: Vec::new(),
        };
        template
            .mvl_scaled_representative_values
            .reserve(template.mvl.into());
        for _ in 0..template.mvl {
            template
                .mvl_scaled_representative_values
                .push(reader.read_u16::<BigEndian>()?);
        }
        Ok(template)
    }
}
