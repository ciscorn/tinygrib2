use std::io::BufRead;

use byteorder::{BigEndian, NativeEndian, ReadBytesExt};

use crate::{Error, Result};

/// Section 0: INDICATOR SECTION (IS)
#[derive(Debug)]
pub struct IndicatorSectionHeader {
    pub identifier: u32,
    pub reserved: u16,
    pub discipline: u8,
    pub edition_number: u8,
    pub total_length: u64,
}

impl IndicatorSectionHeader {
    /// Read Section 0: INDICATOR SECTION (IS)
    pub fn read<R: BufRead>(reader: &mut R) -> Result<Self> {
        Ok(Self {
            identifier: 0x47524942, // "GRIB"
            reserved: reader.read_u16::<NativeEndian>()?,
            discipline: reader.read_u8()?,
            edition_number: {
                let edition_number = reader.read_u8()?;
                if edition_number != 2 {
                    return Err(Error::InvalidData(format!(
                        "edition number must be 2 (grib2), but got {}",
                        edition_number
                    )));
                }
                edition_number
            },
            total_length: reader.read_u64::<BigEndian>()?,
        })
    }
}

/// Common header fields for section 1 to 8
#[derive(Debug)]
pub struct SectionHeader {
    pub section_length: u32,
    pub number_of_section: u8,
}

impl SectionHeader {
    pub fn read<R: BufRead>(reader: &mut R, allow_end: bool) -> Result<Self> {
        let buf = reader.read_u32::<byteorder::BigEndian>()?;
        Ok(if allow_end && buf == 0x37373737 {
            // End Section
            SectionHeader {
                section_length: 4,
                number_of_section: 8,
            }
        } else {
            SectionHeader {
                section_length: buf,
                number_of_section: reader.read_u8()?,
            }
        })
    }

    pub fn ensure_section_number(&self, number: u8) -> Result<()> {
        if self.number_of_section != number {
            return Err(Error::InvalidData(format!(
                "number of section must be {}, but got {}",
                number, self.number_of_section
            )));
        }
        Ok(())
    }
}

/// Section 1: IDENTIFICATION SECTION (IDS)
#[derive(Debug)]
pub struct IdentificationSectionHeader {
    pub section_length: u32,
    pub centre: u16,
    pub sub_centre: u16,
    pub tables_version: u8,
    pub local_tables_version: u8,
    pub significance_of_reference_time: u8,
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub production_status_of_processed_data: u8,
    pub type_of_processed_data: u8,
    pub template_number: Option<u16>,
}

impl IdentificationSectionHeader {
    /// Read Section 1: IDENTIFICATION SECTION (IDS)
    pub fn read<R: BufRead>(header: SectionHeader, reader: &mut R) -> Result<Self> {
        header.ensure_section_number(1)?;
        Ok(Self {
            section_length: header.section_length,
            centre: reader.read_u16::<BigEndian>()?,
            sub_centre: reader.read_u16::<BigEndian>()?,
            tables_version: reader.read_u8()?,
            local_tables_version: reader.read_u8()?,
            significance_of_reference_time: reader.read_u8()?,
            year: reader.read_u16::<BigEndian>()?,
            month: reader.read_u8()?,
            day: reader.read_u8()?,
            hour: reader.read_u8()?,
            minute: reader.read_u8()?,
            second: reader.read_u8()?,
            production_status_of_processed_data: reader.read_u8()?,
            type_of_processed_data: reader.read_u8()?,
            template_number: match header.section_length {
                21 => None,
                _ => Some(reader.read_u16::<BigEndian>()?),
            },
        })
    }

    pub fn body_len(&self) -> u32 {
        match self.section_length {
            21 => 0,
            _ => self.section_length - 23,
        }
    }
}

/// Section 2: LOCAL USE SECTION (LOC)
#[derive(Debug)]
pub struct LocalUseSectionHeader {
    pub section_length: u32,
}

impl LocalUseSectionHeader {
    /// Read Section 2: LOCAL USE SECTION (LOC)
    pub fn read<R: BufRead>(
        header: SectionHeader,
        _reader: &mut R,
    ) -> Result<LocalUseSectionHeader> {
        header.ensure_section_number(2)?;
        Ok(Self {
            section_length: header.section_length,
        })
    }

