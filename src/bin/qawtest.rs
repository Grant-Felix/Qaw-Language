//! qawtest - Qaw 测试运行器（v0.18 第一版）
//!
//! 设计：Qaw 借鉴 Rust `#[test]` 属性机制，但采用极简实现。
//! 测试函数用 `// @test` 注释标记，紧随其后的 `func` 声明即测试函数。
//!
//! 子命令 / 参数：
//! - `qawtest`              扫描当前目录（"./"）所有 .qaw 文件
//! - `qawtest <dir>`         扫描指定目录下所有 .qaw 文件
//! - `qawtest <file.qaw>`    运行单个文件
//! - `--filter <substr>`     只跑名字包含 <substr> 的测试
//! - `--help` / `--version`
//!
//! 输出格式（与 Rust test 兼容）：
//! ```
//! running 3 tests
//! test test_addition ... ok
//! test test_subtraction ... FAILED: ...
//! test test_multiplication ... ok
//!
//! test result: 2 passed; 1 failed; 0 ignored
//! ```
//!
//! 实现策略：
//! 1. 文本扫描：`// @test` 注释 → 下一个 `func NAME(` → 测试声明位置；
//! 2. 解析：复用 crate 内 `Parser` 得到完整 AST；
//! 3. 执行：对每个测试函数，构造一个独立的 `Interpreter`，先 `exec_program`
//!    （剥离 `main` 后）以注册其它函数，再 `exec_stmt` 直接执行测试函数体；
//! 4. 状态映射：Ok/Return → pass；Error/Break → fail。
//!
//! 不支持的特性（v0.18 第一版）：
//! - 泛型参数化的测试
//! - `#[test]` 属性语法（仅 `// @test` 注释）
//! - setup/teardown 钩子
//! - 失败即中止模式（默认跑完所有测试）

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

// `src/bin/qawtest.rs` 是独立 binary target，其模块根在 `src/bin/`。
// 但 lexer/parser/... 实际位于 `src/`，需用 `#[path]` 重定向。
#[path = "../ast/mod.rs"]
mod ast;
#[path = "../env.rs"]
mod env;
#[path = "../interpreter.rs"]
mod interpreter;
#[path = "../lexer.rs"]
mod lexer;
#[path = "../parser/mod.rs"]
mod parser;
#[path = "../value.rs"]
mod value;

use crate::ast::{new_program, Expr, ExprData};
use crate::interpreter::{EvalStatus, Interpreter};
use crate::lexer::Lexer;
use crate::parser::Parser;

const VERSION: &str = "0.18.0";

fn usage() {
    print!(
        "qawtest {VERSION} — Qaw 测试运行器\n\
         \n\
         用法: qawtest [选项] [target]\n\
         \n\
         参数:\n\
         \n\
         \x20\x20<无>              扫描当前目录 (.) 所有 .qaw 文件\n\
         \x20\x20<dir>            扫描指定目录下所有 .qaw 文件\n\
         \x20\x20<file.qaw>       运行单个文件\n\
         \n\
         选项:\n\
         \n\
         \x20\x20--filter <substr> 只跑名字包含 <substr> 的测试\n\
         \x20\x20-h, --help        显示帮助\n\
         \x20\x20-v, --version     显示版本\n\
         \n\
         Qaw 测试语法：\n\
         \n\
         \x20\x20// @test\n\
         \x20\x20func test_add() {{\n\
         \x20\x20\x20\x20let r = add(2, 3);\n\
         \x20\x20\x20\x20if r != 5 {{ print(\"FAIL\"); }}\n\
         \x20\x20}}\n\
         \n\
         也可指定自定义名字：\n\
         \n\
         \x20\x20// @test name=\"加法测试\"\n\
         \x20\x20func test_addition_named() {{ ... }}\n\
         \n\
         示例:\n\
         \n\
         \x20\x20qawtest\n\
         \x20\x20qawtest examples\n\
         \x20\x20qawtest examples/calc.qaw\n\
         \x20\x20qawtest --filter addition\n"
    );
}

