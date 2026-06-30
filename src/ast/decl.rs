//! 声明类型与构造器（函数、结构体、枚举、程序、参数等）

use super::stmt::MatchArm;
use super::{Expr, ExprData, Kind};

// ============ 声明结构 ============

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub type_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub ret_type: Option<String>,
    pub body: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct FieldDecl {
    pub name: String,
    pub type_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<FieldDecl>,
}

#[derive(Debug, Clone)]
pub struct VariantDecl {
    pub name: String,
    pub payload: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<VariantDecl>,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Expr>,
}

// ============ 构造器 ============

pub fn new_function(
    name: String,
    params: Vec<Param>,
    ret_type: Option<String>,
    body: Expr,
    line: u32,
    col: u32,
) -> Expr {
    Expr {
        kind: Kind::Function,
        line,
        col,
        data: ExprData::Function(Function {
            name,
            params,
            ret_type,
            body: Box::new(body),
        }),
    }
}

pub fn new_struct(
    name: String,
    fields: Vec<FieldDecl>,
    line: u32,
    col: u32,
) -> Expr {
    Expr {
        kind: Kind::StructDecl,
        line,
        col,
        data: ExprData::StructDecl(StructDecl { name, fields }),
    }
}

pub fn new_enum(
    name: String,
    variants: Vec<VariantDecl>,
    line: u32,
    col: u32,
) -> Expr {
    Expr {
        kind: Kind::EnumDecl,
        line,
        col,
        data: ExprData::EnumDecl(EnumDecl { name, variants }),
    }
}

pub fn new_program(items: Vec<Expr>, line: u32, col: u32) -> Expr {
    Expr {
        kind: Kind::Program,
        line,
        col,
        data: ExprData::Program(Program { items }),
    }
}

pub fn match_arm(pattern: String, body: Expr) -> MatchArm {
    MatchArm { pattern, body: Box::new(body) }
}

pub fn field_decl(name: String, type_name: String) -> FieldDecl {
    FieldDecl { name, type_name: Some(type_name) }
}

pub fn variant_decl(name: String, payload: Option<String>) -> VariantDecl {
    VariantDecl { name, payload }
}

pub fn param(name: String, type_name: Option<String>) -> Param {
    Param { name, type_name }
}