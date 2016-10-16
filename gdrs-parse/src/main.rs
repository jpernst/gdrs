#![feature(proc_macro, custom_derive)]

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate clang;
extern crate docopt;
#[macro_use]
extern crate rustc_serialize;
extern crate toml;
extern crate gdrs_obj;
extern crate glob;

use std::env;
use std::fs;
use std::path;
use std::io::Read;
use docopt::Docopt;



const USAGE: &'static str = r#"
Parse Godot source and generate JSON API description.

Usage:
	gdrs-parse [options] <godot-dir>
	gdrs-parse --help

Options:
	-e <extra>, --extra=<extra>       TOML file with extra input
	-o <output>, --output=<output>    File to store JSON output
	-h, --help                        Show this message

Format of extra file is as follows:
flags = ["<clang-flag>", ...]
headers = ["<header-file>", ...]
"#;



#[derive(RustcDecodable)]
#[allow(non_snake_case)]
struct Args {
	pub flag_extra: Option<String>,
	pub flag_output: Option<String>,
	pub flag_help: bool,
    pub arg_godot_dir: String,
}



#[derive(Deserialize)]
struct Input {
	flags: Vec<String>,
	headers: Vec<String>,
}



fn main() {
	let args: Args = Docopt::new(USAGE)
		.and_then(|d| d.argv(env::args().into_iter()).decode())
		.unwrap_or_else(|e| e.exit());

	if args.flag_help {
		println!("{}", USAGE);
		return;
	}

	let mut input = Input{
		flags: vec![
			"-x".into(),
			"c++".into(),
			"-Icore".into(),
			format!("-I{}", path::Path::new("core").join("math").to_string_lossy()),
			"-Itools".into(),
			"-Idrivers".into(),
			format!("-I{}", args.arg_godot_dir),
			"-Imodules".into()
		],
		headers: vec![
			path::Path::new("scene").join("3d").join("physics_joint.h").to_string_lossy().into_owned(),
		],
	};

	if let Some(Input{flags, headers}) = args.flag_extra.as_ref().map(|e| {
		let mut file = fs::File::open(e).unwrap();
		let mut extra = String::new();
		file.read_to_string(&mut extra).unwrap();
		toml::decode_str::<Input>(&extra).unwrap()
	}) {
		input.flags.extend(flags.into_iter());
		input.headers.extend(headers.into_iter());
	}

	let c = clang::Clang::new().unwrap();

	let mut index = clang::Index::new(&c, true, false);
	index.set_thread_options(clang::ThreadOptions{editing: false, indexing: false});

	let mut api = gdrs_obj::Api{
		consts: Vec::new(),
		enums: Vec::new(),
		classes: Vec::new(),
		functions: Vec::new(),
	};

	for header_pat in &input.headers {
		let header = &path::Path::new(&args.arg_godot_dir).join(header_pat);
		for header in glob::glob(&header.to_string_lossy()).unwrap() {
			let header = header.unwrap();

			let mut parser = index.parser(header);
			parser.arguments(&input.flags);
			//let parser = parser.detailed_preprocessing_record(true);
			let parser = parser.skip_function_bodies(true);

			let tu = parser.parse().unwrap();
			tu.get_entity().visit_children(|e, _| {
				let loc = match e.get_location().unwrap().get_expansion_location().file.get_path().strip_prefix(&args.arg_godot_dir) {
					Ok(p) => p.to_owned(),
					Err(_) => { return clang::EntityVisitResult::Continue; },
				};
				if loc.components().any(|c| match c { path::Component::Normal(c) => c == "thirdparty", _ => false }) {
					return clang::EntityVisitResult::Continue;
				}

				match e.get_kind() {
					clang::EntityKind::EnumDecl => {
						let _enum = parse_enum(&e);
						if _enum.name == "const" {
							let gdrs_obj::Enum{variants, ..} = _enum;
							for v in variants.into_iter() {
								api.consts.push(v);
							}
						} else {
							api.enums.push(_enum);
						}

					},
					clang::EntityKind::ClassDecl => {
						let mut class = parse_class(e);
						class.include = loc.to_string_lossy().into_owned();
						api.classes.push(class);
					},
					clang::EntityKind::FunctionDecl => {
						api.functions.push(parse_function(e));
					},
					_ => (),
				}

				clang::EntityVisitResult::Continue
			});
		}
	}
}



fn parse_enum(e: &clang::Entity) -> gdrs_obj::Enum {
	let mut _enum = gdrs_obj::Enum{
		name: e.get_name().unwrap_or_else(|| "const".to_string()),
		underlying: match e.get_enum_underlying_type().unwrap().get_kind() {
			clang::TypeKind::CharS | clang::TypeKind::SChar => gdrs_obj::Typename::Char,
			clang::TypeKind::CharU | clang::TypeKind::UChar => gdrs_obj::Typename::UChar,
			clang::TypeKind::Short => gdrs_obj::Typename::Short,
			clang::TypeKind::UShort => gdrs_obj::Typename::UShort,
			clang::TypeKind::Int => gdrs_obj::Typename::Int,
			clang::TypeKind::UInt => gdrs_obj::Typename::UInt,
			clang::TypeKind::Long => gdrs_obj::Typename::Long,
			clang::TypeKind::ULong => gdrs_obj::Typename::ULong,
			clang::TypeKind::LongLong => gdrs_obj::Typename::LongLong,
			clang::TypeKind::ULongLong => gdrs_obj::Typename::ULongLong,
			ut => panic!("Unsupported enum underlying type: {:?}", ut),
		},
		variants: Vec::new(),
	};

	e.visit_children(|c, _| {
		_enum.variants.push(gdrs_obj::Const{
			name: c.get_name().unwrap(),
			value: match _enum.underlying {
				gdrs_obj::Typename::Char | gdrs_obj::Typename::Short | gdrs_obj::Typename::Int | gdrs_obj::Typename::Long | gdrs_obj::Typename::LongLong
					=> gdrs_obj::Value::Int(c.get_enum_constant_value().map(|(v, _)| v).unwrap()),
				gdrs_obj::Typename::UChar | gdrs_obj::Typename::UShort | gdrs_obj::Typename::UInt | gdrs_obj::Typename::ULong | gdrs_obj::Typename::ULongLong
					=> gdrs_obj::Value::UInt(c.get_enum_constant_value().map(|(_, v)| v).unwrap()),
				_ => unreachable!(),
			}
		});

		clang::EntityVisitResult::Continue
	});

	_enum
}



