//! Qaw 词法分析器（Lexer）
//!
//! 将 UTF-8 源文件切分为 Token 流。
//! 关键字归一化为英文原形，原始字面保存在 lexeme 中。
//!
//! 四形制关键字：
//! - canonical: 英文原形（如 `func`）
//! - abbrev:    英文缩写（如 `fn`）
//! - pinyin_full: 汉语全拼（如 `hanshu`）
//! - pinyin_init: 拼音首字母（≥2 字符，如 `hs`）

use std::fmt;

/// Token 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokKind {
    // 字面量
    IntLit,
    FloatLit,
    StringLit,
    CharLit,
    BoolLit,
    Ident,

    // 关键字（归一化后）
    KwPackage,
    KwImport,
    KwFunc,
    KwReturn,
    KwVar,
    KwLet,
    KwMut,
    KwConst,
    KwType,
    KwStruct,
    KwEnum,
    KwIf,
    KwElse,
    KwFor,
    KwWhile,
    KwLoop,
    KwBreak,
    KwContinue,
    KwMatch,
    KwTrue,
    KwFalse,
    KwDefer,
    KwGo,
    KwChan,
    KwPanic,
    KwUnsafe,
    KwAs,
    KwIn,
    KwFrom,
    KwTo,
    KwDownto,
    KwStep,

    // 运算符
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    EqEq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    Amp,
    Pipe,
    Caret,
    Shl,
    Shr,
    AndAnd,
    OrOr,
    Bang,
    Arrow,        // ->
    FatArrow,     // =>
    DotDot,       // ..
    DotDotEq,     // ..=
    Question,     // ?
    QuestionDot,  // ?.
    QuestionQuestion, // ??
    ColonColon,   // ::

    // 分隔符
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    LAngle,
    RAngle,
    Comma,
    Semi,
    Colon,
    Dot,

    // 特殊
    Eof,
    Error,
}

/// Token：类型 + lexeme + 位置
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokKind,
    pub lexeme: String,
    pub line: u32,
    pub col: u32,
}

impl Token {
    pub fn new(kind: TokKind, lexeme: &str, line: u32, col: u32) -> Self {
        Token {
            kind,
            lexeme: lexeme.to_string(),
            line,
            col,
        }
    }

    pub fn eof(line: u32, col: u32) -> Self {
        Token {
            kind: TokKind::Eof,
            lexeme: String::new(),
            line,
            col,
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}({})", self.kind, self.lexeme)
    }
}

/// 关键字四形映射：(canonical, [(abbrev, pinyin_full, pinyin_init)])
const KEYWORD_ALIASES: &[(&str, &[&str])] = &[
    ("package",  &["pkg",    "baozhuang",  "bz"]),
    ("import",   &["imp",    "daoru",      "dr"]),
    ("func",     &["fn",     "hanshu",     "hs"]),
    ("return",   &["ret",    "fanhui",     "fh"]),
    ("var",      &["var",    "bianliang",  "bl"]),
    ("let",      &["let",    "buke",       "bk"]),
    ("mut",      &["mut",    "kebian",     "kb"]),
    ("const",    &["con",    "changliang", "cl"]),
    ("type",     &["typ",    "leixing",    "lx"]),
    ("struct",   &["struct", "jiegou",     "jg"]),
    ("enum",     &["enum",   "meiju",      "mj"]),
    ("if",       &["if",     "ruguo",      "rg"]),
    ("else",     &["els",    "fouze",      "fz"]),
    ("for",      &["for",    "xunhuan",    "xh"]),
    ("while",    &["whl",    "dang",       "dg"]),
    ("break",    &["brk",    "tiaochu",    "tc"]),
    ("continue", &["cnt",    "jixu",       "jx"]),
    ("match",    &["match",  "pipei",      "pp"]),
    ("true",     &["true",   "zhen",       "zh"]),
    ("false",    &["fls",    "jia",        "ji"]),
    ("defer",    &["def",    "yanchi",     "yc"]),
    ("go",       &["go",     "xiecheng",   "xc"]),
    ("chan",     &["chan",   "tongdao",    "td"]),
    ("panic",    &["panic",  "huangkong",  "hk"]),
    ("unsafe",   &["unsafe", "weixian",    "wx"]),
    ("as",       &["as",     "zuowei",     "zw"]),
    ("in",       &["in",     "zai",        "zai"]),
    ("from",     &["from",   "cong",       "cong"]),
    ("to",       &["to",     "dao",        "dao"]),
];

