"""#1768 L-2：pytest 不经过各测试脚本的 `--binary` 入口，正式 S 二进制只能由环境显式注入。

未设置时保持 `BINARY=None`，依赖它的用例照常报错，不静默跳过。
"""
import os
from pathlib import Path

import tb01c_query_tests

_binary = os.environ.get("S_STRUCTURE_SESSION_BINARY")
if _binary:
    tb01c_query_tests.BINARY = Path(_binary).resolve()
