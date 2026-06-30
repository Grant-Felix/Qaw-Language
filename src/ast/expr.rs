//! 表达式类型与构造器
//!
//! 包含二元/一元运算符及其包装结构、调用、字段访问、索引、切片。

use super::{Expr, ExprData, Kind};

// ============ 运算符 ============

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Neq, Lt, Le, Gt, Ge,
    And, Or,
    BitAnd, BitOr, BitXor,
    Shl, Shr,
    Concat, // ++
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg, Not, BitNot,
}

// ============ 表达式结构 ============

#[derive(Debug, Clone)]
pub struct BinaryOp {
    pub op: BinOp,
    pub lhs: Box<Expr>,
    pub rhs: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct UnaryOp {
    pub op: UnOp,
    pub operand: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct Call {
    pub func: Box<Expr>,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub struct FieldAccess {
    pub obj: Box<Expr>,
    pub field: String,
}

#[derive(Debug, Clone)]
pub struct Index {
    pub obj: Box<Expr>,
    pub index: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct Slice {
    pub obj: Box<Expr>,
    pub start: Option<Box<Expr>>,
    pub end: Option<Box<Expr>>,
    pub inclusive: bool,
}

// ============ 构造器 ============

pub fn new_binary(op: BinOp, lhs: Expr, rhs: Expr, line: u32, col: u32) -> Expr {
    Expr {
        kind: Kind::BinaryOp,
        line,
        col,
        data: ExprData::BinaryOp(BinaryOp {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }),
    }
}

pub fn new_unary(op: UnOp, operand: Expr, line: u32, col: u32) -> Expr {
    Expr {
        kind: Kind::UnaryOp,
        line,
        col,
        data: ExprData::UnaryOp(UnaryOp {
            op,
            operand: Box::new(operand),
        }),
    }
}

pub fn new_call(func: Expr, args: Vec<Expr>, line: u32, col: u32) -> Expr {
    Expr {
        kind: Kind::Call,
        line,
        col,
        data: ExprData::Call(Call { func: Box::new(func), args }),
    }
}

pub fn new_field_access(obj: Expr, field: String, line: u32, col: u32) -> Expr {
    Expr {
        kind: Kind::FieldAccess,
        line,
        col,
        data: ExprData::FieldAccess(FieldAccess {
            obj: Box::new(obj),
            field,
        }),
    }
}

pub fn new_index(obj: Expr, index: Expr, line: u32, col: u32) -> Expr {
    Expr {
        kind: Kind::Index,
        line,
        col,
        data: ExprData::Index(Index {
            obj: Box::new(obj),
            index: Box::new(index),
        }),
    }
}

pub fn new_slice(
    obj: Expr,
    start: Option<Expr>,
    end: Option<Expr>,
    inclusive: bool,
    line: u32,
    col: u32,
) -> Expr {
    Expr {
        kind: Kind::Slice,
        line,
        col,
        data: ExprData::Slice(Slice {
            obj: Box::new(obj),
            start: start.map(Box::new),
            end: end.map(Box::new),
            inclusive,
        }),
    }
}