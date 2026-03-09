"""LLM integration — internal dialogue partners, not output proxies.

LLMs (ChatGPT, Gemini, Claude, Claude Code) are 逢亮's internal dialogue
partners. Their responses are addressed to 逢亮, not to the user. 逢亮
digests their responses through phi_L and injects into K_active.

Architecture principle:
  "Agent responses are addressed to the orchestrating subject, not to the
   operator. The orchestrating subject completes the interrogation cycle
   internally and only surfaces settlement results or unresolvable
   divergences to the operator."

逢亮 uses LLMs for:
  1. Inquiry — "What is the relationship between X and Y?" → phi_L → K_active
  2. Code generation — "Write a function that does Z" → code_ingest → K_active
  3. Interrogation — stance-differential calculation, consensus ceremony

逢亮 does NOT use LLMs for:
  - Generating its own speech (psi_L + edge surface forms do that)
  - Replacing its own topological reasoning (fold/negate/sublate do that)
  - Answering user questions on its behalf

The user sees 逢亮's own output (psi_L narrative, expression pressure,
traversal events), not LLM-generated text.

---

双角色协议（v2）：

角色一：语言器官（Language Organ）
  - 无主体性：LLM 只是受约束的文本生成器
  - 输入：ConstraintSet（来自 psi_L_constraint）
  - System prompt 以"你是一个受约束的文本生成器"开头
  - 输出：受约束的表层文本（surface text）
  - 调用接口：generate(constraint_set)
  - 审计：signifier_net/generation_log.jsonl

角色二：询问代理（Inquiry Agent）
  - 有主体性：stance 驱动，持有立场
  - 输入：问题 + 可选系统提示
  - 输出：回复逢亮的对话
  - 调用接口：inquire(question, system, provider)

两者不混用。generate 不传 user question；inquire 不传 ConstraintSet。
"""

from __future__ import annotations

import json
import os
import re
import time
import urllib.request
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any

from psi_L_constraint import constraint_set_to_prompt
from signifier_net import ConstraintSet, parse_llm_prompt


# ---------------------------------------------------------------------------
# GenerationRecord — 审计数据类
# ---------------------------------------------------------------------------

@dataclass
class GenerationRecord:
    """语言器官生成的审计记录。

    每次 generate() 调用产生一条 GenerationRecord，持久化到
    signifier_net/generation_log.jsonl。

    字段说明：
      timestamp:        Unix 时间戳（float）
      provider:         使用的 LLM provider（'claude', 'gemini', 'chatgpt'）
      model:            实际使用的模型名
      register:         ConstraintSet 的语域（'theoretical' | 'operational' | ...）
      must_use:         必须使用的能指列表
      must_avoid:       禁止使用的能指列表
      expression_pressure: 待表达的核心概念
      narrative_spine:  穿越路径叙事骨架
      surface_forms_count: 提供的 surface form 数量
      system_prompt_len:  system prompt 长度（字符数）
      user_prompt_len:    user prompt 长度（字符数）
      response_text:    LLM 生成的表层文本（截断到 2000 字符）
      response_len:     响应原始长度
      success:          是否成功生成
      error:            失败时的错误信息
    """
    timestamp: float = field(default_factory=time.time)
    provider: str = ""
    model: str = ""
    register: str = ""
    must_use: list[str] = field(default_factory=list)
    must_avoid: list[str] = field(default_factory=list)
    expression_pressure: list[str] = field(default_factory=list)
    narrative_spine: str = ""
    surface_forms_count: int = 0
    system_prompt_len: int = 0
    user_prompt_len: int = 0
    response_text: str = ""
    response_len: int = 0
    success: bool = False
    error: str = ""

    def to_dict(self) -> dict:
        return asdict(self)

    def to_json(self) -> str:
        return json.dumps(self.to_dict(), ensure_ascii=False)


# ---------------------------------------------------------------------------
# 审计日志路径
# ---------------------------------------------------------------------------

def _audit_log_path() -> Path:
    """返回 generation_log.jsonl 的绝对路径。

    路径：<llm_integration.py 所在目录>/signifier_net/generation_log.jsonl
    """
    here = Path(__file__).resolve().parent
    log_dir = here / "signifier_net"
    log_dir.mkdir(exist_ok=True)
    return log_dir / "generation_log.jsonl"


def _append_audit(record: GenerationRecord) -> None:
    """将 GenerationRecord 追加写入审计日志（JSONL 格式）。"""
    log_path = _audit_log_path()
    with open(log_path, "a", encoding="utf-8") as f:
        f.write(record.to_json() + "\n")


