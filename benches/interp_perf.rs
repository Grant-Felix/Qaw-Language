// Qaw 解释器性能基线（v0.18）
//
// 用法：
//   cargo build --release --bin qawc
//   cargo bench --bench interp_perf
//
// 验收标准（docs/v0.10-to-v0.50-规划.md § 4 阶段 A Q5）：
//   1. 解释器版 Hello World 冷启动 < 100ms
//   2. fib(0..9) < 1s
//   3. 冷编译 vs 热编译时间记录

use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

const RUNS: usize = 10;
const THRESHOLD_HELLO_MS: u128 = 100;
const THRESHOLD_FIB_MS: u128 = 1_000;

fn locate_qawc() -> PathBuf {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| ".".to_string());
    let candidates = [
        "target/release/qawc",
        "target/release/qawc.exe",
        "../target/release/qawc",
        "../target/release/qawc.exe",
    ];
    for c in &candidates {
        let p = PathBuf::from(&manifest_dir).join(c);
        if p.exists() {
            return p;
        }
    }
    panic!(
        "未找到 qawc 可执行文件；请先 `cargo build --release --bin qawc`\n尝试路径: {:?}",
        candidates
            .iter()
            .map(|c| PathBuf::from(&manifest_dir).join(c))
            .collect::<Vec<_>>()
    );
}

#[derive(Debug, Clone)]
struct Stats {
    avg_ms_x10: u128,
    min_ms_x10: u128,
    max_ms_x10: u128,
}

fn fmt_ms(ms_x10: u128) -> String {
    if ms_x10 >= 10 {
        format!("{}ms", ms_x10 / 10)
    } else {
        format!("{}.{}ms", ms_x10 / 10, ms_x10 % 10)
    }
}

