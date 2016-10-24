#![feature(proc_macro, custom_derive)]

extern crate serde;
extern crate docopt;
extern crate tempdir;
extern crate clang;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[macro_use]
extern crate rustc_serialize;
extern crate toml;

extern crate gdrs_api;

use std::env;
use std::fs;
use std::path;
use std::io::{self, Write};
use std::ffi::OsStr;
use docopt::Docopt;
use tempdir::TempDir;



const USAGE: &'static str = r#"
Parse Godot source and generate JSON API description.

Usage:
	gdrs-parse [options] <file>...
	gdrs-parse --help

Options:
	-o OUTPUT         Output file [default: -]
	-D DEFINE ...     Define a preprocessor symbol
	-I INCLUDE ...    Add an #include search path
	-h, --help        Show this message
"#;



#[derive(Clone, PartialEq, Eq, Debug)]
enum ParseError {
	Ignored,
	Unsupported,
}



#[derive(RustcDecodable)]
#[allow(non_snake_case)]
struct Args {
	pub flag_o: String,
	pub flag_D: Option<Vec<String>>,
	pub flag_I: Option<Vec<String>>,
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
		if let Some(defines) = defines {
			flags.extend(defines.into_iter().map(|d| format!("-D{}", d)));
		}
		if let Some(includes) = includes {
			flags.extend(includes.into_iter().map(|i| format!("-I{}", i)));
		}

		(output, flags, files)
	};

	let tmp_dir = TempDir::new("gdrs-parse").unwrap();
	let unity_path = tmp_dir.path().join("unity.cpp");
	{
		let mut unity = fs::File::create(&unity_path).unwrap();
		for file in &files {
			writeln!(unity, r#"#include "{}""#, file).unwrap();
		}
	}

	let c = clang::Clang::new().unwrap();
	let mut index = clang::Index::new(&c, true, false);
	index.set_thread_options(clang::ThreadOptions{editing: false, indexing: false});

	let mut parser = index.parser(unity_path);
	parser.arguments(&flags);
	//let parser = parser.detailed_preprocessing_record(true);
	let parser = parser.skip_function_bodies(true);

	let mut api = parse_namespace(parser.parse().unwrap().get_entity()).unwrap();
	api.name = "".to_string();

	let json = serde_json::to_string_pretty(&api).unwrap();
	if output == "-" {
		println!("{}", json);
	} else {
		let mut file = fs::File::create(path::Path::new(&output)).unwrap();
		write!(file, "{}", json).unwrap();
	}
}



