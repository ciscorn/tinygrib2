use std::io::BufRead;

use byteorder::{BigEndian, ReadBytesExt};

use crate::Result;

/// Template 4.0 (analysis or forecast at a horizontal level or in a horizontal layer at a point in time)
///
/// https://github.com/wmo-im/GRIB2/blob/master/GRIB2_Template_4_0_ProductDefinitionTemplate_en.csv
#[derive(Debug)]
pub struct ProductDefinitionTemplate4_0 {
    pub parameter_category: u8,
    pub parameter_number: u8,
    pub type_of_generating_process: u8,
    pub background_generating_process_identifier: u8,
    pub analysis_or_forecast_generating_process_identifier: u8,
    pub hours_of_observational_data_cutoff: u16,
    pub minutes_of_observational_data_cutoff: u8,
    pub indicator_of_unit_of_time_range: u8,
    pub forecast_time: u32,
    pub type_of_first_fixed_surface: u8,
    pub scale_factor_of_first_fixed_surface: u8,
    pub scaled_value_of_first_fixed_surface: u32,
    pub type_of_second_fixed_surface: u8,
    pub scale_factor_of_second_fixed_surface: u8,
    pub scaled_value_of_second_fixed_surface: u32,
}

impl ProductDefinitionTemplate4_0 {
    pub fn read<R: BufRead>(reader: &mut R) -> Result<Self> {
        Ok(Self {
            parameter_category: reader.read_u8()?,
            parameter_number: reader.read_u8()?,
            type_of_generating_process: reader.read_u8()?,
            background_generating_process_identifier: reader.read_u8()?,
            analysis_or_forecast_generating_process_identifier: reader.read_u8()?,
            hours_of_observational_data_cutoff: reader.read_u16::<BigEndian>()?,
            minutes_of_observational_data_cutoff: reader.read_u8()?,
            indicator_of_unit_of_time_range: reader.read_u8()?,
            forecast_time: reader.read_u32::<BigEndian>()?,
            type_of_first_fixed_surface: reader.read_u8()?,
            scale_factor_of_first_fixed_surface: reader.read_u8()?,
            scaled_value_of_first_fixed_surface: reader.read_u32::<BigEndian>()?,
            type_of_second_fixed_surface: reader.read_u8()?,
            scale_factor_of_second_fixed_surface: reader.read_u8()?,
            scaled_value_of_second_fixed_surface: reader.read_u32::<BigEndian>()?,
        })
    }
}

/// Template 4.8 (average, accumulation and/or extreme values or other statistically processed values at a horizontal level or in a horizontal layer in a continuous or non-continuous time interval)
///
/// https://github.com/wmo-im/GRIB2/blob/master/GRIB2_Template_4_8_ProductDefinitionTemplate_en.csv
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
    pub forecast_time: u32,
    pub type_of_first_fixed_surface: u8,
    pub scale_factor_of_first_fixed_surface: u8,
    pub scaled_value_of_first_fixed_surface: u32,
    pub type_of_second_fixed_surface: u8,
    pub scale_factor_of_second_fixed_surface: u8,
    pub scaled_value_of_second_fixed_surface: u32,
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub time_ranges: Vec<ProductDefinitionTemplate8TimeRange>,
}

impl ProductDefinitionTemplate4_8 {
    pub fn read<R: BufRead>(reader: &mut R) -> Result<Self> {
        Ok(Self {
            parameter_category: reader.read_u8()?,
            parameter_number: reader.read_u8()?,
            type_of_generating_process: reader.read_u8()?,
            background_generating_process_identifier: reader.read_u8()?,
            analysis_or_forecast_generating_process_identifier: reader.read_u8()?,
            hours_of_observational_data_cutoff: reader.read_u16::<BigEndian>()?,
            minutes_of_observational_data_cutoff: reader.read_u8()?,
            indicator_of_unit_of_time_range: reader.read_u8()?,
            forecast_time: reader.read_u32::<BigEndian>()?,
            type_of_first_fixed_surface: reader.read_u8()?,
            scale_factor_of_first_fixed_surface: reader.read_u8()?,
            scaled_value_of_first_fixed_surface: reader.read_u32::<BigEndian>()?,
            type_of_second_fixed_surface: reader.read_u8()?,
            scale_factor_of_second_fixed_surface: reader.read_u8()?,
            scaled_value_of_second_fixed_surface: reader.read_u32::<BigEndian>()?,
            year: reader.read_u16::<BigEndian>()?,
            month: reader.read_u8()?,
            day: reader.read_u8()?,
            hour: reader.read_u8()?,
            minute: reader.read_u8()?,
            second: reader.read_u8()?,
            time_ranges: (0..reader.read_u8()?)
                .map(|_| ProductDefinitionTemplate8TimeRange::read(reader))
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

#[derive(Debug)]
pub struct ProductDefinitionTemplate8TimeRange {
    pub total_number_of_data_values_missing: u32,
    pub statistical_process: u8,
    pub type_of_time_increment: u8,
    pub indicator_of_unit_of_time: u8,
    pub length_of_the_time_range: u32,
    pub indicator_of_unit_of_length_of_time_range: u8,
    pub time_increment: u32,
}

impl ProductDefinitionTemplate8TimeRange {
    pub fn read<R: BufRead>(reader: &mut R) -> Result<Self> {
        Ok(Self {
            total_number_of_data_values_missing: reader.read_u32::<BigEndian>()?,
            statistical_process: reader.read_u8()?,
            type_of_time_increment: reader.read_u8()?,
            indicator_of_unit_of_time: reader.read_u8()?,
            length_of_the_time_range: reader.read_u32::<BigEndian>()?,
            indicator_of_unit_of_length_of_time_range: reader.read_u8()?,
            time_increment: reader.read_u32::<BigEndian>()?,
        })
    }
}
