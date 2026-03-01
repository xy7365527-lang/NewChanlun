"""ceremony_state.py — ceremony 步骤状态文件管理。

提供 write_step / read_step / clear_step / is_in_ceremony 四个操作，
状态持久化到项目根目录 .ceremony-step（JSON）。

270号更新：suspended 工位管理——suspend_workstation / unsuspend_workstation / get_suspended_workstations。
状态持久化到 .ceremony-suspended（JSON）。

283号更新：WAL 级 ceremony state（缺口B）+ team init 事务标记（缺口C）。
WAL 持久化到 .chanlun/ceremony_wal.json（原子写入：tmp→rename）。
Team init 标记持久化到 .chanlun/team_init/{team_name}.json。
"""

import hashlib
import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path

_PROJECT_ROOT = Path(__file__).resolve().parent.parent

# 状态文件路径：项目根目录下 .ceremony-step
_STATE_FILE = _PROJECT_ROOT / ".ceremony-step"

# suspended 工位文件路径：项目根目录下 .ceremony-suspended
_SUSPENDED_FILE = _PROJECT_ROOT / ".ceremony-suspended"

# WAL 级 ceremony state（283号缺口B）
_WAL_FILE = _PROJECT_ROOT / ".chanlun" / "ceremony_wal.json"

# Team init 标记目录（283号缺口C）
_TEAM_INIT_DIR = _PROJECT_ROOT / ".chanlun" / "team_init"

# WAL phase 枚举
WAL_PHASES = frozenset({
    "SCAN_DONE", "SPAWN_DONE", "RESCAN_STARTED", "RESCAN_DONE", "EVAL_DONE",
})


def write_step(step: int, phase: str) -> None:
    """写入当前 ceremony 步骤。"""
    payload = {
        "step": step,
        "phase": phase,
        "timestamp": datetime.now(timezone.utc).isoformat(),
    }
    _STATE_FILE.write_text(json.dumps(payload, ensure_ascii=False), encoding="utf-8")


def read_step() -> dict | None:
    """读取当前步骤，文件不存在返回 None。"""
    if not _STATE_FILE.exists():
        return None
    try:
        return json.loads(_STATE_FILE.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError):
        return None


def clear_step() -> None:
    """清除状态文件。"""
    try:
        _STATE_FILE.unlink(missing_ok=True)
    except OSError:
        pass


def is_in_ceremony() -> bool:
    """状态文件存在 = 在 ceremony 中。"""
    return _STATE_FILE.exists()


def _read_suspended_data() -> dict:
    """读取 suspended 文件，返回 {name: {reason, suspended_at}} 字典。"""
    if not _SUSPENDED_FILE.exists():
        return {}
    try:
        data = json.loads(_SUSPENDED_FILE.read_text(encoding="utf-8"))
        if isinstance(data, dict):
            return data
        return {}
    except (json.JSONDecodeError, OSError):
        return {}


def _write_suspended_data(data: dict) -> None:
    """写入 suspended 数据到文件。"""
    _SUSPENDED_FILE.write_text(
        json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8",
    )


def suspend_workstation(name: str, reason: str) -> None:
    """将工位标记为 suspended。

    Args:
        name: 工位名称（与 ceremony_scan workstations 中的 name 字段匹配）
        reason: 挂起原因（如 "stance-repetition"）
    """
    data = _read_suspended_data()
    data[name] = {
        "reason": reason,
        "suspended_at": datetime.now(timezone.utc).isoformat(),
    }
    _write_suspended_data(data)


def unsuspend_workstation(name: str) -> bool:
    """恢复 suspended 工位。返回 True 表示成功移除，False 表示工位不在 suspended 列表中。"""
    data = _read_suspended_data()
    if name not in data:
        return False
    del data[name]
    _write_suspended_data(data)
    return True


def get_suspended_workstations() -> dict:
    """获取所有 suspended 工位。返回 {name: {reason, suspended_at}} 字典。"""
    return _read_suspended_data()


