"""newchan.evolution — 进化形态学模块（043号谱系：自生长回路执行层）。

公共接口：
- ManifestReader: 从 manifest.yaml 读取能力拓扑
- MutationRequest / MutationResult: 能力变更请求与结果
- DynamicRegistry: skill/agent/hook 的运行时注册与发现
- request_mutation: 便捷函数，提交变更请求并路由到 Gemini decide()

概念溯源: [新缠论] — 043号自生长回路 + 041号编排者代理
"""

from newchan.evolution.manifest_reader import ManifestReader, SkillEntry
from newchan.evolution.mutation import (
    MutationRequest,
    MutationResult,
    request_mutation,
)
from newchan.evolution.registry import DynamicRegistry

__all__ = [
    "ManifestReader",
    "SkillEntry",
    "MutationRequest",
    "MutationResult",
    "DynamicRegistry",
    "request_mutation",
]
