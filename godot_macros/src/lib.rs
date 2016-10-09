#![feature(proc_macro, proc_macro_lib)]

extern crate proc_macro;
extern crate syntex_syntax;

use proc_macro::TokenStream;



#[proc_macro_derive(GodotSubclass)]
pub fn godot_subclass(input: TokenStream) -> TokenStream {
	let source = input.to_string();


	source.parse().unwrap()
}
