"""隔离运行：从私有 .so（worktree @ commit 0c44be10a8 构建）跑 ES 1s unn 普查。

消除并发 session 对共享 site-packages .so 的竞态——私有模块路径置于 sys.path 首位，
断言 newchan_rust 确实从 /tmp/unn_es1s_mod 加载（committed strict 代码）。
"""
from __future__ import annotations

import sys

PRIV = "/tmp/unn_es1s_mod"
sys.path.insert(0, PRIV)

import newchan_rust as _nr  # noqa: E402

assert _nr.__file__.startswith(PRIV), f"模块未从私有路径加载：{_nr.__file__}"
print(f"[isolated] newchan_rust ← {_nr.__file__}")

# run_unn_es_1s 的 main 逻辑（不重复——直接导入复用）。其内部 import newchan_rust
# 会命中已缓存的私有模块（sys.modules['newchan_rust'] 已是私有）。
import run_unn_es_1s as R  # noqa: E402

raise SystemExit(R.main())