fn parse_namespace(e: clang::Entity) -> Option<gdrs_api::Namespace> {
	let name = e.get_name();
	if let None = name {
		return None;
	}

	let mut ns = gdrs_api::Namespace{
		name: name.unwrap(),
		consts: Vec::with_capacity(0),
		globals: Vec::with_capacity(0),
		enums: Vec::with_capacity(0),
		aliases: Vec::with_capacity(0),
		functions: Vec::with_capacity(0),
		classes: Vec::with_capacity(0),
		namespaces: Vec::with_capacity(0),
	};

	e.visit_children(|c, _| {
		if c.is_in_system_header() {
			return clang::EntityVisitResult::Continue;
		}
		let loc = c.get_location().unwrap().get_expansion_location().file.get_path();
		if loc.extension() == Some(OsStr::new("cpp")) || loc.components().any(|c| c == path::Component::Normal(OsStr::new("thirdparty"))) {
			return clang::EntityVisitResult::Continue;
		}
		let loc = loc.to_str().unwrap();

		match c.get_kind() {
			clang::EntityKind::VarDecl => {
				if c.get_type().unwrap().is_const_qualified() {
					if let Some(val) = c.get_child(0).and_then(|exp| parse_value(exp)) {
						ns.consts.push(gdrs_api::Const{
							ty: parse_type(c.get_type().unwrap()).or_else(|_| parse_type(c.get_child(0).unwrap().get_type().unwrap())).unwrap(),
							name: c.get_name().unwrap(),
							value: val,
						})
					}
				} else if c.get_storage_class() == Some(clang::StorageClass::Extern) {
					match parse_type(c.get_type().unwrap()) {
						Ok(ty) => ns.globals.push(gdrs_api::Global{
							ty: ty,
							name: c.get_name().unwrap(),
						}),
						Err(ParseError::Unsupported) => {
							let _ = writeln!(io::stderr(), "WARNING: Unsupported extern global type `{}`: {:?}", c.get_name().unwrap(), c);
						},
						_ => (),
					}
				}
			},
			clang::EntityKind::EnumDecl => {
				let _enum = parse_enum(&c);
				if _enum.name == "auto" {
					let gdrs_api::Enum{variants, underlying, ..} = _enum;
					for v in variants.into_iter() {
						ns.consts.push(gdrs_api::Const{
							ty: underlying.clone(),
							name: v.name,
							value: v.value,
						});
					}
				} else {
					ns.enums.push(_enum);
				}
			},
			clang::EntityKind::TypeAliasDecl | clang::EntityKind::TypedefDecl => {
				if let Some(underlying) = c.get_typedef_underlying_type().unwrap().get_declaration() {
					if underlying.get_name().is_none() {
						match underlying.get_kind() {
							clang::EntityKind::EnumDecl => {
								let mut _enum = parse_enum(&underlying);
								_enum.name = c.get_name().unwrap();
								ns.enums.push(_enum);
							},
							clang::EntityKind::ClassDecl | clang::EntityKind::StructDecl | clang::EntityKind::UnionDecl => {
								if let Some(mut class) = parse_class(underlying, loc.to_string()) {
									class.name = c.get_name().unwrap();
									ns.classes.push(class);
								}
							},
							_ => (),
						}
					} else if let Some(alias) = parse_alias(c) {
						ns.aliases.push(alias);
					}
				}
			},
			clang::EntityKind::ClassDecl | clang::EntityKind::StructDecl => {
				if let Some(class) = parse_class(c, loc.to_string()) {
					if class.name != "auto" {
						ns.classes.push(class);
					}
				}
			},
			clang::EntityKind::UnionDecl => {
				if let Some(union) = parse_class(c, loc.to_string()) {
					if union.name != "auto" {
						ns.classes.push(union);
					}
				}
			},
			clang::EntityKind::FunctionDecl => {
				if let Some(func) = parse_function(c) {
					ns.functions.push(func);
				}
			},
			clang::EntityKind::Namespace => {
				if let Some(cns) = parse_namespace(c) {
					if let Some(dns) = ns.namespaces.iter_mut().find(|dns| dns.name == cns.name) {
						dns.merge(cns);
						return clang::EntityVisitResult::Continue;
					}

					ns.namespaces.push(cns);
				}
			},
			_ => (),
		}

		clang::EntityVisitResult::Continue
	});

	Some(ns)
}



fn parse_enum(e: &clang::Entity) -> gdrs_api::Enum {
	let underlying = parse_type(e.get_enum_underlying_type().unwrap()).unwrap();
	let mut _enum = gdrs_api::Enum{
		name: e.get_name().unwrap_or_else(|| "auto".to_string()),
		underlying: underlying.clone(),
		variants: Vec::new(),
	};

	e.visit_children(|c, _| {
		_enum.variants.push(gdrs_api::Variant{
			name: c.get_name().unwrap(),
			value: match _enum.underlying.name {
				gdrs_api::TypeName::Char | gdrs_api::TypeName::Short | gdrs_api::TypeName::Int | gdrs_api::TypeName::Long | gdrs_api::TypeName::LongLong
					=> gdrs_api::Value::Int(c.get_enum_constant_value().map(|(v, _)| v).unwrap()),
				gdrs_api::TypeName::UChar | gdrs_api::TypeName::UShort | gdrs_api::TypeName::UInt | gdrs_api::TypeName::ULong | gdrs_api::TypeName::ULongLong
					=> gdrs_api::Value::UInt(c.get_enum_constant_value().map(|(_, v)| v).unwrap()),
				_ => unreachable!(),
			},
		});

		clang::EntityVisitResult::Continue
	});

	_enum
}