    pub fn body_len(&self) -> u32 {
        self.section_length - 5
    }
}

/// Section 3: GRID DEFINITION SECTION (GDS)
#[derive(Debug)]
pub struct GridDefinitionSectionHeader {
    pub section_length: u32,
    pub source_of_grid_definition: u8,
    pub number_of_data_points: u32,
    pub number_of_octects_for_number_of_points: u8,
    pub interpretation_of_number_of_points: u8,
    pub template_number: u16,
}

impl GridDefinitionSectionHeader {
    /// Read Section 3: GRID DEFINITION SECTION (GDS)
    pub fn read<R: BufRead>(header: &SectionHeader, reader: &mut R) -> Result<Self> {
        header.ensure_section_number(3)?;
        Ok(Self {
            section_length: header.section_length,
            source_of_grid_definition: reader.read_u8()?,
            number_of_data_points: reader.read_u32::<BigEndian>()?,
            number_of_octects_for_number_of_points: reader.read_u8()?,
            interpretation_of_number_of_points: reader.read_u8()?,
            template_number: reader.read_u16::<BigEndian>()?,
        })
    }

    pub fn body_len(&self) -> u32 {
        self.section_length - 14
    }
}

/// Section 4: PRODUCT DEFINITION SECTION (PDS)
#[derive(Debug)]
pub struct ProductDefinitionSectionHeader {
    pub section_length: u32,
    pub nv: u16,
    pub template_number: u16,
}

impl ProductDefinitionSectionHeader {
    /// Read Section 4: PRODUCT DEFINITION SECTION (PDS)
    pub fn read<R: BufRead>(header: &SectionHeader, reader: &mut R) -> Result<Self> {
        header.ensure_section_number(4)?;
        Ok(ProductDefinitionSectionHeader {
            section_length: header.section_length,
            nv: reader.read_u16::<BigEndian>()?,
            template_number: reader.read_u16::<BigEndian>()?,
        })
    }

    pub fn body_len(&self) -> u32 {
        self.section_length - 9
    }
}

/// Section 5: Data Representation Section (DRS)
#[derive(Debug)]
pub struct DataRepresentationSectionHeader {
    pub section_length: u32,
    pub number_of_values: u32,
    pub template_number: u16,
}

impl DataRepresentationSectionHeader {
    /// Read Section 5: Data Representation Section (DRS)
    pub fn read<R: BufRead>(
        header: &SectionHeader,
        reader: &mut R,
    ) -> Result<DataRepresentationSectionHeader> {
        header.ensure_section_number(5)?;
        Ok(Self {
            section_length: header.section_length,
            number_of_values: reader.read_u32::<BigEndian>()?,
            template_number: reader.read_u16::<BigEndian>()?,
        })
    }

    pub fn body_len(&self) -> u32 {
        self.section_length - 11
    }
}

/// Section 6: BIT-MAP SECTION (BITMAP)
#[derive(Debug)]
pub struct BitmapSectionHeader {
    pub section_length: u32,
    pub bit_map_indicator: u8,
}

impl BitmapSectionHeader {
    /// Read Section 6: BIT-MAP SECTION (BITMAP)
    pub fn read<R: BufRead>(header: &SectionHeader, reader: &mut R) -> Result<Self> {
        header.ensure_section_number(6)?;
        Ok(Self {
            section_length: header.section_length,
            bit_map_indicator: reader.read_u8()?,
        })
    }

    pub fn body_len(&self) -> u32 {
        self.section_length - (5 + 1)
    }
}

/// Section 7: DATA SECTION (DATA)
#[derive(Debug)]
pub struct DataSectionHeader {
    pub section_length: u32,
}

impl DataSectionHeader {
    /// Read Section 7: DATA SECTION (DATA)
    pub fn read(header: &SectionHeader) -> Result<Self> {
        header.ensure_section_number(7)?;
        Ok(Self {
            section_length: header.section_length,
        })
    }

    pub fn body_len(&self) -> u32 {
        self.section_length - 5
    }
}
