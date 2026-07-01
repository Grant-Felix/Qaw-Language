//! qawdoc — Qaw 文档生成器（v0.18）
//!
//! 用法:
//!   qawdoc <file.qaw> [-o out.md]
//!   qawdoc <dir> [--recursive]
//!
//! 行为：
//! - 读取 .qaw 源文件
//! - 提取 `///` 起始的连续 doc 注释块
//! - 关联到紧随其后的顶层声明（`func` / `struct` / `enum` / `let` / `var` / `const`，含四形制关键字）
//! - 输出 Markdown（项目标题 + Functions / Types / Constants / Variables 分节）
//!
//! 实现策略：纯文本扫描，不调用 lexer/parser。最小化、确定性、可单元测试。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const VERSION: &str = "0.18.0";

// ---------- 数据模型 ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclKind {
    Function,
    Struct,
    Enum,
    Let,
    Var,
    Const,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Decl {
    pub kind: DeclKind,
    pub name: String,
    pub doc: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocFile {
    pub project_name: String,
    pub decls: Vec<Decl>,
}

// ---------- 帮助文本 ----------

fn usage() {
    print!(
        "qawdoc {VERSION} — Qaw 文档生成器\n\
         \n\
         用法: qawdoc <file.qaw> [-o out.md]\n\
         \x20\x20    qawdoc <dir> [--recursive]\n\
         \n\
         选项:\n\
         \n\
         \x20\x20-o <file>      输出到文件（默认 stdout）\n\
         \x20\x20--recursive   递归处理目录\n\
         \x20\x20-h, --help    显示帮助\n\
         \x20\x20-v, --version 显示版本\n\
         \n\
         语法:\n\
         \n\
         \x20\x20/// 文档注释（紧跟 func/struct/enum/let/var/const 声明）\n\
         \n\
         支持关键字（含四形制）：\n\
         \n\
         \x20\x20func / fn / hanshu / hs\n\
         \x20\x20struct / jiegou / jg\n\
         \x20\x20enum / meiju / mj\n\
         \x20\x20let / buke / bk\n\
         \x20\x20var / bianliang / bl\n\
         \x20\x20const / con / changliang / cl\n"
    );
}

// ---------- 文本解析（公开以供测试） ----------

/// trimmed 行是否以 `///` 开头
pub fn is_doc_line(line: &str) -> bool {
    line.trim_start().starts_with("///")
}

/// 取 doc 注释行 `///` 之后的内容（去掉前缀 + 一个可选空格）
fn doc_line_content(line: &str) -> String {
    let trimmed = line.trim_start();
    let after = trimmed.strip_prefix("///").unwrap_or("");
    after.strip_prefix(' ').unwrap_or(after).to_string()
}

/// 提取从 `start` 起的一段连续 doc 注释块。
/// 返回 (内容行列表, 结束索引-不含)
pub fn extract_doc_block(lines: &[&str], start: usize) -> Option<(Vec<String>, usize)> {
    if start >= lines.len() || !is_doc_line(lines[start]) {
        return None;
    }
    let mut out = Vec::new();
    let mut i = start;
    while i < lines.len() && is_doc_line(lines[i]) {
        out.push(doc_line_content(lines[i]));
        i += 1;
    }
    Some((out, i))
}

/// 第一行是否形如 `<keyword> <ident>` 的顶层声明。
/// 仅识别关键字首 token，名称必须以字母/下划线开头。
pub fn parse_decl(line: &str) -> Option<(DeclKind, String)> {
    let kw = first_token(line)?;
    let rest = line[kw.len()..].trim_start();
    let name = first_ident(rest)?;
    let kind = match kw {
        "func" | "fn" | "hanshu" | "hs" => DeclKind::Function,
        "struct" | "jiegou" | "jg" => DeclKind::Struct,
        "enum" | "meiju" | "mj" => DeclKind::Enum,
        "let" | "buke" | "bk" => DeclKind::Let,
        "var" | "bianliang" | "bl" => DeclKind::Var,
        "const" | "con" | "changliang" | "cl" => DeclKind::Const,
        _ => return None,
    };
    Some((kind, name.to_string()))
}

fn first_token(line: &str) -> Option<&str> {
    let s = line.trim_start();
    if s.is_empty() {
        return None;
    }
    let end = s
        .char_indices()
        .find(|(_, c)| c.is_whitespace() || *c == '(' || *c == ':' || *c == '=' || *c == '<' || *c == '{')
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    if end == 0 {
        None
    } else {
        Some(&s[..end])
    }
}

fn first_ident(s: &str) -> Option<String> {
    let s = s.trim_start();
    if s.is_empty() {
        return None;
    }
    let mut end = 0usize;
    let mut chars = s.char_indices();
    let (_, first) = chars.next()?;
    if !(first.is_alphabetic() || first == '_') {
        return None;
    }
    end += first.len_utf8();
    for (i, c) in chars {
        if !(c.is_alphanumeric() || c == '_') {
            break;
        }
        end = i + c.len_utf8();
    }
    Some(s[..end].to_string())
}

/// 一行中 `{` 与 `}` 的数量（用于顶层 brace depth 跟踪）。
/// 简单实现：不解析字符串/注释；已知 Qaw 示例文件不会触发歧义。
fn count_braces(line: &str) -> (i32, i32) {
    let mut open = 0;
    let mut close = 0;
    for c in line.chars() {
        if c == '{' {
            open += 1;
        } else if c == '}' {
            close += 1;
        }
    }
    (open, close)
}

/// 从源文本解析出一个 DocFile。
/// `project_name` 用于 Markdown 标题。
pub fn parse_file(source: &str, project_name: &str) -> DocFile {
    let lines: Vec<&str> = source.lines().collect();
    let mut decls: Vec<Decl> = Vec::new();
    let mut brace_depth: i32 = 0;
    let mut i = 0;
    while i < lines.len() {
        let raw = lines[i];
        let trimmed = raw.trim_start();

        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        if is_doc_line(trimmed) {
            let (doc_lines, end) = match extract_doc_block(&lines, i) {
                Some(v) => v,
                None => {
                    i += 1;
                    continue;
                }
            };
            // 跳过空行寻找下一行声明
            let mut j = end;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }
            if j < lines.len() {
                let next = lines[j].trim_start();
                if let Some((kind, name)) = parse_decl(next) {
                    if brace_depth == 0 {
                        let doc = doc_lines.join("\n");
                        decls.push(Decl {
                            kind,
                            name,
                            doc: doc.trim().to_string(),
                        });
                    }
                    let (o, c) = count_braces(lines[j]);
                    brace_depth += o - c;
                    if brace_depth < 0 {
                        brace_depth = 0;
                    }
                    i = j + 1;
                    continue;
                }
            }
            // doc 块后面没有匹配的顶层声明 → 丢弃
            i = end;
            continue;
        }

        // 普通行：维护 brace depth
        let (o, c) = count_braces(raw);
        brace_depth += o - c;
        if brace_depth < 0 {
            brace_depth = 0;
        }
        i += 1;
    }
    DocFile {
        project_name: project_name.to_string(),
        decls,
    }
}