fn parse_alias(e: clang::Entity) -> Option<gdrs_api::TypeAlias> {
	match parse_type(e.get_typedef_underlying_type().unwrap()) {
		Ok(ty) => Some(gdrs_api::TypeAlias{
			name: e.get_name().unwrap(),
			ty: ty,
		}),
		Err(ParseError::Unsupported) => {
			let _ = writeln!(io::stderr(), "WARNING: Unsupported alias type `{}`: {:?}", e.get_name().unwrap(), e);
			None
		},
		Err(ParseError::Ignored) => None,
	}
}



fn parse_class(e: clang::Entity, loc: String) -> Option<gdrs_api::Class> {
	if !e.is_definition() || e.get_template().is_some() {
		return None;
	}

	let mut class = gdrs_api::Class{
		include: loc.clone(),
		name: e.get_name().unwrap_or_else(|| "auto".to_string()),
		inherits: None,
		is_pod: e.get_type().unwrap().is_pod(),
		is_union: e.get_kind() == clang::EntityKind::UnionDecl,
		consts: Vec::with_capacity(0),
		enums: Vec::with_capacity(0),
		aliases: Vec::with_capacity(0),
		fields: Vec::with_capacity(0),
		anon_unions: Vec::with_capacity(0),
		ctors: Vec::with_capacity(0),
		methods: Vec::with_capacity(0),
		virtual_dtor: false,
		classes: Vec::with_capacity(0),
	};

	e.visit_children(|c, _| {
		let access = match c.get_accessibility() {
			Some(clang::Accessibility::Private) => {
				if class.is_pod && c.get_kind() == clang::EntityKind::FieldDecl {
					let _ = writeln!(io::stderr(), "WARNING: Private POD field `{:?}`: {:?}", c, e);
					class.is_pod = false;
				}
				return clang::EntityVisitResult::Continue;
			},
			Some(a) => a,
			None => return clang::EntityVisitResult::Continue,
		};

		match c.get_kind() {
			clang::EntityKind::BaseSpecifier => {
				if access == clang::Accessibility::Public {
					if class.inherits.is_some() {
						let _ = writeln!(io::stderr(), "WARNING: Multiple inheritance `{:?}`: {:?}", c, e);
					} else {
						match parse_type(c.get_type().unwrap()) {
							Ok(t) => class.inherits = Some(t.name),
							Err(ParseError::Unsupported) => {
								let _ = writeln!(io::stderr(), "WARNING: Unsupported base type `{:?}`: {:?}", c, e);
							},
							Err(ParseError::Ignored) => (),
						}
					}
				} else {
					let _ = writeln!(io::stderr(), "WARNING: Non-public inheritance `{:?}`: {:?}", c, e);
				}
			},
			clang::EntityKind::EnumDecl => {
				let _enum = parse_enum(&c);
				if _enum.name == "auto" {
					let gdrs_api::Enum{variants, ..} = _enum;
					for v in variants.into_iter() {
						class.consts.push(gdrs_api::Const{
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
				if let Some(underlying) = c.get_typedef_underlying_type().unwrap().get_declaration() {
					if underlying.get_name().is_none() {
						match underlying.get_kind() {
							clang::EntityKind::EnumDecl => {
								let mut _enum = parse_enum(&underlying);
								_enum.name = c.get_name().unwrap();
								class.enums.push(_enum);
							},
							clang::EntityKind::ClassDecl | clang::EntityKind::StructDecl => {
								if let Some(mut nested) = parse_class(underlying, loc.clone()) {
									nested.name = c.get_name().unwrap();
									class.classes.push(nested);
								}
							},
							_ => (),
						}
					} else if let Some(alias) = parse_alias(c) {
						class.aliases.push(alias);
					}
				}
			},
			clang::EntityKind::FieldDecl | clang::EntityKind::VarDecl => {
				if c.get_type().unwrap().is_const_qualified() {
					if let Some(val) = c.get_child(0).and_then(|exp| parse_value(exp)) {
						class.consts.push(gdrs_api::Const{
							ty: parse_type(c.get_type().unwrap()).or_else(|_| parse_type(c.get_child(0).unwrap().get_type().unwrap())).unwrap(),
							name: c.get_name().unwrap(),
							value: val,
						})
					}
				} else {
					let ty = match parse_type(c.get_type().unwrap()) {
						Ok(ty) => ty,
						Err(ParseError::Unsupported) => {
							let _ = writeln!(io::stderr(), "WARNING: Unsupported field type `{:?}`: {:?}", c.get_type().unwrap(), c);
							return clang::EntityVisitResult::Continue;
						},
						Err(ParseError::Ignored) => return clang::EntityVisitResult::Continue,
					};

					class.fields.push(gdrs_api::Field{
						access: if let clang::Accessibility::Protected = access { gdrs_api::Access::Protected } else { gdrs_api::Access::Public },
						is_static: c.get_storage_class() == Some(clang::StorageClass::Static),
						name: c.get_name().unwrap(),
						ty: ty,
					});
				}
			},
			clang::EntityKind::Constructor => {
				if let Some(ctor) = parse_function(c) {
					class.ctors.push(ctor);
				}
			},
			clang::EntityKind::Method => {
				if let Some(method) = parse_function(c) {
					class.methods.push(method);
				}
			},
			clang::EntityKind::Destructor => {
				if c.is_virtual_method() {
					class.virtual_dtor = true;
				}
			},
			clang::EntityKind::ClassDecl | clang::EntityKind::StructDecl => {
				if let Some(nested) = parse_class(c, loc.clone()) {
					if nested.name != "auto" {
						class.classes.push(nested);
					}
				}
			},
			clang::EntityKind::UnionDecl => {
				if let Some(union) = parse_class(c, loc.to_string()) {
					if union.name != "auto" {
						class.classes.push(union);
					} else {
						class.anon_unions.push(union);
					}
				}
			},
			_ => (),
		}

		clang::EntityVisitResult::Continue
	});

	Some(class)
}



fn parse_function(e: clang::Entity) -> Option<gdrs_api::Function> {
	let ty = e.get_type().unwrap();
	let result = ty.get_result_type().unwrap();

	Some(gdrs_api::Function{
		name: e.get_name().unwrap(),
		params: {
			if let Some(params) = e.get_arguments()
				.map(|vp| vp.into_iter().map(|p| (parse_type(p.get_type().unwrap()), p.get_name().unwrap_or_else(|| "".to_string()), p.get_child(0)))
				.collect::<Vec<_>>())
			{
				if let Some(i) = params.iter().position(|&(ref p, _, _)| p.is_err()) {
					let param = e.get_arguments().unwrap()[i];
					if params[i].0.as_ref().unwrap_err() == &ParseError::Unsupported {
						let _ = writeln!(io::stderr(), "WARNING: Unsupported param type `{:?}`: {:?}", param, e);
					}
					return None;
				}

				params.into_iter().map(|(p, n, d)| gdrs_api::Param{
					ty: p.unwrap(),
					name: n,
					default: d.and_then(|d| parse_value(d)),
				}).collect()
			} else {
				Vec::with_capacity(0)
			}
		},
		return_ty: if result.get_kind() == clang::TypeKind::Void { None } else {
			match parse_type(result) {
				Ok(r) => Some(r),
				Err(ParseError::Unsupported) => {
					let _ = writeln!(io::stderr(), "WARNING: Unsupported return type `{:?}`: {:?}", result, e);
					return None;
				},
				_ => return None,
			}
		},
		semantic: if e.is_virtual_method() {
			gdrs_api::FunctionSemantic::Virtual
		} else if e.is_static_method() {
			gdrs_api::FunctionSemantic::Static
		} else if e.get_kind() == clang::EntityKind::Method {
			gdrs_api::FunctionSemantic::Method
		} else {
			gdrs_api::FunctionSemantic::Free
		},
		access: if let Some(clang::Accessibility::Protected) = e.get_accessibility() { gdrs_api::Access::Protected } else { gdrs_api::Access::Public },
		is_const: e.is_const_method(),
	})
}



fn parse_type(mut t: clang::Type) -> Result<gdrs_api::TypeRef, ParseError> {
	t = t.get_elaborated_type().unwrap_or(t);

	let semantic = match t.get_kind() {
		clang::TypeKind::Pointer | clang::TypeKind::IncompleteArray => {
			t = t.get_pointee_type().or_else(|| t.get_element_type()).map(|t| t.get_elaborated_type().unwrap_or(t)).unwrap();
			if t.get_kind() == clang::TypeKind::Pointer {
				t = t.get_pointee_type().map(|t| t.get_elaborated_type().unwrap_or(t)).unwrap();
				gdrs_api::TypeSemantic::PointerToPointer
			} else {
				gdrs_api::TypeSemantic::Pointer
			}
		},
		clang::TypeKind::LValueReference => {
			t = t.get_pointee_type().map(|t| t.get_elaborated_type().unwrap_or(t)).unwrap();
			if t.get_kind() == clang::TypeKind::Pointer {
				t = t.get_pointee_type().map(|t| t.get_elaborated_type().unwrap_or(t)).unwrap();
				gdrs_api::TypeSemantic::ReferenceToPointer
			} else {
				gdrs_api::TypeSemantic::Reference
			}
		},
		clang::TypeKind::ConstantArray => {
			let size = t.get_size().unwrap();
			t = t.get_element_type().map(|t| t.get_elaborated_type().unwrap_or(t)).unwrap();
			match t.get_kind() {
				clang::TypeKind::ConstantArray => {
					let size1 = t.get_size().unwrap();
					t = t.get_element_type().map(|t| t.get_elaborated_type().unwrap_or(t)).unwrap();
					gdrs_api::TypeSemantic::ArrayOfArray(size, size1)
				},
				clang::TypeKind::Pointer => {
					t = t.get_pointee_type().map(|t| t.get_elaborated_type().unwrap_or(t)).unwrap();
					gdrs_api::TypeSemantic::ArrayOfPointer(size)
				},
				_ => gdrs_api::TypeSemantic::Array(size),
			}
		},
		_ => gdrs_api::TypeSemantic::Value,
	};

	Ok(gdrs_api::TypeRef{
		name: match t.get_kind() {
			clang::TypeKind::Auto
			| clang::TypeKind::Unexposed
			| clang::TypeKind::BlockPointer
			| clang::TypeKind::MemberPointer
			=> return Err(ParseError::Ignored),

			clang::TypeKind::Bool => gdrs_api::TypeName::Bool,
			clang::TypeKind::CharS | clang::TypeKind::SChar => gdrs_api::TypeName::Char,
			clang::TypeKind::CharU | clang::TypeKind::UChar => gdrs_api::TypeName::UChar,
			clang::TypeKind::WChar => gdrs_api::TypeName::WChar,
			clang::TypeKind::Short => gdrs_api::TypeName::Short,
			clang::TypeKind::UShort => gdrs_api::TypeName::UShort,
			clang::TypeKind::Int => gdrs_api::TypeName::Int,
			clang::TypeKind::UInt => gdrs_api::TypeName::UInt,
			clang::TypeKind::Long => gdrs_api::TypeName::Long,
			clang::TypeKind::ULong => gdrs_api::TypeName::ULong,
			clang::TypeKind::LongLong => gdrs_api::TypeName::LongLong,
			clang::TypeKind::ULongLong => gdrs_api::TypeName::ULongLong,
			clang::TypeKind::Float => gdrs_api::TypeName::Float,
			clang::TypeKind::Double => gdrs_api::TypeName::Double,

			clang::TypeKind::Void if semantic != gdrs_api::TypeSemantic::Value => gdrs_api::TypeName::Void,

			k if k == clang::TypeKind::Typedef || k == clang::TypeKind::Enum || k == clang::TypeKind::Record => {
				let mut p = t.get_declaration().unwrap();
				let mut name_path = Vec::new();
				loop {
					name_path.push(p.get_name().unwrap_or_else(|| "auto".to_string()));
					p = p.get_semantic_parent().unwrap();
					while p.get_kind() == clang::EntityKind::UnexposedDecl && p.get_name().is_none() {
						p = p.get_semantic_parent().unwrap();
					}
					if p.get_kind() == clang::EntityKind::TranslationUnit {
						break;
					}
				}

				match k {
					clang::TypeKind::Enum | clang::TypeKind::Typedef => {
						gdrs_api::TypeName::TypeName(name_path)
					},
					clang::TypeKind::Record => {
						if let Some(params) = t.get_template_argument_types().map(|vp| vp.into_iter().map(|p| parse_type(p.unwrap())).collect::<Vec<_>>()) {
							if let Some(i) = params.iter().position(|p| p.is_err()) {
								match *params[i].as_ref().unwrap_err() {
									ParseError::Unsupported => {
										let _ = writeln!(io::stderr(), "WARNING: Unsupported template param type `{:?}`", t.get_template_argument_types().unwrap()[i]);
										return Err(ParseError::Unsupported);
									},
									ParseError::Ignored => return Err(ParseError::Ignored),
								}
							}

							gdrs_api::TypeName::Class(
								name_path,
								params.into_iter().map(|p| p.unwrap()).collect()
							)
						} else {
							gdrs_api::TypeName::Class(name_path, Vec::with_capacity(0))
						}
					},
					_ => unreachable!(),
				}
			},

			k => {
				let _ = writeln!(io::stderr(), "WARNING: Unsupported type kind `{:?}`", k);
				return Err(ParseError::Unsupported);
			},
		},
		semantic: semantic,
		is_const: t.is_const_qualified(),
	})
}



fn parse_value(exp: clang::Entity) -> Option<gdrs_api::Value> {
	if let (Some(kind), Some(val)) = (exp.get_type().map(|t| t.get_kind()), exp.evaluate()) {
		match val {
			clang::EvaluationResult::Integer(i)
				if kind == clang::TypeKind::CharU
				|| kind == clang::TypeKind::UChar
				|| kind == clang::TypeKind::UShort
				|| kind == clang::TypeKind::UInt
				|| kind == clang::TypeKind::ULong
				|| kind == clang::TypeKind::ULongLong
				|| kind == clang::TypeKind::Bool
			=> Some(gdrs_api::Value::UInt(i as u64)),
			clang::EvaluationResult::Integer(i)
				if kind == clang::TypeKind::CharS
				|| kind == clang::TypeKind::SChar
				|| kind == clang::TypeKind::WChar
				|| kind == clang::TypeKind::Short
				|| kind == clang::TypeKind::Int
				|| kind == clang::TypeKind::Long
				|| kind == clang::TypeKind::LongLong
			=> Some(gdrs_api::Value::Int(i)),
			clang::EvaluationResult::Float(d) if kind == clang::TypeKind::Float => Some(gdrs_api::Value::Float(d as f32)),
			clang::EvaluationResult::Float(d) if kind == clang::TypeKind::Double => Some(gdrs_api::Value::Double(d)),
			clang::EvaluationResult::String(s) => Some(gdrs_api::Value::String(s.to_string_lossy().into_owned())),
			v => {
				let _ = writeln!(io::stderr(), "WARNING: Unsupported evaluation result `{:?}`: {:?}", v, exp);
				return None;
			},
		}
	} else {
		None
	}
}
