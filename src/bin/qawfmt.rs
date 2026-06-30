//! qawfmt - Qaw 代码格式化工具（v0.11 基础集）
//!
//! 子命令：
//! - <file>            格式化并输出到 stdout
//! - -i, --in-place    原地覆盖
//! - -d, --diff        只显示 unified diff（不修改）
//! - --check           检查是否需要格式化（exit 0/1）
//!
//! 格式化规则（v0.11 简单版）：
//!   1. 删除每行尾部空白
//!   2. 连续多个空行合并为单个空行
//!   3. 文件末尾保证单个换行符
//!
//! 不修改：缩进、注释内容、字符串字面量、标识符名。
//!
//! 实现策略：纯文本处理（不调用 parser/lexer）。

use std::fs;
use std::path::Path;
use std::process::ExitCode;

const VERSION: &str = "0.11.0";

fn usage() {
    print!(
        "qawfmt {VERSION} — Qaw 代码格式化工具\n\
         \n\
         用法: qawfmt [选项] <file>\n\
         \n\
         选项:\n\
         \n\
         \x20\x20-i, --in-place    原地覆盖\n\
         \x20\x20-d, --diff         只显示 diff，不修改\n\
         \x20\x20\x20\x20\x20\x20--check       检查是否需要格式化（exit 0/1）\n\
         \x20\x20-h, --help         显示帮助\n\
         \x20\x20-v, --version      显示版本\n\
         \n\
         格式化规则 (v0.11 基础集):\n\
         \n\
         \x20\x20• 删除行尾空白\n\
         \x20\x20• 合并连续空行\n\
         \x20\x20• 保证文件末尾单个换行\n\
         \n\
         示例:\n\
         \n\
         \x20\x20qawfmt examples/hello.qaw\n\
         \x20\x20qawfmt -i examples/hello.qaw\n\
         \x20\x20qawfmt -d examples/hello.qaw\n\
         \x20\x20qawfmt --check examples/hello.qaw\n"
    );
}

