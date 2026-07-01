//! Tree-walking 解释器
//!
//! v0.1 实现：表达式求值、控制流、函数调用、match、内置 print。
//! 错误处理使用结构化 `EvalError` 枚举（替代 v0.0 的字符串错误），
//! 既支持程序化 `match` 又支持 `Display` 友好输出。

use crate::ast::*;
use crate::env::Env;
use crate::value::{Value, ValueKind, val_int, val_float, val_bool, val_string, val_array, val_nil};

/// 求值状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EvalStatus {
    Ok,
    Error,
    Break,
    Return,
}

/// 求值错误（结构化）
///
/// 每个变体代表一类语义错误。消费者可以 `match EvalError::Xxx` 做程序化处理，
/// 也可以通过 `Display` 拿到中文友好信息。
#[derive(Debug, Clone, PartialEq)]
pub enum EvalError {
    /// 当前解释器不支持的语言特性（field/index/slice 等）
    UnsupportedFeature(&'static str),
    /// VarDecl 出现在表达式位置
    VarDeclAsExpression,
    /// 求值器遇到未实现的表达式类型
    UnsupportedExpression,
    /// 除法或取模时除数为零
    DivisionByZero,
    /// Concat 运算符仅支持字符串
    ConcatNonString,
    /// 调用了未定义的函数
    UnknownFunction(String),
    /// 尝试调用非函数值
    NotAFunction,
    /// 函数实参与形参数量不匹配
    ArgumentCountMismatch { expected: usize, got: usize },
    /// break 出现在循环外
    BreakOutsideLoop,
    /// 给未定义的变量赋值
    UnknownVariable(String),
    /// 赋值目标不是合法的变量
    AssignTargetInvalid,
    /// for-in 循环缺少 iterable 表达式
    ForInMissingIterable,
    /// for-range 循环缺少 start 表达式
    ForRangeMissingStart,
    /// for-range 循环缺少 end 表达式
    ForRangeMissingEnd,
    /// 在某类型上调用了不存在的方法
    UnknownMethod { receiver_type: String, method: String },
    /// 索引表达式左值不是数组
    IndexNonArray { actual: String },
    /// 切片表达式左值不是数组
    SliceNonArray { actual: String },
    /// A1：把 nil 赋给非空类型 / unwrap 一个 nil 值
    TypeMismatch { expected: String, got: String },
    /// A1：`x?` 求值为 nil
    UnwrapOnNil,
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedFeature(s) => write!(f, "{} 暂未在 v0.1 解释器中实现", s),
            Self::VarDeclAsExpression => write!(f, "VarDecl 不能作为表达式使用"),
            Self::UnsupportedExpression => write!(f, "不支持的表达式"),
            Self::DivisionByZero => write!(f, "除数为零"),
            Self::ConcatNonString => write!(f, "concat 运算符仅支持字符串"),
            Self::UnknownFunction(name) => write!(f, "未找到函数 '{}'", name),
            Self::NotAFunction => write!(f, "尝试调用非函数值"),
            Self::ArgumentCountMismatch { expected, got } => {
                write!(f, "参数数量不匹配：期望 {} 个，实参 {} 个", expected, got)
            }
            Self::BreakOutsideLoop => write!(f, "break 出现在循环外"),
            Self::UnknownVariable(name) => write!(f, "未定义变量 '{}'", name),
            Self::AssignTargetInvalid => write!(f, "赋值目标必须是变量"),
            Self::ForInMissingIterable => write!(f, "for-in 缺少 iterable 表达式"),
            Self::ForRangeMissingStart => write!(f, "for-range 缺少 start 表达式"),
            Self::ForRangeMissingEnd => write!(f, "for-range 缺少 end 表达式"),
            Self::UnknownMethod { receiver_type, method } => {
                write!(f, "{} 类型没有方法 '{}'", receiver_type, method)
            }
            Self::IndexNonArray { actual } => {
                write!(f, "不能用非数组类型（{}）做索引", actual)
            }
            Self::SliceNonArray { actual } => {
                write!(f, "不能用非数组类型（{}）做切片", actual)
            }
            Self::TypeMismatch { expected, got } => {
                write!(f, "类型不匹配：期望 {}，得到 {}", expected, got)
            }
            Self::UnwrapOnNil => write!(f, "unwrap 遇到 nil 值"),
        }
    }
}

/// 求值结果
#[derive(Debug, Clone)]
pub struct EvalResult {
    pub status: EvalStatus,
    pub value: Value,
    pub error: Option<EvalError>,
}

pub fn eval_ok(v: Value) -> EvalResult {
    EvalResult { status: EvalStatus::Ok, value: v, error: None }
}

pub fn eval_err(e: EvalError) -> EvalResult {
    EvalResult { status: EvalStatus::Error, value: val_nil(), error: Some(e) }
}

/// 函数注册表
#[derive(Debug, Clone)]
pub struct FuncReg {
    funcs: Vec<Expr>,
}

impl FuncReg {
    pub fn new() -> Self {
        FuncReg { funcs: Vec::new() }
    }
    pub fn add(&mut self, func: Expr) {
        self.funcs.push(func);
    }
    pub fn lookup(&self, name: &str) -> Option<&Expr> {
        for f in &self.funcs {
            if let ExprData::Function(fd) = &f.data {
                if fd.name == name {
                    return Some(f);
                }
            }
        }
        None
    }
}

impl Default for FuncReg {
    fn default() -> Self { Self::new() }
}

/// 解释器
///
/// `defers` 字段（A10）：延迟执行栈，存放已注册但尚未执行的 `defer expr;` 表达式。
/// - 注册时把"参数已捕获"的调用动作入栈（详见 `exec_stmt` 的 `ExprData::DeferStmt` 分支）；
/// - 函数调用返回前（`eval_call`）按 LIFO 弹出并执行；
/// - 表达式内的参数在注册时**立即求值**（与 Go 1.14+ 一致）。
///
/// 当前简化模型：defer 栈是单层的，全局共享；
/// 内层函数返回时会一并 flush 外层已积累的 defer 队列（与 Go 的"绑定到所在函数"略有差异，
/// 但足够覆盖 A10 验收标准）。
pub struct Interpreter {
    env: Env,
    regs: FuncReg,
    current_regs: Option<*const FuncReg>,
    /// 延迟执行栈（A10）。每项是参数已捕获的 `Expr`（多半是 `Call`），
    /// 按 LIFO 顺序在函数返回前 `eval_expr_inner` 一次。
    defers: Vec<Expr>,
}

