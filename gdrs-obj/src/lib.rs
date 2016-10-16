#![feature(proc_macro)]

extern crate serde;
#[macro_use]
extern crate serde_derive;



#[derive(Serialize, Deserialize)]
pub struct Api {
	pub consts: Vec<Const>,
	pub enums: Vec<Enum>,
	pub classes: Vec<Class>,
	pub functions: Vec<Function>,
}



#[derive(Serialize, Deserialize)]
pub struct Class {
	pub include: String,
	pub name: String,
	pub consts: Vec<Const>,
	pub enums: Vec<Enum>,
	pub fields: Vec<Field>,
	pub methods: Vec<Function>,
}



#[derive(Serialize, Deserialize)]
pub enum Access {
	Public,
	Protected,
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
	Array(usize),
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
	pub return_ty: Option<Type>,
	pub name: String,
	pub args: Option<Vec<Arg>>,
}



#[derive(Serialize, Deserialize)]
pub struct Type {
	pub is_const: bool,
	pub semantic: TypeSemantic,
	pub name: Typename,
}



#[derive(Serialize, Deserialize)]
pub enum Typename {
	Void,
	Bool,
	Char,
	UChar,
	Short,
	UShort,
	Int,
	UInt,
	Long,
	ULong,
	LongLong,
	ULongLong,
	Float,
	Double,
	Class(String, Option<Vec<Type>>),
	Enum(String),
}



#[derive(Serialize, Deserialize)]
pub struct Arg {
	pub name: String,
	pub ty: Type,
}



#[derive(Serialize, Deserialize)]
pub struct Enum {
	pub name: String,
	pub underlying: Typename,
	pub variants: Vec<Const>,
}



#[derive(Serialize, Deserialize)]
pub struct Const {
	pub name: String,
	pub value: Value,
}



#[derive(Serialize, Deserialize)]
pub enum Value {
	Int(i64),
	UInt(u64),
	Float(f32),
	Double(f64),
}
