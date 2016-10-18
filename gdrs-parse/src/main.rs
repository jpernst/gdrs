#![feature(proc_macro, custom_derive)]

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
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
use std::io::Write;
use docopt::Docopt;



const USAGE: &'static str = r#"
Parse Godot source and generate JSON API description.

Usage:
	gdrs-parse [-o <output>] [-I <include> | -D <define>]... <file>...
	gdrs-parse --help

Options:
	-I <include>  Add an #include search path
	-D <define>   Define a preprocessor symbol
	-o <output>   Output file [default: -]
	-h, --help    Show this message
"#;



#[derive(RustcDecodable)]
#[allow(non_snake_case)]
struct Args {
	pub flag_o: String,
	pub flag_I: Option<Vec<String>>,
	pub flag_D: Option<Vec<String>>,
	pub flag_help: bool,
	pub arg_file: Vec<String>,
}



fn main() {
	let (output, flags, files) = {
		let Args{flag_o: output, flag_I: includes, flag_D: defines, flag_help: help, arg_file: files} = Docopt::new(USAGE)
			.and_then(|d| d.argv(env::args().into_iter()).decode())
			.unwrap_or_else(|e| e.exit());

		if help {
			println!("{}", USAGE);
			return;
		}

		let mut flags = vec!["-xc++".to_string()];
		if let Some(includes) = includes {
			flags.extend(includes.into_iter().map(|i| format!("-I{}", i)));
		}
		if let Some(defines) = defines {
			flags.extend(defines.into_iter().map(|d| format!("-D{}", d)));
		}

		(output, flags, files)
	};

	let c = clang::Clang::new().unwrap();

	let mut index = clang::Index::new(&c, true, false);
	index.set_thread_options(clang::ThreadOptions{editing: false, indexing: false});

	let mut api = gdrs_obj::Api{
		consts: Vec::new(),
		enums: Vec::new(),
		aliases: Vec::new(),
		classes: Vec::new(),
		functions: Vec::new(),
	};

	for file_pat in &files {
		for file in glob::glob(file_pat).unwrap() {
			let file = file.unwrap();

			let mut parser = index.parser(file);
			parser.arguments(&flags);
			//let parser = parser.detailed_preprocessing_record(true);
			let parser = parser.skip_function_bodies(true);

			let tu = parser.parse().unwrap();
			tu.get_entity().visit_children(|e, _| {
				if e.is_in_system_header() {
					return clang::EntityVisitResult::Continue;
				}
				let loc = e.get_location().unwrap().get_expansion_location().file.get_path();
				if loc.components().any(|c| match c { path::Component::Normal(c) => c == "thirdparty", _ => false }) {
					return clang::EntityVisitResult::Continue;
				}

				match e.get_kind() {
					clang::EntityKind::VarDecl => {
						if e.get_type().unwrap().is_const_qualified() {
							if let Some(val) = e.get_child(0).and_then(|exp| parse_value(exp)) {
								api.consts.push(gdrs_obj::Const{
									ty: parse_type(e.get_type().unwrap()).or_else(|| parse_type(e.get_child(0).unwrap().get_type().unwrap())).unwrap(),
									name: e.get_name().unwrap(),
									value: val,
								})
							}
						} else {
							println!("WARNING: Unsupported global variable: {}", e.get_name().unwrap());
						}
					},
					clang::EntityKind::EnumDecl => {
						let _enum = parse_enum(&e);
						if _enum.name == "const" {
							let gdrs_obj::Enum{variants, underlying, ..} = _enum;
							for v in variants.into_iter() {
								api.consts.push(gdrs_obj::Const{
									ty: underlying.clone(),
									name: v.name,
									value: v.value,
								});
							}
						} else {
							api.enums.push(_enum);
						}
					},
					clang::EntityKind::TypeAliasDecl | clang::EntityKind::TypedefDecl => {
						api.aliases.push(parse_alias(e));
					},
					clang::EntityKind::ClassDecl => {
						let mut class = parse_class(e);
						class.include = loc.to_string_lossy().into_owned();
						api.classes.push(class);
					},
					clang::EntityKind::FunctionDecl => {
						if let Some(func) = parse_function(e) {
							api.functions.push(func);
						}
					},
					_ => (),
				}

				clang::EntityVisitResult::Continue
			});
		}
	}

	let json = serde_json::to_string_pretty(&api).unwrap();
	if output == "-" {
		println!("{}", json);
	} else {
		let mut file = fs::File::create(path::Path::new(&output)).unwrap();
		write!(file, "{}", json).unwrap();
	}
}



