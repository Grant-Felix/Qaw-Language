//! Qawc - Qaw 语言编译器入口
//!
//! 子命令：
//! - lex <file>       词法分析并打印 Token
//! - parse <file>     解析为 AST 并打印
//! - run <file>       解释执行
//! - build <file>     编译为原生可执行文件（C 后端）
//!
//! Crate 级别允许 dead_code：v0.1 故意"超规格"实现 AST / Token / Value API
//! （含未来 v0.5+ 才启用的字段与变体），当前解释器尚未全部使用。
//! 见 docs/2-版本更新一览.md 与 docs/book/ 蓝图。

#![allow(dead_code)]

use std::process::ExitCode;

mod lexer;
mod ast;
mod parser;
mod value;
mod env;
mod interpreter;
mod codegen;

use lexer::Lexer;
use parser::Parser;
use interpreter::Interpreter;

fn read_file(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("无法读取 {}: {}", path, e))
}

fn cmd_lex(path: &str) -> ExitCode {
    let src = match read_file(path) {
        Ok(s) => s,
        Err(e) => { eprintln!("{}", e); return ExitCode::from(1); }
    };
    let mut lex = Lexer::new(&src);
    println!("=== 词法分析: {} ===", path);
    for tok in lex.tokenize() {
        if tok.kind == lexer::TokKind::Eof { break; }
        println!("{:4}:{:<3}  {:<20}  {:?}", tok.line, tok.col, format!("{:?}", tok.kind), tok.lexeme);
    }
    ExitCode::SUCCESS
}

fn cmd_parse(path: &str) -> ExitCode {
    let src = match read_file(path) {
        Ok(s) => s,
        Err(e) => { eprintln!("{}", e); return ExitCode::from(1); }
    };
    let lex = Lexer::new(&src);
    let mut p = Parser::new(lex);
    let prog = p.parse_program();
    println!("=== 语法分析: {} ===", path);
    println!();
    println!("--- AST ---");
    ast::print_expr(&prog, 0);
    if let Some(err) = p.last_error() {
        eprintln!("\n解析错误: {}:{}: {}", err.line, err.col, err.message);
        ExitCode::from(1)
    } else {
        println!("\n=== 解析成功 ===");
        ExitCode::SUCCESS
    }
}

fn cmd_run(path: &str) -> ExitCode {
    let src = match read_file(path) {
        Ok(s) => s,
        Err(e) => { eprintln!("{}", e); return ExitCode::from(1); }
    };
    let lex = Lexer::new(&src);
    let mut p = Parser::new(lex);
    let prog = p.parse_program();
    if let Some(err) = p.last_error() {
        eprintln!("解析错误: {}:{}: {}", err.line, err.col, err.message);
        return ExitCode::from(1);
    }
    let mut interp = Interpreter::new();
    let result = interp.exec_program(&prog);
    match result.status {
        interpreter::EvalStatus::Ok | interpreter::EvalStatus::Return => ExitCode::SUCCESS,
        interpreter::EvalStatus::Error => {
            if let Some(err) = &result.error {
                eprintln!("运行时错误: {}", err);
            } else {
                eprintln!("运行时错误（未知）");
            }
            ExitCode::from(1)
        }
        interpreter::EvalStatus::Break => ExitCode::from(1),
    }
}

fn cmd_build(path: &str, _out_path: &str) -> ExitCode {
    // v0.1 POC 阶段：build 子命令尚未完整支持（只对最简单 hello.qaw 可用）。
    // 暂时直接调用解释器执行，以保证端到端可工作。
    eprintln!("[warn] build 子命令为 v0.5 MVP 占位实现，目前回退到 run（解释器）");
    cmd_run(path)
}

fn usage() {
    eprintln!(
        "用法: qawc <command> [options] <file>\n\
         \n\
         命令:\n\
         \n\
         lex <file>      词法分析并打印 Token\n\
         parse <file>    解析为 AST 并打印\n\
         run <file>      解析并执行（Tree-walking 解释器）\n\
         build <file>    编译为原生可执行文件（C 后端）\n\
         -o <path>       输出文件路径（仅 build）\n\
         version         打印版本\n\
         help            显示帮助\n\
         \n\
         示例:\n\
         \n\
         qawc run examples/hello.qaw\n\
         qawc build examples/hello.qaw -o hello"
    );
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        usage();
        return ExitCode::from(1);
    }

    match args[1].as_str() {
        "version" | "-V" | "--version" => {
            println!("qawc v0.5 MVP（纯 Rust 实现）");
            ExitCode::SUCCESS
        }
        "help" | "-h" | "--help" => {
            usage();
            ExitCode::SUCCESS
        }
        "lex" => {
            if args.len() < 3 { usage(); return ExitCode::from(1); }
            cmd_lex(&args[2])
        }
        "parse" | "check" => {
            if args.len() < 3 { usage(); return ExitCode::from(1); }
            cmd_parse(&args[2])
        }
        "run" => {
            if args.len() < 3 { usage(); return ExitCode::from(1); }
            cmd_run(&args[2])
        }
        "build" => {
            if args.len() < 3 { usage(); return ExitCode::from(1); }
            // 找 -o 参数
            let mut out = "a.out".to_string();
            let mut path_idx = 2;
            let mut i = 3;
            while i < args.len() {
                if args[i] == "-o" && i + 1 < args.len() {
                    out = args[i + 1].clone();
                    i += 2;
                } else {
                    path_idx = i;
                    i += 1;
                }
            }
            cmd_build(&args[path_idx], &out)
        }
        _ => {
            eprintln!("未知命令: {}", args[1]);
            usage();
            ExitCode::from(1)
        }
    }
}
