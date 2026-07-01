//! 语句类型与构造器

use super::{Expr, ExprData, Kind, TypeAnnotation};

// ============ 语句结构 ============

#[derive(Debug, Clone)]
pub struct VarDecl {
    pub name: String,
    pub type_annotation: Option<TypeAnnotation>,
    pub is_mut: bool,
    pub init: Option<Box<Expr>>,
}

#[derive(Debug, Clone)]
pub struct Assign {
    pub target: Box<Expr>,
    pub value: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct ExprStmt {
    pub expr: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub cond: Box<Expr>,
    pub then_block: Box<Expr>,
    pub else_block: Option<Box<Expr>>,
}

#[derive(Debug, Clone)]
pub struct WhileStmt {
    pub cond: Box<Expr>,
    pub body: Box<Expr>,
}

#[derive(Debug, Clone, Copy)]
pub enum ForKind {
    ForIn,
    ForRange,
}

#[derive(Debug, Clone)]
pub struct ForStmt {
    pub kind: ForKind,
    pub var_name: String,
    pub iterable: Option<Box<Expr>>,
    pub start: Option<Box<Expr>>,
    pub end: Option<Box<Expr>>,
    pub step: Option<Box<Expr>>,
    pub body: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct ReturnStmt {
    pub value: Option<Box<Expr>>,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: String,
    pub body: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct MatchStmt {
    pub scrutinee: Box<Expr>,
    pub arms: Vec<MatchArm>,
}

/// `defer expr;` —— 函数退出前按 LIFO 顺序执行的延迟表达式（A10）。
#[derive(Debug, Clone)]
pub struct DeferStmt {
    pub expr: Box<Expr>,
}

// ============ 构造器 ============

pub fn new_var_decl(
    name: String,
    type_annotation: Option<TypeAnnotation>,
    is_mut: bool,
    init: Option<Expr>,
    line: u32,
    col: u32,
) -> Expr {
    Expr {
        kind: Kind::VarDecl,
        line,
        col,
        data: ExprData::VarDecl(VarDecl {
            name,
            type_annotation,
            is_mut,
            init: init.map(Box::new),
        }),
    }
}

pub fn new_assign(target: Expr, value: Expr, line: u32, col: u32) -> Expr {
    Expr {
        kind: Kind::Assign,
        line,
        col,
        data: ExprData::Assign(Assign {
            target: Box::new(target),
            value: Box::new(value),
        }),
    }
}

pub fn new_expr_stmt(expr: Expr, line: u32, col: u32) -> Expr {
    Expr {
        kind: Kind::ExprStmt,
        line,
        col,
        data: ExprData::ExprStmt(ExprStmt { expr: Box::new(expr) }),
    }
}

pub fn new_block(stmts: Vec<Expr>, line: u32, col: u32) -> Expr {
    Expr {
        kind: Kind::Block,
        line,
        col,
        data: ExprData::Block(Block { stmts }),
    }
}

pub fn new_if(
    cond: Expr,
    then_block: Expr,
    else_block: Option<Expr>,
    line: u32,
    col: u32,
) -> Expr {
    Expr {
        kind: Kind::IfStmt,
        line,
        col,
        data: ExprData::IfStmt(IfStmt {
            cond: Box::new(cond),
            then_block: Box::new(then_block),
            else_block: else_block.map(Box::new),
        }),
    }
}

pub fn new_while(cond: Expr, body: Expr, line: u32, col: u32) -> Expr {
    Expr {
        kind: Kind::WhileStmt,
        line,
        col,
        data: ExprData::WhileStmt(WhileStmt {
            cond: Box::new(cond),
            body: Box::new(body),
        }),
    }
}

pub fn new_for_in(
    var: String,
    iterable: Expr,
    body: Expr,
    line: u32,
    col: u32,
) -> Expr {
    Expr {
        kind: Kind::ForStmt,
        line,
        col,
        data: ExprData::ForStmt(ForStmt {
            kind: ForKind::ForIn,
            var_name: var,
            iterable: Some(Box::new(iterable)),
            start: None,
            end: None,
            step: None,
            body: Box::new(body),
        }),
    }
}

pub fn new_for_range(
    var: String,
    start: Expr,
    end: Expr,
    step: Option<Expr>,
    body: Expr,
    line: u32,
    col: u32,
) -> Expr {
    Expr {
        kind: Kind::ForStmt,
        line,
        col,
        data: ExprData::ForStmt(ForStmt {
            kind: ForKind::ForRange,
            var_name: var,
            iterable: None,
            start: Some(Box::new(start)),
            end: Some(Box::new(end)),
            step: step.map(Box::new),
            body: Box::new(body),
        }),
    }
}

pub fn new_return(value: Option<Expr>, line: u32, col: u32) -> Expr {
    Expr {
        kind: Kind::ReturnStmt,
        line,
        col,
        data: ExprData::ReturnStmt(ReturnStmt { value: value.map(Box::new) }),
    }
}

pub fn new_break(line: u32, col: u32) -> Expr {
    Expr { kind: Kind::BreakStmt, line, col, data: ExprData::BreakStmt }
}

pub fn new_continue(line: u32, col: u32) -> Expr {
    Expr { kind: Kind::ContinueStmt, line, col, data: ExprData::ContinueStmt }
}

pub fn new_match(
    scrutinee: Expr,
    arms: Vec<MatchArm>,
    line: u32,
    col: u32,
) -> Expr {
    Expr {
        kind: Kind::MatchStmt,
        line,
        col,
        data: ExprData::MatchStmt(MatchStmt {
            scrutinee: Box::new(scrutinee),
            arms,
        }),
    }
}

/// 构造 `defer expr;` 节点（A10）。
pub fn new_defer(expr: Expr, line: u32, col: u32) -> Expr {
    Expr {
        kind: Kind::DeferStmt,
        line,
        col,
        data: ExprData::DeferStmt(DeferStmt { expr: Box::new(expr) }),
    }
}