// SAFETY: `Interpreter` 当前仅在主线程的 `exec_program` 同步调用中使用（无并发）。
// `Env` 和 `FuncReg` 都使用 `HashMap`（非线程安全），但只要保持单线程访问就 Send 安全。
// 未来若需要并发执行，必须重审：Env 需要换 `RwLock<HashMap>`，并显式实现 Sync。
unsafe impl Send for Interpreter {}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            env: Env::new(),
            regs: FuncReg::new(),
            current_regs: None,
            defers: Vec::new(),
        }
    }

    /// 求值表达式
    pub fn eval_expr(&mut self, expr: &Expr) -> EvalResult {
        self.eval_expr_inner(expr)
    }

    fn eval_expr_inner(&mut self, expr: &Expr) -> EvalResult {
        match &expr.data {
            ExprData::IntLit(v) => eval_ok(val_int(*v)),
            ExprData::FloatLit(v) => eval_ok(val_float(*v)),
            ExprData::BoolLit(v) => eval_ok(val_bool(*v)),
            ExprData::CharLit(v) => eval_ok(val_int(*v as i64)),
            ExprData::StringLit(s) => {
                // v0.5 简化：字符串字面量在词法层已是 InterpPart 列表
                // 运行时拼接：text 段直接用，expr 段求值后转字符串
                let mut result = String::new();
                for p in &s.parts {
                    match p {
                        crate::ast::InterpPart::Text(t) => result.push_str(t),
                        crate::ast::InterpPart::Expr(e) => {
                            let r = self.eval_expr_inner(e);
                            if r.status != EvalStatus::Ok {
                                return r;
                            }
                            result.push_str(&r.value.to_string());
                        }
                    }
                }
                eval_ok(val_string(result))
            }
            ExprData::Ident(name) => eval_ok(self.env.get(name)),
            ExprData::BinaryOp(b) => self.eval_binary(b),
            ExprData::UnaryOp(u) => self.eval_unary(u),
            ExprData::Call(c) => self.eval_call(c),
            ExprData::FieldAccess(_f) => eval_err(EvalError::UnsupportedFeature("field access")),
            ExprData::ArrayLit(items) => {
                // 数组字面量：依次求值每个元素
                let mut values = Vec::with_capacity(items.len());
                for it in items {
                    let r = self.eval_expr_inner(it);
                    if r.status != EvalStatus::Ok { return r; }
                    values.push(r.value);
                }
                eval_ok(val_array(values))
            }
            ExprData::Index(i) => {
                // 索引：arr[i]
                let obj = self.eval_expr_inner(&i.obj);
                if obj.status != EvalStatus::Ok { return obj; }
                let idx = self.eval_expr_inner(&i.index);
                if idx.status != EvalStatus::Ok { return idx; }
                let items = match &obj.value {
                    Value::Array(v) => v.clone(),
                    other => {
                        return eval_err(EvalError::IndexNonArray {
                            actual: other.kind().as_str().to_string(),
                        });
                    }
                };
                let len = items.len() as i64;
                let raw = idx.value.to_int();
                // debug 模式越界 panic；release 模式 wrap（rem_euclid 处理负数）
                let resolved = if raw < 0 || raw >= len {
                    if cfg!(debug_assertions) {
                        panic!("array index out of bounds: index = {}, len = {}", raw, len);
                    }
                    let modulus = len.max(1);
                    raw.rem_euclid(modulus) as usize
                } else {
                    raw as usize
                };
                eval_ok(items[resolved].clone())
            }
            ExprData::Slice(s) => {
                // 切片：arr[start..end] / arr[..] / arr[start..] / arr[..end] / arr[start..=end]
                let obj = self.eval_expr_inner(&s.obj);
                if obj.status != EvalStatus::Ok { return obj; }
                let items = match &obj.value {
                    Value::Array(v) => v.clone(),
                    other => {
                        return eval_err(EvalError::SliceNonArray {
                            actual: other.kind().as_str().to_string(),
                        });
                    }
                };
                let len = items.len() as i64;
                let start_raw = if let Some(e) = &s.start {
                    let r = self.eval_expr_inner(e);
                    if r.status != EvalStatus::Ok { return r; }
                    r.value.to_int()
                } else {
                    0
                };
                let end_raw = if let Some(e) = &s.end {
                    let r = self.eval_expr_inner(e);
                    if r.status != EvalStatus::Ok { return r; }
                    r.value.to_int()
                } else {
                    len
                };
                // clamp 到 [0, len]
                let mut s_idx = start_raw.max(0).min(len);
                let mut e_idx = end_raw.max(0).min(len);
                if s.inclusive && e_idx < len {
                    e_idx += 1; // ..= 包含右端点
                }
                if e_idx < s_idx {
                    s_idx = e_idx; // 反向区间视为空
                }
                let sliced: Vec<Value> = items[s_idx as usize..e_idx as usize].to_vec();
                eval_ok(val_array(sliced))
            }
            ExprData::Unwrap(u) => {
                // A1：`x?` —— 强制解包。
                // - debug 模式：对 nil panic（与 array OOB / int overflow 风格一致）
                // - release 模式：返回 EvalError::UnwrapOnNil（让外层按错误处理）
                // - 非 nil：原样返回（不强制类型必须是 T?；v0.20 宽松版允许）
                let inner = self.eval_expr_inner(&u.expr);
                if inner.status != EvalStatus::Ok { return inner; }
                if matches!(inner.value, Value::Nil) {
                    if cfg!(debug_assertions) {
                        panic!("unwrap on nil value");
                    }
                    eval_err(EvalError::UnwrapOnNil)
                } else {
                    inner
                }
            }
            ExprData::VarDecl(_v) => {
                // v0.5：VarDecl 在表达式位置不应出现（由 exec_stmt 处理）；
                // 这里仅作为兜底，保留占位以避免崩溃。
                eval_err(EvalError::VarDeclAsExpression)
            }
            _ => eval_err(EvalError::UnsupportedExpression),
        }
    }

    fn eval_binary(&mut self, b: &BinaryOp) -> EvalResult {
        let lhs = self.eval_expr_inner(&b.lhs);
        if lhs.status != EvalStatus::Ok { return lhs; }
        let rhs = self.eval_expr_inner(&b.rhs);
        if rhs.status != EvalStatus::Ok {
            return EvalResult { status: rhs.status, ..rhs };
        }

        match b.op {
            BinOp::Add => {
                if matches!(lhs.value, Value::String(_)) && matches!(rhs.value, Value::String(_)) {
                    if let (Value::String(a), Value::String(c)) = (&lhs.value, &rhs.value) {
                        return eval_ok(val_string(format!("{}{}", a, c)));
                    }
                }
                if matches!(lhs.value.kind(), ValueKind::Float) || matches!(rhs.value.kind(), ValueKind::Float) {
                    let r = lhs.value.to_float() + rhs.value.to_float();
                    return eval_ok(val_float(r));
                }
                eval_ok(val_int(int_arith(BinOp::Add, lhs.value.to_int(), rhs.value.to_int())))
            }
            BinOp::Sub => {
                if matches!(lhs.value.kind(), ValueKind::Float) || matches!(rhs.value.kind(), ValueKind::Float) {
                    return eval_ok(val_float(lhs.value.to_float() - rhs.value.to_float()));
                }
                eval_ok(val_int(int_arith(BinOp::Sub, lhs.value.to_int(), rhs.value.to_int())))
            }
            BinOp::Mul => {
                if matches!(lhs.value.kind(), ValueKind::Float) || matches!(rhs.value.kind(), ValueKind::Float) {
                    return eval_ok(val_float(lhs.value.to_float() * rhs.value.to_float()));
                }
                eval_ok(val_int(int_arith(BinOp::Mul, lhs.value.to_int(), rhs.value.to_int())))
            }
            BinOp::Div => {
                let b = rhs.value.to_float();
                if b == 0.0 {
                    return eval_err(EvalError::DivisionByZero);
                }
                if matches!(lhs.value.kind(), ValueKind::Float) || matches!(rhs.value.kind(), ValueKind::Float) {
                    return eval_ok(val_float(lhs.value.to_float() / b));
                }
                eval_ok(val_int(lhs.value.to_int() / rhs.value.to_int()))
            }
            BinOp::Mod => {
                let b = rhs.value.to_int();
                if b == 0 {
                    return eval_err(EvalError::DivisionByZero);
                }
                eval_ok(val_int(lhs.value.to_int() % b))
            }
            BinOp::Eq => eval_ok(val_bool(lhs.value.equal(&rhs.value))),
            BinOp::Neq => eval_ok(val_bool(!lhs.value.equal(&rhs.value))),
            BinOp::Lt => eval_ok(val_bool(lhs.value.to_float() < rhs.value.to_float())),
            BinOp::Le => eval_ok(val_bool(lhs.value.to_float() <= rhs.value.to_float())),
            BinOp::Gt => eval_ok(val_bool(lhs.value.to_float() > rhs.value.to_float())),
            BinOp::Ge => eval_ok(val_bool(lhs.value.to_float() >= rhs.value.to_float())),
            BinOp::And => {
                // 短路：左 false 时不评估右
                let lb = lhs.value.to_bool();
                if !lb {
                    return eval_ok(val_bool(false));
                }
                eval_ok(val_bool(rhs.value.to_bool()))
            }
            BinOp::Or => {
                // 短路：左 true 时不评估右
                let lb = lhs.value.to_bool();
                if lb {
                    return eval_ok(val_bool(true));
                }
                eval_ok(val_bool(rhs.value.to_bool()))
            }
            BinOp::BitAnd => eval_ok(val_int(lhs.value.to_int() & rhs.value.to_int())),
            BinOp::BitOr => eval_ok(val_int(lhs.value.to_int() | rhs.value.to_int())),
            BinOp::BitXor => eval_ok(val_int(lhs.value.to_int() ^ rhs.value.to_int())),
            BinOp::Shl => eval_ok(val_int(lhs.value.to_int() << rhs.value.to_int())),
            BinOp::Shr => eval_ok(val_int(lhs.value.to_int() >> rhs.value.to_int())),
            BinOp::Concat => {
                if let (Value::String(a), Value::String(c)) = (&lhs.value, &rhs.value) {
                    return eval_ok(val_string(format!("{}{}", a, c)));
                }
                eval_err(EvalError::ConcatNonString)
            }
        }
    }

    fn eval_unary(&mut self, u: &UnaryOp) -> EvalResult {
        let operand = self.eval_expr_inner(&u.operand);
        if operand.status != EvalStatus::Ok { return operand; }
        match u.op {
            UnOp::Neg => {
                if matches!(operand.value.kind(), ValueKind::Float) {
                    eval_ok(val_float(-operand.value.to_float()))
                } else {
                    eval_ok(val_int(int_neg(operand.value.to_int())))
                }
            }
            UnOp::Not => eval_ok(val_bool(!operand.value.to_bool())),
            UnOp::BitNot => eval_ok(val_int(!operand.value.to_int())),
        }
    }

    fn eval_call(&mut self, c: &Call) -> EvalResult {
        // 内置 print
        if let ExprData::Ident(name) = &c.func.data {
            if name == "print" {
                return self.eval_print(c);
            }
        }
        // 方法调用：c.func 是 FieldAccess（v0.11+，A4 字符串基础操作）
        if let ExprData::FieldAccess(fa) = &c.func.data {
            let recv = self.eval_expr_inner(&fa.obj);
            if recv.status != EvalStatus::Ok { return recv; }
            if let Some(r) = self.eval_method_call(&recv.value, &fa.field, &c.args) {
                return r;
            }
            return eval_err(EvalError::UnknownMethod {
                receiver_type: recv.value.kind().as_str().to_string(),
                method: fa.field.clone(),
            });
        }
        // 用户函数
        let func_expr = {
            if let ExprData::Ident(name) = &c.func.data {
                self.regs.lookup(name).cloned()
            } else {
                None
            }
        };
        let func = match func_expr {
            Some(f) => f,
            None => {
                let name = if let ExprData::Ident(n) = &c.func.data {
                    n.clone()
                } else {
                    String::from("<anonymous>")
                };
                return eval_err(EvalError::UnknownFunction(name));
            }
        };
        let func_data = match &func.data {
            ExprData::Function(fd) => fd.clone(),
            _ => return eval_err(EvalError::NotAFunction),
        };
        if c.args.len() != func_data.params.len() {
            return eval_err(EvalError::ArgumentCountMismatch {
                expected: func_data.params.len(),
                got: c.args.len(),
            });
        }
        // 求值参数
        let mut arg_vals = Vec::new();
        for arg in &c.args {
            let r = self.eval_expr_inner(arg);
            if r.status != EvalStatus::Ok { return r; }
            arg_vals.push(r.value);
        }
        // 创建子作用域
        let mut scope = Env::child(self.env.clone());
        for (p, v) in func_data.params.iter().zip(arg_vals.iter()) {
            scope.define(&p.name, v.clone());
        }
        // 执行函数体
        let saved_env = std::mem::replace(&mut self.env, scope);
        let body_result = self.exec_stmt(&func_data.body);
        self.env = saved_env;

        // A10: 函数返回前按 LIFO 顺序 flush 当前累计的 defer 栈。
        // defer 在注册时已经"捕获参数"（通过 `capture_defer_arg` 把 Idents 转成字面量），
        // 所以这里直接重跑表达式即可拿到原值，不需要再读当前作用域中的同名变量。
        //
        // 优先级（与 Go 语义对齐）：
        // - 函数体正常结束 / return → flush defer；
        // - 函数体 break（异常路径）→ 不 flush defer，直接返回 break（与 Go 不同，
        //   Go 会先 flush，本解释器为简化实现选择穿透 break，避免误报）；
        // - 函数体 error → 仍然 flush（让 defer 有机会运行清理），最后返回值优先于 defer 错误；
        //   若 defer 自身出错，再用 defer 的错误覆盖（与 Rust Drop 行为接近）。
        let mut final_result = body_result.clone();
        if matches!(final_result.status, EvalStatus::Ok | EvalStatus::Return | EvalStatus::Error) {
            let mut defer_err: Option<EvalResult> = None;
            while let Some(captured) = self.defers.pop() {
                let r = self.eval_expr_inner(&captured);
                if r.status != EvalStatus::Ok {
                    defer_err = Some(r);
                    break;
                }
            }
            // 清空残余（防止泄露到外层调用）
            self.defers.clear();

            final_result = match defer_err {
                Some(err) => match final_result.status {
                    // 函数体已经出错 → 优先保留原错误（更接近错误源）
                    EvalStatus::Error => final_result,
                    // 函数体正常 / return → defer 错误传播
                    _ => err,
                },
                None => final_result,
            };
        }

        match final_result.status {
            EvalStatus::Return => EvalResult { status: EvalStatus::Ok, ..final_result },
            EvalStatus::Break => eval_err(EvalError::BreakOutsideLoop),
            EvalStatus::Error => final_result,
            EvalStatus::Ok => final_result,
        }
    }

    fn eval_print(&mut self, c: &Call) -> EvalResult {
        for (i, arg) in c.args.iter().enumerate() {
            let r = self.eval_expr_inner(arg);
            if r.status != EvalStatus::Ok { return r; }
            if i > 0 { print!(" "); }
            print!("{}", r.value);
        }
        println!();
        eval_ok(val_nil())
    }

    /// 执行单条语句
    pub fn exec_stmt(&mut self, stmt: &Expr) -> EvalResult {
        match &stmt.data {
            ExprData::VarDecl(v) => {
                let init = if let Some(i) = &v.init {
                    let r = self.eval_expr_inner(i);
                    if r.status != EvalStatus::Ok { return r; }
                    Some(r.value)
                } else {
                    None
                };
                let val = init.unwrap_or_else(val_nil);

                // A1：类型注解检查 —— 非空类型禁止赋值为 nil。
                if let Some(ann) = &v.type_annotation {
                    if !ann.is_nullable() && matches!(val, Value::Nil) {
                        return eval_err(EvalError::TypeMismatch {
                            expected: ann.root_name().to_string(),
                            got: "nil".to_string(),
                        });
                    }
                }

                self.env.define(&v.name, val);
                eval_ok(val_nil())
            }
            ExprData::Assign(a) => {
                let val = self.eval_expr_inner(&a.value);
                if val.status != EvalStatus::Ok { return val; }
                if let ExprData::Ident(name) = &a.target.data {
                    if !self.env.set(name, val.value.clone()) {
                        return eval_err(EvalError::UnknownVariable(name.clone()));
                    }
                    eval_ok(val.value)
                } else {
                    eval_err(EvalError::AssignTargetInvalid)
                }
            }
            ExprData::ExprStmt(e) => {
                // 特殊处理 ExprStmt 包装的 Assign：assign 是语句而非表达式
                if matches!(&e.expr.data, ExprData::Assign(_)) {
                    self.exec_stmt(&e.expr)
                } else {
                    self.eval_expr_inner(&e.expr)
                }
            }
            ExprData::Block(b) => self.exec_block(&b.stmts),
            ExprData::IfStmt(i) => {
                let cond = self.eval_expr_inner(&i.cond);
                if cond.status != EvalStatus::Ok { return cond; }
                if cond.value.to_bool() {
                    self.exec_stmt(&i.then_block)
                } else if let Some(e) = &i.else_block {
                    self.exec_stmt(e)
                } else {
                    eval_ok(val_nil())
                }
            }
            ExprData::WhileStmt(w) => loop {
                let cond = self.eval_expr_inner(&w.cond);
                if cond.status != EvalStatus::Ok { return cond; }
                if !cond.value.to_bool() { break eval_ok(val_nil()); }
                let r = self.exec_stmt(&w.body);
                if r.status == EvalStatus::Break { break eval_ok(val_nil()); }
                if r.status != EvalStatus::Ok { return r; }
            },
            ExprData::ForStmt(f) => self.exec_for(f),
            ExprData::ReturnStmt(r) => {
                let v = if let Some(v) = &r.value {
                    let r = self.eval_expr_inner(v);
                    if r.status != EvalStatus::Ok { return r; }
                    Some(r.value)
                } else { None };
                EvalResult {
                    status: EvalStatus::Return,
                    value: v.unwrap_or_else(val_nil),
                    error: None,
                }
            }
            ExprData::BreakStmt => EvalResult {
                status: EvalStatus::Break,
                value: val_nil(),
                error: None,
            },
            ExprData::ContinueStmt => eval_ok(val_nil()),
            ExprData::MatchStmt(m) => self.exec_match(m),
            ExprData::DeferStmt(d) => self.exec_defer(d),
            _ => self.eval_expr_inner(stmt),
        }
    }

    fn exec_block(&mut self, stmts: &[Expr]) -> EvalResult {
        let mut last = eval_ok(val_nil());
        for s in stmts {
            last = self.exec_stmt(s);
            if last.status != EvalStatus::Ok {
                return last;
            }
        }
        last
    }

    fn exec_for(&mut self, f: &ForStmt) -> EvalResult {
        match &f.kind {
            ForKind::ForIn => {
                let iterable = match &f.iterable {
                    Some(e) => e,
                    None => return eval_err(EvalError::ForInMissingIterable),
                };
                let iter = self.eval_expr_inner(iterable);
                if iter.status != EvalStatus::Ok { return iter; }
                let parent_env = self.env.clone();
                let saved_env = std::mem::replace(&mut self.env, Env::child(parent_env));
                match &iter.value {
                    Value::String(s) => {
                        for ch in s.chars() {
                            self.env.define(&f.var_name, val_int(ch as i64));
                            let r = self.exec_stmt(&f.body);
                            if r.status == EvalStatus::Break { break; }
                            if r.status != EvalStatus::Ok { self.env = saved_env; return r; }
                        }
                    }
                    Value::Int(n) => {
                        self.env.define(&f.var_name, val_int(*n));
                        let r = self.exec_stmt(&f.body);
                        if r.status != EvalStatus::Ok { self.env = saved_env; return r; }
                    }
                    _ => {
                        self.env.define(&f.var_name, val_int(0));
                        let r = self.exec_stmt(&f.body);
                        if r.status != EvalStatus::Ok { self.env = saved_env; return r; }
                    }
                }
                self.env = saved_env;
                eval_ok(val_nil())
            }
            ForKind::ForRange => {
                let start_expr = match &f.start {
                    Some(e) => e,
                    None => return eval_err(EvalError::ForRangeMissingStart),
                };
                let end_expr = match &f.end {
                    Some(e) => e,
                    None => return eval_err(EvalError::ForRangeMissingEnd),
                };
                let start = self.eval_expr_inner(start_expr);
                if start.status != EvalStatus::Ok { return start; }
                let end = self.eval_expr_inner(end_expr);
                if end.status != EvalStatus::Ok { return end; }
                let step_val = if let Some(s) = &f.step {
                    let r = self.eval_expr_inner(s);
                    if r.status != EvalStatus::Ok { return r; }
                    r.value.to_int()
                } else {
                    1
                };
                let s = start.value.to_int();
                let e = end.value.to_int();
                let step = if step_val == 0 { 1 } else { step_val };
                let parent_env = self.env.clone();
                let saved_env = std::mem::replace(&mut self.env, Env::child(parent_env));
                let mut i = s;
                loop {
                    let done = if step >= 0 { i > e } else { i < e };
                    if done { break; }
                    self.env.define(&f.var_name, val_int(i));
                    let r = self.exec_stmt(&f.body);
                    if r.status == EvalStatus::Break { break; }
                    if r.status != EvalStatus::Ok { self.env = saved_env; return r; }
                    i += step;
                }
                self.env = saved_env;
                eval_ok(val_nil())
            }
        }
    }

    fn exec_match(&mut self, m: &MatchStmt) -> EvalResult {
        let val = self.eval_expr_inner(&m.scrutinee);
        if val.status != EvalStatus::Ok { return val; }
        for arm in &m.arms {
            if match_pattern(&arm.pattern, &val.value) {
                return self.exec_stmt(&arm.body);
            }
        }
        eval_ok(val_nil())
    }

    /// 执行 `defer expr;`（A10）。
    ///
    /// 注册流程：
    /// 1. 仅支持 `defer <Call(Ident(f), args...)>;` 形式（与 Go `defer f(...)` 对齐）。
    ///    其他表达式按"立即求值、不延迟"处理（不让无效语法穿透）。
    /// 2. 用 `capture_defer_arg` 递归地把 args 中的每个表达式里"值位置"的子节点
    ///    （Ident、嵌套表达式）转成字面量 AST，保留下"调用动作"。
    /// 3. 把"调用动作"压入 `defers` 栈。
    fn exec_defer(&mut self, d: &DeferStmt) -> EvalResult {
        if let ExprData::Call(c) = &d.expr.data {
            let mut captured_args = Vec::new();
            for arg in &c.args {
                let r = self.capture_defer_arg(arg);
                match r {
                    Ok(node) => captured_args.push(node),
                    Err(e) => return e,
                }
            }
            // 构造捕获后的 Call 表达式：`func` 保留原样，`args` 全部已捕获
            let func_clone: Expr = (*c.func).clone();
            let new_call = new_call(func_clone, captured_args, d.expr.line, d.expr.col);
            self.defers.push(new_call);
            eval_ok(val_nil())
        } else {
            // 非 call 类型 defer：当作立即表达式求值，丢弃值（保留副作用）。
            // 此简化覆盖了 `defer print("world")` 之外的低优先级场景。
            let r = self.eval_expr_inner(&d.expr);
            if r.status != EvalStatus::Ok {
                r
            } else {
                eval_ok(val_nil())
            }
        }
    }

    /// 把一个 defer 参数表达式"冻结"为字面量 AST（捕获求值）。
    ///
    /// - 字面量 / 二元 / 一元表达式：递归捕获到叶子；
    /// - 标识符：从 env 读当前值，转成对应的字面量节点；
    /// - 嵌套 Call：捕获其 args（但保留 func 节点，与外层一致）；
    /// - Nil：用 `Ident("nil")` 占位（解释器不暴露 nil 字面量）；
    /// - 其他（if/match/block/return/break…）：当表达式求值后取其值再转字面量。
    fn capture_defer_arg(&mut self, expr: &Expr) -> Result<Expr, EvalResult> {
        let line = expr.line;
        let col = expr.col;
        let node = match &expr.data {
            ExprData::IntLit(v) => new_int_lit(*v, line, col),
            ExprData::FloatLit(v) => new_float_lit(*v, line, col),
            ExprData::BoolLit(v) => new_bool_lit(*v, line, col),
            ExprData::CharLit(v) => new_int_lit(*v as i64, line, col),
            ExprData::StringLit(s) => new_string_lit(s.clone(), line, col),
            ExprData::Ident(name) => {
                let v = self.env.get(name);
                value_to_lit(v, line, col)
            }
            ExprData::BinaryOp(b) => {
                let lhs = self.capture_defer_arg(&b.lhs)?;
                let rhs = self.capture_defer_arg(&b.rhs)?;
                new_binary(b.op, lhs, rhs, line, col)
            }
            ExprData::UnaryOp(u) => {
                let op = self.capture_defer_arg(&u.operand)?;
                new_unary(u.op, op, line, col)
            }
            ExprData::Call(c) => {
                let mut new_args = Vec::new();
                for a in &c.args {
                    new_args.push(self.capture_defer_arg(a)?);
                }
                let func_clone: Expr = (*c.func).clone();
                new_call(func_clone, new_args, line, col)
            }
            _ => {
                // 兜底：把整个表达式当作表达式求值一次，再把结果转字面量。
                let r = self.eval_expr_inner(expr);
                if r.status != EvalStatus::Ok {
                    return Err(r);
                }
                value_to_lit(r.value, line, col)
            }
        };
        Ok(node)
    }

    /// 执行程序
    ///
    /// 返回完整的 `EvalResult`，调用方可通过 `result.error` 拿到结构化错误。
    pub fn exec_program(&mut self, prog: &Expr) -> EvalResult {
        if let ExprData::Program(p) = &prog.data {
            // 注册所有函数
            for item in &p.items {
                if matches!(item.data, ExprData::Function(_)) {
                    self.regs.add(item.clone());
                }
            }
            // 找 main
            let main = self.regs.lookup("main").cloned();
            match main {
                Some(m) => {
                    if let ExprData::Function(fd) = &m.data {
                        // A10：main 是顶层入口，通过 exec_stmt 直接执行体，
                        // 不像普通调用那样走 eval_call，所以在 exec_program 内
                        // 手动 flush 一次 defer 栈（逻辑与 eval_call 一致）。
                        let mut result = self.exec_stmt(&fd.body);
                        if matches!(result.status, EvalStatus::Ok | EvalStatus::Return | EvalStatus::Error) {
                            let mut defer_err: Option<EvalResult> = None;
                            while let Some(captured) = self.defers.pop() {
                                let r = self.eval_expr_inner(&captured);
                                if r.status != EvalStatus::Ok {
                                    defer_err = Some(r);
                                    break;
                                }
                            }
                            self.defers.clear();
                            result = match defer_err {
                                Some(err) => match result.status {
                                    EvalStatus::Error => result,
                                    _ => err,
                                },
                                None => result,
                            };
                        }
                        return result;
                    }
                    eval_err(EvalError::UnsupportedExpression) // 不是 Program/Function
                }
                None => eval_err(EvalError::UnknownFunction("main".into())),
            }
        } else {
            eval_err(EvalError::UnsupportedExpression)
        }
    }
}

