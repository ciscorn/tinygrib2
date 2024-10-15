use std::io::{BufRead, Read};

use byteorder::ReadBytesExt;

use crate::message::*;
use crate::{Error, Result};

pub trait MessageReader<R: BufRead> {
    fn handle_indicator(&mut self, _is: IndicatorSectionHeader) -> Result<()> {
        // do nothing
        Ok(())
    }

    fn handle_identification(
        &mut self,
        _ids: IdentificationSectionHeader,
        _reader: &mut std::io::Take<&mut R>,
    ) -> Result<()> {
        // do nothing
        Ok(())
    }

    fn handle_local_use(
        &mut self,
        _loc: LocalUseSectionHeader,
        _reader: &mut std::io::Take<&mut R>,
    ) -> Result<()> {
        // do nothing
        Ok(())
    }

    fn handle_grid_definition(
        &mut self,
        _gds: GridDefinitionSectionHeader,
        _reader: &mut std::io::Take<&mut R>,
    ) -> Result<()> {
        // do nothing
        Ok(())
    }

    fn handle_product_definition(
        &mut self,
        _pds: ProductDefinitionSectionHeader,
        _reader: &mut std::io::Take<&mut R>,
    ) -> Result<()> {
        // do nothing
        Ok(())
    }

    fn handle_data_representation(
        &mut self,
        _drs: DataRepresentationSectionHeader,
        _reader: &mut std::io::Take<&mut R>,
    ) -> Result<()> {
        // do nothing
        Ok(())
    }

    fn handle_bitmap(
        &mut self,
        _bitmap: BitmapSectionHeader,
        _reader: &mut std::io::Take<&mut R>,
    ) -> Result<()> {
        // do nothing
        Ok(())
    }

    fn handle_data(
        &mut self,
        _data: DataSectionHeader,
        _reader: &mut std::io::Take<&mut R>,
    ) -> Result<()> {
        // do nothing
        Ok(())
    }

    fn read_next_message(&mut self, reader: &mut R) -> Result<Option<()>> {
        match reader.read_u32::<byteorder::LittleEndian>() {
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e.into()),
            Ok(0x42495247) => {} // b"GRIB"
            Ok(_) => {
                return Err(Error::InvalidData(
                    "message identifier must be 'GRIB'".to_string(),
                ));
            }
        };

        // Indicator Section (0)
        let is: IndicatorSectionHeader = IndicatorSectionHeader::read(reader)?;
        self.handle_indicator(is)?;

        // Identification Section (1)
        let ids = IdentificationSectionHeader::read(SectionHeader::read(reader, false)?, reader)?;
        {
            let mut reader = reader.take(ids.body_len() as u64);
            self.handle_identification(ids, &mut reader)?;
            std::io::copy(&mut reader, &mut std::io::sink())?;
        }

        let mut next_header = SectionHeader::read(reader, false)?;

        'outer: loop {
            // Local Use Section (2)
            if next_header.number_of_section == 2 {
                let loc = LocalUseSectionHeader::read(next_header, reader)?;
                {
                    let mut reader = reader.take(loc.body_len() as u64);
                    self.handle_local_use(loc, &mut reader)?;
                    std::io::copy(&mut reader, &mut std::io::sink())?;
                }

                next_header = SectionHeader::read(reader, false)?;
            }

            // Grid Definition Section (3)
            {
                let gds = GridDefinitionSectionHeader::read(&next_header, reader)?;
                let mut reader = reader.take(gds.body_len() as u64);
                self.handle_grid_definition(gds, &mut reader)?;
                std::io::copy(&mut reader, &mut std::io::sink())?;
            }

            next_header = SectionHeader::read(reader, false)?;

            loop {
                // Product Definition Section (4)
                {
                    let pds = ProductDefinitionSectionHeader::read(&next_header, reader)?;
                    let mut reader = reader.take(pds.body_len() as u64);
                    self.handle_product_definition(pds, &mut reader)?;
                    std::io::copy(&mut reader, &mut std::io::sink())?;
                }

                // Data Representation Section (5)
                {
                    let drs = DataRepresentationSectionHeader::read(
                        &SectionHeader::read(reader, false)?,
                        reader,
                    )?;
                    let mut reader = reader.take(drs.body_len() as u64);
                    self.handle_data_representation(drs, &mut reader)?;
                    std::io::copy(&mut reader, &mut std::io::sink())?;
                }

                // Bit-Map Section (6)
                {
                    let bitmap =
                        BitmapSectionHeader::read(&SectionHeader::read(reader, false)?, reader)?;
                    let mut reader = reader.take(bitmap.body_len() as u64);
                    self.handle_bitmap(bitmap, &mut reader)?;
                    std::io::copy(&mut reader, &mut std::io::sink())?;
                }

                // Data Section (7)
                {
                    let data = DataSectionHeader::read(&SectionHeader::read(reader, false)?)?;
                    let mut reader = reader.take(data.body_len() as u64);
                    self.handle_data(data, &mut reader)?;
                    std::io::copy(&mut reader, &mut std::io::sink())?;
                }

                // Next Section
                next_header = SectionHeader::read(reader, true)?;
                match next_header.number_of_section {
                    2 | 3 => break,
                    4 => {}
                    8 => break 'outer,
                    _ => return Err(Error::InvalidData("invalid section number".to_string())),
                }
            }
        }

        Ok(Some(()))
    }
}