/// 核心格式化函数（纯文本）
///
/// 规则：
/// - 每行 `trim_end()` 去掉尾部空白
/// - 连续空行合并为单个空行
/// - 若结果非空且最后一行非空，则补一个换行（保证文件末尾单个换行符）
pub fn format_text(src: &str) -> String {
    let lines: Vec<&str> = src.lines().collect();
    let mut result: Vec<String> = Vec::new();
    let mut prev_blank = false;
    for line in &lines {
        let trimmed = line.trim_end();
        let is_blank = trimmed.is_empty();
        if is_blank && prev_blank {
            continue;
        }
        result.push(trimmed.to_string());
        prev_blank = is_blank;
    }
    // 文件末尾换行
    if !result.last().map(|l| l.is_empty()).unwrap_or(true) {
        result.push(String::new());
    }
    result.join("\n")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiffOp {
    Keep,
    Del,
    Add,
}

/// LCS 长度表（用于生成最小编辑脚本）
fn lcs_table(a: &[&str], b: &[&str]) -> Vec<Vec<usize>> {
    let n = a.len();
    let m = b.len();
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in 0..n {
        for j in 0..m {
            dp[i + 1][j + 1] = if a[i] == b[j] {
                dp[i][j] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }
    dp
}

/// 通过回溯 LCS 表生成 edit script
fn build_edits(a: &[&str], b: &[&str], dp: &[Vec<usize>]) -> Vec<DiffOp> {
    let mut ops = Vec::new();
    let mut i = a.len();
    let mut j = b.len();
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && a[i - 1] == b[j - 1] {
            ops.push(DiffOp::Keep);
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            ops.push(DiffOp::Add);
            j -= 1;
        } else {
            ops.push(DiffOp::Del);
            i -= 1;
        }
    }
    ops.reverse();
    ops
}

/// 渲染 unified diff（默认 3 行上下文）
///
/// 输出格式与 GNU `diff -u` 兼容：
/// ```text
/// --- a/path
/// +++ b/path
/// @@ -a_start,a_count +b_start,b_count @@
///  context
/// -removed
/// +added
/// ```
pub fn render_unified_diff(path: &str, src: &str, dst: &str) -> String {
    let a: Vec<&str> = src.lines().collect();
    let b: Vec<&str> = dst.lines().collect();
    let src_nl = src.ends_with('\n');
    let dst_nl = dst.ends_with('\n');

    // 特殊情况：行内容一致，仅末尾换行状态不同。
    // 显式渲染 "末尾补/删换行" 的差异，避免依赖 LCS（sentinel 会扰乱 hunk 计数）。
    if a == b && src_nl != dst_nl {
        return render_trailing_newline_diff(path, &a, !src_nl && dst_nl);
    }

    if a == b {
        return String::new();
    }

    let dp = lcs_table(&a, &b);
    let ops = build_edits(&a, &b, &dp);
    let n = ops.len();

    let _ = src_nl;
    let _ = dst_nl;

    // pos_a[i] = 应用 ops[0..i] 之前在 `a` 中的 1-based 行号
    let mut pos_a = vec![0usize; n + 1];
    let mut pos_b = vec![0usize; n + 1];
    pos_a[0] = 1;
    pos_b[0] = 1;
    for i in 0..n {
        pos_a[i + 1] = pos_a[i];
        pos_b[i + 1] = pos_b[i];
        match ops[i] {
            DiffOp::Keep => {
                pos_a[i + 1] += 1;
                pos_b[i + 1] += 1;
            }
            DiffOp::Del => pos_a[i + 1] += 1,
            DiffOp::Add => pos_b[i + 1] += 1,
        }
    }

    const CONTEXT: usize = 3;
    let mut out = String::new();
    out.push_str(&format!("--- {}\n", path));
    out.push_str(&format!("+++ {}（qawfmt 格式化后）\n", path));

    let mut i = 0;
    while i < n {
        if ops[i] == DiffOp::Keep {
            i += 1;
            continue;
        }

        // 向后扩展 context
        let mut start = i;
        let mut back = 0;
        while start > 0 && back < CONTEXT && ops[start - 1] == DiffOp::Keep {
            start -= 1;
            back += 1;
        }

        // 向前跳过所有变更
        let mut end = i;
        while end < n && ops[end] != DiffOp::Keep {
            end += 1;
        }
        // 向前扩展 context
        let mut fwd = 0;
        while end < n && fwd < CONTEXT && ops[end] == DiffOp::Keep {
            end += 1;
            fwd += 1;
        }

        let a_start = pos_a[start];
        let a_count = pos_a[end] - pos_a[start];
        let b_start = pos_b[start];
        let b_count = pos_b[end] - pos_b[start];
        out.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            a_start, a_count, b_start, b_count
        ));

        for k in start..end {
            match ops[k] {
                DiffOp::Keep => {
                    let line = a[pos_a[k] - 1];
                    out.push_str(&format!(" {}\n", line));
                }
                DiffOp::Del => {
                    let line = a[pos_a[k] - 1];
                    out.push_str(&format!("-{}\n", line));
                }
                DiffOp::Add => {
                    let line = b[pos_b[k] - 1];
                    out.push_str(&format!("+{}\n", line));
                }
            }
        }

        i = end;
    }
    out
}

fn read_source(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("无法读取 {}: {}", path.display(), e))
}

