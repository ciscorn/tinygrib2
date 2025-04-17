use std::io::Read;

use byteorder::{BigEndian, ReadBytesExt};

use super::GribRead;
use crate::Result;

/// Template 4.0 (analysis or forecast at a horizontal level or in a horizontal layer at a point in time)
#[derive(Debug)]
pub struct ProductDefinitionTemplate4_0 {
    pub parameter_category: u8,
    pub parameter_number: u8,
    pub type_of_generating_process: u8,
    pub background_process: u8,
    pub generating_process_identifier: u8,
    pub hours_after_data_cutoff: u16,
    pub minutes_after_data_cutoff: u8,
    pub indicator_of_unit_of_time_range: u8,
    pub forecast_time: i32,
    pub type_of_first_fixed_surface: u8,
    pub scale_factor_of_first_fixed_surface: i8,
    pub scaled_value_of_first_fixed_surface: u32,
    pub type_of_second_fixed_surface: u8,
    pub scale_factor_of_second_fixed_surface: i8,
    pub scaled_value_of_second_fixed_surface: u32,
}

impl ProductDefinitionTemplate4_0 {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(Self {
            parameter_category: reader.read_grib_value()?,
            parameter_number: reader.read_grib_value()?,
            type_of_generating_process: reader.read_grib_value()?,
            background_process: reader.read_grib_value()?,
            generating_process_identifier: reader.read_grib_value()?,
            hours_after_data_cutoff: reader.read_grib_value()?,
            minutes_after_data_cutoff: reader.read_grib_value()?,
            indicator_of_unit_of_time_range: reader.read_grib_value()?,
            forecast_time: reader.read_grib_value()?,
            type_of_first_fixed_surface: reader.read_grib_value()?,
            scale_factor_of_first_fixed_surface: reader.read_grib_value()?,
            scaled_value_of_first_fixed_surface: reader.read_grib_value()?,
            type_of_second_fixed_surface: reader.read_grib_value()?,
            scale_factor_of_second_fixed_surface: reader.read_grib_value()?,
            scaled_value_of_second_fixed_surface: reader.read_grib_value()?,
        })
    }
}

/// Template 4.8 (average, accumulation and/or extreme values or other statistically processed values at a horizontal level or in a horizontal layer in a continuous or non-continuous time interval)
#[derive(Debug)]
pub struct ProductDefinitionTemplate4_8 {
    pub parameter_category: u8,
    pub parameter_number: u8,
    pub type_of_generating_process: u8,
    pub background_generating_process_identifier: u8,
    pub analysis_or_forecast_generating_process_identifier: u8,
    pub hours_of_observational_data_cutoff: u16,
    pub minutes_of_observational_data_cutoff: u8,
    pub indicator_of_unit_of_time_range: u8,
    pub forecast_time: i32,
    pub type_of_first_fixed_surface: u8,
    pub scale_factor_of_first_fixed_surface: i8,
    pub scaled_value_of_first_fixed_surface: u32,
    pub type_of_second_fixed_surface: u8,
    pub scale_factor_of_second_fixed_surface: i8,
    pub scaled_value_of_second_fixed_surface: u32,
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub time_ranges: Vec<ProductDefinitionTemplate4_8TimeRange>,
}

impl ProductDefinitionTemplate4_8 {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(Self {
            parameter_category: reader.read_grib_value()?,
            parameter_number: reader.read_grib_value()?,
            type_of_generating_process: reader.read_grib_value()?,
            background_generating_process_identifier: reader.read_grib_value()?,
            analysis_or_forecast_generating_process_identifier: reader.read_grib_value()?,
            hours_of_observational_data_cutoff: reader.read_grib_value()?,
            minutes_of_observational_data_cutoff: reader.read_grib_value()?,
            indicator_of_unit_of_time_range: reader.read_grib_value()?,
            forecast_time: reader.read_grib_value()?,
            type_of_first_fixed_surface: reader.read_grib_value()?,
            scale_factor_of_first_fixed_surface: reader.read_grib_value()?,
            scaled_value_of_first_fixed_surface: reader.read_grib_value()?,
            type_of_second_fixed_surface: reader.read_grib_value()?,
            scale_factor_of_second_fixed_surface: reader.read_grib_value()?,
            scaled_value_of_second_fixed_surface: reader.read_grib_value()?,
            year: reader.read_grib_value()?,
            month: reader.read_grib_value()?,
            day: reader.read_grib_value()?,
            hour: reader.read_grib_value()?,
            minute: reader.read_grib_value()?,
            second: reader.read_grib_value()?,
            time_ranges: (0..reader.read_grib_value::<u8>()?)
                .map(|_| ProductDefinitionTemplate4_8TimeRange::read(reader))
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

#[derive(Debug)]
pub struct ProductDefinitionTemplate4_8TimeRange {
    pub total_number_of_data_values_missing: u32,
    pub statistical_process: u8,
    pub type_of_time_increment: u8,
    pub indicator_of_unit_of_time: u8,
    pub length_of_the_time_range: u32,
    pub indicator_of_unit_of_length_of_time_range: u8,
    pub time_increment: u32,
}

impl ProductDefinitionTemplate4_8TimeRange {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(Self {
            total_number_of_data_values_missing: reader.read_grib_value()?,
            statistical_process: reader.read_grib_value()?,
            type_of_time_increment: reader.read_grib_value()?,
            indicator_of_unit_of_time: reader.read_grib_value()?,
            length_of_the_time_range: reader.read_grib_value()?,
            indicator_of_unit_of_length_of_time_range: reader.read_grib_value()?,
            time_increment: reader.read_grib_value()?,
        })
    }
}

#[derive(Debug)]
pub struct ProductDefinitionTemplate4_50011 {
    pub template_8: ProductDefinitionTemplate4_8,
    pub rader_operating_info1: u64,
    pub rader_operating_info2: u64,
    pub rader_operating_info3: u64,
}

impl ProductDefinitionTemplate4_50011 {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(Self {
            template_8: ProductDefinitionTemplate4_8::read(reader)?,
            rader_operating_info1: reader.read_u64::<BigEndian>()?,
            rader_operating_info2: reader.read_u64::<BigEndian>()?,
            rader_operating_info3: reader.read_u64::<BigEndian>()?,
        })
    }
}
