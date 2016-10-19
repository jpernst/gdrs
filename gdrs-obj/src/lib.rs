#![feature(proc_macro)]

extern crate serde;
#[macro_use]
extern crate serde_derive;



#[derive(Clone, Serialize, Deserialize)]
pub struct Namespace {
	pub name: String,
	pub consts: Vec<Const>,
	pub enums: Vec<Enum>,
	pub aliases: Vec<TypeAlias>,
	pub classes: Vec<Class>,
	pub functions: Vec<Function>,
	pub namespaces: Vec<Namespace>,
}



#[derive(Clone, Serialize, Deserialize)]
pub struct Const {
	pub name: String,
	pub ty: TypeRef,
	pub value: Value,
}



#[derive(Clone, Serialize, Deserialize)]
pub struct Enum {
	pub name: String,
	pub underlying: TypeRef,
	pub variants: Vec<Variant>,
}



#[derive(Clone, Serialize, Deserialize)]
pub struct Variant {
	pub name: String,
	pub value: Value,
}



#[derive(Clone, Serialize, Deserialize)]
pub struct TypeAlias {
	pub name: String,
	pub ty: TypeRef,
}



#[derive(Clone, Serialize, Deserialize)]
pub struct Class {
	pub include: String,
	pub name: String,
	pub consts: Vec<Const>,
	pub enums: Vec<Enum>,
	pub aliases: Vec<TypeAlias>,
	pub fields: Vec<Field>,
	pub methods: Vec<Function>,
}



#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Access {
	Public,
	Protected,
}



#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FunctionSemantic {
	Free,
	Static,
	Method,
	Virtual,
}



#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeSemantic {
	Value,
	Pointer,
	DoublePointer,
	Reference,
	Array(usize),
}



#[derive(Clone, Serialize, Deserialize)]
pub struct Field {
	pub name: String,
	pub ty: TypeRef,
	pub access: Access,
	pub is_static: bool,
}



#[derive(Clone, Serialize, Deserialize)]
pub struct Function {
	pub name: String,
	pub params: Vec<Param>,
	pub return_ty: Option<TypeRef>,
	pub semantic: FunctionSemantic,
	pub access: Access,
	pub is_const: bool,
}



#[derive(Clone, Serialize, Deserialize)]
pub struct TypeRef {
	pub name: TypeName,
	pub semantic: TypeSemantic,
	pub is_const: bool,
}



#[derive(Clone, Serialize, Deserialize)]
pub enum TypeName {
	Void,
	Bool,
	Char,
	UChar,
	WChar,
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
	TypeName(Vec<String>),
	Class(Vec<String>, Vec<TypeRef>),
}



#[derive(Clone, Serialize, Deserialize)]
pub struct Param {
	pub name: String,
	pub ty: TypeRef,
	pub default: Option<Value>,
}



#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
	Int(i64),
	UInt(u64),
	Float(f32),
	Double(f64),
	String(String),
}