# ═══════════════════════════════════════════════════════════════
# WAL 级 ceremony state（283号缺口B）
# ═══════════════════════════════════════════════════════════════


def _atomic_write_json(path: Path, payload: dict) -> None:
    """原子写入 JSON：tmp→os.replace()。POSIX 上原子，Windows 上 best-effort。"""
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp_path = path.with_suffix(".json.tmp")
    tmp_path.write_text(
        json.dumps(payload, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    os.replace(str(tmp_path), str(path))


def write_ceremony_state(
    epoch: int,
    phase: str,
    rescan_hash: str | None = None,
    workstations: list[str] | None = None,
) -> None:
    """WAL 级 ceremony 状态写入。原子落盘（tmp→rename）。

    恢复规则（由 Lead 在 ceremony 启动时读取）：
    - 文件不存在 → 正常启动（epoch=1）
    - phase=RESCAN_STARTED 且无 RESCAN_DONE → 重做 rescan（幂等）
    - phase=RESCAN_DONE → 执行 evaluate
    - phase=SPAWN_DONE → 检查 checkpoint；无 checkpoint → 降级为 rescan
    """
    if phase not in WAL_PHASES:
        raise ValueError(f"Invalid WAL phase: {phase}. Must be one of {sorted(WAL_PHASES)}")

    # 读取旧状态以保留 prev_rescan_hash
    old = read_ceremony_state()
    prev_hash = old.get("rescan_hash") if old else None

    payload = {
        "epoch": epoch,
        "phase": phase,
        "last_transition_ts": datetime.now(timezone.utc).isoformat(),
        "rescan_hash": rescan_hash,
        "workstations": list(workstations) if workstations else [],
        "prev_rescan_hash": prev_hash,
    }
    _atomic_write_json(_WAL_FILE, payload)


def read_ceremony_state() -> dict | None:
    """读取 WAL 状态。文件不存在或损坏返回 None。"""
    if not _WAL_FILE.exists():
        return None
    try:
        return json.loads(_WAL_FILE.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError):
        return None


def clear_ceremony_state() -> None:
    """清除 WAL 状态（ceremony 正常终止时调用）。"""
    try:
        _WAL_FILE.unlink(missing_ok=True)
    except OSError:
        pass
    # 同时清理可能残留的 tmp 文件
    tmp = _WAL_FILE.with_suffix(".json.tmp")
    try:
        tmp.unlink(missing_ok=True)
    except OSError:
        pass


def compute_rescan_hash(scan_output: dict) -> str:
    """从 scan 输出计算 hash，用于不动点检测。

    只取 workstations 的 name 列表排序后 hash，忽略 timestamp 等易变字段。
    """
    ws_names = sorted(
        w.get("name", "") for w in scan_output.get("workstations", [])
    )
    content = json.dumps(ws_names, ensure_ascii=False, sort_keys=True)
    return hashlib.sha256(content.encode("utf-8")).hexdigest()[:16]


# ═══════════════════════════════════════════════════════════════
# Team init 事务标记（283号缺口C）
# ═══════════════════════════════════════════════════════════════


def mark_team_init_started(team_name: str) -> None:
    """标记子蜂群初始化开始。写入 .chanlun/team_init/{team_name}.json"""
    payload = {
        "team_name": team_name,
        "status": "started",
        "started_at": datetime.now(timezone.utc).isoformat(),
    }
    target = _TEAM_INIT_DIR / f"{team_name}.json"
    _atomic_write_json(target, payload)


def mark_team_init_complete(team_name: str) -> None:
    """标记子蜂群初始化完成。"""
    target = _TEAM_INIT_DIR / f"{team_name}.json"
    existing = {}
    if target.exists():
        try:
            existing = json.loads(target.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, OSError):
            pass
    existing["status"] = "complete"
    existing["completed_at"] = datetime.now(timezone.utc).isoformat()
    _atomic_write_json(target, existing)


def get_incomplete_team_inits() -> list[str]:
    """扫描未完成的 team 初始化。

    ceremony_scan 调用此函数，将未完成的初始化转化为 P0 工位。
    """
    if not _TEAM_INIT_DIR.is_dir():
        return []
    incomplete = []
    for f in _TEAM_INIT_DIR.iterdir():
        if f.suffix == ".json" and not f.name.endswith(".tmp"):
            try:
                data = json.loads(f.read_text(encoding="utf-8"))
                if data.get("status") != "complete":
                    incomplete.append(data.get("team_name", f.stem))
            except (json.JSONDecodeError, OSError):
                incomplete.append(f.stem)
    return incomplete


# CLI 入口：供 hook 脚本调用
if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(
            "Usage: ceremony_state.py <write|read|clear|check"
            "|suspend|unsuspend|list-suspended"
            "|write-ceremony|read-ceremony|clear-ceremony"
            "|mark-team-init|mark-team-complete|list-incomplete-teams> [args...]"
        )
        sys.exit(1)

    cmd = sys.argv[1]

    if cmd == "write":
        if len(sys.argv) < 4:
            print("Usage: ceremony_state.py write <step> <phase>")
            sys.exit(1)
        write_step(int(sys.argv[2]), sys.argv[3])
    elif cmd == "read":
        data = read_step()
        if data is None:
            sys.exit(1)
        print(json.dumps(data, ensure_ascii=False))
    elif cmd == "clear":
        clear_step()
    elif cmd == "check":
        sys.exit(0 if is_in_ceremony() else 1)
    elif cmd == "suspend":
        if len(sys.argv) < 4:
            print("Usage: ceremony_state.py suspend <name> <reason>")
            sys.exit(1)
        suspend_workstation(sys.argv[2], sys.argv[3])
    elif cmd == "unsuspend":
        if len(sys.argv) < 3:
            print("Usage: ceremony_state.py unsuspend <name>")
            sys.exit(1)
        removed = unsuspend_workstation(sys.argv[2])
        if not removed:
            print(f"工位 '{sys.argv[2]}' 不在 suspended 列表中")
            sys.exit(1)
    elif cmd == "list-suspended":
        suspended = get_suspended_workstations()
        print(json.dumps(suspended, ensure_ascii=False, indent=2))
    # ── WAL ceremony state CLI ──
    elif cmd == "write-ceremony":
        if len(sys.argv) < 4:
            print("Usage: ceremony_state.py write-ceremony <epoch> <phase> [rescan_hash] [workstations_json]")
            sys.exit(1)
        epoch_val = int(sys.argv[2])
        phase_val = sys.argv[3]
        rh = sys.argv[4] if len(sys.argv) > 4 else None
        ws = json.loads(sys.argv[5]) if len(sys.argv) > 5 else None
        write_ceremony_state(epoch_val, phase_val, rescan_hash=rh, workstations=ws)
    elif cmd == "read-ceremony":
        data = read_ceremony_state()
        if data is None:
            sys.exit(1)
        print(json.dumps(data, ensure_ascii=False, indent=2))
    elif cmd == "clear-ceremony":
        clear_ceremony_state()
    # ── Team init CLI ──
    elif cmd == "mark-team-init":
        if len(sys.argv) < 3:
            print("Usage: ceremony_state.py mark-team-init <team_name>")
            sys.exit(1)
        mark_team_init_started(sys.argv[2])
    elif cmd == "mark-team-complete":
        if len(sys.argv) < 3:
            print("Usage: ceremony_state.py mark-team-complete <team_name>")
            sys.exit(1)
        mark_team_init_complete(sys.argv[2])
    elif cmd == "list-incomplete-teams":
        teams = get_incomplete_team_inits()
        print(json.dumps(teams, ensure_ascii=False, indent=2))
    else:
        print(f"Unknown command: {cmd}")
        sys.exit(1)