/// 单个测试声明（从文本注释中提取）。
///
/// - `name`: 用于显示 / `--filter` 匹配（可来自 `// @test name="..."`），
/// - `func_name`: 实际在 AST 中可查到的函数名（永远 = func 后面的标识符）。
#[derive(Debug, Clone)]
struct TestDecl {
    name: String,
    func_name: String,
    /// `func` 关键字所在的 1-based 行号（在源文件里）。
    func_line: u32,
}

/// 测试运行结果（用于输出）。
#[derive(Debug, Clone)]
struct TestOutcome {
    name: String,
    passed: bool,
    message: String,
}

/// 从源码中扫描 `// @test` 注释并定位紧随其后的 `func` 声明。
///
/// 规则（v0.18 简单版）：
/// - `// @test` 出现在行首（允许前导空白）；
/// - 也允许 `// @test` 出现在行尾（行内注释，如 `let _ = 1; // @test`）；
/// - 注释行可附带 `name="..."` 提供自定义名字（否则用函数名）；
/// - 紧跟其后（中间可有空白行）的第一个 `func NAME(` 声明即测试函数。
fn find_test_decls(source: &str) -> Vec<TestDecl> {
    let mut out: Vec<TestDecl> = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    for (idx, raw) in lines.iter().enumerate() {
        let line = raw.trim_start();
        if !is_test_marker(line) {
            continue;
        }
        let custom_name = parse_test_name_attr(line);

        // 向前找 func 声明（最多向下 32 行，避免误匹配到下一个测试）
        let max = lines.len().min(idx + 1 + 32);
        for (offset, raw_l) in lines.iter().enumerate().take(max).skip(idx + 1) {
            let l = raw_l.trim_start();
            if l.starts_with("func ") {
                if let Some(func_name) = extract_func_name(l) {
                    let name = custom_name
                        .clone()
                        .unwrap_or_else(|| func_name.clone());
                    out.push(TestDecl {
                        name,
                        func_name,
                        func_line: (offset + 1) as u32,
                    });
                }
                break;
            }
            // 注释行 / 空行可跳过；其他 token 视为分隔，停止搜索
            if !l.is_empty() && !l.starts_with("//") {
                break;
            }
        }
    }
    out
}

/// 判断一行是否为 `// @test` 标记（行首或行尾）。
fn is_test_marker(trimmed: &str) -> bool {
    if trimmed.starts_with("// @test") {
        return true;
    }
    if let Some(rest) = trimmed.strip_prefix("//") {
        // 行尾：`code; // @test`
        if rest.trim_start().starts_with("@test") {
            // 确保 @test 是 token 起始（前面不是字母数字下划线）
            let after_at = &rest.trim_start()[5..]; // "@test" 之后
            // 简化：行尾 @test 后通常跟空白 / 行末 / 属性
            if after_at.is_empty()
                || after_at.starts_with(' ')
                || after_at.starts_with('\t')
                || after_at.starts_with("name=")
            {
                return true;
            }
        }
    }
    false
}

/// 从 `// @test name="..."` 注解里提取自定义名字。
fn parse_test_name_attr(trimmed: &str) -> Option<String> {
    let body = trimmed.strip_prefix("//")?.trim_start();
    let rest = body.strip_prefix("@test")?.trim_start();
    if rest.is_empty() {
        return None;
    }
    // 期望形式: `name="..."` 或 `name='...'`
    let eq = rest.find('=')?;
    let key = rest[..eq].trim();
    if key != "name" {
        return None;
    }
    let value = rest[eq + 1..].trim();
    Some(value.trim_matches('"').trim_matches('\'').to_string())
}

/// 从 `func NAME(` 一行中提取函数名。
fn extract_func_name(line: &str) -> Option<String> {
    let rest = line.strip_prefix("func ")?.trim_start();
    let paren = rest.find('(')?;
    Some(rest[..paren].trim().to_string())
}

