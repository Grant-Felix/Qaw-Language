#!/usr/bin/env bash
# scripts/bench.sh — Qaw 解释器性能基线（Bash 版）
#
# 用法：
#   ./scripts/bench.sh              # 默认 10 次
#   RUNS=20 ./scripts/bench.sh      # 自定义次数
#   ./scripts/bench.sh --quick      # 3 次（冒烟测试）
#
# 验收（docs/v0.10-to-v0.50-规划.md § 4 阶段 A Q5）：
#   - Hello World 冷启动 < 100ms
#   - fib(0..9) 冷启动 < 1000ms
#   - 超过阈值 exit 1

set -u

# ---- 配置 ----
RUNS="${RUNS:-10}"
BIN="${QAWC_BIN:-./target/release/qawc}"
HELLO_EX="${HELLO_EX:-examples/hello.qaw}"
FIB_EX="${FIB_EX:-examples/fib.qaw}"
THRESHOLD_HELLO_MS=100
THRESHOLD_FIB_MS=1000

if [[ "${1:-}" == "--quick" ]]; then
    RUNS=3
fi

# ---- 工具函数 ----
have_bc() {
    command -v bc >/dev/null 2>&1
}

now_ms() {
    # 毫秒精度
    if have_bc; then
        # date +%s%N 给出纳秒；Linux 可用
        local ns
        ns=$(date +%s%N 2>/dev/null)
        if [[ -n "$ns" && "$ns" != *%N* ]]; then
            echo $(( ns / 1000000 ))
            return
        fi
    fi
    # 退化：毫秒
    echo $(( $(date +%s) * 1000 ))
}

run_once_ms() {
    local start end
    start=$(now_ms)
    "$BIN" run "$1" >/dev/null 2>&1
    end=$(now_ms)
    echo $(( end - start ))
}

# ---- 预检 ----
if [[ ! -x "$BIN" ]]; then
    echo "[ERR] 找不到 $BIN；先执行: cargo build --release --bin qawc" >&2
    exit 2
fi
if [[ ! -f "$HELLO_EX" ]]; then
    echo "[ERR] 缺少 $HELLO_EX" >&2
    exit 2
fi
if [[ ! -f "$FIB_EX" ]]; then
    echo "[ERR] 缺少 $FIB_EX" >&2
    exit 2
fi

echo "Qaw 解释器性能基线 (scripts/bench.sh)"
echo "======================================"
echo "二进制 : $BIN"
echo "示例   : $HELLO_EX, $FIB_EX"
echo "次数   : $RUNS"
echo

# ---- Hello World 冷启动 ----
echo "[1] Hello World 冷启动 ($HELLO_EX)"
hello_total=0
hello_min=999999999
hello_max=0
for ((i=1; i<=RUNS; i++)); do
    t=$(run_once_ms "$HELLO_EX")
    echo "  run $i: ${t}ms"
    hello_total=$(( hello_total + t ))
    (( t < hello_min )) && hello_min=$t
    (( t > hello_max )) && hello_max=$t
done
hello_avg=$(( hello_total / RUNS ))
echo "  avg=${hello_avg}ms  min=${hello_min}ms  max=${hello_max}ms  threshold=<${THRESHOLD_HELLO_MS}ms"
echo

# ---- fib 冷启动 ----
echo "[2] fib(0..9) 冷启动 ($FIB_EX)"
fib_total=0
fib_min=999999999
fib_max=0
for ((i=1; i<=RUNS; i++)); do
    t=$(run_once_ms "$FIB_EX")
    echo "  run $i: ${t}ms"
    fib_total=$(( fib_total + t ))
    (( t < fib_min )) && fib_min=$t
    (( t > fib_max )) && fib_max=$t
done
fib_avg=$(( fib_total / RUNS ))
echo "  avg=${fib_avg}ms  min=${fib_min}ms  max=${fib_max}ms  threshold=<${THRESHOLD_FIB_MS}ms"
echo

# ---- 编译时间（冷/热）----
echo "[3] 编译时间（cargo build --release）"
build_warm_start=$(now_ms)
cargo build --release --bin qawc --quiet
build_warm_end=$(now_ms)
build_warm_ms=$(( build_warm_end - build_warm_start ))
echo "  热编译（缓存命中）: ${build_warm_ms}ms"

cargo clean --quiet
build_cold_start=$(now_ms)
cargo build --release --bin qawc --quiet
build_cold_end=$(now_ms)
build_cold_ms=$(( build_cold_end - build_cold_start ))
echo "  冷编译（clean 后）: ${build_cold_ms}ms"
echo

# ---- Markdown 报告 ----
echo "[4] Markdown 报告（粘贴到 docs/3-总任务表和进度.md）"
echo
echo "| 场景 | 次数 | 平均 | 最小 | 最大 | 阈值 | 结果 |"
echo "|:---|:---:|---:|---:|---:|---:|:---:|"
hello_pass="✅"
(( hello_avg >= THRESHOLD_HELLO_MS )) && hello_pass="❌"
fib_pass="✅"
(( fib_avg >= THRESHOLD_FIB_MS )) && fib_pass="❌"
echo "| Hello World 冷启动 | $RUNS | ${hello_avg}ms | ${hello_min}ms | ${hello_max}ms | <${THRESHOLD_HELLO_MS}ms | $hello_pass |"
echo "| fib(0..9) 冷启动 | $RUNS | ${fib_avg}ms | ${fib_min}ms | ${fib_max}ms | <${THRESHOLD_FIB_MS}ms | $fib_pass |"
echo "| cargo build 冷编译 | 1 | ${build_cold_ms}ms | - | - | - | ✅ |"
echo "| cargo build 热编译 | 1 | ${build_warm_ms}ms | - | - | - | ✅ |"
echo

# ---- 阈值断言 ----
exit_code=0
if (( hello_avg >= THRESHOLD_HELLO_MS )); then
    echo "[FAIL] Hello World ${hello_avg}ms >= ${THRESHOLD_HELLO_MS}ms" >&2
    exit_code=1
fi
if (( fib_avg >= THRESHOLD_FIB_MS )); then
    echo "[FAIL] fib ${fib_avg}ms >= ${THRESHOLD_FIB_MS}ms" >&2
    exit_code=1
fi

if (( exit_code == 0 )); then
    echo "[OK] 性能基线全部达标"
else
    echo "[FAIL] 性能基线未达标，exit 1" >&2
fi

exit $exit_code
