use std::process;
use std::env;
use std::path::Path;

mod resources;


fn main() {
	let args: Vec<String> = env::args().collect();
	if args.len() != 2 {
		println!("usage: {} <RESOURCES_PATH>", args[0]);
		process::exit(1);
	}
	let resources_path = Path::new(&args[1]);
	match resources::load_resources(&resources_path) {
		Ok(_resources) => {
			println!("Ready to run game !");
		},
		Err(err) => {
			println!("Cannot load resources from `{}`: {:?}", resources_path.display(), err);
			process::exit(1);
		}
	}
}