fn parse_enum(e: &clang::Entity) -> gdrs_obj::Enum {
	let underlying = parse_type(e.get_enum_underlying_type().unwrap()).unwrap();
	let mut _enum = gdrs_obj::Enum{
		name: e.get_name().unwrap_or_else(|| "const".to_string()),
		underlying: underlying.clone(),
		variants: Vec::new(),
	};

	e.visit_children(|c, _| {
		_enum.variants.push(gdrs_obj::Variant{
			name: c.get_name().unwrap(),
			value: match _enum.underlying.name {
				gdrs_obj::Typename::Char | gdrs_obj::Typename::Short | gdrs_obj::Typename::Int | gdrs_obj::Typename::Long | gdrs_obj::Typename::LongLong
					=> gdrs_obj::Value::Int(c.get_enum_constant_value().map(|(v, _)| v).unwrap()),
				gdrs_obj::Typename::UChar | gdrs_obj::Typename::UShort | gdrs_obj::Typename::UInt | gdrs_obj::Typename::ULong | gdrs_obj::Typename::ULongLong
					=> gdrs_obj::Value::UInt(c.get_enum_constant_value().map(|(_, v)| v).unwrap()),
				_ => unreachable!(),
			},
		});

		clang::EntityVisitResult::Continue
	});

	_enum
}



fn parse_alias(e: clang::Entity) -> gdrs_obj::Alias {
	gdrs_obj::Alias{
		name: e.get_name().unwrap(),
		ty: parse_type(e.get_typedef_underlying_type().unwrap()).unwrap(),
	}
}



fn parse_class(e: clang::Entity) -> gdrs_obj::Class {
	let mut class = gdrs_obj::Class{
		include: String::new(),
		name: e.get_name().unwrap(),
		consts: Vec::new(),
		enums: Vec::new(),
		aliases: Vec::new(),
		fields: Vec::new(),
		methods: Vec::new(),
	};

	e.visit_children(|c, _| {
		let access = c.get_accessibility().unwrap();
		if access == clang::Accessibility::Private {
			return clang::EntityVisitResult::Continue
		}

		match c.get_kind() {
			clang::EntityKind::EnumDecl => {
				let _enum = parse_enum(&c);
				if _enum.name == "const" {
					let gdrs_obj::Enum{variants, ..} = _enum;
					for v in variants.into_iter() {
						class.consts.push(gdrs_obj::Const{
							ty: _enum.underlying.clone(),
							name: v.name,
							value: v.value,
						});
					}
				} else {
					class.enums.push(_enum);
				}
			},
			clang::EntityKind::TypeAliasDecl | clang::EntityKind::TypedefDecl => {
				class.aliases.push(parse_alias(c));
			},
			clang::EntityKind::FieldDecl | clang::EntityKind::VarDecl => {
				if c.get_type().unwrap().is_const_qualified() {
					if let Some(val) = c.get_child(0).and_then(|exp| parse_value(exp)) {
						class.consts.push(gdrs_obj::Const{
							ty: parse_type(c.get_type().unwrap()).or_else(|| parse_type(c.get_child(0).unwrap().get_type().unwrap())).unwrap(),
							name: c.get_name().unwrap(),
							value: val,
						})
					}
				} else {
					let ty = parse_type(c.get_type().unwrap());
					if ty.is_none() {
						return clang::EntityVisitResult::Continue;
					}
					let ty = ty.unwrap();

					class.fields.push(gdrs_obj::Field{
						access: if let clang::Accessibility::Protected = access { gdrs_obj::Access::Protected } else { gdrs_obj::Access::Public },
						is_static: c.get_storage_class() == Some(clang::StorageClass::Static),
						name: c.get_name().unwrap(),
						ty: ty,
					});
				}
			},
			clang::EntityKind::Method => {
				if let Some(method) = parse_function(c) {
					class.methods.push(method);
				}
			},
			_ => (),
		}

		clang::EntityVisitResult::Continue
	});

	class
}



