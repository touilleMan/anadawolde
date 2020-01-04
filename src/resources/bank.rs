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
        let mut packed_buff = vec![0;entry.packed_size.into()];
        fd.read(&mut packed_buff)?;
        UnpackContext::unpack(&packed_buff, &mut ret)?;
    } else {
        fd.read(&mut ret)?;
    }
    Ok(ret)
}

struct UnpackContext<'a, 'b> {
    size: u16,
    datasize: u32,
    crc: u32,
    chk: u32,

    src: &'a[u8],
    src_pos: usize,
    dst: &'b mut [u8],
    dst_pos: usize,
}

impl<'a, 'b> UnpackContext<'a, 'b> {

    fn get_u32(&mut self) -> u32 {
        self.src_pos -= 4;
        u32::from_be_bytes(self.src[self.src_pos..self.src_pos+4].try_into().unwrap())
    }

    fn unpack(src: &'a[u8], dst: &'b mut [u8]) -> io::Result<()>{
        let mut ctx = Self{
            size: 0,
            datasize: 0,
            crc: 0,
            chk: 0,
            src: src,
            src_pos: src.len(),
            dst: dst,
            dst_pos: 0,
        };

        ctx.datasize = ctx.get_u32();
        ctx.crc = ctx.get_u32();
        ctx.chk = ctx.get_u32();
        ctx.dst_pos = (ctx.datasize - 1) as usize;

        ctx.crc ^= ctx.chk;
        loop {
            if !ctx.next_chunk() {
                ctx.size = 1;
                if !ctx.next_chunk() {
                    ctx.dec_unk1(3, 0);
                } else {
                    ctx.dec_unk2(8);
                }
            } else {
                let c: u16 = ctx.get_code(2);
                if c == 3 {
                    ctx.dec_unk1(8, 8);
                } else {
                    if c < 2 {
                        ctx.size = c + 2;
                        ctx.dec_unk2((c + 9).try_into().unwrap());  // TODO: investigate this...
                    } else {
                        ctx.size = ctx.get_code(8);
                        ctx.dec_unk2(12);
                    }
                }
            }
            if ctx.datasize <= 0 {
                break;
            }
        }

        if ctx.crc == 0 {
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "Bad CRC check"))
        }
    }

    fn dec_unk1(&mut self, num_chunks: u8, add_count: u8) {
        let mut count: u16 = self.get_code(num_chunks) + add_count as u16 + 1;
        self.datasize -= count as u32;
        while count > 0 {
            count -= 1;
            assert!(self.dst_pos >= self.src_pos);
            self.dst[self.dst_pos] = self.get_code(8) as u8;
            self.dst_pos -= 1;
        }
    }

    fn dec_unk2(&mut self, num_chunks: u8) {
        let i: u16 = self.get_code(num_chunks);
        let mut count: u16 = self.size + 1;
        // debug(DBG_BANK, "Bank::decUnk2(%d) i=%d count=%d", num_chunks, i, count);
        self.datasize -= count as u32;
        while count > 0 {
            count -= 1;
            // assert(_oBuf >= _iBuf && _oBuf >= _startBuf);
            self.dst[self.dst_pos] = self.dst[self.dst_pos + i as usize];
            self.dst_pos -= 1;
        }
    }

    fn get_code(&mut self, mut num_chunks: u8) -> u16 {
        let mut c: u16 = 0;
        while num_chunks > 0 {
            num_chunks -= 1;
            c <<= 1;
            if self.next_chunk() {
                c |= 1;
            }           
        }
        c
    }

    fn next_chunk(&mut self) -> bool {
        let mut cf = self.rcr(false);
        if self.chk == 0 {
            // assert(_iBuf >= _startBuf);
            self.chk = self.get_u32();
            self.crc ^= self.chk;
            cf = self.rcr(true);
        }
        cf
    }

    fn rcr(&mut self, cf: bool) -> bool {
        let rcf: bool = (self.chk & 1) == 1;
        self.chk >>= 1;
        if cf {
            self.chk |= 0x80000000;
        }
        rcf
    }

}