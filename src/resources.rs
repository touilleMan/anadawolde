use std::fs;
use std::io;
use std::io::Read;
use std::path::Path;
use std::mem::size_of;
use std::convert::TryInto;

pub struct Resources {
}

#[allow(dead_code)]
#[derive(Debug)]
enum MemEntryState {
    NotNeeded=0,
    Loaded=1,
    LoadMe=2,
    EndOfMemlist=255,
}

#[allow(dead_code)]
#[derive(Debug)]
struct MemEntry {
    state: u8,         // 0x00,
    type_: u8,         // 0x01
    buf_ptr: u16,      // 0x02
    unused_4: u16,     // 0x04
    rank_num: u8,      // 0x06
    bank_id: u8,       // 0x07
    bank_offset: u32,  // 0x08
    unused_c: u16,     // 0x0C
    packed_size: u16,  // 0x0E
    unused_10: u16,    // 0x10
    size: u16,         // 0x12
}


pub fn load_resources(resources_path: &Path) -> io::Result<Resources> {
    let memlist_path = resources_path.join("MEMLIST.BIN");
    let _entries = load_memlist(&memlist_path)?;
    return Ok(Resources{});
}


fn load_memlist(memlist_path: &Path) -> io::Result<Vec<MemEntry>> {
    let mut memlist_content = fs::File::open(&memlist_path)?;

    let mut entries = Vec::new();

    let mut entry_buff = [0; size_of::<MemEntry>()];
    loop {
        let size = memlist_content.read(&mut entry_buff)?;
        if size != size_of::<MemEntry>() || (entry_buff[0] == MemEntryState::EndOfMemlist as u8) {
            break;
        }

        let entry = MemEntry{
            state: entry_buff[0],
            type_: entry_buff[1],
            buf_ptr: u16::from_be_bytes(entry_buff[2..4].try_into().unwrap()),
            unused_4: u16::from_be_bytes(entry_buff[4..6].try_into().unwrap()),
            rank_num: entry_buff[6],
            bank_id: entry_buff[7],
            bank_offset: u32::from_be_bytes(entry_buff[8..12].try_into().unwrap()),
            unused_c: u16::from_be_bytes(entry_buff[12..14].try_into().unwrap()),
            packed_size: u16::from_be_bytes(entry_buff[14..16].try_into().unwrap()),
            unused_10: u16::from_be_bytes(entry_buff[16..18].try_into().unwrap()),
            size: u16::from_be_bytes(entry_buff[18..20].try_into().unwrap()),
        };
        println!("Loaded {:?}", entry);
        entries.push(entry);
    }

    Ok(entries)
}


fn load_bank_entry(entry: &MemEntry, buff: &mut [u8]) -> io::Result<()> {
    Ok(())
}