fn parse_class(e: clang::Entity) -> gdrs_obj::Class {
	let mut class = gdrs_obj::Class{
		include: String::new(),
		name: e.get_name().unwrap(),
		fields: Vec::new(),
		methods: Vec::new(),
		enums: Vec::new(),
		consts: Vec::new(),
	};

	e.visit_children(|c, _| {
		let access = c.get_accessibility().unwrap();
		if access == clang::Accessibility::Private {
			return clang::EntityVisitResult::Continue
		}

		println!("{:?}", c.get_kind());
		match c.get_kind() {
			clang::EntityKind::EnumDecl => {
				let _enum = parse_enum(&c);
				if _enum.name == "const" {
					let gdrs_obj::Enum{variants, ..} = _enum;
					for v in variants.into_iter() {
						class.consts.push(v);
					}
				} else {
					class.enums.push(_enum);
				}
			},
			clang::EntityKind::FieldDecl => {
				let ty = parse_type(c.get_type().unwrap());
				if ty.is_none() {
					return clang::EntityVisitResult::Continue;
				}
				let ty = ty.unwrap();

				class.fields.push(gdrs_obj::Field{
					access: if let clang::Accessibility::Public = access { gdrs_obj::Access::Public } else { gdrs_obj::Access::Protected },
					name: c.get_name().unwrap(),
					ty: ty,
				});
			},
			clang::EntityKind::Method => {
				class.methods.push(parse_function(c));
			},
			_ => (),
		}

		clang::EntityVisitResult::Continue
	});

	class
}



fn parse_function(e: clang::Entity) -> gdrs_obj::Function {
	gdrs_obj::Function{
		access: gdrs_obj::Access::Public,
		semantic: gdrs_obj::FunctionSemantic::Free,
		return_ty: None,
		name: e.get_name().unwrap(),
		args: None,
	}
}



fn parse_type(mut t: clang::Type) -> Option<gdrs_obj::Type> {
	let semantic = match t.get_kind() {
		clang::TypeKind::Pointer => {
			t = t.get_pointee_type().unwrap();
			gdrs_obj::TypeSemantic::Pointer
		},
		clang::TypeKind::LValueReference => {
			t = t.get_pointee_type().unwrap();
			gdrs_obj::TypeSemantic::Reference
		},
		clang::TypeKind::ConstantArray => {
			let size = t.get_size().unwrap();
			t = t.get_element_type().unwrap();
			gdrs_obj::TypeSemantic::Array(size)
		},
		_ => gdrs_obj::TypeSemantic::Value,
	};

	Some(gdrs_obj::Type{
		is_const: t.is_const_qualified(),
		semantic: semantic,
		name: match t.get_kind() {
			clang::TypeKind::Bool => gdrs_obj::Typename::Bool,
			clang::TypeKind::CharS | clang::TypeKind::SChar => gdrs_obj::Typename::Bool,
			clang::TypeKind::CharU | clang::TypeKind::UChar => gdrs_obj::Typename::Bool,
			clang::TypeKind::Short => gdrs_obj::Typename::Short,
			clang::TypeKind::UShort => gdrs_obj::Typename::UShort,
			clang::TypeKind::Int => gdrs_obj::Typename::Int,
			clang::TypeKind::UInt => gdrs_obj::Typename::UInt,
			clang::TypeKind::Long => gdrs_obj::Typename::Long,
			clang::TypeKind::ULong => gdrs_obj::Typename::ULong,
			clang::TypeKind::LongLong => gdrs_obj::Typename::LongLong,
			clang::TypeKind::ULongLong => gdrs_obj::Typename::ULongLong,
			clang::TypeKind::Float => gdrs_obj::Typename::Float,
			clang::TypeKind::Double => gdrs_obj::Typename::Double,

			clang::TypeKind::Record => {
				if let Some(params) = t.get_template_argument_types().map(|va| va.into_iter().map(|a| parse_type(a.unwrap())).collect::<Vec<_>>()) {
					if params.iter().any(|p| p.is_none()) {
						return None;
					}

					gdrs_obj::Typename::Class(
						t.get_declaration().unwrap().get_name().unwrap(),
						Some(params.into_iter().map(|p| p.unwrap()).collect())
					)
				} else {
					gdrs_obj::Typename::Class(t.get_declaration().unwrap().get_name().unwrap(), None)
				}
			},

			clang::TypeKind::Enum => gdrs_obj::Typename::Enum(t.get_declaration().unwrap().get_name().unwrap()),

			k => {
				println!("WARNING: Unsupported type kind {:?}", k);
				return None;
			},
		},
	})
}