/// 把解析后的 Program 中所有名为 `main` 的函数剥离掉。
///
/// 这样 `Interpreter::exec_program` 不会运行 main（只用来注册其它函数），
/// 避免在每个测试执行时重复产生 main 的副作用。
fn strip_main(prog: &Expr) -> Expr {
    if let ExprData::Program(p) = &prog.data {
        let items: Vec<Expr> = p
            .items
            .iter()
            .filter(|it| !is_main_function(it))
            .cloned()
            .collect();
        return new_program(items, prog.line, prog.col);
    }
    prog.clone()
}

fn is_main_function(item: &Expr) -> bool {
    if let ExprData::Function(f) = &item.data {
        f.name == "main"
    } else {
        false
    }
}

/// 在解析后的 Program 中按名字找到函数体（Block 节点）。
///
/// 返回测试函数的 body（`ExprData::Block(...)` 节点），
/// 可直接交给 `Interpreter::exec_stmt` 执行。
fn find_function_body(prog: &Expr, name: &str) -> Option<Expr> {
    if let ExprData::Program(p) = &prog.data {
        for item in &p.items {
            if let ExprData::Function(f) = &item.data {
                if f.name == name {
                    return Some((*f.body).clone());
                }
            }
        }
    }
    None
}

/// 解析源码为 Program 节点；返回 (AST, parse_error_message)。
fn parse_source(source: &str) -> Result<Expr, String> {
    let lex = Lexer::new(source);
    let mut p = Parser::new(lex);
    let prog = p.parse_program();
    if let Some(err) = p.last_error() {
        return Err(format!("{}:{}: {}", err.line, err.col, err.message));
    }
    Ok(prog)
}

/// 执行单个测试。
///
/// `stripped` 是剥离 main 后的 Program（仅用于注册函数）。
/// `body` 是测试函数的 Block 节点。
///
/// 每个测试用全新 Interpreter，避免环境被上一个测试污染。
#[allow(unused_variables)]
fn run_test(prog: &Expr, stripped: &Expr, body: &Expr) -> TestOutcome {
    let mut interp = Interpreter::new();
    // 注册所有函数（剥离 main 后不会触发执行），失败结果（缺 main）可忽略。
    let _ = interp.exec_program(stripped);

    let result = interp.exec_stmt(body);
    match result.status {
        EvalStatus::Ok | EvalStatus::Return => TestOutcome {
            name: String::new(),
            passed: true,
            message: String::new(),
        },
        EvalStatus::Error => {
            let msg = result
                .error
                .map(|e| e.to_string())
                .unwrap_or_else(|| "未知错误".to_string());
            TestOutcome {
                name: String::new(),
                passed: false,
                message: msg,
            }
        }
        EvalStatus::Break => TestOutcome {
            name: String::new(),
            passed: false,
            message: "break 出现在循环外".to_string(),
        },
    }
}

/// 收集目录下的 .qaw 文件（按路径名排序）。
fn collect_qaw_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(dir)
        .map_err(|e| format!("无法读取目录 {}: {}", dir.display(), e))?
    {
        let entry = entry.map_err(|e| format!("遍历目录失败: {}", e))?;
        let p = entry.path();
        if p.is_file()
            && p.extension()
                .map(|e| e == "qaw")
                .unwrap_or(false)
        {
            out.push(p);
        }
    }
    out.sort();
    Ok(out)
}

/// 解析单个文件并返回 (Program, 测试声明列表)。错误用 String 描述。
fn scan_file(path: &Path) -> Result<(Expr, Vec<TestDecl>), String> {
    let source = fs::read_to_string(path)
        .map_err(|e| format!("无法读取 {}: {}", path.display(), e))?;
    let prog = parse_source(&source)?;
    let decls = find_test_decls(&source);
    // 二次校验：每个声明的函数名必须在 Program 中能找到对应的函数体
    // （避免用户在源码里写了 // @test 但 func 名拼错 / func 不存在）
    let mut valid = Vec::new();
    for d in decls {
        if find_function_body(&prog, &d.func_name).is_some() {
            valid.push(d);
        }
    }
    Ok((prog, valid))
}

/// 解析 CLI 参数。
struct CliOpts {
    target: Option<String>,
    filter: Option<String>,
    show_help: bool,
    show_version: bool,
}

