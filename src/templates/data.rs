use std::io::{Error as IoError, ErrorKind, Read};

use itertools::Itertools;

use crate::templates::data_representation::DataRepresentationTemplate5_200;
use crate::templates::read_octets;
use crate::{Error, Result};

use super::{DataRepresentationTemplate5_0, DataRepresentationTemplate5_3};

/// The most a body reserves before any of its bytes have arrived. `size` comes
/// from the file, and a corrupt section header must not become a huge
/// allocation while the data that would justify it is still missing.
const READ_INITIAL_CAPACITY: usize = 64 * 1024;

/// Reads `size` bytes from the reader in one call.
///
/// The decoders used to pull the section through `Read` one byte at a time.
/// When the chain is a `Take` over a buffered decompressor that turns every
/// byte into a handful of calls and bounds checks - more than the decoding
/// itself. The body is taken in one read and unpacked from the slice instead.
fn read_block<R: Read>(reader: &mut R, size: usize) -> Result<Vec<u8>> {
    let mut body = Vec::with_capacity(size.min(READ_INITIAL_CAPACITY));
    reader.take(size as u64).read_to_end(&mut body)?;
    if body.len() < size {
        return Err(Error::IO(IoError::new(
            ErrorKind::UnexpectedEof,
            "data section is shorter than its declared length",
        )));
    }
    Ok(body)
}

/// The error a plane wider than the 32 bits a value can hold is rejected with.
fn too_wide(bits: u32) -> Error {
    Error::IO(IoError::new(
        ErrorKind::InvalidInput,
        format!("{bits} bits do not fit in a value"),
    ))
}

/// Splits values out of an in-memory byte plane, most significant bit first.
struct BitPlane<'a> {
    bytes: &'a [u8],
    bit: usize,
}

impl<'a> BitPlane<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, bit: 0 }
    }

    /// Reads up to 32 bits, the width a value can have here.
    ///
    /// The check is not a debug assertion: a widened plane used to be decoded
    /// happily in release builds, with the top bits silently dropped.
    #[inline]
    fn read(&mut self, bits: u32) -> Result<u32> {
        if bits > 32 {
            return Err(too_wide(bits));
        }
        let mut value = 0u32;
        let mut remaining = bits;
        while remaining > 0 {
            let byte = *self.bytes.get(self.bit >> 3).ok_or_else(|| {
                Error::IO(IoError::new(
                    ErrorKind::UnexpectedEof,
                    "data section ends before its values do",
                ))
            })?;
            let available = 8 - (self.bit & 7) as u32;
            let take = remaining.min(available);
            let shift = available - take;
            value = (value << take) | ((u32::from(byte) >> shift) & ((1u32 << take) - 1));
            self.bit += take as usize;
            remaining -= take;
        }
        Ok(value)
    }
}

/// The bytes a plane of `count` values of `bits` bits occupies.
fn plane_bytes(count: usize, bits: u32) -> usize {
    count.saturating_mul(bits as usize).div_ceil(8)
}

/// Reads one packed plane and returns its values.
fn read_plane_values<R: Read>(reader: &mut R, count: usize, bits: u32) -> Result<Vec<u32>> {
    if bits > 32 {
        return Err(too_wide(bits));
    }
    let plane = read_block(reader, plane_bytes(count, bits))?;
    let mut plane = BitPlane::new(&plane);
    (0..count).map(|_| plane.read(bits)).collect()
}

/// Template 7.0: Grid point data - simple packing
///
/// NAN is represented as i32::MIN
pub fn read_data_7_0<R: Read>(
    reader: &mut R,
    number_of_values: u32,
    tmpl: &DataRepresentationTemplate5_0,
) -> Result<Vec<i32>> {
    let bits = u32::from(tmpl.bits_per_value);
    if bits > 32 {
        return Err(Error::IO(IoError::new(
            ErrorKind::InvalidInput,
            "bits per value must be at most 32",
        )));
    }
    let count = number_of_values as usize;
    let plane = read_block(reader, plane_bytes(count, bits))?;
    let mut plane = BitPlane::new(&plane);
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        // TODO: handle NA value?
        values.push(plane.read(bits)? as i32);
    }
    Ok(values)
}

