//! Qaw 运行时值

use std::fmt;

/// 值类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    Int,
    Float,
    Bool,
    String,
    Array,
    Nil,
}

impl ValueKind {
    /// 类型名（用于错误信息）
    pub fn as_str(&self) -> &'static str {
        match self {
            ValueKind::Int => "int",
            ValueKind::Float => "float",
            ValueKind::Bool => "bool",
            ValueKind::String => "string",
            ValueKind::Array => "array",
            ValueKind::Nil => "nil",
        }
    }
}

/// 值（tagged union）
///
/// 注意：`Value` 不实现 `Copy`（自 v0.15 起，`Array` 变体持有 `Vec<Value>`）。
/// 调用方需要按所有权传递，或显式 `.clone()`。
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Array(Vec<Value>),
    Nil,
}

// 提供字符串访问
impl Value {
    /// 转为字符串表示
    pub fn as_str(&self) -> &str {
        match self {
            Value::String(s) => s,
            _ => "",
        }
    }

    /// 数组长度（非数组返回 0）
    pub fn array_len(&self) -> usize {
        match self {
            Value::Array(v) => v.len(),
            _ => 0,
        }
    }
}

impl Value {
    pub fn kind(&self) -> ValueKind {
        match self {
            Value::Int(_) => ValueKind::Int,
            Value::Float(_) => ValueKind::Float,
            Value::Bool(_) => ValueKind::Bool,
            Value::String(_) => ValueKind::String,
            Value::Array(_) => ValueKind::Array,
            Value::Nil => ValueKind::Nil,
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Nil => false,
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
        }
    }

    pub fn to_int(&self) -> i64 {
        match self {
            Value::Int(n) => *n,
            Value::Float(f) => *f as i64,
            Value::Bool(b) => if *b { 1 } else { 0 },
            Value::String(s) => s.parse().unwrap_or(0),
            Value::Array(_) => 0,
            Value::Nil => 0,
        }
    }

    pub fn to_float(&self) -> f64 {
        match self {
            Value::Int(n) => *n as f64,
            Value::Float(f) => *f,
            Value::Bool(b) => if *b { 1.0 } else { 0.0 },
            Value::String(s) => s.parse().unwrap_or(0.0),
            Value::Array(_) => 0.0,
            Value::Nil => 0.0,
        }
    }

    pub fn to_bool(&self) -> bool {
        self.is_truthy()
    }

    pub fn equal(&self, other: &Value) -> bool {
        if std::mem::discriminant(self) != std::mem::discriminant(other) {
            // 跨类型：尝试数值比较
            if matches!(self.kind(), ValueKind::Int | ValueKind::Float)
                && matches!(other.kind(), ValueKind::Int | ValueKind::Float)
            {
                return self.to_float() == other.to_float();
            }
            return false;
        }
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            _ => false,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(v) => write!(f, "{}", v),
            Value::Bool(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "{}", s),
            Value::Array(items) => {
                write!(f, "[")?;
                for (i, v) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            Value::Nil => write!(f, "nil"),
        }
    }
}

pub fn val_int(n: i64) -> Value { Value::Int(n) }
pub fn val_float(f: f64) -> Value { Value::Float(f) }
pub fn val_bool(b: bool) -> Value { Value::Bool(b) }
pub fn val_string(s: impl Into<String>) -> Value { Value::String(s.into()) }
pub fn val_array(items: Vec<Value>) -> Value { Value::Array(items) }
pub fn val_nil() -> Value { Value::Nil }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_int() {
        let v = val_int(42);
        assert_eq!(v.kind(), ValueKind::Int);
        assert_eq!(v.to_int(), 42);
    }

    #[test]
    fn test_truthy() {
        assert!(!val_nil().is_truthy());
        assert!(!val_bool(false).is_truthy());
        assert!(val_bool(true).is_truthy());
        assert!(!val_int(0).is_truthy());
        assert!(val_int(-1).is_truthy());
        assert!(!val_string("").is_truthy());
        assert!(!val_array(vec![]).is_truthy());
        assert!(val_array(vec![val_int(1)]).is_truthy());
    }

    #[test]
    fn test_equal() {
        assert!(val_int(5).equal(&val_int(5)));
        assert!(!val_int(5).equal(&val_int(6)));
        assert!(val_string("hi").equal(&val_string("hi")));
        assert!(val_nil().equal(&val_nil()));
        assert!(val_int(5).equal(&val_float(5.0)));
    }

    #[test]
    fn test_array_kind_and_len() {
        let v = val_array(vec![val_int(1), val_int(2), val_int(3)]);
        assert_eq!(v.kind(), ValueKind::Array);
        assert_eq!(v.array_len(), 3);
    }

    #[test]
    fn test_array_equal() {
        let a = val_array(vec![val_int(1), val_int(2)]);
        let b = val_array(vec![val_int(1), val_int(2)]);
        let c = val_array(vec![val_int(1), val_int(3)]);
        assert!(a.equal(&b));
        assert!(!a.equal(&c));
        assert!(!a.equal(&val_int(1)));
    }

    #[test]
    fn test_array_display() {
        let v = val_array(vec![val_int(1), val_string("x"), val_bool(true)]);
        assert_eq!(format!("{}", v), "[1, x, true]");
    }
}