fn parse_args(args: &[String]) -> Result<CliOpts, String> {
    let mut opts = CliOpts {
        target: None,
        filter: None,
        show_help: false,
        show_version: false,
    };
    let mut i = 1;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "-h" | "--help" => {
                opts.show_help = true;
                i += 1;
            }
            "-V" | "--version" => {
                opts.show_version = true;
                i += 1;
            }
            "--filter" => {
                if i + 1 >= args.len() {
                    return Err("--filter 需要参数".to_string());
                }
                opts.filter = Some(args[i + 1].clone());
                i += 2;
            }
            other if other.starts_with("--") || (other.starts_with('-') && other.len() > 1) => {
                return Err(format!("未知选项: {}", other));
            }
            _ => {
                if opts.target.is_some() {
                    return Err("只支持单个目标参数（目录或文件）".to_string());
                }
                opts.target = Some(args[i].clone());
                i += 1;
            }
        }
    }
    Ok(opts)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let opts = match parse_args(&args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("错误: {}", e);
            usage();
            return ExitCode::from(1);
        }
    };

    if opts.show_help {
        usage();
        return ExitCode::SUCCESS;
    }
    if opts.show_version {
        println!("qawtest {}", VERSION);
        return ExitCode::SUCCESS;
    }

    let target = opts.target.unwrap_or_else(|| ".".to_string());
    let path = Path::new(&target);

    if !path.exists() {
        eprintln!("错误: 路径不存在: {}", path.display());
        return ExitCode::from(1);
    }

    let files: Vec<PathBuf> = if path.is_dir() {
        match collect_qaw_files(path) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("错误: {}", e);
                return ExitCode::from(1);
            }
        }
    } else {
        vec![path.to_path_buf()]
    };

    if files.is_empty() {
        println!("running 0 tests");
        println!();
        println!("test result: 0 passed; 0 failed; 0 ignored");
        return ExitCode::SUCCESS;
    }

    // 收集所有测试
    let mut all: Vec<(PathBuf, Expr, Expr, Vec<TestDecl>)> = Vec::new();
    let mut parse_errors: Vec<String> = Vec::new();

    for f in &files {
        match scan_file(f) {
            Ok((prog, decls)) => {
                if !decls.is_empty() {
                    let stripped = strip_main(&prog);
                    all.push((f.clone(), prog, stripped, decls));
                }
            }
            Err(e) => parse_errors.push(format!("{}: {}", f.display(), e)),
        }
    }

    if !parse_errors.is_empty() {
        for e in &parse_errors {
            eprintln!("警告: {}", e);
        }
    }

    // 应用 filter
    struct PlannedTest {
        file: PathBuf,
        name: String,
    }
    let mut planned: Vec<PlannedTest> = Vec::new();
    for (file, _prog, _stripped, decls) in &all {
        for d in decls {
            let match_ok = match &opts.filter {
                Some(needle) => d.name.contains(needle.as_str()),
                None => true,
            };
            if match_ok {
                planned.push(PlannedTest {
                    file: file.clone(),
                    name: d.name.clone(),
                });
            }
        }
    }

    println!("running {} tests", planned.len());

    let mut outcomes: Vec<(PathBuf, TestOutcome)> = Vec::new();
    for t in &planned {
        // 找到对应的 prog/stripped
        let (_file, prog, stripped, decls) = all
            .iter()
            .find(|(file, _, _, _)| file == &t.file)
            .expect("file must exist");
        // 按名字取函数体（如果存在多个同名，取第一个）
        let body = decls
            .iter()
            .find(|d| d.name == t.name)
            .and_then(|d| find_function_body(prog, &d.func_name));

        let outcome = match body {
            Some(body) => {
                let mut r = run_test(prog, stripped, &body);
                r.name = t.name.clone();
                r
            }
            None => TestOutcome {
                name: t.name.clone(),
                passed: false,
                message: "找不到测试函数体".to_string(),
            },
        };
        outcomes.push((t.file.clone(), outcome));
    }

    let mut passed = 0usize;
    let mut failed = 0usize;
    for (_file, o) in &outcomes {
        if o.passed {
            println!("test {} ... ok", o.name);
            passed += 1;
        } else {
            println!("test {} ... FAILED: {}", o.name, o.message);
            failed += 1;
        }
    }

    println!();
    println!(
        "test result: {} passed; {} failed; 0 ignored",
        passed, failed
    );

    if failed > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