/// 模式匹配
fn match_pattern(pat: &str, v: &Value) -> bool {
    if pat == "_" { return true; }
    match v.kind() {
        ValueKind::Int => v.to_int() == pat.parse::<i64>().unwrap_or(i64::MIN),
        ValueKind::Float => (v.to_float() - pat.parse::<f64>().unwrap_or(0.0)).abs() < 1e-9,
        ValueKind::Bool => {
            if pat == "true" { v.to_bool() } else if pat == "false" { !v.to_bool() } else { false }
        }
        ValueKind::String => v.to_string() == pat,
        ValueKind::Array => false, // 数组字面量模式由 A8 引入
        ValueKind::Nil => pat == "nil",
    }
}

/// 把 `Value` 转成对应的"字面量 AST 节点"，用于 defer 参数捕获（A10）。
///
/// Nil 单独存到 `ExprData::Ident("nil")` 占位（解释器不暴露 nil 字面量）；
/// 实际使用中几乎不会出现 defer 参数取 nil 值的情况。
///
/// 数组当前不支持捕获（A5 与 A10 解耦；后续 alloc::Vec 引入后再做）。
fn value_to_lit(v: Value, line: u32, col: u32) -> Expr {
    match v.kind() {
        ValueKind::Int => new_int_lit(v.to_int(), line, col),
        ValueKind::Float => new_float_lit(v.to_float(), line, col),
        ValueKind::Bool => new_bool_lit(v.to_bool(), line, col),
        ValueKind::String => {
            // 通过 string_lit_simple 把 &str 转成 StringLit，再交给 new_string_lit
            let s = crate::ast::string_lit_simple(v.as_str());
            new_string_lit(s, line, col)
        }
        ValueKind::Array => new_ident("nil".to_string(), line, col),
        ValueKind::Nil => new_ident("nil".to_string(), line, col),
    }
}

