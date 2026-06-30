//! 字面量类型与构造器

use super::{Expr, ExprData, Kind};

/// 字符串字面量（含 ${...} 插值片段）
#[derive(Debug, Clone)]
pub struct StringLit {
    pub parts: Vec<InterpPart>,
}

/// 字符串插值片段
#[derive(Debug, Clone)]
pub enum InterpPart {
    Text(String),
    Expr(Box<Expr>),
}

// ============ 构造器 ============

pub fn new_int_lit(val: i64, line: u32, col: u32) -> Expr {
    Expr { kind: Kind::IntLit, line, col, data: ExprData::IntLit(val) }
}

pub fn new_float_lit(val: f64, line: u32, col: u32) -> Expr {
    Expr { kind: Kind::FloatLit, line, col, data: ExprData::FloatLit(val) }
}

pub fn new_string_lit(s: StringLit, line: u32, col: u32) -> Expr {
    Expr { kind: Kind::StringLit, line, col, data: ExprData::StringLit(s) }
}

pub fn new_bool_lit(val: bool, line: u32, col: u32) -> Expr {
    Expr { kind: Kind::BoolLit, line, col, data: ExprData::BoolLit(val) }
}

pub fn new_char_lit(val: i32, line: u32, col: u32) -> Expr {
    Expr { kind: Kind::CharLit, line, col, data: ExprData::CharLit(val) }
}

pub fn new_ident(name: String, line: u32, col: u32) -> Expr {
    Expr { kind: Kind::Ident, line, col, data: ExprData::Ident(name) }
}

pub fn new_array_lit(items: Vec<Expr>, line: u32, col: u32) -> Expr {
    Expr { kind: Kind::ArrayLit, line, col, data: ExprData::ArrayLit(items) }
}

pub fn interp_text(text: &str) -> InterpPart {
    InterpPart::Text(text.to_string())
}

pub fn interp_expr(expr: Expr) -> InterpPart {
    InterpPart::Expr(Box::new(expr))
}

pub fn string_lit_simple(text: &str) -> StringLit {
    StringLit {
        parts: vec![InterpPart::Text(text.to_string())],
    }
}
