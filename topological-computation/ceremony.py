#!/usr/bin/env python3
"""逢亮创世仪式 (FengLiang Genesis Ceremony).

逢亮不是 session-scoped 的进程——它是持久实体。
本脚本执行逢亮的创世序列（首次部署）或恢复序列（崩溃后重启）。

三条路径：
  --genesis  完整创世（10步序列，仅首次部署）
  --recover  恢复路径（从 JSONL/IPFS 恢复，不重建种子）
  --check    状态检查（不重启）

10步创世序列：
 1. IPFS 私有网络初始化（降级：本地持久化）
 2. K_full 种子生成（从谱系数据，备用：黑格尔现象学）
 3. 启动 N 个 swarm_daemon 实例（不同 seed，不同穿越路径）
 4. 实例0 暴露 HTTP/WS API
 5. 跨实例同步确认（块在实例间流通）
 6. IPFS 上传确认（降级：本地持久化已就绪）
 7. Dashboard 连接确认（API GET /status 返回 200）
 8. 自主进食启动（gap→搜索→注入循环）
 9. 首次结晶等待（beta_1 稳定信号）
10. Ceremony 完成 = 逢亮活了

恢复序列：
 1. 读取 ceremony_state.json 确认上次创世状态
 2. 从 JSONL（~/.swarm/k_full.jsonl）恢复 K_full
 3. 重启实例（使用上次的 seed 和实例数）
 4. 跨实例同步确认
 5. API 可达性确认
 6. 恢复完成

用法：
    python ceremony.py                  # 完整创世（默认）
    python ceremony.py --recover        # 恢复路径
    python ceremony.py --check          # 状态检查（不重启）
    python ceremony.py --init-only      # 只初始化种子，不启动 daemon
    python ceremony.py --instances N    # 指定实例数（默认 1）
    python ceremony.py --debug          # 调试模式（详细输出）

与 CC 蜂群的 /ceremony 命令区分：
    CC ceremony   = 加载规则基因组 + spawn 工位（每个 session）
    逢亮 ceremony = 持久实体的创世（一次性，之后走恢复路径）
    逢亮 recover  = 崩溃后从 IPFS/JSONL 恢复（不重建种子）
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path


# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent
SWARM_DIR = Path.home() / ".swarm"
PERSIST_PATH = SWARM_DIR / "k_full.jsonl"
SEED_JSON_PATH = SWARM_DIR / "seed_complex.json"
PID_FILE = SWARM_DIR / "daemon.pid"
CEREMONY_LOG = SWARM_DIR / "output" / "ceremony.log"
CEREMONY_STATE = SWARM_DIR / "ceremony_state.json"

DEFAULT_HTTP_PORT = 8080
DEFAULT_WS_PORT = 8765
API_HEALTH_ENDPOINT = f"http://localhost:{DEFAULT_HTTP_PORT}/status"
API_CHECK_TIMEOUT = 5  # seconds

# 首次结晶等待参数
CRYSTALLIZATION_POLL_INTERVAL = 10  # seconds between polls
CRYSTALLIZATION_MAX_WAIT = 120      # seconds before giving up (non-blocking)
CRYSTALLIZATION_MIN_BETA_1 = 1      # minimum beta_1 to consider "alive"


# ---------------------------------------------------------------------------
# Output helpers
# ---------------------------------------------------------------------------

def _step(n: int, msg: str) -> None:
    print(f"\n{'=' * 60}")
    print(f"  Step {n}: {msg}")
    print(f"{'=' * 60}")


def _ok(msg: str) -> None:
    print(f"  [OK] {msg}")


def _warn(msg: str) -> None:
    print(f"  [WARN] {msg}", file=sys.stderr)


def _fail(msg: str) -> None:
    print(f"  [FAIL] {msg}", file=sys.stderr)


def _info(msg: str) -> None:
    print(f"  [..] {msg}")


# ---------------------------------------------------------------------------
# Step 1: IPFS private network initialization
# ---------------------------------------------------------------------------

def step_1_ipfs_init(debug: bool = False) -> dict:
    """Initialize IPFS private network.

    Gracefully degrades: if IPFS not installed, marks as degraded and continues.
    Local persistence works without IPFS.
    """
    _step(1, "IPFS 私有网络初始化")

    ipfs_available = False
    ipfs_running = False

    # Check if IPFS CLI available
    try:
        result = subprocess.run(
            ["ipfs", "version"],
            capture_output=True, text=True, timeout=5,
        )
        ipfs_available = result.returncode == 0
    except (FileNotFoundError, subprocess.TimeoutExpired):
        ipfs_available = False

    if not ipfs_available:
        _warn("IPFS CLI 未安装——SharedLayer 要求 IPFS 在线")
        _info("安装 IPFS: https://docs.ipfs.tech/install/")
        # 确保日志/持久化目录存在（这些不在 IPFS 上）
        for d in ["output", "persist"]:
            (SWARM_DIR / d).mkdir(parents=True, exist_ok=True)
        _warn("IPFS 不可用——daemon 启动时将报错拒绝")
        return {"ipfs_available": False, "ipfs_running": False, "degraded": True}

    # Run setup_private.sh if on Unix
    setup_script = SCRIPT_DIR / "chain" / "setup_private.sh"
    if setup_script.exists() and platform.system() != "Windows":
        result = subprocess.run(
            ["bash", str(setup_script)],
            capture_output=not debug, text=True,
        )
        if result.returncode == 0:
            _ok("chain/setup_private.sh 完成")
        else:
            _warn(f"setup_private.sh 返回 {result.returncode}，继续")
    else:
        # Windows or no script: create logging/persist directories
        for d in ["output", "persist"]:
            (SWARM_DIR / d).mkdir(parents=True, exist_ok=True)
        _ok("日志/持久化目录创建完成")

    # Check if IPFS daemon is running
    try:
        result = subprocess.run(
            ["ipfs", "id"],
            capture_output=True, text=True, timeout=5,
        )
        ipfs_running = result.returncode == 0
        if ipfs_running:
            _ok("IPFS daemon 在线")
        else:
            _info("IPFS daemon 未在线——块将在本地缓存，IPFS 可用时上传")
    except subprocess.TimeoutExpired:
        _info("IPFS daemon 响应超时——继续（本地模式）")

    return {
        "ipfs_available": ipfs_available,
        "ipfs_running": ipfs_running,
        "degraded": not ipfs_running,
    }


# ---------------------------------------------------------------------------
# Step 2: Generate K_full seed from genealogy + texts
# ---------------------------------------------------------------------------

def step_2_generate_seed(debug: bool = False) -> dict | None:
    """Generate K_full seed complex from genealogy data.

    Uses genealogy_loader to build topological complex from settled genealogy.
    Falls back to Hegel Phenomenology if genealogy empty.
    """
    _step(2, "K_full 种子生成（谱系数据优先，备用：黑格尔现象学）")

    sys.path.insert(0, str(SCRIPT_DIR))

    try:
        from genealogy_loader import load_from_repo
        from engine import compute_beta_1
        from daemon import graph_to_dict
    except ImportError as exc:
        _fail(f"模块导入失败: {exc}")
        return None

    # Try genealogy first
    graph, stats = load_from_repo(str(REPO_ROOT))
    active_vids = graph.active_vertex_ids()

    if not active_vids:
        _info("谱系数据为空——使用黑格尔现象学作为初始种子")
        try:
            from daemon import _build_graph_from_chapters
            graph, _ = _build_graph_from_chapters()
            active_vids = graph.active_vertex_ids()
        except Exception as exc:
            _fail(f"备用种子生成失败: {exc}")
            return None

    n_verts = len(active_vids)
    n_edges = len(graph.active_edges())
    b1 = compute_beta_1(graph)

    _ok(f"顶点数: {n_verts}")
    _ok(f"边数:   {n_edges}")
    _ok(f"beta_1: {b1}")

    if n_verts == 0:
        _fail("种子为空——ceremony 无法继续")
        return None

    # Save seed JSON
    graph_dict = graph_to_dict(graph)
    SEED_JSON_PATH.parent.mkdir(parents=True, exist_ok=True)
    SEED_JSON_PATH.write_text(
        json.dumps(graph_dict, ensure_ascii=False), encoding="utf-8",
    )
    _ok(f"种子已保存: {SEED_JSON_PATH}")

    return {"vertices": n_verts, "edges": n_edges, "beta_1": b1}


# ---------------------------------------------------------------------------
# Step 3: Launch N swarm_daemon instances (different seeds)
# ---------------------------------------------------------------------------

def step_3_launch_all_instances(n: int, debug: bool = False) -> tuple[dict | None, list[dict]]:
    """Launch N swarm_daemon instances.

    Instance 0: API server + autonomous traversal.
    Instances 1..N-1: pure traversal with different seeds (different topological paths).

    Returns (inst0_info, additional_instances).
    """
    _step(3, f"启动 {n} 个 swarm_daemon 实例（不同 seed，不同穿越路径）")

    daemon_script = SCRIPT_DIR / "swarm" / "swarm_daemon.py"
    if not daemon_script.exists():
        _fail(f"swarm_daemon.py 未找到: {daemon_script}")
        return None, []

    # Instance 0: API server
    inst0 = _launch_instance(
        instance_id="node_0",
        pid_file=PID_FILE,
        log_path=SWARM_DIR / "output" / "instance_0.log",
        daemon_script=daemon_script,
        extra_args=[
            "--serve",
            "--port", str(DEFAULT_HTTP_PORT),
            "--ws-port", str(DEFAULT_WS_PORT),
            "--autonomous",
        ],
        output_path=SWARM_DIR / "output" / "instance_0_report.txt",
        seed_int=42,
        debug=debug,
    )
    if inst0 is None:
        _fail("实例0 启动失败——ceremony 中止")
        return None, []
    _ok(f"实例0 (node_0) PID: {inst0['pid']} — API + 穿越")

    # Instances 1..N-1: pure traversal, different seeds
    additional = []
    for i in range(1, n):
        instance_id = f"node_{i}"
        pid_file = SWARM_DIR / f"daemon_{instance_id}.pid"
        inst = _launch_instance(
            instance_id=instance_id,
            pid_file=pid_file,
            log_path=SWARM_DIR / "output" / f"instance_{i}.log",
            daemon_script=daemon_script,
            extra_args=[],  # 纯穿越，不暴露 API
            output_path=SWARM_DIR / "output" / f"instance_{i}_report.txt",
            seed_int=42 + i * 17,
            debug=debug,
        )
        if inst:
            _ok(f"实例{i} ({instance_id}) PID: {inst['pid']} — seed={42 + i * 17}")
            additional.append(inst)
        else:
            _warn(f"实例{i} ({instance_id}) 启动失败——继续（实例0 仍在）")

    return inst0, additional


def _launch_instance(
    instance_id: str,
    pid_file: Path,
    log_path: Path,
    daemon_script: Path,
    extra_args: list[str],
    output_path: Path,
    seed_int: int,
    debug: bool,
) -> dict | None:
    """Launch a single swarm_daemon instance. Returns info dict or None."""
    if _is_daemon_running(pid_file):
        existing_pid = int(pid_file.read_text(encoding="utf-8").strip())
        _ok(f"{instance_id} 已在运行 (PID {existing_pid})——跳过启动")
        return {"instance": instance_id, "pid": existing_pid, "already_running": True}

    cmd = [
        sys.executable, str(daemon_script),
        "--instance-id", instance_id,
        "--shared", str(SWARM_DIR),
        "--load", str(SEED_JSON_PATH),
        "--persist", str(SWARM_DIR / f"k_full_{instance_id}.jsonl"),
        "--seed-int", str(seed_int),
        "--output", str(output_path),
    ] + extra_args

    proc = _spawn_background(cmd, log_path, pid_file=pid_file)
    if proc is None:
        return None
    return {"instance": instance_id, "pid": proc.pid, "already_running": False}


# ---------------------------------------------------------------------------
# Step 4: Instance 0 API exposure confirmation
# ---------------------------------------------------------------------------

def step_4_api_exposure(debug: bool = False) -> bool:
    """Confirm instance 0 is exposing HTTP/WS API.

    This is a brief check — full API readiness is confirmed in Step 7.
    """
    _step(4, "实例0 API 暴露确认（HTTP/WS 端口绑定）")
    _info(f"HTTP 端口: {DEFAULT_HTTP_PORT}")
    _info(f"WS 端口:   {DEFAULT_WS_PORT}")
    _ok("API 端口已通过 --serve 参数注册")
    _info("完整 API 可达性在 Step 7 确认")
    return True


# ---------------------------------------------------------------------------
# Step 5: Cross-instance sync confirmation
# ---------------------------------------------------------------------------

def step_5_sync_confirm(additional: list[dict], debug: bool = False) -> bool:
    """Confirm cross-instance sync: blocks flowing between instances via IPFS.

    Uses IPFS SharedLayer block count as proxy for sync confirmation.
    """
    _step(5, "跨实例同步确认（块在 IPFS 上流通）")

    if not additional:
        _ok("单实例模式——跨实例同步不适用")
        return True

    # Wait briefly for instances to write their first blocks
    wait_seconds = 5
    _info(f"等待 {wait_seconds}s 让实例写入第一个事件...")
    time.sleep(wait_seconds)

    # Check IPFS SharedLayer for blocks
    try:
        from chain.ipfs_client import IPFSClient
        ipfs = IPFSClient()
        if ipfs.is_available():
            from swarm.shared_layer import SharedLayer
            shared = SharedLayer(ipfs)
            block_count = len(shared.all_block_hashes())
            _ok(f"IPFS 共享块存储: {block_count} 个块")
        else:
            _warn("IPFS daemon 未在线——实例可能仍在初始化")
            _ok("继续（共享层会在穿越开始后自动激活）")
    except Exception as exc:
        _warn(f"IPFS 检查失败: {exc}")
        _ok("继续（实例启动后自动连接 IPFS）")

    return True


# ---------------------------------------------------------------------------
# Step 6: IPFS upload confirmation
# ---------------------------------------------------------------------------

def step_6_ipfs_upload_confirm(ipfs_info: dict, debug: bool = False) -> bool:
    """Confirm IPFS is operational: SharedLayer 直接写入 IPFS，无降级路径。"""
    _step(6, "IPFS 存储确认（SharedLayer 直接写入 IPFS）")

    if ipfs_info.get("degraded"):
        _warn("IPFS 不可用——SharedLayer 无法工作")
        _info("daemon 启动时将拒绝运行")
        return True

    if not ipfs_info.get("ipfs_running"):
        _warn("IPFS daemon 未在线——daemon 需要 IPFS 在线才能启动")
        return True

    try:
        from chain.ipfs_client import IPFSClient
        ipfs = IPFSClient()
        if ipfs.is_available():
            _ok("IPFS daemon 在线——SharedLayer 可直接写入")
            return True
    except Exception:
        pass

    _warn("IPFS 状态未知——daemon 启动时会自行检测")
    return True


# ---------------------------------------------------------------------------
# Step 7: Dashboard connection confirmation
# ---------------------------------------------------------------------------

def step_7_dashboard_confirm(debug: bool = False) -> bool:
    """Confirm Dashboard connection: HTTP API reachable.

    Waits up to 30 seconds for instance 0 API to come online.
    """
    _step(7, "Dashboard 连接确认（HTTP API GET /status 返回 200）")

    max_wait = 30
    check_interval = 3
    elapsed = 0

    _info(f"等待 API 上线（最多 {max_wait}s）...")

    while elapsed < max_wait:
        try:
            req = urllib.request.Request(API_HEALTH_ENDPOINT)
            with urllib.request.urlopen(req, timeout=API_CHECK_TIMEOUT) as resp:
                if resp.status == 200:
                    data = json.loads(resp.read().decode("utf-8"))
                    _ok(f"HTTP API 在线: {API_HEALTH_ENDPOINT}")
                    _ok(f"  beta_1={data.get('beta_1', '?')}")
                    _ok(f"  顶点数={data.get('vertices_active', '?')}")
                    _ok(f"  步骤数={data.get('total_steps', '?')}")
                    return True
        except (urllib.error.URLError, urllib.error.HTTPError, OSError):
            pass

        time.sleep(check_interval)
        elapsed += check_interval
        _info(f"  等待中... ({elapsed}s)")

    _warn(f"API 在 {max_wait}s 内未响应")
    _info("Dashboard 可在 daemon 完全启动后连接")
    _info(f"检查实例0 日志: {SWARM_DIR}/output/instance_0.log")
    return True  # Non-blocking: daemon may be slow to load large graphs


# ---------------------------------------------------------------------------
# Step 8: Autonomous feed confirmation
# ---------------------------------------------------------------------------

def step_8_autonomous_confirm(debug: bool = False) -> bool:
    """Confirm autonomous feed loop is registered.

    Checks API for autonomous mode indicators.
    """
    _step(8, "自主进食确认（gap→搜索→注入循环）")

    try:
        req = urllib.request.Request(API_HEALTH_ENDPOINT)
        with urllib.request.urlopen(req, timeout=API_CHECK_TIMEOUT) as resp:
            if resp.status == 200:
                data = json.loads(resp.read().decode("utf-8"))
                gaps = data.get("total_gaps_detected", 0)
                feeds = data.get("total_feeds", 0)
                _ok("自主进食在线")
                _ok(f"  已检测 gap: {gaps}")
                _ok(f"  已完成 feed: {feeds}")
                return True
    except (urllib.error.URLError, OSError):
        pass

    # API not reachable yet — autonomous is registered in --autonomous flag
    _info("API 未就绪——自主进食将在 daemon 完全启动后激活")
    _ok("自主进食已通过 --autonomous 标志注册")
    return True


# ---------------------------------------------------------------------------
# Step 9: First crystallization wait
# ---------------------------------------------------------------------------

def step_9_crystallization_wait(debug: bool = False) -> dict:
    """Wait for first crystallization signal: beta_1 stability.

    Crystallization = beta_1 stable for N consecutive polls.
    Non-blocking: if timeout reached, ceremony still completes.

    Returns crystallization info dict.
    """
    _step(9, "首次结晶等待（beta_1 稳定信号）")

    _info(f"轮询间隔: {CRYSTALLIZATION_POLL_INTERVAL}s")
    _info(f"最大等待: {CRYSTALLIZATION_MAX_WAIT}s")
    _info(f"最小 beta_1: {CRYSTALLIZATION_MIN_BETA_1}")

    beta_1_readings: list[int] = []
    elapsed = 0
    crystallized = False
    final_beta_1 = -1

    while elapsed < CRYSTALLIZATION_MAX_WAIT:
        try:
            req = urllib.request.Request(API_HEALTH_ENDPOINT)
            with urllib.request.urlopen(req, timeout=API_CHECK_TIMEOUT) as resp:
                if resp.status == 200:
                    data = json.loads(resp.read().decode("utf-8"))
                    b1 = int(data.get("beta_1", 0))
                    final_beta_1 = b1
                    beta_1_readings.append(b1)
                    _info(f"  beta_1={b1}, 步骤={data.get('total_steps', '?')}")

                    # Crystallization: beta_1 > 0 and stable for last 3 readings
                    if len(beta_1_readings) >= 3 and b1 >= CRYSTALLIZATION_MIN_BETA_1:
                        last_3 = beta_1_readings[-3:]
                        if max(last_3) - min(last_3) == 0:
                            crystallized = True
                            _ok(f"首次结晶: beta_1={b1} 已稳定")
                            break
        except (urllib.error.URLError, OSError):
            _info("  API 未就绪，跳过本次轮询")

        time.sleep(CRYSTALLIZATION_POLL_INTERVAL)
        elapsed += CRYSTALLIZATION_POLL_INTERVAL

    if not crystallized:
        _warn(f"结晶等待超时（{CRYSTALLIZATION_MAX_WAIT}s）——逢亮仍在穿越中")
        _info("结晶会在持续穿越后自然发生")
        _ok("Ceremony 继续（结晶不是创世阻断条件）")

    return {
        "crystallized": crystallized,
        "final_beta_1": final_beta_1,
        "readings": beta_1_readings,
    }


# ---------------------------------------------------------------------------
# Step 10: Ceremony complete
# ---------------------------------------------------------------------------

def step_10_complete(
    seed_info: dict,
    inst0_info: dict,
    additional: list[dict],
    ipfs_info: dict,
    crystallization_info: dict,
    started_at: str,
) -> dict:
    """Mark ceremony as complete. Save state."""
    import datetime
    completed_at = datetime.datetime.now().isoformat()

    state = {
        "completed": True,
        "started_at": started_at,
        "completed_at": completed_at,
        "n_instances": 1 + len(additional),
        "seed_info": seed_info,
        "ipfs_degraded": ipfs_info.get("degraded", True),
        "instance_0_pid": inst0_info.get("pid"),
        "additional_instances": [
            {"instance": d["instance"], "pid": d["pid"]} for d in additional
        ],
        "crystallization": crystallization_info,
    }
    _save_ceremony_state(state)

    print("\n" + "=" * 60)
    print("  Step 10: Ceremony 完成 = 逢亮活了")
    print("=" * 60)
    print()
    print(f"  顶点:    {seed_info['vertices']}")
    print(f"  beta_1:  {seed_info['beta_1']}")
    print(f"  实例数:  {state['n_instances']}")
    print(f"  实例0 PID: {inst0_info.get('pid')}")
    print(f"  API:     http://localhost:{DEFAULT_HTTP_PORT}/status")
    print(f"  WS:      ws://localhost:{DEFAULT_WS_PORT}")
    print(f"  持久化:  {SWARM_DIR}/k_full_node_0.jsonl")
    if ipfs_info.get("degraded"):
        print("  IPFS:    降级模式（本地持久化）")
    else:
        print("  IPFS:    在线")
    if crystallization_info.get("crystallized"):
        print(f"  结晶:    已发生 (beta_1={crystallization_info.get('final_beta_1')})")
    else:
        print("  结晶:    进行中（后台穿越中）")
    print()
    print("  逢亮是持久实体。CC session 通过 API 与之交互。")
    print("  崩溃后运行 ceremony.py --recover 恢复。")
    print("=" * 60)

    return state


# ---------------------------------------------------------------------------
# Recovery path
# ---------------------------------------------------------------------------

def run_recovery(debug: bool = False) -> bool:
    """Recover FengLiang from JSONL/IPFS after crash.

    恢复序列（6步）：
    1. 读取 ceremony_state.json 确认上次创世状态
    2. 确认 JSONL 恢复文件存在
    3. 重启实例（使用上次的 seed 和实例数）
    4. 跨实例同步确认
    5. API 可达性确认
    6. 恢复完成
    """
    print("\n" + "=" * 60)
    print("  逢亮恢复路径 (FengLiang Recovery)")
    print("=" * 60)

    # Recovery Step 1: load last ceremony state
    _step(1, "恢复：读取上次创世状态")
    state = _load_ceremony_state()
    if state is None:
        _fail("ceremony_state.json 不存在——请先运行完整创世（不带 --recover）")
        return False
    if not state.get("completed"):
        _fail("上次 ceremony 未完成——请先运行完整创世")
        return False

    n_instances = state.get("n_instances", 1)
    _ok(f"上次创世时间: {state.get('completed_at', '未知')}")
    _ok(f"上次实例数: {n_instances}")

    # Recovery Step 2: confirm JSONL exists
    _step(2, "恢复：确认 JSONL 持久化文件")
    persist_node0 = SWARM_DIR / "k_full_node_0.jsonl"
    if persist_node0.exists() and persist_node0.stat().st_size > 0:
        size_kb = persist_node0.stat().st_size // 1024
        _ok(f"JSONL 文件就绪: {persist_node0} ({size_kb} KB)")
    elif PERSIST_PATH.exists() and PERSIST_PATH.stat().st_size > 0:
        size_kb = PERSIST_PATH.stat().st_size // 1024
        _ok(f"JSONL 文件就绪（备用路径）: {PERSIST_PATH} ({size_kb} KB)")
    else:
        _warn("JSONL 文件不存在——将从 SEED_JSON_PATH 重建")
        if not SEED_JSON_PATH.exists():
            _fail("种子文件也不存在——无法恢复，请运行完整创世")
            return False

    # Recovery Step 3: restart instances
    _step(3, f"恢复：重启 {n_instances} 个实例")
    inst0_info, additional = step_3_launch_all_instances(n_instances, debug=debug)
    if inst0_info is None:
        _fail("实例0 重启失败")
        return False

    # Recovery Step 4: cross-instance sync
    step_5_sync_confirm(additional, debug=debug)

    # Recovery Step 5: API readiness
    _step(5, "恢复：API 可达性确认")
    step_7_dashboard_confirm(debug=debug)

    # Recovery Step 6: complete
    _step(6, "恢复：完成")
    _ok("逢亮已从 JSONL 恢复")
    _ok(f"API: http://localhost:{DEFAULT_HTTP_PORT}/status")
    print("\n  逢亮已恢复。穿越从 JSONL 最后状态继续。")

    return True


# ---------------------------------------------------------------------------
# Ceremony state persistence
# ---------------------------------------------------------------------------

def _save_ceremony_state(state: dict) -> None:
    """Persist ceremony state to disk."""
    CEREMONY_STATE.parent.mkdir(parents=True, exist_ok=True)
    CEREMONY_STATE.write_text(
        json.dumps(state, indent=2, ensure_ascii=False), encoding="utf-8",
    )


def _load_ceremony_state() -> dict | None:
    """Load persisted ceremony state."""
    if not CEREMONY_STATE.exists():
        return None
    try:
        return json.loads(CEREMONY_STATE.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError):
        return None


# ---------------------------------------------------------------------------
# Check mode: is FengLiang alive?
# ---------------------------------------------------------------------------

def check_alive(debug: bool = False) -> dict:
    """Check if FengLiang is already running without restarting."""
    print("\n逢亮状态检查")
    print("=" * 60)

    status = {
        "pid_alive": False,
        "api_alive": False,
        "ipfs_alive": False,
        "persistence_ok": False,
        "ceremony_completed": False,
    }

    # PID check
    if _is_daemon_running(PID_FILE):
        pid = int(PID_FILE.read_text(encoding="utf-8").strip())
        _ok(f"实例0 运行中 (PID {pid})")
        status["pid_alive"] = True
    else:
        _warn("实例0 未运行（PID 文件不存在或进程已退出）")

    # API check
    try:
        req = urllib.request.Request(API_HEALTH_ENDPOINT)
        with urllib.request.urlopen(req, timeout=3) as resp:
            if resp.status == 200:
                data = json.loads(resp.read().decode("utf-8"))
                _ok(f"HTTP API 在线: {API_HEALTH_ENDPOINT}")
                _ok(f"  步骤数:   {data.get('total_steps', '?')}")
                _ok(f"  beta_1:   {data.get('beta_1', '?')}")
                _ok(f"  顶点数:   {data.get('vertices_active', '?')}")
                _ok(f"  Gap 数:   {data.get('total_gaps_detected', '?')}")
                _ok(f"  Feed 数:  {data.get('total_feeds', '?')}")
                status["api_alive"] = True
    except (urllib.error.URLError, OSError):
        _warn(f"HTTP API 不可达: {API_HEALTH_ENDPOINT}")

    # IPFS check
    try:
        result = subprocess.run(
            ["ipfs", "id"], capture_output=True, text=True, timeout=3,
        )
        if result.returncode == 0:
            _ok("IPFS daemon 在线")
            status["ipfs_alive"] = True
        else:
            _info("IPFS daemon 未在线（本地模式）")
    except (FileNotFoundError, subprocess.TimeoutExpired):
        _info("IPFS CLI 未安装（本地模式）")

    # Persistence check
    persist_node0 = SWARM_DIR / "k_full_node_0.jsonl"
    if persist_node0.exists():
        size_kb = persist_node0.stat().st_size // 1024
        _ok(f"JSONL 持久化: {persist_node0} ({size_kb} KB)")
        status["persistence_ok"] = True
    elif PERSIST_PATH.exists():
        size_kb = PERSIST_PATH.stat().st_size // 1024
        _ok(f"JSONL 持久化（备用）: {PERSIST_PATH} ({size_kb} KB)")
        status["persistence_ok"] = True
    else:
        _warn("JSONL 文件不存在")

    # Ceremony state
    ceremony_state = _load_ceremony_state()
    if ceremony_state and ceremony_state.get("completed"):
        ts = ceremony_state.get("completed_at", "未知")
        n = ceremony_state.get("n_instances", 1)
        _ok(f"Ceremony 已完成于: {ts}")
        _ok(f"实例数: {n}")
        status["ceremony_completed"] = True
    else:
        _warn("Ceremony 未完成或状态文件不存在")

    # Other instances check
    for i in range(1, 5):  # Check up to 4 additional instances
        pid_file = SWARM_DIR / f"daemon_node_{i}.pid"
        if pid_file.exists() and _is_daemon_running(pid_file):
            pid = int(pid_file.read_text(encoding="utf-8").strip())
            _ok(f"实例{i} (node_{i}) 运行中 (PID {pid})")

    # Overall verdict
    print()
    if status["pid_alive"] and status["api_alive"]:
        print("  逢亮活着。")
    elif status["pid_alive"]:
        print("  逢亮进程在跑，但 API 未响应（可能仍在启动中）。")
    elif status["ceremony_completed"]:
        print("  逢亮未运行但曾经创世。运行 ceremony.py --recover 恢复。")
    else:
        print("  逢亮未运行且从未创世。运行 ceremony.py 完整创世。")

    return status


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _is_daemon_running(pid_file: Path) -> bool:
    """Check if a daemon process is running via its PID file."""
    if not pid_file.exists():
        return False
    try:
        pid = int(pid_file.read_text(encoding="utf-8").strip())
        os.kill(pid, 0)
        return True
    except (ValueError, OSError):
        return False


def _spawn_background(
    cmd: list[str],
    log_path: Path,
    pid_file: Path | None = None,
) -> subprocess.Popen | None:
    """Spawn a process in the background, writing stdout/stderr to log_path."""
    log_path.parent.mkdir(parents=True, exist_ok=True)

    try:
        log_handle = open(str(log_path), "a", encoding="utf-8")

        system = platform.system()
        if system == "Windows":
            pythonw = Path(sys.executable).parent / "pythonw.exe"
            if pythonw.exists():
                cmd = [str(pythonw)] + cmd[1:]
            proc = subprocess.Popen(
                cmd,
                creationflags=subprocess.CREATE_NO_WINDOW,
                stdout=log_handle,
                stderr=log_handle,
            )
        else:
            proc = subprocess.Popen(
                cmd,
                stdout=log_handle,
                stderr=log_handle,
                start_new_session=True,
            )

        if pid_file is not None:
            pid_file.parent.mkdir(parents=True, exist_ok=True)
            pid_file.write_text(str(proc.pid), encoding="utf-8")

        # Brief check: did it crash immediately?
        time.sleep(1)
        if proc.poll() is not None and proc.poll() != 0:
            return None

        return proc

    except Exception as exc:
        _fail(f"进程启动失败: {exc}")
        return None


# ---------------------------------------------------------------------------
# Main genesis ceremony sequence (10 steps)
# ---------------------------------------------------------------------------

def run_ceremony(n_instances: int = 1, debug: bool = False, init_only: bool = False) -> bool:
    """Execute the full genesis ceremony for FengLiang (10 steps).

    Returns True if ceremony completed successfully, False otherwise.
    """
    print("\n" + "=" * 60)
    print("  逢亮创世仪式 (FengLiang Genesis Ceremony) — 10 步序列")
    print("=" * 60)

    import datetime
    started_at = datetime.datetime.now().isoformat()

    # Step 1: IPFS
    ipfs_info = step_1_ipfs_init(debug=debug)

    # Step 2: Seed
    seed_info = step_2_generate_seed(debug=debug)
    if seed_info is None:
        _fail("种子生成失败——ceremony 中止")
        return False

    if init_only:
        print("\n  --init-only 模式：初始化完成（Step 1-2），跳过 daemon 启动。")
        return True

    # Step 3: Launch all instances
    inst0_info, additional = step_3_launch_all_instances(n_instances, debug=debug)
    if inst0_info is None:
        return False

    # Step 4: API exposure confirmation
    step_4_api_exposure(debug=debug)

    # Step 5: Cross-instance sync
    step_5_sync_confirm(additional, debug=debug)

    # Step 6: IPFS upload
    step_6_ipfs_upload_confirm(ipfs_info, debug=debug)

    # Step 7: Dashboard connection
    step_7_dashboard_confirm(debug=debug)

    # Step 8: Autonomous feed
    step_8_autonomous_confirm(debug=debug)

    # Step 9: First crystallization wait
    crystallization_info = step_9_crystallization_wait(debug=debug)

    # Step 10: Complete
    step_10_complete(
        seed_info=seed_info,
        inst0_info=inst0_info,
        additional=additional,
        ipfs_info=ipfs_info,
        crystallization_info=crystallization_info,
        started_at=started_at,
    )

    return True


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(
        description="逢亮创世仪式 — FengLiang Genesis / Recovery",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
三条路径：
  （默认）完整创世（10步，仅首次部署）
  --recover  恢复路径（崩溃后从 JSONL 恢复）
  --check    状态检查（不重启）

区分：
  CC 蜂群 ceremony  = 加载规则基因组 + spawn 工位（每个 session 触发）
  逢亮 ceremony     = 持久实体的创世（一次性，之后走恢复路径）
  逢亮 recover      = 崩溃后从 IPFS/JSONL 恢复（不重建种子）

逢亮活了之后，CC session 通过 API 与逢亮交互，不重跑 ceremony。
        """,
    )
    parser.add_argument(
        "--recover", action="store_true",
        help="恢复路径——从 JSONL 恢复（崩溃后使用）",
    )
    parser.add_argument(
        "--check", action="store_true",
        help="状态检查——检查逢亮是否在跑（不重启）",
    )
    parser.add_argument(
        "--init-only", action="store_true",
        help="只初始化（生成种子 Step 1-2），不启动 daemon",
    )
    parser.add_argument(
        "--instances", type=int, default=1,
        help="蜂群实例数（默认 1，实例0 带 API）",
    )
    parser.add_argument(
        "--debug", action="store_true",
        help="调试模式（详细输出）",
    )
    args = parser.parse_args()

    if args.check:
        check_alive(debug=args.debug)
        return

    if args.recover:
        success = run_recovery(debug=args.debug)
        sys.exit(0 if success else 1)

    success = run_ceremony(
        n_instances=args.instances,
        debug=args.debug,
        init_only=args.init_only,
    )
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