fn measure(bin: &PathBuf, args: &[&str], runs: usize) -> Stats {
    let mut samples_us: Vec<u128> = Vec::with_capacity(runs);
    for _ in 0..runs {
        let start = Instant::now();
        let output = Command::new(bin)
            .args(args)
            .output()
            .unwrap_or_else(|e| panic!("无法执行 {:?} {:?}: {}", bin, args, e));
        if !output.status.success() {
            panic!(
                "qawc 执行失败（status={}）\n--- stdout ---\n{}\n--- stderr ---\n{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }
        samples_us.push(start.elapsed().as_micros());
    }
    let total_us: u128 = samples_us.iter().sum();
    let min_us = *samples_us.iter().min().unwrap();
    let max_us = *samples_us.iter().max().unwrap();
    // 毫秒 × 10（保留 1 位小数）
    let avg_ms_x10 = (total_us * 10) / (runs as u128 * 1_000);
    let min_ms_x10 = (min_us * 10) / 1_000;
    let max_ms_x10 = (max_us * 10) / 1_000;
    Stats { avg_ms_x10, min_ms_x10, max_ms_x10 }
}

fn print_stats_row(label: &str, stats: &Stats, threshold_ms: u128) -> bool {
    let pass = stats.avg_ms_x10 / 10 < threshold_ms;
    let mark = if pass { "PASS" } else { "FAIL" };
    println!(
        "  {:<24} avg={:>8}  min={:>8}  max={:>8}  threshold=<{}ms  [{}]",
        label,
        fmt_ms(stats.avg_ms_x10),
        fmt_ms(stats.min_ms_x10),
        fmt_ms(stats.max_ms_x10),
        threshold_ms,
        mark
    );
    pass
}

fn main() {
    let bin = locate_qawc();
    println!("Qaw 解释器性能基线 (interp_perf)");
    println!("=================================");
    println!("二进制 : {}", bin.display());
    println!("运行次数: {}", RUNS);
    println!();

    // --- 冷启动 = 每个 run 一次冷进程（OS page cache 视为热） ---
    println!("[1] 冷启动（每次 fork 独立子进程）");
    let hello_cold = measure(&bin, &["run", "examples/hello.qaw"], RUNS);
    let hello_pass = print_stats_row("hello.qaw 冷启动", &hello_cold, THRESHOLD_HELLO_MS);

    let fib_cold = measure(&bin, &["run", "examples/fib.qaw"], RUNS);
    let fib_pass = print_stats_row("fib.qaw 冷启动", &fib_cold, THRESHOLD_FIB_MS);
    println!();

    // --- 编译时间（qawc run 内含 lex+parse） ---
    println!("[2] 编译时间（cargo build --release --bin qawc）");
    let build_start = Instant::now();
    let build_status = Command::new("cargo")
        .args(["build", "--release", "--bin", "qawc", "--quiet"])
        .status()
        .expect("无法执行 cargo build");
    let build_warm_ms = build_start.elapsed().as_millis();
    let build_passed = build_status.success();
    println!(
        "  {:<24} build_warm={:>6}ms  status={}",
        "cargo build 热编译",
        build_warm_ms,
        if build_passed { "ok" } else { "FAIL" }
    );

    let clean_start = Instant::now();
    let _ = Command::new("cargo")
        .args(["clean", "--quiet"])
        .status();
    let clean_status = Command::new("cargo")
        .args(["build", "--release", "--bin", "qawc", "--quiet"])
        .status()
        .expect("无法执行 cargo build (冷)");
    let build_cold_ms = clean_start.elapsed().as_millis();
    let cold_passed = clean_status.success();
    println!(
        "  {:<24} build_cold={:>6}ms  status={}",
        "cargo build 冷编译",
        build_cold_ms,
        if cold_passed { "ok" } else { "FAIL" }
    );
    println!();

    // --- Markdown 报告（可粘贴到 docs/3） ---
    println!("[3] Markdown 报告（粘贴到 docs/3-总任务表和进度.md 或 v0.10-to-v0.50-规划.md）");
    println!();
    println!("| 场景 | 次数 | 平均 | 最小 | 最大 | 阈值 | 结果 |");
    println!("|:---|:---:|---:|---:|---:|---:|:---:|");
    println!(
        "| Hello World 冷启动 | {} | {} | {} | {} | <{}ms | {} |",
        RUNS,
        fmt_ms(hello_cold.avg_ms_x10),
        fmt_ms(hello_cold.min_ms_x10),
        fmt_ms(hello_cold.max_ms_x10),
        THRESHOLD_HELLO_MS,
        if hello_pass { "✅" } else { "❌" }
    );
    println!(
        "| fib(0..9) 冷启动 | {} | {} | {} | {} | <{}ms | {} |",
        RUNS,
        fmt_ms(fib_cold.avg_ms_x10),
        fmt_ms(fib_cold.min_ms_x10),
        fmt_ms(fib_cold.max_ms_x10),
        THRESHOLD_FIB_MS,
        if fib_pass { "✅" } else { "❌" }
    );
    println!(
        "| cargo build 冷编译 | 1 | {}ms | - | - | - | {} |",
        build_cold_ms,
        if cold_passed { "✅" } else { "❌" }
    );
    println!(
        "| cargo build 热编译 | 1 | {}ms | - | - | - | {} |",
        build_warm_ms,
        if build_passed { "✅" } else { "❌" }
    );
    println!();

    // --- 阈值断言 ---
    // v0.18 第一版：仅警告，不 panic（性能不达标仍可通过）
    let mut warnings = Vec::new();
    if !hello_pass {
        warnings.push(format!(
            "Hello World 冷启动 {}ms >= {}ms 阈值（v0.18 仅警告）",
            fmt_ms(hello_cold.avg_ms_x10),
            THRESHOLD_HELLO_MS
        ));
    }
    if !fib_pass {
        warnings.push(format!(
            "fib 冷启动 {}ms >= {}ms 阈值（v0.18 仅警告）",
            fmt_ms(fib_cold.avg_ms_x10),
            THRESHOLD_FIB_MS
        ));
    }
    if !warnings.is_empty() {
        eprintln!("[WARN] 性能基线未达标：");
        for w in &warnings {
            eprintln!("  - {}", w);
        }
    } else {
        println!("[OK] 性能基线全部达标");
    }
}