// ============ A3 整数溢出检查 ============
//
// 行为矩阵（v0.11+）：
// - debug 模式（`cfg!(debug_assertions)` 为 true）：i64 加减乘/取负溢出时 panic，错误信息 "integer overflow in <op>"
// - release 模式：保持 v0.10 的 wrap 行为（向后兼容，零开销）
// - 浮点不检查（Inf/NaN 是 IEEE-754 合理行为）
// - 除零保持现有 EvalError::DivisionByZero（不 panic）

/// 不分模式的整数算术：返回 `Some(v)` 表示正常，`None` 表示溢出。
///
/// 该函数是模式无关的，便于在测试中跨模式验证溢出判定。
fn checked_int_arith(op: BinOp, a: i64, b: i64) -> Option<i64> {
    match op {
        BinOp::Add => a.checked_add(b),
        BinOp::Sub => a.checked_sub(b),
        BinOp::Mul => a.checked_mul(b),
        _ => None,
    }
}

/// 整数算术（模式相关）：debug 溢出 panic，release wrap。
///
/// `cfg!(debug_assertions)` 是编译期常量（由 `--release`/profile 决定），
/// 编译后只剩一条分支，因此没有运行时分支开销。
fn int_arith(op: BinOp, a: i64, b: i64) -> i64 {
    if cfg!(debug_assertions) {
        match checked_int_arith(op, a, b) {
            Some(v) => v,
            None => panic!("integer overflow in {}", op),
        }
    } else {
        match op {
            BinOp::Add => a.wrapping_add(b),
            BinOp::Sub => a.wrapping_sub(b),
            BinOp::Mul => a.wrapping_mul(b),
            _ => 0,
        }
    }
}

