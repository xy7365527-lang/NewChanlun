"""Claude 异质审计工位 — 通过 Anthropic API 委托审计任务给外部 Claude 实例。

实现异质审计：不同实例、不同 context 审查同一产出。
API key 从环境变量 ANTHROPIC_API_KEY 读取。

概念溯源: [新缠论] — 异质审计基础设施
"""

from __future__ import annotations

from newchan.claude_audit.engine import AuditResult, ClaudeAuditor

__all__ = [
    "AuditResult",
    "ClaudeAuditor",
]