/// 渲染"仅末尾换行缺失/多余"的 diff（行内容相同，仅末尾 \n 状态不同）
///
/// `adding` 为 true：源缺末尾 \n，目标有 → 在末尾插入一个空行
/// `adding` 为 false：源多余末尾 \n → 删除一个空行（罕见）
fn render_trailing_newline_diff(path: &str, lines: &[&str], adding: bool) -> String {
    let header = format!("--- {}\n+++ {}（qawfmt 格式化后）\n", path, path);
    let n = lines.len();
    if n == 0 {
        // 文件完全为空但末尾换行状态不同
        let (a_n, b_n) = if adding { (0, 1) } else { (1, 0) };
        return format!("{}@@ -{},0 +{},{} @@\n", header, a_n, b_n, 1);
    }
    let last_line = lines.last().copied().unwrap_or("");
    if adding {
        // 源: n 行，无末尾 \n → 目标: n 行 + 末尾 \n
        // hunk header：@@ -n,1 +n,2 @@ 作用在最后一行
        format!(
            "{}@@ -{},1 +{},2 @@\n {}\n+\n",
            header,
            n,
            n + 1,
            last_line
        )
    } else {
        // 源: n+1 行（含末尾空行）→ 目标: n 行
        format!(
            "{}@@ -{},2 +{},1 @@\n-{}\n {}\n",
            header,
            n + 1,
            n,
            "",
            last_line
        )
    }
}

fn cmd_stdout(path: &Path) -> ExitCode {
    let src = match read_source(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("错误: {}", e);
            return ExitCode::from(1);
        }
    };
    let formatted = format_text(&src);
    print!("{}", formatted);
    ExitCode::SUCCESS
}

fn cmd_in_place(path: &Path) -> ExitCode {
    let src = match read_source(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("错误: {}", e);
            return ExitCode::from(1);
        }
    };
    let formatted = format_text(&src);
    if let Err(e) = fs::write(path, &formatted) {
        eprintln!("错误: 无法写入 {}: {}", path.display(), e);
        return ExitCode::from(1);
    }
    if formatted == src {
        println!("{} 无需变更（已是格式化格式）", path.display());
    } else {
        println!("{} 已原地格式化", path.display());
    }
    ExitCode::SUCCESS
}

fn cmd_check(path: &Path) -> ExitCode {
    let src = match read_source(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("错误: {}", e);
            return ExitCode::from(1);
        }
    };
    let formatted = format_text(&src);
    if formatted == src {
        println!("{} 无需格式化", path.display());
        ExitCode::SUCCESS
    } else {
        println!("{} 需要格式化", path.display());
        ExitCode::from(1)
    }
}

