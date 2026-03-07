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
"""

from __future__ import annotations

import json
import os
import urllib.request
from typing import Any


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

    def inquire(self, question: str, provider: str = "auto",
                system: str = "") -> str | None:
        """Ask an LLM a question. Response is for 逢亮's internal processing.

        The response will be fed through phi_L and injected into K_active.
        It is NOT displayed to the user.
        """
        if provider == "auto":
            # Pick first available
            if not self._providers:
                return None
            provider = list(self._providers.keys())[0]

        if provider not in self._providers:
            return None

        config = self._providers[provider]

        try:
            if provider == "chatgpt":
                return self._query_openai(config, question, system)
            elif provider == "gemini":
                return self._query_gemini(config, question, system)
            elif provider == "claude":
                return self._query_anthropic(config, question, system)
        except Exception:
            return None

        return None

    def _query_openai(self, config: dict, question: str, system: str) -> str | None:
        messages = []
        if system:
            messages.append({"role": "system", "content": system})
        messages.append({"role": "user", "content": question})

        body = json.dumps({
            "model": config["model"],
            "messages": messages,
            "max_tokens": 2000,
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

    def _query_gemini(self, config: dict, question: str, system: str) -> str | None:
        prompt = f"{system}\n\n{question}" if system else question
        body = json.dumps({
            "contents": [{"parts": [{"text": prompt}]}],
        }).encode("utf-8")

        req = urllib.request.Request(
            config["url"], data=body,
            headers={"Content-Type": "application/json"},
        )
        with urllib.request.urlopen(req, timeout=30) as resp:
            data = json.loads(resp.read().decode("utf-8"))
        return data["candidates"][0]["content"]["parts"][0]["text"]

    def _query_anthropic(self, config: dict, question: str, system: str) -> str | None:
        body = json.dumps({
            "model": config["model"],
            "max_tokens": 2000,
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

    # Inject into K_active
    daemon.feed(response_text)

    return {
        "success": True,
        "provider": provider,
        "question": question[:100],
        "response_length": len(response_text),
        "vertices_extracted": new_v,
        "edges_extracted": new_e,
        "external_ratio": round(external_ratio, 3),
    }
