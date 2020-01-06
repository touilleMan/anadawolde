use std::io;
use std::io::Seek;
use std::io::Read;
use std::fs;
use std::path::Path;
use std::convert::TryInto;

use super::memlib::MemEntry;


pub fn load_bank_entry(bank_path: &Path, entry: &MemEntry) -> io::Result<Vec<u8>> {
    let mut fd = fs::File::open(&bank_path)?;
    fd.seek(io::SeekFrom::Start(entry.bank_offset.into()))?;
    let mut ret = vec![0;entry.size.into()];
    if entry.packed_size != entry.size {
        let mut bank_entry_buff = vec![0;entry.packed_size.into()];
        fd.read(&mut bank_entry_buff)?;
        unpack(&bank_entry_buff, &mut ret)?;
    } else {
        fd.read(&mut ret)?;
    }
    Ok(ret)
}

fn to_u32(buff: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(
        buff[offset..offset + 4].try_into().unwrap()
    )
}

struct BitStream<'a> {
    data: &'a[u8],
    current_word_pos: usize,
    current_word: u32,
    current_bit_shift: usize,
    current_word_is_first: bool,
}

impl<'a> BitStream<'a> {
    fn new(data: &'a[u8]) -> Self {
        let current_word_pos = data.len() - 4;
        let current_word = to_u32(data, current_word_pos);
        BitStream{
            data,
            current_word_pos,
            current_word,
            current_bit_shift: 0,
            current_word_is_first: true,
        }
    }

    fn get_bit(&mut self) -> io::Result<u8> {
        Ok(self.next().unwrap())
    }

    fn get_bits(&mut self, count: usize) -> io::Result<u32> {
        let mut bits: u32 = 0;
        for _ in 0..count {
            bits <<= 1;
            bits |= self.get_bit()? as u32;
        }
        Ok(bits)
    }

}

impl<'a> Iterator for BitStream<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        // First byte decoding is broken: switch to next bytes as soon as
        // the value is 1 instead of reading each of the 32 bits.
        // This is most likely a flaw in the original implementation: each word
        // is shifted to the rigt and bitmaked with 1 << 31 in order to know
        // when it is entirely consumed. However the first word is loaded
        // is different place where the bitmask is not applied.
        if self.current_word_is_first && (self.current_word >> self.current_bit_shift) <= 1 {
            self.current_word_is_first = false;
            self.current_bit_shift = 32;
        }

        if self.current_bit_shift == 32 {
            self.current_bit_shift = 0;
            self.current_word_pos -= 4;
            self.current_word = to_u32(self.data, self.current_word_pos);
        }

        let bit = ((self.current_word >> self.current_bit_shift) & 1) as u8;
        self.current_bit_shift += 1;

        Some(bit)
    }
}

struct BuffWriter<'a> {
    buff: &'a mut [u8],
    pos: usize,
}

impl<'a> BuffWriter<'a> {
    fn new(buff: &'a mut [u8]) -> Self {
        Self {
            pos: buff.len(),
            buff: buff,
        }
    }

    fn copy_bytes(&mut self, size: u32, bs: &mut BitStream) -> io::Result<()>{
        for _ in 0..size {
            self.pos -= 1;
            self.buff[self.pos] = bs.get_bits(8)? as u8;
        }
        Ok(())
    }

    fn duplicate_bytes(&mut self, size: u32, offset: u32) {
        for _ in 0..size {
            self.pos -= 1;
            self.buff[self.pos] = self.buff[self.pos + offset as usize];
        }
    }
}

fn unpack(src: &[u8], dst: & mut [u8]) -> io::Result<()> {
    // Sanity checks
    if src.len() <= 8 {
        return Err(io::Error::new(io::ErrorKind::Other, "Packed size must be > 8"));
    }
    if src.len() % 4 != 0 {
        return Err(io::Error::new(io::ErrorKind::Other, "Packed size must be a multiple of 4"));
    }
    let data_size_pos = src.len() - 4;
    let crc_pos = data_size_pos - 4;
    let data_end = crc_pos;
    let data_size = to_u32(src, data_size_pos);
    let expected_crc = to_u32(src, crc_pos);
    let mut bs = BitStream::new(&src[..data_end]);

    // Check unpacked size
    if data_size > dst.len().try_into().unwrap() {
        return Err(io::Error::new(io::ErrorKind::Other, "Unpacked size too big for output buffer"));
    }

    // Check CRC
    let mut crc: u32 = expected_crc;
    for i in (0..data_end).step_by(4) {
        crc ^= to_u32(src, i);
    }
    if crc != 0 {
        return Err(io::Error::new(io::ErrorKind::Other, "Bad CRC check"))
    }

    // All good, now let's do the unpacking !
    let mut dst_cursor = BuffWriter::new(dst);
    let mut unpacked_size: u32 = 0;
    let mut size: u32;
    while unpacked_size < data_size {
        if bs.get_bit()? == 0 {
            if bs.get_bit()? == 0 {
                // copy between 1 and 8 bytes
                size = bs.get_bits(3)? + 1;
                dst_cursor.copy_bytes(size, &mut bs)?;
            } else {
                // duplicate 2 bytes
                size = 2;
                let offset = bs.get_bits(8)?;
                dst_cursor.duplicate_bytes(size, offset);
            }
        } else {
            let c = bs.get_bits(2)?;
            match c {
                0 => {
                    // duplicate 3 bytes
                    size = 3;
                    let offset = bs.get_bits(9)?;
                    dst_cursor.duplicate_bytes(size, offset);
                }
                1 => {
                    // duplicate 4 bytes
                    size = 4;
                    let offset = bs.get_bits(10)?;
                    dst_cursor.duplicate_bytes(size, offset);
                }
                2 => {
                    // duplicate between 1 and 256 bytes
                    size = bs.get_bits(8)? + 1;
                    let offset = bs.get_bits(12)?;
                    dst_cursor.duplicate_bytes(size, offset);
                }
                _ => { // c == 3
                    // copy between 9 and 264 bytes
                    size = bs.get_bits(8)? + 1 + 8;
                    dst_cursor.copy_bytes(size, &mut bs)?;
                }
            }
        }
        unpacked_size += size;
    }

    if unpacked_size != data_size {
        return Err(io::Error::new(io::ErrorKind::Other, "Final unpacked size doesn't mach with expected unpacked size"))
    }

    Ok(())
}