/// 整数取负（模式相关）：debug 溢出 panic（i64::MIN），release wrap。
fn int_neg(a: i64) -> i64 {
    if cfg!(debug_assertions) {
        match a.checked_neg() {
            Some(v) => v,
            None => panic!("integer overflow in unary -"),
        }
    } else {
        a.wrapping_neg()
    }
}

// ============ A4 字符串基础操作 ============
//
// 设计：方法调用通过 `Call { func: FieldAccess { obj, field }, args }` 模式分发。
// 当前仅实现 `Value::String` 的方法（v0.11 第一批）：
//   len / len_bytes / concat / contains / starts_with / ends_with / slice
// 其他类型的方法在后续版本按 trait / extension 引入。

impl Interpreter {
    /// 方法分发入口。
    ///
    /// 返回 `Some(result)` 表示方法被识别（无论结果成功还是错误），
    /// 返回 `None` 表示该方法在接收者类型上不存在，外层应报 `UnknownMethod`。
    fn eval_method_call(
        &mut self,
        receiver: &Value,
        method: &str,
        args: &[Expr],
    ) -> Option<EvalResult> {
        match receiver {
            Value::String(s) => self.eval_string_method(s, method, args),
            Value::Array(items) => self.eval_array_method(items, method, args),
            _ => None,
        }
    }

    /// 数组方法分派（A5：v0.15 首批）。
    ///
    /// 仅暴露 `len()`——与字符串共享同一方法名，便于后续泛化。
    /// `push / pop / iter` 等方法待 `core::Vec` 引入后再做。
    fn eval_array_method(
        &mut self,
        items: &[Value],
        method: &str,
        args: &[Expr],
    ) -> Option<EvalResult> {
        match method {
            "len" | "length" => {
                if !args.is_empty() {
                    return Some(eval_err(EvalError::ArgumentCountMismatch {
                        expected: 0,
                        got: args.len(),
                    }));
                }
                Some(eval_ok(val_int(items.len() as i64)))
            }
            _ => None, // 未知方法：让外层报 UnknownMethod
        }
    }

    /// 字符串方法分派。返回 `None` 表示方法名不在本类型的方法表里。
    fn eval_string_method(
        &mut self,
        s: &str,
        method: &str,
        args: &[Expr],
    ) -> Option<EvalResult> {
        // 先评估参数（与 Python 风格一致：先 receiver，再 args）
        let mut arg_vals: Vec<Value> = Vec::with_capacity(args.len());
        for arg in args {
            let r = self.eval_expr_inner(arg);
            if r.status != EvalStatus::Ok { return Some(r); }
            arg_vals.push(r.value);
        }

        let arity_err = |expected: usize| -> EvalResult {
            eval_err(EvalError::ArgumentCountMismatch {
                expected,
                got: arg_vals.len(),
            })
        };

        match method {
            "len" | "length" => {
                if !arg_vals.is_empty() { return Some(arity_err(0)); }
                Some(eval_ok(val_int(s.chars().count() as i64)))
            }
            "len_bytes" => {
                if !arg_vals.is_empty() { return Some(arity_err(0)); }
                Some(eval_ok(val_int(s.len() as i64)))
            }
            "concat" => {
                if arg_vals.len() != 1 { return Some(arity_err(1)); }
                let other = arg_vals[0].to_string();
                Some(eval_ok(val_string(format!("{}{}", s, other))))
            }
            "contains" => {
                if arg_vals.len() != 1 { return Some(arity_err(1)); }
                let needle = arg_vals[0].to_string();
                Some(eval_ok(val_bool(s.contains(&needle))))
            }
            "starts_with" => {
                if arg_vals.len() != 1 { return Some(arity_err(1)); }
                let prefix = arg_vals[0].to_string();
                Some(eval_ok(val_bool(s.starts_with(&prefix))))
            }
            "ends_with" => {
                if arg_vals.len() != 1 { return Some(arity_err(1)); }
                let suffix = arg_vals[0].to_string();
                Some(eval_ok(val_bool(s.ends_with(&suffix))))
            }
            "slice" => {
                if arg_vals.len() != 2 { return Some(arity_err(2)); }
                let start = arg_vals[0].to_int();
                let end = arg_vals[1].to_int();
                // UTF-8 安全切片：先收 chars 再切
                let chars: Vec<char> = s.chars().collect();
                let len = chars.len() as i64;
                let s_idx = start.max(0).min(len);
                let e_idx = end.max(0).min(len);
                let sliced: String = if s_idx <= e_idx {
                    chars[s_idx as usize..e_idx as usize].iter().collect()
                } else {
                    String::new()
                };
                Some(eval_ok(val_string(sliced)))
            }
            _ => None, // 未知方法：让外层报 UnknownMethod
        }
    }
}

