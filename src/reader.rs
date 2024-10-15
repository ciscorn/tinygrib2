use std::io::{BufRead, Read};

use byteorder::ReadBytesExt;

use crate::message::*;
use crate::{Error, Result};

pub trait MessageReader<R: BufRead> {
    fn handle_indicator(&mut self, _is: IndicatorSection) -> Result<()> {
        // do nothing
        Ok(())
    }

    fn handle_identification(
        &mut self,
        _ids: IdentificationSection,
        _reader: &mut std::io::Take<&mut R>,
    ) -> Result<()> {
        // do nothing
        Ok(())
    }

    fn handle_local_use(
        &mut self,
        _loc: LocalUseSection,
        _reader: &mut std::io::Take<&mut R>,
    ) -> Result<()> {
        // do nothing
        Ok(())
    }

    fn handle_grid_definition(
        &mut self,
        _gds: GridDefinitionSection,
        _reader: &mut std::io::Take<&mut R>,
    ) -> Result<()> {
        // do nothing
        Ok(())
    }

    fn handle_product_definition(
        &mut self,
        _pds: ProductDefinitionSection,
        _reader: &mut std::io::Take<&mut R>,
    ) -> Result<()> {
        // do nothing
        Ok(())
    }

    fn handle_data_representation(
        &mut self,
        _drs: DataRepresentationSection,
        _reader: &mut std::io::Take<&mut R>,
    ) -> Result<()> {
        // do nothing
        Ok(())
    }

    fn handle_bitmap(
        &mut self,
        _bitmap: BitmapSection,
        _reader: &mut std::io::Take<&mut R>,
    ) -> Result<()> {
        // do nothing
        Ok(())
    }

    fn handle_data(
        &mut self,
        _data: DataSection,
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

        // 0. Indicator Section
        let is: IndicatorSection = IndicatorSection::read(reader)?;
        self.handle_indicator(is)?;

        // 1. Identification Section
        let ids = IdentificationSection::read(SectionHeader::read(reader, false)?, reader)?;
        {
            let mut reader = reader.take(ids.template_size() as u64);
            self.handle_identification(ids, &mut reader)?;
            std::io::copy(&mut reader, &mut std::io::sink())?;
        }

        let mut next_header = SectionHeader::read(reader, false)?;

        'outer: loop {
            // 2. Local Use Section
            if next_header.number_of_section == 2 {
                let loc = LocalUseSection::read(next_header, reader)?;
                {
                    let mut reader = reader.take(loc.body_size() as u64);
                    self.handle_local_use(loc, &mut reader)?;
                    std::io::copy(&mut reader, &mut std::io::sink())?;
                }

                next_header = SectionHeader::read(reader, false)?;
            }

            // 3. Grid Definition Section
            let gds = GridDefinitionSection::read(&next_header, reader)?;
            {
                let mut reader = reader.take(gds.template_size() as u64);
                self.handle_grid_definition(gds, &mut reader)?;
                std::io::copy(&mut reader, &mut std::io::sink())?;
            }

            next_header = SectionHeader::read(reader, false)?;

            loop {
                // 4. Product Definition Section
                let pds = ProductDefinitionSection::read(&next_header, reader)?;
                {
                    let mut reader = reader.take(pds.template_size() as u64);
                    self.handle_product_definition(pds, &mut reader)?;
                    std::io::copy(&mut reader, &mut std::io::sink())?;
                }

                // 5. Data Representation Section
                let drs =
                    DataRepresentationSection::read(&SectionHeader::read(reader, false)?, reader)?;
                {
                    let mut reader = reader.take(drs.template_size() as u64);
                    self.handle_data_representation(drs, &mut reader)?;
                    std::io::copy(&mut reader, &mut std::io::sink())?;
                };

                // 6. Bit-Map Section
                let bitmap = BitmapSection::read(&SectionHeader::read(reader, false)?, reader)?;
                {
                    let mut reader = reader.take(bitmap.body_size() as u64);
                    self.handle_bitmap(bitmap, &mut reader)?;
                    std::io::copy(&mut reader, &mut std::io::sink())?;
                }

                // 7. Data Section
                let data = DataSection::read(&SectionHeader::read(reader, false)?)?;
                {
                    let mut reader = reader.take(data.body_size() as u64);
                    self.handle_data(data, &mut reader)?;
                    std::io::copy(&mut reader, &mut std::io::sink())?;
                };

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