/// 关键字 token kind 映射
fn keyword_kind(canonical: &str) -> Option<TokKind> {
    Some(match canonical {
        "package" => TokKind::KwPackage,
        "import" => TokKind::KwImport,
        "func" => TokKind::KwFunc,
        "return" => TokKind::KwReturn,
        "var" => TokKind::KwVar,
        "let" => TokKind::KwLet,
        "mut" => TokKind::KwMut,
        "const" => TokKind::KwConst,
        "type" => TokKind::KwType,
        "struct" => TokKind::KwStruct,
        "enum" => TokKind::KwEnum,
        "if" => TokKind::KwIf,
        "else" => TokKind::KwElse,
        "for" => TokKind::KwFor,
        "while" => TokKind::KwWhile,
        "loop" => TokKind::KwLoop,
        "break" => TokKind::KwBreak,
        "continue" => TokKind::KwContinue,
        "match" => TokKind::KwMatch,
        "true" => TokKind::KwTrue,
        "false" => TokKind::KwFalse,
        "defer" => TokKind::KwDefer,
        "go" => TokKind::KwGo,
        "chan" => TokKind::KwChan,
        "panic" => TokKind::KwPanic,
        "unsafe" => TokKind::KwUnsafe,
        "as" => TokKind::KwAs,
        "in" => TokKind::KwIn,
        "from" => TokKind::KwFrom,
        "to" => TokKind::KwTo,
        _ => return None,
    })
}

/// 关键字归一化：返回 canonical（如果是关键字）或 None
fn keyword_normalize(s: &str) -> Option<&'static str> {
    for (canonical, forms) in KEYWORD_ALIASES {
        if s == *canonical {
            return Some(canonical);
        }
        for form in forms.iter() {
            if s == *form {
                return Some(canonical);
            }
        }
    }
    None
}

/// 词法错误
#[derive(Debug, Clone)]
pub struct LexError {
    pub line: u32,
    pub col: u32,
    pub message: String,
}