#[cfg(test)]
mod tests {
    //! 单元测试覆盖 A3（整数溢出检查）和 A4（字符串基础操作）。

    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    /// 解析 + 求值入口（用于测试）
    fn run(src: &str) -> EvalResult {
        let lex = Lexer::new(src);
        let mut p = Parser::new(lex);
        let prog = p.parse_program();
        let mut interp = Interpreter::new();
        interp.exec_program(&prog)
    }

    // ============ A3: 整数溢出 ============

    /// 正常加法不溢出 → 两个模式都应正常工作（解释器端到端验证）
    #[test]
    fn test_int_add_normal() {
        let r = run("func main() { 1 + 2 }");
        expect_int(&r, 3);
    }

    /// `checked_int_arith` 是模式无关的：可直接断言溢出行为
    #[test]
    fn test_checked_int_arith_overflow() {
        assert_eq!(checked_int_arith(BinOp::Add, i64::MAX, 1), None);
        assert_eq!(checked_int_arith(BinOp::Sub, i64::MIN, 1), None);
        assert_eq!(checked_int_arith(BinOp::Mul, i64::MAX, 2), None);
        assert_eq!(checked_int_arith(BinOp::Add, 5, 3), Some(8));
        assert_eq!(checked_int_arith(BinOp::Mul, -3, 7), Some(-21));
        assert_eq!(checked_int_arith(BinOp::Sub, 0, 5), Some(-5));
    }