/// Template 7.3: Grid point data - complex packing and spatial differencing
///
/// NAN is represented as i32::MIN
pub fn read_data_7_3<R: Read>(
    mut reader: &mut R,
    tmpl: &DataRepresentationTemplate5_3,
) -> Result<Vec<i32>> {
    let tmpl2 = &tmpl.template_2;
    let tmpl0 = &tmpl2.template_0;
    assert_eq!(
        tmpl.order_of_spatial_differencing, 2,
        "Only 2nd order is supported"
    );
    assert_eq!(tmpl.number_of_octets_extra_descriptors, 2);
    let z1: i32 = read_octets(&mut reader, tmpl.number_of_octets_extra_descriptors)?;
    let z2: i32 = read_octets(&mut reader, tmpl.number_of_octets_extra_descriptors)?;
    let z_min: i32 = read_octets(&mut reader, tmpl.number_of_octets_extra_descriptors)?;
    let ng = tmpl2.number_of_groups_of_data_values;
    // Each plane starts on a byte boundary, so it is one contiguous run of
    // bytes: taking them in one read is what the bit reader used to do a byte
    // at a time.
    let group_refs = read_plane_values(&mut reader, ng as usize, u32::from(tmpl0.bits_per_value))?;
    let group_widths = read_plane_values(
        &mut reader,
        ng as usize,
        u32::from(tmpl2.number_of_bits_used_for_the_group_widths),
    )?;
    let group_lengths = read_plane_values(
        &mut reader,
        ng as usize,
        u32::from(tmpl2.number_of_bits_for_scaled_group_lengths),
    )?;

    let group_width = |gw: u32| tmpl2.reference_for_group_widths as u32 + gw;
    if group_widths.iter().any(|&gw| group_width(gw) > 32) {
        return Err(Error::IO(IoError::new(
            ErrorKind::InvalidInput,
            "group width must be at most 32",
        )));
    }
    let group_length = |gi: u32, gl: u32| {
        if gi < ng - 1 {
            tmpl2.reference_for_group_lengths
                + (tmpl2.length_increment_for_the_group_lengths as u32 * gl)
        } else {
            tmpl2.true_length_of_last_group
        }
    };
    // The plane's length and the value count both come out of the header, so
    // they are computed with checked arithmetic: an overflowing or absurd
    // group must be an error, not a panic or a wild reservation.
    let too_long = || {
        Error::IO(IoError::new(
            ErrorKind::InvalidData,
            "data section declares more values than it can hold",
        ))
    };
    let total_bits = group_widths
        .iter()
        .zip_eq(&group_lengths)
        .enumerate()
        .try_fold(0u64, |sum, (gi, (&gw, &gl))| {
            sum.checked_add(
                u64::from(group_width(gw)).checked_mul(u64::from(group_length(gi as u32, gl)))?,
            )
        })
        .ok_or_else(too_long)?;
    let values_bytes = usize::try_from(total_bits.div_ceil(8)).map_err(|_| too_long())?;
    let values_data = read_block(&mut reader, values_bytes)?;
    // Only as many values as the bytes that actually arrived can stand for;
    // groups of zero width emit values without consuming any, and the vector
    // grows into those as the old decoder's did.
    let total_values = group_lengths
        .iter()
        .enumerate()
        .try_fold(0usize, |sum, (gi, &gl)| {
            sum.checked_add(group_length(gi as u32, gl) as usize)
        })
        .ok_or_else(too_long)?;
    let capacity = total_values.min(values_data.len().saturating_mul(8));
    let mut values_plane = BitPlane::new(&values_data);

    let mut values: Vec<i32> = Vec::with_capacity(capacity);
    for (gi, ((gref, gw), gl)) in group_refs
        .into_iter()
        .zip_eq(group_widths)
        .zip_eq(group_lengths)
        .enumerate()
    {
        let width = group_width(gw);
        for _ in 0..group_length(gi as u32, gl) {
            let v = values_plane.read(width)?;
            values.push(z_min + gref as i32 + v as i32);
        }
    }
    values[0] = z1;
    values[1] = z2;
    for i in 2..values.len() {
        values[i] = values[i] + (2 * values[i - 1]) - values[i - 2];
    }
    Ok(values)
}

