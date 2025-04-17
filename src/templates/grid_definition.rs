use std::io::Read;

use super::GribRead;
use crate::Result;

/// Template 3.0 (Latitude/longitude)
#[derive(Debug)]
pub struct GridDefinitionTemplate3_0 {
    pub shape_of_earth: u8,
    pub scale_factor_of_radius: u8,
    pub scale_value_of_radius: u32,
    pub scale_factor_of_major_axis: u8,
    pub scale_value_of_major_axis: u32,
    pub scale_factor_of_minor_axis: u8,
    pub scale_value_of_minor_axis: u32,
    pub n_i: u32,
    pub n_j: u32,
    pub basic_angle: u32,
    pub subdivisions_of_basic_angle: u32,
    pub la1: i32,
    pub lo1: i32,
    pub resolution_and_component_flags: u8,
    pub la2: i32,
    pub lo2: i32,
    pub d_i: u32,
    pub d_j: u32,
    pub scanning_mode: u8,
}

impl GridDefinitionTemplate3_0 {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let tmpl = Self {
            shape_of_earth: reader.read_grib_value()?,
            scale_factor_of_radius: reader.read_grib_value()?,
            scale_value_of_radius: reader.read_grib_value()?,
            scale_factor_of_major_axis: reader.read_grib_value()?,
            scale_value_of_major_axis: reader.read_grib_value()?,
            scale_factor_of_minor_axis: reader.read_grib_value()?,
            scale_value_of_minor_axis: reader.read_grib_value()?,
            n_i: reader.read_grib_value()?,
            n_j: reader.read_grib_value()?,
            basic_angle: reader.read_grib_value()?,
            subdivisions_of_basic_angle: reader.read_grib_value()?,
            la1: reader.read_grib_value()?,
            lo1: reader.read_grib_value()?,
            resolution_and_component_flags: reader.read_grib_value()?,
            la2: reader.read_grib_value()?,
            lo2: reader.read_grib_value()?,
            d_i: reader.read_grib_value()?,
            d_j: reader.read_grib_value()?,
            scanning_mode: reader.read_grib_value()?,
        };
        Ok(tmpl)
    }
}