// ---------- Markdown 渲染 ----------

pub fn render_markdown(file: &DocFile) -> String {
    let mut out = String::new();
    let title = if file.project_name.is_empty() {
        "Untitled"
    } else {
        file.project_name.as_str()
    };
    out.push_str(&format!("# {title}\n\n"));

    // 按节分组（保持出现顺序）
    let mut sections: [Vec<&Decl>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let order = |k: DeclKind| -> usize {
        match k {
            DeclKind::Function => 0,
            DeclKind::Struct | DeclKind::Enum => 1,
            DeclKind::Const => 2,
            DeclKind::Let | DeclKind::Var => 3,
        }
    };
    for d in &file.decls {
        sections[order(d.kind)].push(d);
    }

    let titles = ["Functions", "Types", "Constants", "Variables"];
    for idx in 0..4 {
        if sections[idx].is_empty() {
            continue;
        }
        out.push_str(&format!("## {}\n\n", titles[idx]));
        for d in &sections[idx] {
            out.push_str(&format!("### {}\n\n", d.name));
            if !d.doc.is_empty() {
                out.push_str(&d.doc);
                out.push_str("\n\n");
            }
        }
    }
    out
}

// ---------- 文件收集 ----------

fn collect_qaw_files(path: &Path, recursive: bool) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if path.is_file() {
        if is_qaw(path) {
            out.push(path.to_path_buf());
        }
        return out;
    }
    if !path.is_dir() {
        return out;
    }
    let entries = match fs::read_dir(path) {
        Ok(e) => e,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_file() {
            if is_qaw(&p) {
                out.push(p);
            }
        } else if recursive && p.is_dir() {
            out.extend(collect_qaw_files(&p, recursive));
        }
    }
    out.sort();
    out
}

fn is_qaw(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()) == Some("qaw")
}

fn project_name_from(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string()
}

