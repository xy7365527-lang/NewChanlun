"""审计模式配置 — 每种 audit_type 的 system prompt 和参数。

概念溯源: [新缠论] — 异质审计模式注册表
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

AuditType = Literal[
    "code_review", "security", "architecture", "math_verification",
]

_VALID_AUDIT_TYPES: frozenset[str] = frozenset(
    ["code_review", "security", "architecture", "math_verification"],
)


@dataclass(frozen=True, slots=True)
class AuditModeConfig:
    """单个审计模式的配置。"""

    system_prompt: str
    temperature: float
    max_tokens: int


_MODE_CONFIGS: dict[AuditType, AuditModeConfig] = {
    "code_review": AuditModeConfig(
        system_prompt=(
            "You are a senior code reviewer performing an independent audit. "
            "Focus on: correctness, edge cases, error handling, naming, "
            "immutability violations, and potential bugs. "
            "Structure your response as JSON with keys: "
            '"summary", "issues" (array of {severity, location, description, suggestion}), '
            '"verdict" ("pass"|"fail"|"warning").'
        ),
        temperature=0.2,
        max_tokens=4096,
    ),
    "security": AuditModeConfig(
        system_prompt=(
            "You are a security auditor performing an independent security review. "
            "Focus on: injection vulnerabilities (SQL, XSS, command), "
            "authentication/authorization flaws, secret exposure, "
            "input validation gaps, OWASP Top 10. "
            "Structure your response as JSON with keys: "
            '"summary", "vulnerabilities" (array of {severity, category, location, '
            'description, remediation}), "risk_level" ("critical"|"high"|"medium"|"low"|"none").'
        ),
        temperature=0.1,
        max_tokens=4096,
    ),
    "architecture": AuditModeConfig(
        system_prompt=(
            "You are a software architect performing an independent architecture review. "
            "Focus on: separation of concerns, coupling/cohesion, "
            "extensibility, consistency with stated patterns, "
            "violation of SOLID principles. "
            "Structure your response as JSON with keys: "
            '"summary", "findings" (array of {category, description, impact, recommendation}), '
            '"architecture_health" ("healthy"|"concerns"|"critical").'
        ),
        temperature=0.3,
        max_tokens=4096,
    ),
    "math_verification": AuditModeConfig(
        system_prompt=(
            "You are a mathematical verification agent. "
            "Verify the logical correctness of proofs, derivations, and formal arguments. "
            "Check: axiom validity, inference rule application, completeness of cases, "
            "hidden assumptions, logical gaps. "
            "Structure your response as JSON with keys: "
            '"summary", "steps_verified" (array of {step, status, issue}), '
            '"conclusion" ("valid"|"invalid"|"incomplete"), "gaps" (array of strings).'
        ),
        temperature=0.1,
        max_tokens=4096,
    ),
}


def get_mode_config(audit_type: str) -> AuditModeConfig:
    """获取审计模式配置。

    Raises
    ------
    ValueError
        audit_type 不在合法集合中。
    """
    if audit_type not in _VALID_AUDIT_TYPES:
        raise ValueError(
            f"Unknown audit_type: {audit_type!r}. "
            f"Valid types: {sorted(_VALID_AUDIT_TYPES)}"
        )
    return _MODE_CONFIGS[audit_type]
