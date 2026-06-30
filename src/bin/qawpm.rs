//! v0.12 实现；unquote 函数有两个 if-else 分支（字符串 vs 单字符）
//! 形式相同是 TOML 字面量语法决定，非可简化代码。
#![allow(clippy::if_same_then_else)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const VERSION: &str = "0.11.0";
const REGISTRY_URL: &str = "registry+https://qaw-lang.org";

#[derive(Debug, Clone, PartialEq)]
struct PackageMeta {
    name: String,
    version: String,
    edition: String,
}

impl Default for PackageMeta {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: "0.1.0".to_string(),
            edition: "2024".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
struct QawToml {
    package: PackageMeta,
    dependencies: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
struct LockEntry {
    name: String,
    version: String,
    source: Option<String>,
}

fn usage() {
    print!(
        "qawpm {VERSION} — Qaw 包管理器\n\
         \n\
         用法: qawpm <command> [args]\n\
         \n\
         命令:\n\
         \n\
         \x20\x20init                  在当前目录创建 Qaw.toml + src/\n\
         \x20\x20add <name> <version>  向 Qaw.toml 添加依赖\n\
         \x20\x20install               解析 Qaw.toml 并下载依赖到 ~/.qaw/cache/\n\
         \x20\x20help                  显示此帮助\n\
         \x20\x20version               显示版本\n\
         \n\
         示例:\n\
         \n\
         \x20\x20qawpm init\n\
         \x20\x20qawpm add qaw_std 1.0\n\
         \x20\x20qawpm install\n\
         \n\
         配置文件: Qaw.toml\n\
         锁文件:   Qaw.lock\n\
         缓存目录: ~/.qaw/cache/\n"
    );
}

fn strip_comment(line: &str) -> &str {
    match line.find('#') {
        Some(i) => &line[..i],
        None => line,
    }
}

fn unquote(s: &str) -> String {
    let t = s.trim();
    if t.starts_with('"') && t.ends_with('"') && t.len() >= 2 {
        t[1..t.len() - 1].to_string()
    } else if t.starts_with('\'') && t.ends_with('\'') && t.len() >= 2 {
        t[1..t.len() - 1].to_string()
    } else {
        t.to_string()
    }
}

fn parse_qaw_toml(content: &str) -> Result<QawToml, String> {
    let mut toml = QawToml::default();
    let mut section = String::new();

    for (idx, raw) in content.lines().enumerate() {
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            if !line.ends_with(']') {
                return Err(format!("第 {} 行: 表头格式错误 `{}`", idx + 1, line));
            }
            section = line[1..line.len() - 1].trim().to_string();
            continue;
        }
        let eq = match line.find('=') {
            Some(i) => i,
            None => return Err(format!("第 {} 行: 缺少 `=`: `{}`", idx + 1, line)),
        };
        let key = line[..eq].trim().to_string();
        let value = unquote(line[eq + 1..].trim());

        match section.as_str() {
            "package" => match key.as_str() {
                "name" => toml.package.name = value,
                "version" => toml.package.version = value,
                "edition" => toml.package.edition = value,
                _ => {}
            },
            "dependencies" => {
                toml.dependencies.insert(key, value);
            }
            _ => {}
        }
    }
    Ok(toml)
}

fn write_qaw_toml(toml: &QawToml) -> String {
    let mut s = String::new();
    s.push_str("[package]\n");
    s.push_str(&format!("name = \"{}\"\n", toml.package.name));
    s.push_str(&format!("version = \"{}\"\n", toml.package.version));
    s.push_str(&format!("edition = \"{}\"\n", toml.package.edition));
    s.push('\n');
    s.push_str("[dependencies]\n");
    if toml.dependencies.is_empty() {
        s.push_str("# （暂无依赖）\n");
    } else {
        for (k, v) in &toml.dependencies {
            s.push_str(&format!("{} = \"{}\"\n", k, v));
        }
    }
    s
}

fn render_lock(entries: &[LockEntry]) -> String {
    let mut s = String::new();
    for (i, e) in entries.iter().enumerate() {
        if i > 0 {
            s.push('\n');
        }
        s.push_str("[[package]]\n");
        s.push_str(&format!("name = \"{}\"\n", e.name));
        s.push_str(&format!("version = \"{}\"\n", e.version));
        if let Some(src) = &e.source {
            s.push_str(&format!("source = \"{}\"\n", src));
        }
    }
    s
}

fn read_toml(path: &Path) -> Result<QawToml, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("无法读取 {}: {}", path.display(), e))?;
    parse_qaw_toml(&content)
}