fn render_path(path: &Path, recursive: bool) -> Result<String, String> {
    let files = collect_qaw_files(path, recursive);
    if files.is_empty() {
        if path.is_file() {
            return Err(format!("不是 .qaw 文件: {}", path.display()));
        }
        return Err(format!("目录中没有 .qaw 文件: {}", path.display()));
    }
    let mut out = String::new();
    for (i, file) in files.iter().enumerate() {
        if i > 0 {
            out.push_str("\n---\n\n");
        }
        let source = fs::read_to_string(file)
            .map_err(|e| format!("无法读取 {}: {}", file.display(), e))?;
        let project = project_name_from(file);
        let doc = parse_file(&source, &project);
        out.push_str(&render_markdown(&doc));
    }
    Ok(out)
}

// ---------- CLI 入口 ----------

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("用法: qawdoc <file.qaw> [-o out.md] [--recursive] [dir]");
        eprintln!("  --help     显示帮助");
        eprintln!("  --version  显示版本");
        return ExitCode::from(1);
    }

    let mut output_path: Option<String> = None;
    let mut recursive = false;
    let mut input: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                usage();
                return ExitCode::SUCCESS;
            }
            "-v" | "-V" | "--version" => {
                println!("qawdoc {VERSION}");
                return ExitCode::SUCCESS;
            }
            "-o" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("错误: -o 需要参数");
                    return ExitCode::from(1);
                }
                output_path = Some(args[i].clone());
            }
            "--recursive" | "-r" => {
                recursive = true;
            }
            a if a.starts_with('-') => {
                eprintln!("错误: 未知选项 {a}");
                return ExitCode::from(1);
            }
            _ => {
                if input.is_some() {
                    eprintln!("错误: 只能指定一个输入");
                    return ExitCode::from(1);
                }
                input = Some(args[i].clone());
            }
        }
        i += 1;
    }

    let input = match input {
        Some(p) => p,
        None => {
            eprintln!("错误: 缺少输入文件或目录");
            return ExitCode::from(1);
        }
    };

    let path = PathBuf::from(&input);
    let markdown = match render_path(&path, recursive) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("错误: {e}");
            return ExitCode::from(1);
        }
    };

    match output_path {
        Some(out) => {
            if let Err(e) = fs::write(&out, &markdown) {
                eprintln!("错误: 无法写入 {out}: {e}");
                return ExitCode::from(1);
            }
            println!("已写入 {out}");
        }
        None => {
            print!("{markdown}");
        }
    }
    ExitCode::SUCCESS
}