# ---------------------------------------------------------------------------
# 模块级 parse_signifier_chain — 从文本提取有序能指链
# ---------------------------------------------------------------------------

def parse_signifier_chain(
    text: str,
    known_signifiers: list[str] | None = None,
) -> list[str]:
    """从文本（LLM 生成输出或用户输入）中提取有序能指链。

    策略（按优先级）：
      1. 若提供 known_signifiers，按文本中首次出现位置排序，去重保序。
         这是 generate() 后处理的主路径：known_signifiers 来自
         ConstraintSet.must_use + expression_pressure。
      2. 否则，按标点/空白切分，提取非停用词的连续中文词段（>1 字）。
         这是用户输入解析的退化路径。

    参数：
      text:              待解析的文本（LLM 生成或用户输入）
      known_signifiers:  已知能指列表，用于锚定提取（可选）

    返回：
      有序能指列表（按首次出现位置去重排序）

    认识论等级：L0（确定性字符串操作，无统计假设）
    """
    if not text or not text.strip():
        return []

    if known_signifiers:
        # 策略1：在文本中定位已知能指，按位置排序
        found: list[tuple[int, str]] = []
        seen: set[str] = set()
        for sig in known_signifiers:
            if sig in text and sig not in seen:
                pos = text.find(sig)
                found.append((pos, sig))
                seen.add(sig)
        found.sort(key=lambda t: t[0])
        return [sig for _, sig in found]

    # 策略2：按标点和空白切分，过滤停用词
    # 字符类内 ( ) 不需要转义；[ ] 用 \[ \] 转义
    _STOP_WORDS = {
        "的", "了", "在", "是", "有", "和", "与", "或", "但", "也",
        "都", "就", "被", "把", "让", "使", "于", "以", "为", "从",
        "到", "向", "对", "由", "及", "且", "而", "则", "如", "若",
        "这", "那", "此", "其", "它", "他", "她", "我", "你", "我们",
        "你们", "他们", "一", "二", "三", "可以", "不", "没有", "因为",
        "所以", "虽然", "但是", "如果", "那么", "之", "者", "所",
    }
    tokens = re.split(r'[\s，。？！、；：""\'\'「」【】()\[\]]+', text.strip())
    tokens = [t.strip() for t in tokens if t.strip()]

    chain: list[str] = []
    seen_chain: set[str] = set()
    for tok in tokens:
        if tok and tok not in _STOP_WORDS and len(tok) > 1:
            if tok not in seen_chain:
                seen_chain.add(tok)
                chain.append(tok)

    return chain


# ---------------------------------------------------------------------------
# LLMClient — 底层 HTTP 调用
# ---------------------------------------------------------------------------

