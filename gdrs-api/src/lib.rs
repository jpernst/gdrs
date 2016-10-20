#![feature(proc_macro)]

extern crate serde;
#[macro_use]
extern crate serde_derive;



#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Namespace {
	pub name: String,
	pub consts: Vec<Const>,
	pub globals: Vec<Global>,
	pub enums: Vec<Enum>,
	pub aliases: Vec<TypeAlias>,
	pub functions: Vec<Function>,
	pub classes: Vec<Class>,
	pub namespaces: Vec<Namespace>,
}



#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Const {
	pub name: String,
	pub ty: TypeRef,
	pub value: Value,
}



#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Global {
	pub name: String,
	pub ty: TypeRef,
}



#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Enum {
	pub name: String,
	pub underlying: TypeRef,
	pub variants: Vec<Variant>,
}



#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Variant {
	pub name: String,
	pub value: Value,
}



#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TypeAlias {
	pub name: String,
	pub ty: TypeRef,
}



#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Class {
	pub include: String,
	pub name: String,
	pub inherits: Option<TypeName>,
	pub is_pod: bool,
	pub consts: Vec<Const>,
	pub enums: Vec<Enum>,
	pub aliases: Vec<TypeAlias>,
	pub fields: Vec<Field>,
	pub ctors: Vec<Function>,
	pub methods: Vec<Function>,
	pub virtual_dtor: bool,
	pub classes: Vec<Class>,
}



#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Access {
	Public,
	Protected,
}



#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum FunctionSemantic {
	Free,
	Static,
	Method,
	Virtual,
}



#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum TypeSemantic {
	Value,
	Pointer,
	PointerToPointer,
	Reference,
	ReferenceToPointer,
	Array(usize),
	ArrayOfPointer(usize),
}



#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Field {
	pub name: String,
	pub ty: TypeRef,
	pub access: Access,
	pub is_static: bool,
}



#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Function {
	pub name: String,
	pub params: Vec<Param>,
	pub return_ty: Option<TypeRef>,
	pub semantic: FunctionSemantic,
	pub access: Access,
	pub is_const: bool,
}



#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TypeRef {
	pub name: TypeName,
	pub semantic: TypeSemantic,
	pub is_const: bool,
}



#[derive(Clone, Debug, Serialize, Deserialize)]
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



#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Param {
	pub name: String,
	pub ty: TypeRef,
	pub default: Option<Value>,
}



#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Value {
	Int(i64),
	UInt(u64),
	Float(f32),
	Double(f64),
	String(String),
}
