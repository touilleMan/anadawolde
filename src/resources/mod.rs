use std::path::Path;
use std::io;

mod memlib;
mod bank;

pub struct Resources {
}

pub fn load_resources(resources_path: &Path) -> io::Result<Resources> {
    let memlist_path = resources_path.join("MEMLIST.BIN");
    let entries = memlib::load_memlist(&memlist_path)?;
    for entry in entries {
    	println!("LOADING BANK{:02X} 0x{:X} -> 0x{:X}", entry.bank_id, entry.bank_offset, entry.bank_offset + entry.packed_size as u32);
        let entry_path = resources_path.join(format!("BANK{:02X}", entry.bank_id));
        let bank_buff = bank::load_bank_entry(&entry_path, &entry)?;
        // println!("~~~~~~~~~{:?}", bank_buff);
    }
    return Ok(Resources{});
}