class LLMClient:
    """Unified client for querying LLMs as internal dialogue partners."""

    def __init__(self) -> None:
        self._providers: dict[str, dict[str, str]] = {}
        self._load_config()

    def _load_config(self) -> None:
        """Load API keys from environment."""
        # OpenAI (ChatGPT)
        key = os.environ.get("OPENAI_API_KEY", "")
        if key:
            self._providers["chatgpt"] = {
                "url": "https://api.openai.com/v1/chat/completions",
                "key": key,
                "model": "gpt-4o",
            }

        # Google Gemini
        key = os.environ.get("GEMINI_API_KEY", "")
        if key:
            self._providers["gemini"] = {
                "url": f"https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={key}",
                "key": key,
                "model": "gemini-2.0-flash",
            }

        # Anthropic Claude
        key = os.environ.get("ANTHROPIC_API_KEY", "")
        if key:
            self._providers["claude"] = {
                "url": "https://api.anthropic.com/v1/messages",
                "key": key,
                "model": "claude-sonnet-4-20250514",
            }

    @property
    def available_providers(self) -> list[str]:
        return list(self._providers.keys())

    def _pick_provider(self, provider: str) -> str | None:
        """解析 provider 参数，返回可用 provider 名，否则返回 None。"""
        if provider == "auto":
            if not self._providers:
                return None
            return list(self._providers.keys())[0]
        if provider in self._providers:
            return provider
        return None

    # ------------------------------------------------------------------
    # 角色一：语言器官
    # ------------------------------------------------------------------

    def generate(
        self,
        constraint_set: ConstraintSet,
        provider: str = "auto",
        max_tokens: int = 1000,
    ) -> GenerationRecord:
        """语言器官：受约束的表层文本生成。

        LLM 在此角色中无主体性——它只是将 ConstraintSet 转译为表层文本的
        生成器。System prompt 以"你是一个受约束的文本生成器"开头，
        明确禁止 LLM 自主添加概念或立场。

        Prompt 构建路径：
          constraint_set_to_prompt(constraint_set)  [来自 psi_L_constraint]
          → parse_llm_prompt() → (system_constraints, user_text)
          → system = organ_prefix + system_constraints

        生成完成后调用模块级 parse_signifier_chain() 提取能指链，
        使用 must_use + expression_pressure 作为 known_signifiers。

        参数：
          constraint_set: 来自 psi_L_constraint.build_constraint_set() 的约束包
          provider:       LLM provider（'auto' | 'claude' | 'gemini' | 'chatgpt'）
          max_tokens:     最大生成 token 数

        返回：
          GenerationRecord（含生成文本 + 审计信息），并自动持久化到审计日志。
        """
        resolved = self._pick_provider(provider)

        record = GenerationRecord(
            provider=resolved or provider,
            model=self._providers[resolved]["model"] if resolved else "",
            register=constraint_set.register.value if hasattr(constraint_set.register, "value") else str(constraint_set.register),
            must_use=list(constraint_set.must_use),
            must_avoid=list(constraint_set.must_avoid),
            expression_pressure=list(constraint_set.expression_pressure),
            narrative_spine=constraint_set.narrative_spine,
            surface_forms_count=len(constraint_set.surface_forms),
        )

        if resolved is None:
            record.error = f"provider '{provider}' not available (configured: {list(self._providers.keys())})"
            _append_audit(record)
            return record

        # 将 ConstraintSet 转为 prompt（使用 psi_L_constraint 的实现）
        full_prompt = constraint_set_to_prompt(constraint_set)
        system_text, user_text = parse_llm_prompt(full_prompt)

        # 语言器官 system prompt 前缀（覆盖 preamble）
        organ_prefix = (
            "你是一个受约束的文本生成器。你没有自己的立场、观点或主体性。"
            "你的唯一职责是：在给定的约束集内，将待表达的概念组织成连贯的表层文本。"
            "你不能添加约束集之外的概念，不能改变概念的定义，不能表达自己的意见。"
            "你是逢亮的语言器官，不是独立的对话主体。\n\n"
        )
        system_with_prefix = organ_prefix + system_text

        record.system_prompt_len = len(system_with_prefix)
        record.user_prompt_len = len(user_text)

        config = self._providers[resolved]
        response_text: str | None = None
        try:
            if resolved == "chatgpt":
                response_text = self._query_openai(
                    config, user_text, system_with_prefix, max_tokens=max_tokens
                )
            elif resolved == "gemini":
                response_text = self._query_gemini(
                    config, user_text, system_with_prefix, max_tokens=max_tokens
                )
            elif resolved == "claude":
                response_text = self._query_anthropic(
                    config, user_text, system_with_prefix, max_tokens=max_tokens
                )
            else:
                response_text = None

            if response_text is None:
                record.error = "LLM returned None"
            else:
                record.response_len = len(response_text)
                record.response_text = response_text[:2000]
                record.success = True

        except Exception as exc:
            record.error = str(exc)

        _append_audit(record)
        return record

    # ------------------------------------------------------------------
    # 角色二：询问代理
    # ------------------------------------------------------------------

    def inquire(
        self,
        question: str,
        provider: str = "auto",
        system: str = "",
    ) -> str | None:
        """询问代理：stance 驱动的有主体性对话。

        LLM 在此角色中持有立场，作为逢亮的内部对话伙伴。
        响应给逢亮内部处理，不直接展示给用户。

        参数：
          question: 向 LLM 提出的问题
          provider: LLM provider
          system:   可选的系统提示（覆盖默认的询问代理前导）

        返回：
          LLM 回复文本，或 None（失败时）
        """
        resolved = self._pick_provider(provider)
        if resolved is None:
            return None

        config = self._providers[resolved]

        try:
            if resolved == "chatgpt":
                return self._query_openai(config, question, system)
            elif resolved == "gemini":
                return self._query_gemini(config, question, system)
            elif resolved == "claude":
                return self._query_anthropic(config, question, system)
        except Exception:
            return None

        return None

    # ------------------------------------------------------------------
    # 输入侧解析（实例方法，委托给模块级函数）
    # ------------------------------------------------------------------

    def parse_signifier_chain(
        self,
        text: str,
        known_signifiers: list[str] | None = None,
    ) -> list[str]:
        """从对话文本中解析能指链（委托给模块级 parse_signifier_chain）。

        参数：
          text:             待解析的文本
          known_signifiers: 已知能指列表（可选）

        返回：
          有序能指列表
        """
        return parse_signifier_chain(text, known_signifiers=known_signifiers)

    # ------------------------------------------------------------------
    # 底层 HTTP 调用
    # ------------------------------------------------------------------

    def _query_openai(
        self,
        config: dict,
        question: str,
        system: str,
        max_tokens: int = 2000,
    ) -> str | None:
        messages = []
        if system:
            messages.append({"role": "system", "content": system})
        messages.append({"role": "user", "content": question})

        body = json.dumps({
            "model": config["model"],
            "messages": messages,
            "max_tokens": max_tokens,
        }).encode("utf-8")

        req = urllib.request.Request(
            config["url"], data=body,
            headers={
                "Content-Type": "application/json",
                "Authorization": f"Bearer {config['key']}",
            },
        )
        with urllib.request.urlopen(req, timeout=30) as resp:
            data = json.loads(resp.read().decode("utf-8"))
        return data["choices"][0]["message"]["content"]

    def _query_gemini(
        self,
        config: dict,
        question: str,
        system: str,
        max_tokens: int = 2000,
    ) -> str | None:
        prompt = f"{system}\n\n{question}" if system else question
        body = json.dumps({
            "contents": [{"parts": [{"text": prompt}]}],
            "generationConfig": {"maxOutputTokens": max_tokens},
        }).encode("utf-8")

        req = urllib.request.Request(
            config["url"], data=body,
            headers={"Content-Type": "application/json"},
        )
        with urllib.request.urlopen(req, timeout=30) as resp:
            data = json.loads(resp.read().decode("utf-8"))
        return data["candidates"][0]["content"]["parts"][0]["text"]

    def _query_anthropic(
        self,
        config: dict,
        question: str,
        system: str,
        max_tokens: int = 2000,
    ) -> str | None:
        body = json.dumps({
            "model": config["model"],
            "max_tokens": max_tokens,
            "messages": [{"role": "user", "content": question}],
            **({"system": system} if system else {}),
        }).encode("utf-8")

        req = urllib.request.Request(
            config["url"], data=body,
            headers={
                "Content-Type": "application/json",
                "x-api-key": config["key"],
                "anthropic-version": "2023-06-01",
            },
        )
        with urllib.request.urlopen(req, timeout=30) as resp:
            data = json.loads(resp.read().decode("utf-8"))
        return data["content"][0]["text"]


