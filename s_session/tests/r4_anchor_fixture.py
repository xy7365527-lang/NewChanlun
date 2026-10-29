"""#1527 测试专用造锚夹具（Python 侧等价 Rust `admission_anchor::test_prepare`）。

只在 s_session/tests/ 下可达；生产模块不得引用（test_tb01c_r4_anchor_lock 机械锁）。
这不是生产首签算法：生产上首份锚由谁、凭什么签发归决策票 #1567，未决前生产无签锚入口。
"""

import contextlib
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import stat
from unittest.mock import patch

import recovery_admission as admission
import s_service_control as control

DEFAULT_LIMIT = "1000000"


def _encode(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def _identity(path):
    item = path.stat()
    return {"device": item.st_dev, "inode": item.st_ino, "uid": item.st_uid,
            "mode": stat.S_IMODE(item.st_mode)}


class TestAnchor:
    """已锚原件：owner 绑定 + state/s-admission/{binding,limits}.json + 两项 env 声明。"""

    def __init__(self, db, state_dir, *, limits=None, pending=False):
        self.db = Path(db).resolve(strict=True)
        self.state = Path(state_dir).resolve(strict=True)
        self.container = self.state / (admission.PENDING if pending else admission.PUBLISHED)
        self.container.mkdir(mode=0o700)
        values = {key: DEFAULT_LIMIT for key in admission.LIMIT_FIELDS}
        values.update(limits or {})
        self.raw = _encode(values) + b"\n"
        self.limits_path = self.container / "limits.json"
        self.limits_path.write_bytes(self.raw)
        self.limits_path.chmod(0o600)
        binding_path = self.container / "binding.json"
        binding_path.touch(mode=0o600)
        self.binding = {"db": str(self.db), "path": str(self.state / admission.PUBLISHED / "limits.json"),
                        "sha256": hashlib.sha256(self.raw).hexdigest(),
                        "file_identity": _identity(self.limits_path),
                        "binding_identity": _identity(binding_path),
                        "directory_identity": _identity(self.container)}
        binding_path.write_bytes(_encode(self.binding))
        self.owner_path = Path(str(self.db) + ".control-owner.json")
        self.owner_path.write_bytes(_encode({"db": str(self.db), "state_dir": str(self.state),
                                             "admission": self.binding}))
        self.owner_path.chmod(0o600)

    def env(self):
        return {admission.ENV_SHA: self.binding["sha256"],
                admission.ENV_IDENTITY: _encode(self.binding).decode()}


def test_prepare(db, state_dir, **kwargs):
    """建库（若缺）与 state 目录，再造一份已发布锚；同一库重复调用不覆盖既有 owner。"""
    db, state_dir = Path(db), Path(state_dir)
    state_dir.mkdir(parents=True, exist_ok=True)
    db.parent.mkdir(parents=True, exist_ok=True)
    if not db.exists():
        with contextlib.closing(sqlite3.connect(db)) as connection:
            connection.execute("CREATE TABLE IF NOT EXISTS sentinel(value TEXT)")
            connection.commit()
    if Path(str(db.resolve()) + ".control-owner.json").exists():
        return None
    return TestAnchor(db, state_dir, **kwargs)


def anchored_controller(config, state_dir, *, clock=None, **kwargs):
    """S 控制器测试入口：先造锚，再在锚声明 env 下构造（R4 只在构造时捕获 env）。

    原地补 config：db 换成解析后的真实路径（控制器后续命令用锚定的同一 DB），缺省补
    startup_timeout_ms。R4 在构造时固定本次启动绝对期限，故用假钟的用例须经 clock 在构造时注入。
    """
    config.setdefault("startup_timeout_ms", "2000")
    anchor = test_prepare(config["db"], state_dir, **kwargs)
    config["db"] = str(Path(config["db"]).resolve())
    with contextlib.ExitStack() as stack:
        stack.enter_context(patch.dict(os.environ, anchor.env() if anchor else {}))
        if clock is not None:
            stack.enter_context(patch.object(control.time, "monotonic", side_effect=clock))
        controller = control.Controller(config, Path(state_dir))
        # 原生命周期测试的夹具合同不具生产资格。合同门由 #1657 专用测试真实调用。
        controller.check_admission_contract = lambda: controller.admission.child_environment(
            controller.runtime_environment(controller.admission_deadline))
        return controller