fn parse_function(e: clang::Entity) -> Option<gdrs_obj::Function> {
	let ty = e.get_type().unwrap();
	let result = ty.get_result_type().unwrap();

	Some(gdrs_obj::Function{
		access: if let Some(clang::Accessibility::Protected) = e.get_accessibility() { gdrs_obj::Access::Protected } else { gdrs_obj::Access::Public },
		semantic: if e.is_virtual_method() {
			gdrs_obj::FunctionSemantic::Virtual
		} else if e.is_static_method() {
			gdrs_obj::FunctionSemantic::Static
		} else if e.get_kind() == clang::EntityKind::Method {
			gdrs_obj::FunctionSemantic::Method
		} else {
			gdrs_obj::FunctionSemantic::Free
		},
		return_ty: if result.get_kind() == clang::TypeKind::Void { None } else { if let Some(r) = parse_type(result) { Some(r) } else {
			println!("WARNING: Unsupported return type: {:?}", result);
			return None;
		}},
		name: e.get_name().unwrap(),
		params: {
			if let Some(params) = e.get_arguments().map(|vp| vp.into_iter().map(|p| (parse_type(p.get_type().unwrap()), p.get_name().unwrap(), p.get_child(0))).collect::<Vec<_>>()) {
				if let Some(i) = params.iter().position(|&(ref p, _, _)| p.is_none()) {
					println!("WARNING: Unsupported param type: {:?}", e.get_arguments().unwrap()[i].get_type());
					return None;
				}

				Some(params.into_iter().map(|(p, n, d)| gdrs_obj::Param{
					ty: p.unwrap(),
					name: n,
					default: d.and_then(|d| parse_value(d)),
				}).collect())
			} else {
				None
			}
		},
		is_const: e.is_const_method(),
	})
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
			clang::TypeKind::Auto => { return None; }

			clang::TypeKind::Bool => gdrs_obj::Typename::Bool,
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
			clang::TypeKind::Float => gdrs_obj::Typename::Float,
			clang::TypeKind::Double => gdrs_obj::Typename::Double,
			clang::TypeKind::Enum => gdrs_obj::Typename::Enum(t.get_declaration().unwrap().get_name().unwrap()),

			clang::TypeKind::Record => {
				if let Some(params) = t.get_template_argument_types().map(|vp| vp.into_iter().map(|p| parse_type(p.unwrap())).collect::<Vec<_>>()) {
					if let Some(i) = params.iter().position(|p| p.is_none()) {
						println!("WARNING: Unsupported template param type: {:?}", t.get_template_argument_types().unwrap()[i]);
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

			k => {
				println!("WARNING: Unsupported type kind: {:?}", k);
				return None;
			},
		},
	})
}



fn parse_value(exp: clang::Entity) -> Option<gdrs_obj::Value> {
	if let (Some(kind), Some(val)) = (exp.get_type().map(|t| t.get_kind()), exp.evaluate()) {
		match val {
			clang::EvaluationResult::Integer(i)
				if kind == clang::TypeKind::CharU
				|| kind == clang::TypeKind::UChar
				|| kind == clang::TypeKind::UShort
				|| kind == clang::TypeKind::UInt
				|| kind == clang::TypeKind::ULong
				|| kind == clang::TypeKind::ULongLong
			=> Some(gdrs_obj::Value::UInt(i as u64)),
			clang::EvaluationResult::Integer(i)
				if kind == clang::TypeKind::CharS
				|| kind == clang::TypeKind::SChar
				|| kind == clang::TypeKind::Short
				|| kind == clang::TypeKind::Int
				|| kind == clang::TypeKind::Long
				|| kind == clang::TypeKind::LongLong
			=> Some(gdrs_obj::Value::Int(i)),
			clang::EvaluationResult::Float(d) if kind == clang::TypeKind::Float => Some(gdrs_obj::Value::Float(d as f32)),
			clang::EvaluationResult::Float(d) if kind == clang::TypeKind::Double => Some(gdrs_obj::Value::Double(d)),
			clang::EvaluationResult::String(s) => Some(gdrs_obj::Value::String(s.to_string_lossy().into_owned())),
			v => {
				println!("WARNING: Unsupported evaluation result: {:?}", v);
				return None;
			},
		}
	} else {
		None
	}
}
