extern crate clang;
extern crate docopt;
extern crate gdrs_obj;



fn main() {
	let c = clang::Clang::new().unwrap();

	let mut index = clang::Index::new(&c, true, false);
	index.set_thread_options(clang::ThreadOptions{editing: false, indexing: false});


	println!("Hello, World!");
}