/// 词法分析器
pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    line: u32,
    col: u32,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.source.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.source.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    /// 跳过分隔
    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.advance();
                }
                Some('/') => {
                    if self.peek_next() == Some('/') {
                        // 行注释
                        while let Some(c) = self.peek() {
                            if c == '\n' {
                                break;
                            }
                            self.advance();
                        }
                    } else if self.peek_next() == Some('*') {
                        // 块注释
                        self.advance(); // /
                        self.advance(); // *
                        while let Some(c) = self.peek() {
                            if c == '*' && self.peek_next() == Some('/') {
                                self.advance();
                                self.advance();
                                break;
                            }
                            self.advance();
                        }
                    } else {
                        return;
                    }
                }
                _ => return,
            }
        }
    }

    /// 读取数字字面量
    fn lex_number(&mut self) -> Token {
        let start_line = self.line;
        let start_col = self.col;
        let start_pos = self.pos;
        let mut is_float = false;

        // 整数部分
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }

        // 小数点
        if self.peek() == Some('.') && self.peek_next().is_some_and(|c| c.is_ascii_digit()) {
            is_float = true;
            self.advance(); // .
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        // 指数
        if matches!(self.peek(), Some('e') | Some('E')) {
            is_float = true;
            self.advance();
            if matches!(self.peek(), Some('+') | Some('-')) {
                self.advance();
            }
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        let lexeme: String = self.source[start_pos..self.pos].iter().collect();
        let kind = if is_float { TokKind::FloatLit } else { TokKind::IntLit };
        Token::new(kind, &lexeme, start_line, start_col)
    }

    /// 读取字符串字面量
    fn lex_string(&mut self) -> Token {
        let start_line = self.line;
        let start_col = self.col;
        self.advance(); // 起始 "
        let mut buf = String::new();
        loop {
            match self.peek() {
                None => break,  // 未闭合，按 EOF 处理
                Some('"') => {
                    self.advance();
                    break;
                }
                Some('\\') => {
                    self.advance();
                    if let Some(c) = self.advance() {
                        match c {
                            'n' => buf.push('\n'),
                            't' => buf.push('\t'),
                            'r' => buf.push('\r'),
                            '\\' => buf.push('\\'),
                            '"' => buf.push('"'),
                            '$' => buf.push('$'),
                            _ => buf.push(c),
                        }
                    }
                }
                Some(c) => {
                    buf.push(c);
                    self.advance();
                }
            }
        }
        Token::new(TokKind::StringLit, &buf, start_line, start_col)
    }

    /// 读取字符字面量
    fn lex_char(&mut self) -> Token {
        let start_line = self.line;
        let start_col = self.col;
        self.advance(); // 起始 '
        let ch = self.advance().unwrap_or('\0');
        if self.peek() == Some('\'') {
            self.advance();
        }
        let s = ch.to_string();
        Token::new(TokKind::CharLit, &s, start_line, start_col)
    }

    /// 读取标识符或关键字
    fn lex_ident_or_keyword(&mut self) -> Token {
        let start_line = self.line;
        let start_col = self.col;
        let start_pos = self.pos;
        // 第一个字符
        if let Some(c) = self.peek() {
            if !is_ident_start(c) {
                // 错误字符
                let s = c.to_string();
                self.advance();
                return Token::new(TokKind::Error, &s, start_line, start_col);
            }
        }
        self.advance();
        // 后续字符
        while let Some(c) = self.peek() {
            if is_ident_byte(c) {
                self.advance();
            } else {
                break;
            }
        }
        let lexeme: String = self.source[start_pos..self.pos].iter().collect();

        // 检查是否为关键字
        if let Some(canonical) = keyword_normalize(&lexeme) {
            if let Some(kind) = keyword_kind(canonical) {
                return Token::new(kind, &lexeme, start_line, start_col);
            }
        }
        Token::new(TokKind::Ident, &lexeme, start_line, start_col)
    }

    /// 下一个 token
    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();

        let line = self.line;
        let col = self.col;
        let c = match self.peek() {
            None => return Token::eof(line, col),
            Some(c) => c,
        };

        // 数字
        if c.is_ascii_digit() {
            return self.lex_number();
        }

        // 标识符 / 关键字
        if is_ident_start(c) {
            return self.lex_ident_or_keyword();
        }

        // 字符串
        if c == '"' {
            return self.lex_string();
        }

        // 字符
        if c == '\'' {
            return self.lex_char();
        }

        // 多字符运算符
        self.advance();
        let next_c = self.peek();
        let two = |a: char, b: char| c == a && next_c == Some(b);

        if two('-', '>') {
            self.advance();
            return Token::new(TokKind::Arrow, "->", line, col);
        }
        if two('=', '>') {
            self.advance();
            return Token::new(TokKind::FatArrow, "=>", line, col);
        }
        if two('=', '=') {
            self.advance();
            return Token::new(TokKind::EqEq, "==", line, col);
        }
        if two('!', '=') {
            self.advance();
            return Token::new(TokKind::NotEq, "!=", line, col);
        }
        if two('<', '=') {
            self.advance();
            return Token::new(TokKind::Le, "<=", line, col);
        }
        if two('>', '=') {
            self.advance();
            return Token::new(TokKind::Ge, ">=", line, col);
        }
        if two('&', '&') {
            self.advance();
            return Token::new(TokKind::AndAnd, "&&", line, col);
        }
        if two('|', '|') {
            self.advance();
            return Token::new(TokKind::OrOr, "||", line, col);
        }
        if two(':', ':') {
            self.advance();
            return Token::new(TokKind::ColonColon, "::", line, col);
        }
        if two('?', '.') {
            self.advance();
            return Token::new(TokKind::QuestionDot, "?.", line, col);
        }
        if two('?', '?') {
            self.advance();
            return Token::new(TokKind::QuestionQuestion, "??", line, col);
        }
        if two('.', '.') {
            self.advance();
            if self.peek() == Some('=') {
                self.advance();
                return Token::new(TokKind::DotDotEq, "..=", line, col);
            }
            return Token::new(TokKind::DotDot, "..", line, col);
        }

        // 单字符
        let s = c.to_string();
        let kind = match c {
            '(' => TokKind::LParen,
            ')' => TokKind::RParen,
            '{' => TokKind::LBrace,
            '}' => TokKind::RBrace,
            '[' => TokKind::LBracket,
            ']' => TokKind::RBracket,
            ',' => TokKind::Comma,
            ';' => TokKind::Semi,
            ':' => TokKind::Colon,
            '.' => TokKind::Dot,
            '+' => TokKind::Plus,
            '-' => TokKind::Minus,
            '*' => TokKind::Star,
            '/' => TokKind::Slash,
            '%' => TokKind::Percent,
            '=' => TokKind::Eq,
            '<' => TokKind::Lt,
            '>' => TokKind::Gt,
            '!' => TokKind::Bang,
            '?' => TokKind::Question,
            _ => TokKind::Error,
        };
        Token::new(kind, &s, line, col)
    }

    /// 分词全部 token
    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let t = self.next_token();
            let is_eof = t.kind == TokKind::Eof;
            tokens.push(t);
            if is_eof {
                break;
            }
        }
        tokens
    }
}

