#![feature(proc_macro)]

extern crate serde;
#[macro_use]
extern crate serde_derive;



#[derive(Serialize, Deserialize)]
pub struct Api {
	pub classes: Vec<Class>,
	pub enums: Vec<Enum>,
	pub functions: Vec<Function>,
}



#[derive(Serialize, Deserialize)]
pub struct Class {
	pub include: String,
	pub name: String,
	pub fields: Vec<Field>,
	pub functions: Vec<Function>,
	pub enums: Vec<Enum>,
}



#[derive(Serialize, Deserialize)]
pub enum Access {
	Public,
	Protected,
	Private,
}



#[derive(Serialize, Deserialize)]
pub enum FunctionSemantic {
	Free,
	Static,
	Method,
	Virtual,
}



#[derive(Serialize, Deserialize)]
pub enum TypeSemantic {
	Value,
	Pointer,
	Reference,
}



#[derive(Serialize, Deserialize)]
pub struct Field {
	pub access: Access,
	pub name: String,
	pub ty: Type,
}



#[derive(Serialize, Deserialize)]
pub struct Function {
	pub access: Access,
	pub semantic: FunctionSemantic,
	pub return_ty: Type,
	pub name: String,
}



#[derive(Serialize, Deserialize)]
pub struct Type {
	pub is_const: bool,
	pub semantic: TypeSemantic,
	pub name: String,
	pub args: Vec<Arg>,
}



#[derive(Serialize, Deserialize)]
pub struct Arg {
	pub name: String,
	pub ty: Type,
}



#[derive(Serialize, Deserialize)]
pub struct Enum {
	pub name: String,
	pub variants: Vec<Variant>,
}



#[derive(Serialize, Deserialize)]
pub struct Variant {
	pub name: String,
	pub value: isize,
}