/// Template 7.200 (Run length packing with level values)
///
/// NAN is represented as i32::MIN
pub fn read_data_7_200<R: Read>(
    reader: &mut R,
    size: usize,
    number_of_values: u32,
    drs_template: &DataRepresentationTemplate5_200,
) -> Result<Vec<i32>> {
    if drs_template.number_of_bits != 8 {
        return Err(Error::UnsupportedData(format!(
            "Only supports 8 bits in our 7.200 implementation, but got {}",
            drs_template.number_of_bits
        )));
    }
    // A leading level byte is read even for an empty body.
    let body = read_block(reader, size.max(1))?;
    if size == 0 {
        return Ok(Vec::new());
    }

    let mut values: Vec<i32> = Vec::with_capacity(number_of_values as usize);
    let mut lv = body[0];
    let mut p = 0usize;
    while p < size {
        p += 1;
        let mut run_length: u32 = 1;
        let mut m: u32 = 1;
        let mut next = 0;
        while p < size {
            next = body[p];
            if next as u16 > drs_template.mv {
                p += 1;
                run_length += (next as u16 - drs_template.mv - 1) as u32 * m;
                m *= (255 - drs_template.mv) as u32;
            } else {
                break;
            }
        }
        let value = match lv {
            0 => i32::MIN,
            _ => drs_template.mvl_scaled_representative_values[(lv - 1) as usize] as i32,
        };
        values.resize(values.len() + run_length as usize, value);
        lv = next;
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use bitstream_io::{BigEndian, BitRead, BitReader};
    use byteorder::ReadBytesExt;

    use super::*;
    use crate::templates::data_representation::{
        DataRepresentationTemplate5_0, DataRepresentationTemplate5_2, DataRepresentationTemplate5_3,
    };

    /// A tiny deterministic generator, so a failure is reproducible.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn below(&mut self, bound: u32) -> u32 {
            (self.next() % u64::from(bound)) as u32
        }
    }

    fn random_bytes(rng: &mut Rng, len: usize) -> Vec<u8> {
        (0..len).map(|_| rng.next() as u8).collect()
    }

    /// The decoder this module replaced, kept so both can be run on the same
    /// bytes and compared.
    fn old_read_data_7_0(
        reader: &mut impl Read,
        number_of_values: u32,
        tmpl: &DataRepresentationTemplate5_0,
    ) -> Result<Vec<i32>> {
        let mut reader = BitReader::<_, BigEndian>::new(reader);
        let mut values = Vec::with_capacity(number_of_values as usize);
        for _ in 0..number_of_values as usize {
            let v: u32 = reader.read_var(tmpl.bits_per_value as u32)?;
            values.push(v as i32);
        }
        Ok(values)
    }

    fn old_read_data_7_3(
        reader: &mut impl Read,
        tmpl: &DataRepresentationTemplate5_3,
    ) -> Result<Vec<i32>> {
        let tmpl2 = &tmpl.template_2;
        let tmpl0 = &tmpl2.template_0;
        let z1: i32 = read_octets(&mut *reader, tmpl.number_of_octets_extra_descriptors)?;
        let z2: i32 = read_octets(&mut *reader, tmpl.number_of_octets_extra_descriptors)?;
        let z_min: i32 = read_octets(&mut *reader, tmpl.number_of_octets_extra_descriptors)?;
        let ng = tmpl2.number_of_groups_of_data_values;
        let mut reader = BitReader::<_, BigEndian>::new(reader);
        let group_refs = (0..ng)
            .map(|_| reader.read_var::<u32>(tmpl0.bits_per_value as u32))
            .collect::<std::io::Result<Vec<u32>>>()?;
        reader.byte_align();
        let group_widths = (0..ng)
            .map(|_| reader.read_var::<u32>(tmpl2.number_of_bits_used_for_the_group_widths as u32))
            .collect::<std::io::Result<Vec<u32>>>()?;
        reader.byte_align();
        let group_lengths = (0..ng)
            .map(|_| reader.read_var::<u32>(tmpl2.number_of_bits_for_scaled_group_lengths as u32))
            .collect::<std::io::Result<Vec<u32>>>()?;
        reader.byte_align();
        let mut values: Vec<i32> = vec![];
        for (gi, ((gref, gw), gl)) in group_refs
            .into_iter()
            .zip_eq(group_widths)
            .zip_eq(group_lengths)
            .enumerate()
        {
            let group_width = tmpl2.reference_for_group_widths as u32 + gw;
            let group_length = if (gi as u32) < ng - 1 {
                tmpl2.reference_for_group_lengths
                    + (tmpl2.length_increment_for_the_group_lengths as u32 * gl)
            } else {
                tmpl2.true_length_of_last_group
            };
            for _ in 0..group_length {
                let v = reader.read_var::<u32>(group_width)?;
                let value = z_min + gref as i32 + v as i32;
                values.push(value);
            }
        }
        values[0] = z1;
        values[1] = z2;
        for i in 2..values.len() {
            values[i] = values[i] + (2 * values[i - 1]) - values[i - 2];
        }
        Ok(values)
    }

    fn old_read_data_7_200(
        reader: &mut impl Read,
        size: usize,
        number_of_values: u32,
        drs_template: &DataRepresentationTemplate5_200,
    ) -> Result<Vec<i32>> {
        let mut values: Vec<i32> = Vec::with_capacity(number_of_values as usize);
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
                0 => i32::MIN,
                _ => drs_template.mvl_scaled_representative_values[(lv - 1) as usize] as i32,
            };
            for _ in 0..run_length {
                values.push(value);
            }
            lv = next;
        }
        Ok(values)
    }

    #[test]
    fn simple_packing_reads_the_same_values_and_bytes() {
        let mut rng = Rng(0x5eed);
        for bits in 0..=32u8 {
            for count in [0u32, 1, 2, 3, 7, 8, 9, 31, 32, 33, 100, 500] {
                let needed = (count as usize * bits as usize).div_ceil(8);
                let body = random_bytes(&mut rng, needed + 8);
                let tmpl = DataRepresentationTemplate5_0 {
                    reference_value: 0.0,
                    binary_scale_factor: 0,
                    decimal_scale_factor: 0,
                    bits_per_value: bits,
                    type_of_original_field_values: 0,
                };
                let mut old_reader = Cursor::new(&body);
                let old = old_read_data_7_0(&mut old_reader, count, &tmpl);
                let mut new_reader = Cursor::new(&body);
                let new = read_data_7_0(&mut new_reader, count, &tmpl);
                match (old, new) {
                    (Ok(old), Ok(new)) => {
                        assert_eq!(old, new, "bits={bits} count={count}");
                        assert_eq!(
                            old_reader.position(),
                            new_reader.position(),
                            "consumed bytes differ: bits={bits} count={count}"
                        );
                    }
                    (Err(_), Err(_)) => {}
                    (old, new) => panic!("bits={bits} count={count}: {old:?} vs {new:?}"),
                }
            }
        }
    }

    fn random_5_3(rng: &mut Rng) -> DataRepresentationTemplate5_3 {
        let ng = 1 + rng.below(8);
        DataRepresentationTemplate5_3 {
            order_of_spatial_differencing: 2,
            number_of_octets_extra_descriptors: 2,
            template_2: DataRepresentationTemplate5_2 {
                template_0: DataRepresentationTemplate5_0 {
                    reference_value: 0.0,
                    binary_scale_factor: 0,
                    decimal_scale_factor: 0,
                    bits_per_value: rng.below(11) as u8,
                    type_of_original_field_values: 0,
                },
                group_splitting_method_used: 0,
                missing_value_management_used: 0,
                primary_missing_value_substitute: 0,
                secondary_missing_value_substitute: 0,
                number_of_groups_of_data_values: ng,
                reference_for_group_widths: rng.below(3) as u8,
                number_of_bits_used_for_the_group_widths: 1 + rng.below(4) as u8,
                reference_for_group_lengths: rng.below(3),
                length_increment_for_the_group_lengths: 1 + rng.below(2) as u8,
                true_length_of_last_group: 2 + rng.below(4),
                number_of_bits_for_scaled_group_lengths: 1 + rng.below(4) as u8,
            },
        }
    }

    #[test]
    fn complex_packing_reads_the_same_values_and_bytes() {
        let mut rng = Rng(0xc0ffee);
        for _ in 0..400 {
            let tmpl = random_5_3(&mut rng);
            // Enough bytes for the three planes and a generous values plane.
            let body = random_bytes(&mut rng, 512);
            let mut old_reader = Cursor::new(&body);
            let old = old_read_data_7_3(&mut old_reader, &tmpl);
            let mut new_reader = Cursor::new(&body);
            let new = read_data_7_3(&mut new_reader, &tmpl);
            match (old, new) {
                (Ok(old), Ok(new)) => {
                    assert_eq!(old, new);
                    assert_eq!(
                        old_reader.position(),
                        new_reader.position(),
                        "consumed bytes differ"
                    );
                }
                (Err(_), Err(_)) => {}
                (old, new) => panic!("{old:?} vs {new:?}"),
            }
        }
    }

    fn template_5_3_with_planes(
        ref_bits: u8,
        width_bits: u8,
        length_bits: u8,
    ) -> DataRepresentationTemplate5_3 {
        DataRepresentationTemplate5_3 {
            order_of_spatial_differencing: 2,
            number_of_octets_extra_descriptors: 2,
            template_2: DataRepresentationTemplate5_2 {
                template_0: DataRepresentationTemplate5_0 {
                    reference_value: 0.0,
                    binary_scale_factor: 0,
                    decimal_scale_factor: 0,
                    bits_per_value: ref_bits,
                    type_of_original_field_values: 0,
                },
                group_splitting_method_used: 0,
                missing_value_management_used: 0,
                primary_missing_value_substitute: 0,
                secondary_missing_value_substitute: 0,
                number_of_groups_of_data_values: 1,
                reference_for_group_widths: 0,
                number_of_bits_used_for_the_group_widths: width_bits,
                reference_for_group_lengths: 0,
                length_increment_for_the_group_lengths: 1,
                true_length_of_last_group: 2,
                number_of_bits_for_scaled_group_lengths: length_bits,
            },
        }
    }

    /// Widened planes used to be decoded with the top bits dropped once the
    /// debug assertions were compiled out.
    #[test]
    fn a_plane_wider_than_a_value_is_rejected_in_release_too() {
        let body = [0u8; 64];
        let simple = DataRepresentationTemplate5_0 {
            reference_value: 0.0,
            binary_scale_factor: 0,
            decimal_scale_factor: 0,
            bits_per_value: 33,
            type_of_original_field_values: 0,
        };
        assert!(read_data_7_0(&mut Cursor::new(&body), 2, &simple).is_err());

        for (ref_bits, width_bits, length_bits) in [(33, 4, 4), (8, 33, 4), (8, 4, 33)] {
            let tmpl = template_5_3_with_planes(ref_bits, width_bits, length_bits);
            assert!(
                read_data_7_3(&mut Cursor::new(&body), &tmpl).is_err(),
                "ref={ref_bits} width={width_bits} length={length_bits}"
            );
        }
    }

    /// A short body declaring a huge values plane has to fail without trying
    /// to reserve the plane: the length is read before a byte of it arrives.
    #[test]
    fn a_declared_values_plane_larger_than_the_body_does_not_reserve_it() {
        let mut tmpl = template_5_3_with_planes(0, 4, 4);
        tmpl.template_2.reference_for_group_widths = 29; // width 29 + 3 = 32
        tmpl.template_2.true_length_of_last_group = u32::MAX;
        let mut body = vec![0u8; 6 + 1 + 1 + 16];
        body[6] = 0b0011_0000; // the width plane: one group of width 32
        assert!(read_data_7_3(&mut Cursor::new(&body), &tmpl).is_err());
    }

    #[test]
    fn run_length_packing_reads_the_same_values() {
        let mut rng = Rng(0x1234_5678);
        for _ in 0..300 {
            let mvl = 1 + rng.below(20) as u16;
            let tmpl = DataRepresentationTemplate5_200 {
                number_of_bits: 8,
                // Kept near 255 so a run does not expand into millions of
                // values: each continuation multiplies by 255 - mv.
                mv: 200 + rng.below(55) as u16,
                mvl,
                decimal_scale_factor: 0,
                mvl_scaled_representative_values: (0..mvl).map(|_| rng.next() as i16).collect(),
            };
            // A valid run-length body: a level byte at most `mvl`, runs
            // extended by bytes above `mv`, and a terminator that is itself
            // the next level byte.
            let body_len = 1 + rng.below(120) as usize;
            let mv = tmpl.mv;
            let mut body = Vec::with_capacity(body_len);
            body.push(rng.below(u32::from(mvl) + 1) as u8);
            while body.len() < body_len {
                for _ in 0..rng.below(2) {
                    if body.len() >= body_len {
                        break;
                    }
                    body.push(mv as u8 + 1 + rng.below(u32::from(255 - mv)) as u8);
                }
                if body.len() < body_len {
                    body.push(rng.below(u32::from(mvl) + 1) as u8);
                }
            }
            let mut old_reader = Cursor::new(&body);
            let old = old_read_data_7_200(&mut old_reader, body.len(), 1024, &tmpl);
            let mut new_reader = Cursor::new(&body);
            let new = read_data_7_200(&mut new_reader, body.len(), 1024, &tmpl);
            match (old, new) {
                (Ok(old), Ok(new)) => {
                    assert_eq!(old, new);
                    assert_eq!(old_reader.position(), new_reader.position());
                }
                (Err(_), Err(_)) => {}
                (old, new) => panic!("{old:?} vs {new:?}"),
            }
        }
    }
}
