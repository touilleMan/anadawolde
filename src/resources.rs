use std::fs;
use std::io;
use std::path::Path;

pub struct Resources {
}


pub fn load_resources(resources_path: &Path) -> io::Result<Resources> {
    let memlist_path = resources_path.join("MEMLIST.BIN");
    let _memlist_content = fs::read(&memlist_path)?;
    return Ok(Resources{});
}


// fn load_bank(&path: Path) -> {

// }