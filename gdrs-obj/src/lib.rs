#![feature(proc_macro)]

extern crate serde;
#[macro_use]
extern crate serde_derive;



#[derive(Clone, Serialize, Deserialize)]
pub struct Api {
	pub consts: Vec<Const>,
	pub enums: Vec<Enum>,
	pub aliases: Vec<Alias>,
	pub classes: Vec<Class>,
	pub functions: Vec<Function>,
}



#[derive(Clone, Serialize, Deserialize)]
pub struct Const {
	pub ty: Type,
	pub name: String,
	pub value: Value,
}



#[derive(Clone, Serialize, Deserialize)]
pub struct Enum {
	pub name: String,
	pub underlying: Type,
	pub variants: Vec<Variant>,
}



#[derive(Clone, Serialize, Deserialize)]
pub struct Variant {
	pub name: String,
	pub value: Value,
}



#[derive(Clone, Serialize, Deserialize)]
pub struct Alias {
	pub name: String,
	pub ty: Type,
}



#[derive(Clone, Serialize, Deserialize)]
pub struct Class {
	pub include: String,
	pub name: String,
	pub aliases: Vec<Alias>,
	pub consts: Vec<Const>,
	pub enums: Vec<Enum>,
	pub fields: Vec<Field>,
	pub methods: Vec<Function>,
}



#[derive(Clone, Serialize, Deserialize)]
pub enum Access {
	Public,
	Protected,
}



#[derive(Clone, Serialize, Deserialize)]
pub enum FunctionSemantic {
	Free,
	Static,
	Method,
	Virtual,
}



#[derive(Clone, Serialize, Deserialize)]
pub enum TypeSemantic {
	Value,
	Pointer,
	Reference,
	Array(usize),
}



#[derive(Clone, Serialize, Deserialize)]
pub struct Field {
	pub access: Access,
	pub is_static: bool,
	pub ty: Type,
	pub name: String,
}



#[derive(Clone, Serialize, Deserialize)]
pub struct Function {
	pub access: Access,
	pub semantic: FunctionSemantic,
	pub return_ty: Option<Type>,
	pub name: String,
	pub params: Option<Vec<Param>>,
	pub is_const: bool,
}



#[derive(Clone, Serialize, Deserialize)]
pub struct Type {
	pub is_const: bool,
	pub semantic: TypeSemantic,
	pub name: Typename,
}



#[derive(Clone, Serialize, Deserialize)]
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
	Enum(String),
	Class(String, Option<Vec<Type>>),
}



#[derive(Clone, Serialize, Deserialize)]
pub struct Param {
	pub ty: Type,
	pub name: String,
	pub default: Option<Value>,
}



#[derive(Clone, Serialize, Deserialize)]
pub enum Value {
	Int(i64),
	UInt(u64),
	Float(f32),
	Double(f64),
	String(String),
}
