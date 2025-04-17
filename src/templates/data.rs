use std::io::Read;

use byteorder::ReadBytesExt;

use crate::templates::data_representation::DataRepresentationTemplate5_200;
use crate::{DataRepresentationSectionHeader, Error, Result};

/// Template 7.200 (Run length packing with level values)
pub fn read_data_7_200<R: Read>(
    reader: &mut R,
    size: usize,
    drs: &DataRepresentationSectionHeader,
    drs_template: &DataRepresentationTemplate5_200,
) -> Result<Vec<Option<u16>>> {
    if drs_template.number_of_bits != 8 {
        return Err(Error::UnsupportedData(format!(
            "Only supports 8 bits in our 7.200 implementation, but got {}",
            drs_template.number_of_bits
        )));
    }

    let mut values: Vec<Option<u16>> = Vec::with_capacity(drs.number_of_values as usize);

    let mut lv = reader.read_u8()?;
    let mut p = 0;
    while p < size {
        p += 1;
        let mut run_length: u32 = 1;
        let mut m: u32 = 1;
        let mut next = 0;
        while p < size {
            next = reader.read_u8()?;
            if next as u16 > drs_template.mv {
                run_length += (next as u16 - drs_template.mv - 1) as u32 * m;
                m *= (255 - drs_template.mv) as u32;
                p += 1;
            } else {
                break;
            }
        }
        let value = match lv {
            0 => None,
            _ => Some(drs_template.mvl_scaled_representative_values[(lv - 1) as usize]),
            // _ => Some((lv - 1) as u16),
        };
        for _ in 0..run_length {
            values.push(value);
        }
        lv = next;
    }

    Ok(values)
}
