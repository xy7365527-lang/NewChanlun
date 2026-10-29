"""#1419 裁定（issuecomment-6265726014）：v2/v1 结构写者只在 `s_v2_seed_testonly` 构建存在。

造 v2/v1 种子库的脚本（launch_s.sh 直接链路、r9/r13、tb01c_query_*、tb02a_*、tb01c/tb03b runtime 及
s-launcher/2 服务控制）开工前调用 `require_seed_binary`，拿到发布构建时立刻失败并给出构建命令，
避免把「v2 结构写者已退役」误判成业务回归。
"""
import json
import subprocess

SEED_FEATURES = "s_session,s_v2_seed_testonly"
BUILD_COMMAND = f"(cd rust && cargo build --features {SEED_FEATURES} --bin s_structure_session)"


def require_seed_binary(binary):
    try:
        probe = subprocess.run([str(binary), "v2-seed-info"], capture_output=True, text=True, timeout=30)
        ok = probe.returncode == 0 and json.loads(probe.stdout).get("s_v2_seed_testonly") is True
    except (OSError, ValueError, subprocess.TimeoutExpired):
        ok = False
    if not ok:
        raise SystemExit(f"{binary} 不是 v2 种子构建（#1419：v2 写者只在 s_v2_seed_testonly 构建存在）；请用 {BUILD_COMMAND}")
    return binary