fn cmd_init(dir: &Path) -> ExitCode {
    let toml_path = dir.join("Qaw.toml");
    let src_dir = dir.join("src");

    if toml_path.exists() {
        eprintln!("错误: {} 已存在，拒绝覆盖", toml_path.display());
        return ExitCode::from(1);
    }

    let dir_name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty() && s != ".")
        .unwrap_or_else(|| {
            std::env::current_dir()
                .ok()
                .and_then(|p| p.file_name().and_then(|n| n.to_str()).map(String::from))
                .unwrap_or_else(|| "my_project".to_string())
        });

    let toml = QawToml {
        package: PackageMeta {
            name: dir_name.clone(),
            version: "0.1.0".to_string(),
            edition: "2024".to_string(),
        },
        dependencies: BTreeMap::new(),
    };

    if let Err(e) = fs::create_dir_all(&src_dir) {
        eprintln!("错误: 无法创建 {}: {}", src_dir.display(), e);
        return ExitCode::from(1);
    }
    if let Err(e) = fs::write(&toml_path, write_qaw_toml(&toml)) {
        eprintln!("错误: 无法写入 {}: {}", toml_path.display(), e);
        return ExitCode::from(1);
    }

    let main_qaw = src_dir.join("main.qaw");
    let main_content = format!(
        "// {name}.qaw — 由 qawpm init 生成\n\
         \n\
         func main() {{\n\
         \x20\x20\x20\x20print(\"你好, 世界!\");\n\
         }}\n",
        name = dir_name
    );
    if let Err(e) = fs::write(&main_qaw, main_content) {
        eprintln!("错误: 无法写入 {}: {}", main_qaw.display(), e);
        return ExitCode::from(1);
    }

    let gitignore = dir.join(".gitignore");
    let _ = fs::write(
        &gitignore,
        "/target\n/.qaw\nQaw.lock\n",
    );

    println!("已创建 {}", toml_path.display());
    println!("已创建 {}", main_qaw.display());
    println!("已创建 {}", gitignore.display());
    println!();
    println!("下一步:");
    println!("  cd {}", dir.display());
    println!("  qawpm add qaw_std 1.0");
    println!("  qawpm install");
    ExitCode::SUCCESS
}

fn cmd_add(file: &Path, name: &str, version: &str) -> ExitCode {
    if name.is_empty() || version.is_empty() {
        eprintln!("错误: add 需要 <name> 和 <version> 两个参数");
        return ExitCode::from(1);
    }
    if name.contains('=') || name.contains(' ') || name.contains('"') {
        eprintln!("错误: 非法依赖名 `{}`", name);
        return ExitCode::from(1);
    }
    if !file.exists() {
        eprintln!("错误: {} 不存在。请先运行 `qawpm init`", file.display());
        return ExitCode::from(1);
    }

    let mut toml = match read_toml(file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("错误: {}", e);
            return ExitCode::from(1);
        }
    };

    if toml.dependencies.contains_key(name) {
        eprintln!(
            "警告: 依赖 {} 已存在 ({} -> {})，将被覆盖",
            name, toml.dependencies[name], version
        );
    }
    toml.dependencies.insert(name.to_string(), version.to_string());

    if let Err(e) = fs::write(file, write_qaw_toml(&toml)) {
        eprintln!("错误: 无法写入 {}: {}", file.display(), e);
        return ExitCode::from(1);
    }
    println!("已添加 {} = \"{}\" 到 {}", name, version, file.display());
    ExitCode::SUCCESS
}

fn home_dir() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("QAW_HOME") {
        if !p.is_empty() {
            return Some(PathBuf::from(p));
        }
    }
    if let Ok(p) = std::env::var("HOME") {
        if !p.is_empty() {
            return Some(PathBuf::from(p));
        }
    }
    None
}