fn cmd_diff(path: &Path) -> ExitCode {
    let src = match read_source(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("错误: {}", e);
            return ExitCode::from(1);
        }
    };
    let formatted = format_text(&src);
    let diff = render_unified_diff(&path.display().to_string(), &src, &formatted);
    if diff.is_empty() {
        return ExitCode::SUCCESS;
    }
    print!("{}", diff);
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        usage();
        return ExitCode::from(1);
    }

    let mut in_place = false;
    let mut diff_only = false;
    let mut check_only = false;
    let mut path_arg: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        let a = args[i].clone();
        match a.as_str() {
            "-h" | "--help" => {
                usage();
                return ExitCode::SUCCESS;
            }
            "-V" | "-v" | "--version" => {
                println!("qawfmt {}", VERSION);
                return ExitCode::SUCCESS;
            }
            "-i" | "--in-place" => {
                in_place = true;
                i += 1;
            }
            "-d" | "--diff" => {
                diff_only = true;
                i += 1;
            }
            "--check" => {
                check_only = true;
                i += 1;
            }
            other if other.starts_with("--") => {
                eprintln!("错误: 未知选项 `{}`", other);
                usage();
                return ExitCode::from(1);
            }
            other if other.starts_with('-') && other.len() > 1 => {
                eprintln!("错误: 未知选项 `{}`", other);
                usage();
                return ExitCode::from(1);
            }
            _ => {
                if path_arg.is_some() {
                    eprintln!("错误: 只支持单个文件参数");
                    usage();
                    return ExitCode::from(1);
                }
                path_arg = Some(a);
                i += 1;
            }
        }
    }

    let path_str = match path_arg {
        Some(p) => p,
        None => {
            eprintln!("错误: 缺少文件参数");
            usage();
            return ExitCode::from(1);
        }
    };
    let path = Path::new(&path_str);

    if check_only {
        return cmd_check(path);
    }
    if diff_only {
        return cmd_diff(path);
    }
    if in_place {
        return cmd_in_place(path);
    }
    cmd_stdout(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_trailing_whitespace() {
        let src = "foo   \nbar\t\nbaz\n";
        let out = format_text(src);
        assert_eq!(out, "foo\nbar\nbaz\n");
    }

    #[test]
    fn collapses_multiple_blank_lines() {
        let src = "a\n\n\n\nb\n";
        let out = format_text(src);
        assert_eq!(out, "a\n\nb\n");
    }

    #[test]
    fn ensures_trailing_newline() {
        let out = format_text("foo");
        assert_eq!(out, "foo\n");
    }

    #[test]
    fn preserves_well_formed_source() {
        let src = "func main() {\n    print(\"hi\");\n}\n";
        let out = format_text(src);
        assert_eq!(out, src);
    }

    #[test]
    fn idempotent_on_already_formatted_input() {
        let src = "a  \n\n\nb\n\nc   \n";
        let once = format_text(src);
        let twice = format_text(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn idempotent_on_well_formed_input() {
        let src = "func main() {\n    print(\"hi\");\n}\n";
        let once = format_text(src);
        let twice = format_text(&once);
        assert_eq!(once, twice);
        assert_eq!(once, src);
    }

    #[test]
    fn empty_input_stays_empty() {
        let out = format_text("");
        assert_eq!(out, "");
    }

    #[test]
    fn handles_unicode_content() {
        let src = "// 你好   \nfunc main() {\n    print(\"世界! 🌍\");\n}\n";
        let out = format_text(src);
        assert_eq!(out, "// 你好\nfunc main() {\n    print(\"世界! 🌍\");\n}\n");
    }

    #[test]
    fn diff_identical_inputs_is_empty() {
        let src = "foo\nbar\n";
        let out = render_unified_diff("test.qaw", src, src);
        assert_eq!(out, "");
    }

    #[test]
    fn diff_shows_single_change() {
        let src = "foo\nbar\n";
        let dst = "foo\nbaz\n";
        let out = render_unified_diff("test.qaw", src, dst);
        assert!(out.contains("--- test.qaw\n"));
        assert!(out.contains("+++ test.qaw"));
        assert!(out.contains("-bar"));
        assert!(out.contains("+baz"));
        assert!(out.contains("@@ -1,2 +1,2 @@"));
    }

    #[test]
    fn diff_handles_addition() {
        let src = "a\nb\n";
        let dst = "a\nb\nc\n";
        let out = render_unified_diff("t.qaw", src, dst);
        assert!(out.contains("+c"));
        assert!(out.contains("@@ -1,2 +1,3 @@"));
    }

    #[test]
    fn diff_handles_deletion() {
        let src = "a\nb\nc\n";
        let dst = "a\nc\n";
        let out = render_unified_diff("t.qaw", src, dst);
        assert!(out.contains("-b"));
        assert!(out.contains("@@ -1,3 +1,2 @@"));
    }

    #[test]
    fn diff_handles_missing_trailing_newline() {
        // 源无末尾换行，目标有末尾换行 — diff 应能体现差异
        let src = "a\nb";
        let dst = "a\nb\n";
        let out = render_unified_diff("t.qaw", src, dst);
        assert!(out.contains("+"), "expected an insertion; got: {out:?}");
    }

    #[test]
    fn diff_silent_when_only_trailing_newline_matches() {
        // 两个输入都以 '\n' 结尾且 lines() 一致 → 无 diff
        let src = "a\nb\n";
        let dst = "a\nb\n";
        let out = render_unified_diff("t.qaw", src, dst);
        assert_eq!(out, "");
    }
}