// ---------- 单元测试 ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_block_extraction_handles_indentation_and_blank_separator() {
        let src = "/// Hello\n/// World\n\nfunc main() {}\n";
        let lines: Vec<&str> = src.lines().collect();
        let (doc, end) = extract_doc_block(&lines, 0).expect("doc block");
        assert_eq!(doc, vec!["Hello", "World"]);
        assert_eq!(end, 2);
    }

    #[test]
    fn doc_block_with_blank_paragraph_break_preserves_separator() {
        let src = "/// Para 1\n///\n/// Para 2\nfunc main() {}\n";
        let lines: Vec<&str> = src.lines().collect();
        let (doc, _end) = extract_doc_block(&lines, 0).unwrap();
        assert_eq!(doc, vec!["Para 1", "", "Para 2"]);
        let joined = doc.join("\n");
        assert!(joined.contains("Para 1\n\nPara 2"));
    }

    #[test]
    fn parse_decl_recognizes_all_six_kinds_and_aliases() {
        let cases = [
            ("func main() {}", DeclKind::Function, "main"),
            ("fn foo()", DeclKind::Function, "foo"),
            ("hs bar()", DeclKind::Function, "bar"),
            ("struct Point { x: int, }", DeclKind::Struct, "Point"),
            ("jg Pt", DeclKind::Struct, "Pt"),
            ("enum Color { Red, }", DeclKind::Enum, "Color"),
            ("mj Mood", DeclKind::Enum, "Mood"),
            ("let x = 1;", DeclKind::Let, "x"),
            ("bk y = 2", DeclKind::Let, "y"),
            ("var z: int = 0;", DeclKind::Var, "z"),
            ("bl w", DeclKind::Var, "w"),
            ("const PI = 3;", DeclKind::Const, "PI"),
            ("cl E", DeclKind::Const, "E"),
        ];
        for (line, exp_kind, exp_name) in cases {
            let (k, n) = parse_decl(line).unwrap_or_else(|| panic!("parse failed: {line}"));
            assert_eq!(k, exp_kind, "kind for {line}");
            assert_eq!(n, exp_name, "name for {line}");
        }
    }

    #[test]
    fn parse_decl_rejects_non_declarations() {
        assert!(parse_decl("// not a decl").is_none());
        assert!(parse_decl("return 0;").is_none());
        assert!(parse_decl("if x {").is_none());
        assert!(parse_decl("match c {").is_none());
        assert!(parse_decl("let").is_none());
        assert!(parse_decl("let 1abc = 1;").is_none()); // 名称不能以数字开头
    }

    #[test]
    fn parse_file_associates_doc_to_next_decl_and_skips_orphan_docs() {
        let src = "\
/// 主程序入口
func main() {
    /// 内部 doc（应在函数体内被忽略）
    let inner = 1;
    print(inner);
}

/// 孤儿注释（无声明）— 之后没有声明，应被丢弃
/// 第二行
";
        let doc = parse_file(src, "demo");
        assert_eq!(doc.project_name, "demo");
        // 顶层只有 main 是 doc'd 声明；inner 在 main 函数体内被忽略；末尾孤儿 doc 被丢弃
        assert_eq!(doc.decls.len(), 1);
        assert_eq!(doc.decls[0].name, "main");
        assert_eq!(doc.decls[0].doc, "主程序入口");
        // 任何 doc 都不应包含被丢弃的内容
        for d in &doc.decls {
            assert!(!d.doc.contains("内部 doc"));
            assert!(!d.doc.contains("孤儿注释"));
        }
    }

    #[test]
    fn parse_file_classifies_into_sections_correctly() {
        let src = "\
/// 函数 f
func f() {}

/// 结构体 S
struct S { x: int, }

/// 枚举 E
enum E { A, B, }

/// 常量 C
const C = 1;

/// 变量 v
var v: int = 0;

/// 不可变量 i
let i = 1;
";
        let doc = parse_file(src, "proj");
        assert_eq!(doc.decls.len(), 6);
        let kinds: Vec<DeclKind> = doc.decls.iter().map(|d| d.kind).collect();
        assert_eq!(
            kinds,
            vec![
                DeclKind::Function,
                DeclKind::Struct,
                DeclKind::Enum,
                DeclKind::Const,
                DeclKind::Var,
                DeclKind::Let,
            ]
        );
    }

    #[test]
    fn render_markdown_produces_expected_layout() {
        let file = DocFile {
            project_name: "demo".to_string(),
            decls: vec![
                Decl {
                    kind: DeclKind::Function,
                    name: "main".to_string(),
                    doc: "主程序入口".to_string(),
                },
                Decl {
                    kind: DeclKind::Struct,
                    name: "Point".to_string(),
                    doc: "点结构体".to_string(),
                },
            ],
        };
        let md = render_markdown(&file);
        assert!(md.starts_with("# demo\n\n"));
        assert!(md.contains("## Functions\n"));
        assert!(md.contains("### main\n"));
        assert!(md.contains("主程序入口"));
        assert!(md.contains("## Types\n"));
        assert!(md.contains("### Point\n"));
        assert!(md.contains("点结构体"));
    }

    #[test]
    fn render_markdown_uses_untitled_for_empty_project_name() {
        let file = DocFile {
            project_name: String::new(),
            decls: vec![],
        };
        let md = render_markdown(&file);
        assert!(md.contains("# Untitled"));
    }

    #[test]
    fn render_markdown_skips_empty_sections() {
        let file = DocFile {
            project_name: "x".to_string(),
            decls: vec![Decl {
                kind: DeclKind::Const,
                name: "PI".to_string(),
                doc: "圆周率".to_string(),
            }],
        };
        let md = render_markdown(&file);
        assert!(!md.contains("## Functions"));
        assert!(!md.contains("## Types"));
        assert!(md.contains("## Constants"));
        assert!(!md.contains("## Variables"));
    }

    #[test]
    fn collect_qaw_files_finds_files_in_directory() {
        let tmp = std::env::temp_dir().join(format!(
            "qawdoc-coll-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("a.qaw"), "/// a\nfunc a() {}\n").unwrap();
        fs::write(tmp.join("b.qaw"), "/// b\nfunc b() {}\n").unwrap();
        fs::write(tmp.join("c.txt"), "not qaw").unwrap();
        let files = collect_qaw_files(&tmp, false);
        let names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["a.qaw".to_string(), "b.qaw".to_string()]);
        fs::remove_dir_all(&tmp).ok();
    }
}