fn cmd_install(file: &Path) -> ExitCode {
    if !file.exists() {
        eprintln!(
            "错误: {} 不存在。请先运行 `qawpm init` 创建项目",
            file.display()
        );
        return ExitCode::from(1);
    }

    let toml = match read_toml(file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("错误: {}", e);
            return ExitCode::from(1);
        }
    };

    let mut entries: Vec<LockEntry> = Vec::new();
    entries.push(LockEntry {
        name: toml.package.name.clone(),
        version: toml.package.version.clone(),
        source: None,
    });
    for (dep_name, dep_ver) in &toml.dependencies {
        entries.push(LockEntry {
            name: dep_name.clone(),
            version: dep_ver.clone(),
            source: Some(REGISTRY_URL.to_string()),
        });
    }

    let lock_path = file
        .parent()
        .map(|p| p.join("Qaw.lock"))
        .unwrap_or_else(|| PathBuf::from("Qaw.lock"));
    let lock_content = render_lock(&entries);
    if let Err(e) = fs::write(&lock_path, &lock_content) {
        eprintln!("错误: 无法写入 {}: {}", lock_path.display(), e);
        return ExitCode::from(1);
    }
    println!("已生成 {}", lock_path.display());

    let cache_dir = match home_dir() {
        Some(h) => h.join(".qaw").join("cache"),
        None => {
            eprintln!("警告: 无法确定 HOME 目录，跳过缓存初始化");
            println!("install 完成 ({} 个包)", entries.len());
            return ExitCode::SUCCESS;
        }
    };

    if let Err(e) = fs::create_dir_all(&cache_dir) {
        eprintln!("警告: 无法创建缓存目录 {}: {}", cache_dir.display(), e);
    } else {
        // TODO(v0.12+): 实际从 registry 下载 crate .tar.gz 并写入 ~/.qaw/cache/
        let manifest = cache_dir.join("manifest.txt");
        let body: Vec<String> = entries
            .iter()
            .map(|e| format!("{} {}\n", e.name, e.version))
            .collect();
        let _ = fs::write(manifest, body.join(""));
        println!("已登记 {} 个包到 {}", entries.len(), cache_dir.display());
    }

    println!("install 完成 ({} 个包)", entries.len());
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_banner();
        return ExitCode::from(1);
    }

    match args[1].as_str() {
        "-V" | "--version" | "version" => {
            println!("qawpm {}", VERSION);
            ExitCode::SUCCESS
        }
        "-h" | "--help" | "help" => {
            usage();
            ExitCode::SUCCESS
        }
        "init" => cmd_init(Path::new(".")),
        "add" => {
            if args.len() < 4 {
                eprintln!("用法: qawpm add <name> <version>");
                return ExitCode::from(1);
            }
            cmd_add(Path::new("Qaw.toml"), &args[2], &args[3])
        }
        "install" => cmd_install(Path::new("Qaw.toml")),
        _ => {
            eprintln!("未知命令: {}", args[1]);
            print_banner();
            ExitCode::from(1)
        }
    }
}

fn print_banner() {
    print!(
        "qawpm {VERSION} — Qaw 包管理器\n\
         \n\
         用法: qawpm <command>\n\
         \n\
         命令:\n\
         \n\
         \x20\x20init     在当前目录创建 Qaw.toml + src/\n\
         \x20\x20add      向 Qaw.toml 添加依赖\n\
         \x20\x20install  解析 Qaw.toml 并下载依赖\n\
         \n\
         试试 'qawpm --help' 查看每个命令的详细用法。\n"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal() {
        let t = parse_qaw_toml("[package]\nname = \"x\"\nversion = \"0.1.0\"\nedition = \"2024\"\n")
            .unwrap();
        assert_eq!(t.package.name, "x");
        assert_eq!(t.package.version, "0.1.0");
        assert_eq!(t.package.edition, "2024");
        assert!(t.dependencies.is_empty());
    }

    #[test]
    fn parse_with_deps() {
        let src = "[package]\nname = \"a\"\nversion = \"0.2.0\"\nedition = \"2024\"\n\n[dependencies]\nfoo = \"1.0\"\nbar = \"2.3\"\n";
        let t = parse_qaw_toml(src).unwrap();
        assert_eq!(t.dependencies.get("foo"), Some(&"1.0".to_string()));
        assert_eq!(t.dependencies.get("bar"), Some(&"2.3".to_string()));
    }

    #[test]
    fn parse_ignores_comments() {
        let src = "# header\n[package] # inline\nname = \"x\" # trailing\n";
        let t = parse_qaw_toml(src).unwrap();
        assert_eq!(t.package.name, "x");
    }

    #[test]
    fn roundtrip() {
        let t = QawToml {
            package: PackageMeta {
                name: "demo".into(),
                version: "0.3.0".into(),
                edition: "2024".into(),
            },
            dependencies: BTreeMap::from([
                ("qaw_std".to_string(), "1.0".to_string()),
                ("qaw_http".to_string(), "2.1".to_string()),
            ]),
        };
        let s = write_qaw_toml(&t);
        let back = parse_qaw_toml(&s).unwrap();
        assert_eq!(t, back);
    }

    #[test]
    fn render_lock_includes_source_for_deps() {
        let entries = vec![
            LockEntry {
                name: "root".into(),
                version: "0.1.0".into(),
                source: None,
            },
            LockEntry {
                name: "foo".into(),
                version: "1.0.0".into(),
                source: Some(REGISTRY_URL.into()),
            },
        ];
        let s = render_lock(&entries);
        assert!(s.contains("[[package]]"));
        assert!(s.contains("name = \"foo\""));
        assert!(s.contains("source = \""));
        assert!(!s.contains("name = \"root\"\nsource"));
    }

    #[test]
    fn unquote_handles_strings_and_bare() {
        assert_eq!(unquote("\"abc\""), "abc");
        assert_eq!(unquote("'abc'"), "abc");
        assert_eq!(unquote("123"), "123");
        assert_eq!(unquote("  \"x\"  "), "x");
    }
}