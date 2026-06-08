#!/usr/bin/env python3
"""三条候选拓扑路径探索 —— Gemini 3 Pro 异质源（OpenAI 配额耗尽后的替代）。

背景：编排者要求用最强 pro 推理模型探索三条统一缠论形态学与 PH 的拓扑路径。
OpenAI API 全模型返回 429 insufficient_quota（账户配额耗尽，gpt-5.5/5.5-pro/4.1 同）。
GOOGLE_API_KEY 可用 → 改用 gemini-3-pro-preview（当前最强推理模型之一，且为真异质源——
非 OpenAI 同族，正是 §18.8 审计当初 403 拿不到的 Google 源）。

提示词与 unified_topology_three_paths.py 完全一致（同一探索任务）。
输出：.chanlun/review-results/gemini-pro-three-paths-<ts>.md，显著标注 provider 替代。
认识论等级：L0（纯定义 + 探索，零新数据）。
"""

import json
import os
import sys
import time
import urllib.request
from datetime import datetime
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# 与 unified_topology_three_paths.py 的 build_prompt 完全相同的探索 prompt
sys.path.insert(0, str(REPO_ROOT / "analysis"))
from unified_topology_three_paths import build_prompt  # noqa: E402

MODEL_CHAIN = ["gemini-3-pro-preview", "gemini-2.5-pro", "gemini-pro-latest"]


def call_gemini(prompt: str) -> tuple[str, str]:
    key = os.environ.get("GOOGLE_API_KEY")
    if not key:
        print("ERROR: GOOGLE_API_KEY not set", file=sys.stderr)
        sys.exit(1)

    last_error: str | None = None
    for model in MODEL_CHAIN:
        url = (
            f"https://generativelanguage.googleapis.com/v1beta/models/"
            f"{model}:generateContent?key={key}"
        )
        body = json.dumps(
            {
                "contents": [{"parts": [{"text": prompt}]}],
                "generationConfig": {"temperature": 0.4, "maxOutputTokens": 32768},
            }
        ).encode("utf-8")
        try:
            print(f"Calling {model} (Gemini REST)...", file=sys.stderr)
            t0 = time.time()
            req = urllib.request.Request(
                url, data=body, headers={"Content-Type": "application/json"}
            )
            with urllib.request.urlopen(req, timeout=600) as r:
                data = json.load(r)
            dt = time.time() - t0
            cand = data["candidates"][0]
            parts = cand.get("content", {}).get("parts", [])
            text = "\n".join(p.get("text", "") for p in parts)
            usage = data.get("usageMetadata")
            print(f"Success {model} in {dt:.0f}s. usage={usage}", file=sys.stderr)
            if not text.strip():
                last_error = f"{model}: empty text, finishReason={cand.get('finishReason')}"
                print(last_error, file=sys.stderr)
                continue
            return model, text
        except Exception as e:  # noqa: BLE001 — surface any failure, then 降级
            last_error = f"{model}: {str(e)[:300]}"
            print(f"Model {model} failed: {last_error}", file=sys.stderr)

    return "none", f"ERROR: all Gemini models failed. Last: {last_error}"


def main() -> None:
    prompt = build_prompt()
    print(f"Prompt length: {len(prompt)} chars", file=sys.stderr)
    model_used, content = call_gemini(prompt)

    ts = datetime.now().strftime("%Y%m%d-%H%M%S")
    out = REPO_ROOT / ".chanlun" / "review-results" / f"gemini-pro-three-paths-{ts}.md"
    header = f"""# Gemini 3 Pro 三条统一拓扑路径探索

> **Provider 替代声明**：编排者原指定 OpenAI gpt-5.5-pro，但 OpenAI 账户 API 配额耗尽
> （429 insufficient_quota，gpt-5.5/gpt-5.5-pro/gpt-4.1 全部命中）。改用 **{model_used}**
> （Google Gemini，**真异质源**——非 OpenAI 同族）。这恰好补上 §18.8 审计当初 Gemini
> 403 降级留下的异质空位。
> 异质探索（L0）：对 Discrete Morse / Sheaf / Reeb+Grammar 三条路径能否统一缠论形态学与 PH。
> 背景：§18 标准装饰 merge tree 路径已被 §18.8 定理级否证。
> 生成：`analysis/unified_topology_three_paths_gemini.py`，{ts}

---

"""
    out.write_text(header + content, encoding="utf-8")
    print(f"Output: {out}", file=sys.stderr)
    print("===CONTENT-BEGIN===")
    print(content)
    print("===CONTENT-END===")


if __name__ == "__main__":
    main()