// ============ 单元测试（cargo test --bin qawtest）============

#[cfg(test)]
mod tests {
    use super::*;

    // ---- find_test_decls ----

    #[test]
    fn finds_simple_test() {
        let src = "\
func add(a: int, b: int) -> int { return a + b; }

// @test
func test_add() {
    let r = add(2, 3);
}
";
        let decls = find_test_decls(src);
        assert_eq!(decls.len(), 1, "should find one test, got {decls:?}");
        assert_eq!(decls[0].name, "test_add");
        assert_eq!(decls[0].func_name, "test_add");
        // `func test_add` 在第 4 行（1-based）：第 1 行 func add，第 2 行空，第 3 行 // @test，第 4 行 func test_add
        assert_eq!(decls[0].func_line, 4);
    }

    #[test]
    fn finds_multiple_tests() {
        let src = "\
// @test
func test_a() {}

// helper
func helper() {}

// @test
func test_b() {}
";
        let decls = find_test_decls(src);
        assert_eq!(decls.len(), 2, "should find two tests, got {decls:?}");
        let names: Vec<&str> = decls.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"test_a"));
        assert!(names.contains(&"test_b"));
    }

    #[test]
    fn empty_source_has_no_tests() {
        let decls = find_test_decls("");
        assert!(decls.is_empty());
    }

    #[test]
    fn no_test_marker_yields_nothing() {
        let src = "\
func add(a: int, b: int) -> int { return a + b; }

func main() {
    print(\"hi\");
}
";
        let decls = find_test_decls(src);
        assert!(decls.is_empty(), "no @test markers expected, got {decls:?}");
    }

    #[test]
    fn test_marker_without_func_is_ignored() {
        let src = "\
// @test
let x = 42;

func main() {}
";
        let decls = find_test_decls(src);
        assert!(decls.is_empty(), "@test without func should be ignored");
    }

    #[test]
    fn test_marker_with_blank_lines_between() {
        let src = "\
// @test

func test_x() {}
";
        let decls = find_test_decls(src);
        assert_eq!(decls.len(), 1, "blank lines between @test and func OK");
        assert_eq!(decls[0].name, "test_x");
    }

    #[test]
    fn test_marker_with_comment_between() {
        let src = "\
// @test
// setup
func test_x() {}
";
        let decls = find_test_decls(src);
        assert_eq!(decls.len(), 1, "comment line between @test and func OK");
        assert_eq!(decls[0].name, "test_x");
    }

    #[test]
    fn parse_test_name_attr_quoted_double() {
        let line = "// @test name=\"hello world\"";
        assert_eq!(
            parse_test_name_attr(line),
            Some("hello world".to_string())
        );
    }

    #[test]
    fn parse_test_name_attr_quoted_single() {
        let line = "// @test name='foo'";
        assert_eq!(parse_test_name_attr(line), Some("foo".to_string()));
    }

    #[test]
    fn parse_test_name_attr_no_name() {
        let line = "// @test";
        assert_eq!(parse_test_name_attr(line), None);
    }

    #[test]
    fn parse_test_name_attr_unknown_attr() {
        let line = "// @test ignore=\"yes\"";
        // 未知属性：当前实现返回 None（用函数名）
        assert_eq!(parse_test_name_attr(line), None);
    }

    #[test]
    fn custom_name_overrides_function_name() {
        let src = "\
// @test name=\"加法测试\"
func test_addition() {}
";
        let decls = find_test_decls(src);
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].name, "加法测试");
        // 实际函数名（用于 AST 查找）仍是 test_addition
        assert_eq!(decls[0].func_name, "test_addition");
    }

    #[test]
    fn is_test_marker_line_start() {
        assert!(is_test_marker("// @test"));
        assert!(is_test_marker("// @test name=\"x\""));
        assert!(!is_test_marker("// regular comment"));
        assert!(!is_test_marker("func foo() {}"));
    }

    #[test]
    fn extract_func_name_basic() {
        assert_eq!(extract_func_name("func foo() {}"), Some("foo".to_string()));
        assert_eq!(
            extract_func_name("func 中文函数() {}"),
            Some("中文函数".to_string())
        );
        assert_eq!(
            extract_func_name("func add(a: int, b: int) -> int { return 0; }"),
            Some("add".to_string())
        );
        assert_eq!(extract_func_name("not a func"), None);
    }

    // ---- strip_main ----

    #[test]
    fn strip_main_removes_main_function() {
        let prog = parse_source(
            "func helper() {}\nfunc main() { print(\"hi\"); }\n",
        )
        .unwrap();
        let stripped = strip_main(&prog);
        if let ExprData::Program(p) = &stripped.data {
            for item in &p.items {
                if let ExprData::Function(f) = &item.data {
                    assert_ne!(f.name, "main", "main must be stripped");
                }
            }
        } else {
            panic!("expected Program");
        }
    }

    #[test]
    fn strip_main_keeps_helpers() {
        let prog = parse_source(
            "func add(a: int, b: int) -> int { return a + b; }\nfunc main() {}\n",
        )
        .unwrap();
        let stripped = strip_main(&prog);
        if let ExprData::Program(p) = &stripped.data {
            let names: Vec<String> = p
                .items
                .iter()
                .filter_map(|it| {
                    if let ExprData::Function(f) = &it.data {
                        Some(f.name.clone())
                    } else {
                        None
                    }
                })
                .collect();
            assert!(names.contains(&"add".to_string()));
            assert!(!names.contains(&"main".to_string()));
        } else {
            panic!("expected Program");
        }
    }

    // ---- find_function_body ----

    #[test]
    fn find_function_body_returns_block() {
        let prog = parse_source("func foo() { let x = 1; }\n").unwrap();
        let body = find_function_body(&prog, "foo").expect("body");
        assert!(matches!(body.data, ExprData::Block(_)));
    }

    #[test]
    fn find_function_body_missing_returns_none() {
        let prog = parse_source("func foo() {}\n").unwrap();
        assert!(find_function_body(&prog, "bar").is_none());
    }

    // ---- run_test (end-to-end with interpreter) ----

    #[test]
    fn run_passing_test_returns_pass() {
        let src = "\
func add(a: int, b: int) -> int { return a + b; }

// @test
func test_add() {
    let r = add(2, 3);
    if r != 5 {
        print(\"FAIL\");
    }
}
";
        let prog = parse_source(src).unwrap();
        let stripped = strip_main(&prog);
        let body = find_function_body(&prog, "test_add").unwrap();
        let mut outcome = run_test(&prog, &stripped, &body);
        outcome.name = "test_add".to_string();
        assert!(outcome.passed, "expected pass, got {outcome:?}");
    }

    #[test]
    fn run_failing_test_returns_fail() {
        // 触发 EvalError（除零）应被报告为失败
        let src = "\
func div(a: int, b: int) -> int { return a / b; }

// @test
func test_div() {
    let r = div(1, 0);
}
";
        let prog = parse_source(src).unwrap();
        let stripped = strip_main(&prog);
        let body = find_function_body(&prog, "test_div").unwrap();
        let mut outcome = run_test(&prog, &stripped, &body);
        outcome.name = "test_div".to_string();
        assert!(!outcome.passed, "expected fail (div by zero), got {outcome:?}");
        assert!(
            outcome.message.contains("除数")
                || outcome.message.contains("零")
                || outcome.message.contains("zero"),
            "fail message should mention division: {}",
            outcome.message
        );
    }

    #[test]
    fn run_return_early_is_pass() {
        let src = "\
// @test
func test_return() {
    return;
}
";
        let prog = parse_source(src).unwrap();
        let stripped = strip_main(&prog);
        let body = find_function_body(&prog, "test_return").unwrap();
        let outcome = run_test(&prog, &stripped, &body);
        assert!(outcome.passed, "early return should be pass, got {outcome:?}");
    }

    #[test]
    fn run_unknown_function_call_is_fail() {
        let src = "\
// @test
func test_missing() {
    nonexistent_fn();
}
";
        let prog = parse_source(src).unwrap();
        let stripped = strip_main(&prog);
        let body = find_function_body(&prog, "test_missing").unwrap();
        let outcome = run_test(&prog, &stripped, &body);
        assert!(!outcome.passed, "unknown fn should fail");
        assert!(
            outcome.message.contains("nonexistent_fn")
                || outcome.message.contains("未找到"),
            "got: {}",
            outcome.message
        );
    }

    #[test]
    fn strip_main_prevents_main_side_effects() {
        // 含 main 的程序：剥离后注册函数应不触发 main 的执行。
        // 这里通过验证 Interpreter 状态来间接确认（main 会调用 print()），
        // 但更直接的是：exec_program(stripped) 返回 Error（无 main），不会执行 print。
        let src = "\
func main() {
    print(\"MAIN-RAN\");
}
";
        let prog = parse_source(src).unwrap();
        let stripped = strip_main(&prog);
        let mut interp = Interpreter::new();
        let r = interp.exec_program(&stripped);
        assert!(matches!(r.status, EvalStatus::Error));
        // 我们没办法直接断言 print 没执行（stdout），但 main 被剥离
        // 已足以验证 strip_main 正确性。
        if let ExprData::Program(p) = &stripped.data {
            let has_main = p.items.iter().any(|it| {
                if let ExprData::Function(f) = &it.data {
                    f.name == "main"
                } else {
                    false
                }
            });
            assert!(!has_main, "stripped program must not contain main");
        } else {
            panic!("expected Program");
        }
    }

    // ---- CLI 参数解析 ----

    #[test]
    fn parse_args_default() {
        let args: Vec<String> = vec!["qawtest".to_string()];
        let opts = parse_args(&args).unwrap();
        assert!(opts.target.is_none());
        assert!(opts.filter.is_none());
    }

    #[test]
    fn parse_args_with_target() {
        let args = vec!["qawtest".to_string(), "examples".to_string()];
        let opts = parse_args(&args).unwrap();
        assert_eq!(opts.target.as_deref(), Some("examples"));
    }

    #[test]
    fn parse_args_with_filter() {
        let args = vec![
            "qawtest".to_string(),
            "--filter".to_string(),
            "add".to_string(),
        ];
        let opts = parse_args(&args).unwrap();
        assert_eq!(opts.filter.as_deref(), Some("add"));
    }

    #[test]
    fn parse_args_filter_missing_value_errors() {
        let args = vec!["qawtest".to_string(), "--filter".to_string()];
        let r = parse_args(&args);
        assert!(r.is_err());
    }

    #[test]
    fn parse_args_unknown_flag_errors() {
        let args = vec!["qawtest".to_string(), "--bogus".to_string()];
        let r = parse_args(&args);
        assert!(r.is_err());
    }

    #[test]
    fn parse_args_two_targets_errors() {
        let args = vec![
            "qawtest".to_string(),
            "a".to_string(),
            "b".to_string(),
        ];
        let r = parse_args(&args);
        assert!(r.is_err());
    }

    // ---- 收集 .qaw 文件 ----

    #[test]
    fn collect_qaw_files_finds_qaw_extension() {
        let dir = std::env::temp_dir().join(format!(
            "qawtest-it-coll-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("a.qaw"), "func a() {}").unwrap();
        fs::write(dir.join("b.qaw"), "func b() {}").unwrap();
        fs::write(dir.join("c.txt"), "not qaw").unwrap();

        let files = collect_qaw_files(&dir).unwrap();
        let names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"a.qaw".to_string()));
        assert!(names.contains(&"b.qaw".to_string()));
        assert!(!names.contains(&"c.txt".to_string()));
        fs::remove_dir_all(&dir).ok();
    }
}