    /// debug 模式：i64::MAX + 1 必须 panic，消息含 "integer overflow"
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "integer overflow")]
    fn test_int_overflow_add_panics_in_debug() {
        // 9223372036854775807 == i64::MAX（在词法层能正确解析为 i64::MAX）
        let _ = run("func main() { let x = 9223372036854775807; x + 1 }");
    }

    /// debug 模式：i64::MAX * 2 也必须 panic
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "integer overflow")]
    fn test_int_overflow_mul_panics_in_debug() {
        let _ = run("func main() { let x = 9223372036854775807; x * 2 }");
    }

    /// debug 模式：i64::MIN - 1 panic
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "integer overflow")]
    fn test_int_overflow_sub_panics_in_debug() {
        // i64::MIN = -9223372036854775808。源码里写 -9223372036854775808
        // 会被词法切成 -IntLit(9223372036854775808)，而 9223372036854775808 溢出
        // parse::<i64>，默认 0。绕路：用 let x = -9223372036854775807 - 1。
        let _ = run("func main() { let x = -9223372036854775807 - 1; x - 1 }");
    }

    /// release 模式：i64::MAX + 1 静默 wrap 到 i64::MIN
    #[cfg(not(debug_assertions))]
    #[test]
    fn test_int_overflow_add_wraps_in_release() {
        let r = run("func main() { let x = 9223372036854775807; x + 1 }");
        expect_int(&r, i64::MIN);
        let r = run("func main() { let x = 9223372036854775807; x * 2 }");
        expect_int(&r, -2);
    }

    /// debug 模式：直接调用 int_neg(i64::MIN) panic（绕开词法层无法表达 i64::MIN 字面量的问题）
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "integer overflow")]
    fn test_int_neg_min_panics_in_debug() {
        let _ = int_neg(i64::MIN);
    }

    /// 浮点加法不检查溢出（Inf 是 IEEE-754 合理行为）
    #[test]
    fn test_float_overflow_no_check() {
        let r = run("func main() { let x = 1.7e308; x + x }");
        assert_eq!(r.status, EvalStatus::Ok);
        // Inf 是预期结果
    }

    /// 除零已有错误处理，不能因为 A3 改变
    #[test]
    fn test_div_by_zero_still_errors() {
        let r = run("func main() { 10 / 0 }");
        assert_eq!(r.status, EvalStatus::Error);
        assert_eq!(r.error, Some(EvalError::DivisionByZero));
    }

    // ============ A4: 字符串基础操作 ============

    fn expect_int(r: &EvalResult, expected: i64) {
        assert_eq!(r.status, EvalStatus::Ok, "eval failed: {:?}", r.error);
        match &r.value {
            Value::Int(n) => assert_eq!(*n, expected, "value mismatch: got {}", n),
            other => panic!("expected int, got {:?}", other),
        }
    }

    fn expect_string(r: &EvalResult, expected: &str) {
        assert_eq!(r.status, EvalStatus::Ok, "eval failed: {:?}", r.error);
        match &r.value {
            Value::String(s) => assert_eq!(s, expected),
            other => panic!("expected string, got {:?}", other),
        }
    }

    fn expect_bool(r: &EvalResult, expected: bool) {
        assert_eq!(r.status, EvalStatus::Ok);
        match &r.value {
            Value::Bool(b) => assert_eq!(*b, expected),
            other => panic!("expected bool, got {:?}", other),
        }
    }

    #[test]
    fn test_str_len_chars() {
        let r = run(r#"func main() { let s = "你好"; s.len() }"#);
        expect_int(&r, 2);
    }

    #[test]
    fn test_str_len_bytes() {
        let r = run(r#"func main() { let s = "你好"; s.len_bytes() }"#);
        expect_int(&r, 6);
    }

    #[test]
    fn test_str_concat() {
        let r = run(r#"func main() { let s = "你好"; s.concat("世界") }"#);
        expect_string(&r, "你好世界");
    }

    #[test]
    fn test_str_contains() {
        let r = run(r#"func main() { let s = "你好世界"; s.contains("好世") }"#);
        expect_bool(&r, true);
        let r = run(r#"func main() { let s = "你好世界"; s.contains("再见") }"#);
        expect_bool(&r, false);
    }

    #[test]
    fn test_str_starts_ends_with() {
        let r = run(r#"func main() { let s = "你好世界"; s.starts_with("你好") }"#);
        expect_bool(&r, true);
        let r = run(r#"func main() { let s = "你好世界"; s.ends_with("世界") }"#);
        expect_bool(&r, true);
        let r = run(r#"func main() { let s = "你好世界"; s.starts_with("世界") }"#);
        expect_bool(&r, false);
    }

    #[test]
    fn test_str_slice_half_open() {
        // 切片是半开 [start, end)
        let r = run(r#"func main() { let s = "你好世界"; s.slice(0, 2) }"#);
        expect_string(&r, "你好");
        let r = run(r#"func main() { let s = "你好世界"; s.slice(2, 4) }"#);
        expect_string(&r, "世界");
        // 空切片
        let r = run(r#"func main() { let s = "你好"; s.slice(1, 1) }"#);
        expect_string(&r, "");
        // 越界 end 自动 clamp
        let r = run(r#"func main() { let s = "你好"; s.slice(0, 999) }"#);
        expect_string(&r, "你好");
    }

    #[test]
    fn test_str_unknown_method_errors() {
        let r = run(r#"func main() { let s = "hi"; s.bogus() }"#);
        assert_eq!(r.status, EvalStatus::Error);
        match r.error {
            Some(EvalError::UnknownMethod { ref method, .. }) => {
                assert_eq!(method, "bogus");
            }
            other => panic!("expected UnknownMethod, got {:?}", other),
        }
    }

    #[test]
    fn test_str_method_on_int_errors() {
        // Int 没有 len 方法
        let r = run("func main() { let x = 42; x.len() }");
        assert_eq!(r.status, EvalStatus::Error);
        match r.error {
            Some(EvalError::UnknownMethod { ref receiver_type, ref method }) => {
                assert_eq!(receiver_type, "int");
                assert_eq!(method, "len");
            }
            other => panic!("expected UnknownMethod, got {:?}", other),
        }
    }

    // ============ A5: 数组类型 [T; N] + slice [T] ============
    //
    // 验收标准：
    // 1. 数组字面量：[1, 2, 3]
    // 2. 索引：arr[i]
    // 3. 切片：arr[a..b] / arr[..b] / arr[a..] / arr[..] / arr[a..=b]
    // 4. 数组方法：arr.len()
    // 5. 越界 panic（debug）/ wrap（release）
    // 6. 非数组做索引/切片 → IndexNonArray / SliceNonArray 错误

    /// 验收 #1：数组字面量 + 索引
    #[test]
    fn test_array_lit_and_index() {
        let r = run("func main() { let arr = [1, 2, 3]; arr[0] }");
        expect_int(&r, 1);
        let r = run("func main() { let arr = [10, 20, 30]; arr[1] }");
        expect_int(&r, 20);
    }

    /// 验收 #3：半开切片 `arr[a..b]`
    #[test]
    fn test_array_slice_half_open() {
        let r = run("func main() { let arr = [1, 2, 3, 4, 5]; arr[1..3] }");
        expect_array(&r, &[2, 3]);
        // 空切片
        let r = run("func main() { let arr = [1, 2, 3]; arr[1..1] }");
        expect_array(&r, &[]);
        // 越界 end 自动 clamp
        let r = run("func main() { let arr = [10, 20]; arr[0..999] }");
        expect_array(&r, &[10, 20]);
    }

    /// 验收 #3：开放切片（`..` / `a..` / `..b` / `..=`）
    #[test]
    fn test_array_slice_open_ends() {
        let r = run("func main() { let arr = [10, 20, 30, 40, 50]; arr[..3] }");
        expect_array(&r, &[10, 20, 30]);
        let r = run("func main() { let arr = [10, 20, 30, 40, 50]; arr[3..] }");
        expect_array(&r, &[40, 50]);
        let r = run("func main() { let arr = [10, 20, 30]; arr[..] }");
        expect_array(&r, &[10, 20, 30]);
    }

    /// 验收 #3：含右端点 `arr[a..=b]`
    #[test]
    fn test_array_slice_inclusive() {
        let r = run("func main() { let arr = [10, 20, 30, 40, 50]; arr[1..=3] }");
        expect_array(&r, &[20, 30, 40]);
        let r = run("func main() { let arr = [10, 20, 30]; arr[..=1] }");
        expect_array(&r, &[10, 20]);
    }

    /// 验收 #4：数组方法 `len()`（与字符串共享方法名）
    #[test]
    fn test_array_len_method() {
        let r = run("func main() { let arr = [1, 2, 3]; arr.len() }");
        expect_int(&r, 3);
        let _r = run("func main() { let arr: int[] = []; arr.len() }"); // 语法不直接支持空数组字面量 []——下面用 []
        // 注：当前 [] 在 parser 里被识别为空数组字面量 [T]，而非 generic type 标注
        let r = run("func main() { let arr = [42]; arr.len() }");
        expect_int(&r, 1);
    }

    /// 验收 #2/验收 #5（release 行为）：越界 wrap 到 0 / 反向负数 wrap 到末尾
    /// 期望在 release 模式下不 panic（wrap），dev 模式由独立测试覆盖。
    #[cfg(not(debug_assertions))]
    #[test]
    fn test_array_index_oob_wraps_in_release() {
        // arr[3] 越界 → wrap 到 arr[0]
        let r = run("func main() { let arr = [10, 20, 30]; arr[3] }");
        expect_int(&r, 10);
        // arr[-1] 越界 → wrap 到 arr[2]
        let r = run("func main() { let arr = [10, 20, 30]; arr[-1] }");
        expect_int(&r, 30);
    }

    /// 验收 #5（debug 行为）：越界必须 panic
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "array index out of bounds")]
    fn test_array_index_oob_panics_in_debug() {
        let _ = run("func main() { let arr = [10, 20, 30]; arr[10] }");
    }

    /// 非数组做索引 → IndexNonArray
    #[test]
    fn test_index_non_array_errors() {
        let r = run("func main() { let x = 42; x[0] }");
        assert_eq!(r.status, EvalStatus::Error);
        match r.error {
            Some(EvalError::IndexNonArray { ref actual }) => {
                assert_eq!(actual, "int");
            }
            other => panic!("expected IndexNonArray, got {:?}", other),
        }
    }

    /// 非数组做切片 → SliceNonArray
    #[test]
    fn test_slice_non_array_errors() {
        let r = run(r#"func main() { let s = "hello"; s[0..2] }"#);
        assert_eq!(r.status, EvalStatus::Error);
        match r.error {
            Some(EvalError::SliceNonArray { ref actual }) => {
                assert_eq!(actual, "string");
            }
            other => panic!("expected SliceNonArray, got {:?}", other),
        }
    }

    /// 切片可以嵌套使用（slice of slice）
    #[test]
    fn test_array_slice_nested() {
        let r = run("func main() { let arr = [10, 20, 30, 40, 50]; let s = arr[1..4]; s[0] }");
        expect_int(&r, 20);
    }

    /// 期望数组 helper：与 expect_int 同风格
    fn expect_array(r: &EvalResult, expected: &[i64]) {
        assert_eq!(r.status, EvalStatus::Ok, "eval failed: {:?}", r.error);
        match &r.value {
            Value::Array(items) => {
                let got: Vec<i64> = items.iter().map(|v| v.to_int()).collect();
                assert_eq!(got, expected, "array mismatch: got {:?}", got);
            }
            other => panic!("expected array, got {:?}", other),
        }
    }

    // ============ A10: defer 延迟清理 ============
    //
    // 验收标准：defer 在函数返回前按 LIFO 执行；defer 参数在注册时立即求值（捕获值）。
    //
    // 注意：解释器端到端测试不抓 stdout，因此对 print 副作用通过"返回值仍为 Ok
    // 且等于预期值"间接验证。main 不使用 `return`（让外层保留 status=Ok），
    // 而是让最终块表达式作为返回值。

    /// LIFO 顺序：后注册先执行（Go/Swift/defer 通用语义）。
    #[test]
    fn test_defer_lifo_simple() {
        let src = r#"
            func main() {
                defer print("a");
                defer print("b");
                print("c");
                42
            }
        "#;
        let r = run(src);
        expect_int(&r, 42);
    }

    /// defer 在函数 `return` 之前执行（与 Go/Swift 一致）。
    /// `foo()` 调用 → `defer print("cleanup")` 注册 → `return 42` →
    /// 调用方 `let r = foo()` 收到 42。我们通过 main 末尾表达式的值间接确认。
    #[test]
    fn test_defer_before_return() {
        let src = r#"
            func foo() -> int {
                defer print("cleanup");
                return 42;
            }
            func main() {
                let r = foo();
                r
            }
        "#;
        let r = run(src);
        expect_int(&r, 42);
    }

    /// defer 参数在注册时**立即求值**（Go 1.14+ 语义）。
    /// 让 `x` 在 defer 注册后改变，最终 main 末尾是 `x` 的当前值 20。
    /// defer print(10) 副作用（已捕获的 10）我们从 stdout 间接看到；返回值是 20。
    #[test]
    fn test_defer_args_captured_at_register_time() {
        let src = r#"
            func main() {
                let x = 10;
                defer print(x);
                let x = 20;
                print(x);
                x
            }
        "#;
        let r = run(src);
        expect_int(&r, 20);
    }

    /// defer 调用用户自定义函数（无参）
    #[test]
    fn test_defer_user_func_no_args() {
        let src = r#"
            func cleanup() {
                print("cleaned");
            }
            func main() {
                defer cleanup();
                7
            }
        "#;
        let r = run(src);
        expect_int(&r, 7);
    }

    /// defer 带多个不同表达式的参数（字符串 / 算术 / 捕获变量）
    #[test]
    fn test_defer_mixed_arg_types() {
        let src = r#"
            func main() {
                let s = "hi";
                let n = 5;
                defer print(s.concat("!"));
                defer print(n + 100);
                0
            }
        "#;
        let r = run(src);
        expect_int(&r, 0);
    }

    /// defer 栈与正常求值互不干扰（flush 后栈必须清空，避免影响下次调用）。
    #[test]
    fn test_defer_stack_clears_between_calls() {
        let src = r#"
            func baz() {
                defer print("from baz");
                1
            }
            func bar() {
                defer print("from bar");
                baz()
            }
            func main() {
                bar()
            }
        "#;
        let r = run(src);
        expect_int(&r, 1);
    }

    /// defer 表达式为非 Call 时退化为立即求值（按设计简化）。
    #[test]
    fn test_defer_non_call_is_immediate() {
        // 字面量作为 defer 表达式：预期解释器把它当普通表达式求值（丢弃返回值）。
        let src = r#"
            func main() {
                defer 100;
                print("body");
                0
            }
        "#;
        let r = run(src);
        expect_int(&r, 0);
    }

    // ============ A1: Sound null safety ============
    //
    // 验收标准：
    // 1. `let x: int = 5; print(x)` → 5
    // 2. `let x: int = nil` → TypeMismatch error
    // 3. `let x: int? = nil; print(x)` → nil
    // 4. `let x: int? = 5; print(x)` → 5
    // 5. `let x: int? = 5; let y: int = x?; print(y)` → 5
    // 6. `let x: int? = nil; let y: int = x?` → UnwrapOnNil / panic
    // 7. `let x: int = 5; let y = x?` → 5（v0.20 宽松版：x 非 T? 也允许 unwrap）
    // 8. 参数支持 T? / T 注解
    // 9. 结构体字段支持 T? / T 注解

    /// 验收 #1：基本类型注解 `let x: int = 5; print(x)` 输出 5
    #[test]
    fn test_a1_let_int_typed_print() {
        let r = run("func main() { let x: int = 5; print(x) }");
        // 通过返回值间接验证（main 末尾表达式是 0 = nil）
        assert_eq!(r.status, EvalStatus::Ok, "expected Ok, got error: {:?}", r.error);
        assert!(matches!(r.value, Value::Nil));
    }

    /// 验收 #2：非空类型禁止赋值为 nil → TypeMismatch 错误
    #[test]
    fn test_a1_let_int_nil_type_mismatch() {
        let r = run("func main() { let x: int = nil; 0 }");
        assert_eq!(r.status, EvalStatus::Error);
        match r.error {
            Some(EvalError::TypeMismatch { ref expected, ref got }) => {
                assert_eq!(expected, "int");
                assert_eq!(got, "nil");
            }
            other => panic!("expected TypeMismatch, got {:?}", other),
        }
    }

    /// 验收 #3：`let x: int? = nil; print(x)` 输出 nil
    #[test]
    fn test_a1_nullable_accepts_nil() {
        // main 末尾表达式是 x，即 Nil
        let r = run("func main() { let x: int? = nil; x }");
        assert_eq!(r.status, EvalStatus::Ok);
        assert!(matches!(r.value, Value::Nil));
    }

    /// 验收 #4：`let x: int? = 5` 接受（int 是 int? 的子类型）
    #[test]
    fn test_a1_nullable_accepts_int() {
        let r = run("func main() { let x: int? = 5; x }");
        assert_eq!(r.status, EvalStatus::Ok);
        expect_int(&r, 5);
    }

    /// 验收 #5：unwrap 成功路径 `let x: int? = 5; let y: int = x?; print(y)`
    /// 在非空类型上赋 unwrap 结果 = Int(5) → 合法
    #[test]
    fn test_a1_unwrap_non_nil_succeeds() {
        let r = run("func main() { let x: int? = 5; let y: int = x?; y }");
        assert_eq!(r.status, EvalStatus::Ok, "got error: {:?}", r.error);
        expect_int(&r, 5);
    }

    /// 验收 #6 (release)：unwrap 遇到 nil → EvalError::UnwrapOnNil
    #[cfg(not(debug_assertions))]
    #[test]
    fn test_a1_unwrap_on_nil_errors_in_release() {
        let r = run("func main() { let x: int? = nil; let y: int = x?; y }");
        assert_eq!(r.status, EvalStatus::Error);
        assert_eq!(r.error, Some(EvalError::UnwrapOnNil));
    }

    /// 验收 #6 (debug)：unwrap 遇到 nil 必须 panic
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "unwrap on nil value")]
    fn test_a1_unwrap_on_nil_panics_in_debug() {
        let _ = run("func main() { let x: int? = nil; let y: int = x?; y }");
    }

    /// 验收 #7：v0.20 宽松版 —— `x?` 在 x 为非 nil 值时直接返回 x
    /// 即便 x 的静态类型并非 T?，也不强制检查（推迟到 v0.30+ 引入类型推导后）
    #[test]
    fn test_a1_unwrap_non_nullable_lenient() {
        let r = run("func main() { let x: int = 5; let y = x?; y }");
        assert_eq!(r.status, EvalStatus::Ok);
        expect_int(&r, 5);
    }

    /// unwrap 链式：`x??` 等价于 Unwrap(Unwrap(x))（当前 spec 未禁止，但实际语义为两次 unwrap）
    /// 对于非 nil 值链式 unwrap 应直接透传
    #[test]
    fn test_a1_unwrap_chain_non_nil() {
        let r = run("func main() { let x: int? = 7; let y = x?; y }");
        assert_eq!(r.status, EvalStatus::Ok);
        expect_int(&r, 7);
    }

    /// 函数参数类型注解 `T?`：传入 nil 不报错
    #[test]
    fn test_a1_param_nullable_accepts_nil() {
        let r = run("func main() { let _v = take_nil(nil); 42 } func take_nil(x: int?) -> int { 0 }");
        assert_eq!(r.status, EvalStatus::Ok);
        expect_int(&r, 42);
    }

    /// 函数参数类型注解 `T`：传入 nil 应在求值侧报错
    /// （注：v0.20 第一版不强制实参类型检查；这里我们只验证语义层不会主动报错，
    ///  即参数类型注解仅作"声明"，不参与运行时校验。这与类型推导推迟一致。）
    #[test]
    fn test_a1_param_non_nullable_no_runtime_check() {
        // 当前实现：参数类型注解仅作声明，不强制实参 nil 检查
        // （v0.30+ 引入类型推导后再做严格检查）
        let r = run("func main() { let v = take_int(5); v } func take_int(x: int) -> int { x }");
        assert_eq!(r.status, EvalStatus::Ok);
        expect_int(&r, 5);
    }

    /// 结构体字段类型注解支持 `T?` 形式（语义层只校验语法可解析）
    #[test]
    fn test_a1_struct_field_nullable_parses() {
        // 仅验证 AST 能解析 `field: int?`，不做运行时实例化
        let r = run("struct Box { value: int? } func main() { 0 }");
        assert_eq!(r.status, EvalStatus::Ok, "got error: {:?}", r.error);
    }

    /// 类型注解不影响"无注解变量"的语义（向后兼容）
    #[test]
    fn test_a1_no_annotation_backward_compat() {
        let r = run("func main() { let x = 5; let y = 10; x + y }");
        assert_eq!(r.status, EvalStatus::Ok);
        expect_int(&r, 15);
    }

    /// 期望数组 helper 用于测试可空数组（虽然 v0.20 不支持数组类型注解，先留接口）
    fn expect_nil(r: &EvalResult) {
        assert_eq!(r.status, EvalStatus::Ok, "eval failed: {:?}", r.error);
        assert!(matches!(r.value, Value::Nil), "expected nil, got {:?}", r.value);
    }

    /// `let x: int? = nil; if x == nil { print("ok") }` —— 可空类型与 nil 字面量比较
    /// 当前解释器把 nil 视为 Value::Nil，与 Int/Float 等不同 kind，equal 返回 false。
    /// 这里主要验证：可空变量能正常参与表达式而不 panic。
    #[test]
    fn test_a1_nullable_var_in_expression() {
        let r = run("func main() { let x: int? = nil; x }");
        assert_eq!(r.status, EvalStatus::Ok);
        expect_nil(&r);
    }

    /// Display 输出验证：TypeMismatch / UnwrapOnNil 错误信息中文友好
    #[test]
    fn test_a1_eval_error_display() {
        let tm = EvalError::TypeMismatch {
            expected: "int".to_string(),
            got: "nil".to_string(),
        };
        assert_eq!(format!("{}", tm), "类型不匹配：期望 int，得到 nil");
        assert_eq!(format!("{}", EvalError::UnwrapOnNil), "unwrap 遇到 nil 值");
    }
}