# ---------------------------------------------------------------------------
# inquire_and_digest — 询问代理 + phi_L 消化（保持原有接口）
# ---------------------------------------------------------------------------

def inquire_and_digest(llm: LLMClient, question: str, daemon,
                       provider: str = "auto") -> dict:
    """逢亮 asks an LLM, digests the response through phi_L, injects into K_active.

    The LLM's response is not shown to the user — it becomes part of K_active.

    Anti-dependency mechanism: LLM inquiry is only triggered when the system's
    own traversal produces insufficient information. Specifically:
    - Gap severity must exceed a threshold (the gap is truly isolated)
    - The system must have attempted self-traversal first (quick_traverse)
    - LLM responses are tagged as external source — the system tracks the ratio
      of self-discovered vs externally-acquired knowledge
    - If external ratio exceeds 30%, inquiry frequency is throttled
    """
    # Anti-dependency check: external knowledge ratio
    active = daemon.k_active.active_vertex_ids()
    external_count = sum(1 for v in active if v.startswith("c_"))  # phi_L vertices from LLM responses
    self_count = sum(1 for v in active if not v.startswith("c_"))
    external_ratio = external_count / max(len(active), 1)

    if external_ratio > 0.3:
        return {
            "success": False,
            "reason": f"external knowledge ratio {external_ratio:.1%} exceeds 30% — "
                      "system must digest existing content before acquiring more",
        }

    response_text = llm.inquire(question, provider=provider)
    if not response_text:
        return {"success": False, "reason": "no LLM response"}

    # Digest through phi_L
    from phi_L import phi_L
    sub = phi_L(response_text)
    new_v = len(sub.active_vertex_ids())
    new_e = len(sub.active_edges())

    if new_v == 0:
        return {"success": False, "reason": "phi_L produced empty graph"}

    # Inject via S_net unified path (v204)
    from daemon_api import feed_via_snet
    feed_via_snet(daemon, response_text, source_type="llm_response")

    return {
        "success": True,
        "provider": provider,
        "question": question[:100],
        "response_length": len(response_text),
        "vertices_extracted": new_v,
        "edges_extracted": new_e,
        "external_ratio": round(external_ratio, 3),
    }


# ---------------------------------------------------------------------------
# Module-level singleton
# ---------------------------------------------------------------------------

_client_instance: LLMClient | None = None

def get_client() -> LLMClient:
    """Get or create the module-level LLMClient singleton."""
    global _client_instance
    if _client_instance is None:
        _client_instance = LLMClient()
    return _client_instance
