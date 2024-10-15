use std::io::BufRead;

use byteorder::{BigEndian, ReadBytesExt};

use crate::Result;

/// Template 3.0 (Latitude/longitude)
///
/// https://github.com/wmo-im/GRIB2/blob/master/GRIB2_Template_3_0_GridDefinitionTemplate_en.csv
#[derive(Debug)]
pub struct GridDefinitionTemplate3_0 {
    pub shape_of_earth: u8,
    pub scale_factor_of_radius: u8,
    pub scale_value_of_radius: u32,
    pub scale_factor_of_major_axis: u8,
    pub scale_value_of_major_axis: u32,
    pub scale_factor_of_minor_axis: u8,
    pub scale_value_of_minor_axis: u32,
    pub ni: u32,
    pub nj: u32,
    pub basic_angle: u32,
    pub subdivisions_of_basic_angle: u32,
    pub la1: u32,
    pub lo1: u32,
    pub resolution_and_component_flags: u8,
    pub la2: u32,
    pub lo2: u32,
    pub di: u32,
    pub dj: u32,
    pub scanning_mode: u8,
}

impl GridDefinitionTemplate3_0 {
    pub fn read<R: BufRead>(reader: &mut R) -> Result<Self> {
        let temp = Self {
            shape_of_earth: reader.read_u8()?,
            scale_factor_of_radius: reader.read_u8()?,
            scale_value_of_radius: reader.read_u32::<BigEndian>()?,
            scale_factor_of_major_axis: reader.read_u8()?,
            scale_value_of_major_axis: reader.read_u32::<BigEndian>()?,
            scale_factor_of_minor_axis: reader.read_u8()?,
            scale_value_of_minor_axis: reader.read_u32::<BigEndian>()?,
            ni: reader.read_u32::<BigEndian>()?,
            nj: reader.read_u32::<BigEndian>()?,
            basic_angle: reader.read_u32::<BigEndian>()?,
            subdivisions_of_basic_angle: reader.read_u32::<BigEndian>()?,
            la1: reader.read_u32::<BigEndian>()?,
            lo1: reader.read_u32::<BigEndian>()?,
            resolution_and_component_flags: reader.read_u8()?,
            la2: reader.read_u32::<BigEndian>()?,
            lo2: reader.read_u32::<BigEndian>()?,
            di: reader.read_u32::<BigEndian>()?,
            dj: reader.read_u32::<BigEndian>()?,
            scanning_mode: reader.read_u8()?,
        };
        Ok(temp)
    }
}