/// 标识符起始字符：字母或下划线或 UTF-8 多字节首字节
fn is_ident_start(c: char) -> bool {
    if c == '_' {
        return true;
    }
    if c.is_ascii_alphabetic() {
        return true;
    }
    // UTF-8 多字节字符首字节（>= 0xC0）
    let b = c as u32;
    b >= 0xC0
}

/// 标识符后续字符：字母数字下划线 + UTF-8 续字节 + UTF-8 首字节
fn is_ident_byte(c: char) -> bool {
    if c == '_' {
        return true;
    }
    if c.is_ascii_alphanumeric() {
        return true;
    }
    let b = c as u32;
    // UTF-8 续字节（10xxxxxx = 0x80..=0xBF）或首字节（>= 0xC0）
    (0x80..=0xBF).contains(&b) || b >= 0xC0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tok(src: &str) -> Vec<TokKind> {
        let mut lex = Lexer::new(src);
        lex.tokenize().into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn test_empty() {
        assert_eq!(tok(""), vec![TokKind::Eof]);
    }

    #[test]
    fn test_int() {
        assert_eq!(tok("42"), vec![TokKind::IntLit, TokKind::Eof]);
    }

    #[test]
    fn test_string() {
        assert_eq!(tok("\"hi\""), vec![TokKind::StringLit, TokKind::Eof]);
    }

    #[test]
    fn test_keyword_func() {
        let kinds = tok("func");
        assert_eq!(kinds, vec![TokKind::KwFunc, TokKind::Eof]);
    }

    #[test]
    fn test_keyword_fn() {
        assert_eq!(tok("fn"), vec![TokKind::KwFunc, TokKind::Eof]);
    }

    #[test]
    fn test_keyword_hanshu() {
        assert_eq!(tok("hanshu"), vec![TokKind::KwFunc, TokKind::Eof]);
    }

    #[test]
    fn test_keyword_hs() {
        assert_eq!(tok("hs"), vec![TokKind::KwFunc, TokKind::Eof]);
    }

    #[test]
    fn test_keyword_package() {
        // pkg / baozhuang / bz 都是 package 的等价形态（词法层归一化为 KwPackage）
        let kinds = tok("package pkg baozhuang bz foo");
        assert_eq!(kinds, vec![TokKind::KwPackage, TokKind::KwPackage, TokKind::KwPackage, TokKind::KwPackage, TokKind::Ident, TokKind::Eof]);
    }

    #[test]
    fn test_operators() {
        assert_eq!(tok("+ - * /"), vec![TokKind::Plus, TokKind::Minus, TokKind::Star, TokKind::Slash, TokKind::Eof]);
    }

    #[test]
    fn test_arrow() {
        assert_eq!(tok("->"), vec![TokKind::Arrow, TokKind::Eof]);
    }

    #[test]
    fn test_comments() {
        // 行注释
        let kinds = tok("// comment\n42");
        assert_eq!(kinds, vec![TokKind::IntLit, TokKind::Eof]);
        // 块注释
        let kinds = tok("/* hi */42");
        assert_eq!(kinds, vec![TokKind::IntLit, TokKind::Eof]);
    }

    #[test]
    fn test_string_interp_preserved() {
        // 字符串插值在词法层原样保留，运行时再切分
        let kinds = tok("\"hi ${name}\"");
        assert_eq!(kinds, vec![TokKind::StringLit, TokKind::Eof]);
    }

    #[test]
    fn test_chinese_ident() {
        // 中文字符作为标识符
        let kinds = tok("中文变量 42");
        assert_eq!(kinds, vec![TokKind::Ident, TokKind::IntLit, TokKind::Eof]);
    }
}
