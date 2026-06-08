#!/usr/bin/env python3
"""三条候选拓扑数学路径的异质探索（gpt-5.4-pro）。

背景：§18 标准装饰 merge tree 路径已被 §18.8 异质审计（GPT-5.1）定理级否证。
本脚本让最新 pro 级模型（gpt-5.4-pro）探索三条可能统一缠论形态学与 PH 的新拓扑数学路径：
  1. Discrete Morse Theory
  2. Sheaf Theory on Price Curves
  3. Reeb Graph with Grammar

对每条路径要求：可行性 + 障碍 + 现成文献 + 能否克服 §18.8 三个失败原因，最后给出排名。

输出：.chanlun/review-results/gpt-pro-three-paths-<ts>.md
认识论等级：L0（纯定义 + 探索，零新数据）。
"""

import os
import sys
import time
from datetime import datetime
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# 编排者指令（2026-05-29，多次更正后定稿）：主模型 gpt-5.5（最新最强，走 chat/completions）。
# 有序降级链——(model, api) 元组，"chat"=chat/completions、"resp"=Responses API。
# 404/不可用自动降级到下一个，实际使用的模型名记录在输出中。
MODEL_CHAIN: list[tuple[str, str]] = [
    ("gpt-5.5", "chat"),       # 编排者指定主模型
    ("gpt-5.5-pro", "resp"),   # pro 变体（Responses API）
    ("gpt-5.4-pro", "resp"),
    ("o3", "chat"),            # 编排者指定 fallback
    ("o4-mini", "chat"),
    ("gpt-4.1", "chat"),
]


def build_prompt() -> str:
    return r"""你是同时精通代数拓扑（TDA / persistent homology / discrete Morse theory / sheaf
theory / Reeb graphs）与缠论（缠中说禅技术分析理论）的研究者。本任务是开放性数学探索，
不要客套，直接给硬判断。用中文回答，数学符号用 LaTeX。

# 背景：一条已被否证的路径

我们试图为「缠论形态学」与「持续同调（persistent homology, PH）」找一个统一的数学框架。
缠论从价格曲线 p:[0,T]→ℝ 提取离散结构：笔 → 段（线段）→ 中枢 → 走势类型。
PH 从同一曲线提取：sublevel filtration → merge tree（H0）→ barcode / persistence diagram。

**第一次尝试**：假设存在一个「装饰 merge tree」S=(T_merge, τ)（带 birth/death 的 H0 merge
tree + 时间区间装饰 τ_I + 时间偏序 τ_≺），缠论与 PH 都是 S 的遗忘函子投影：
- F_PH: S ↦ barcode（遗忘时间装饰）
- F_Chan: S ↦ 笔/段/中枢序列（显著性阈值过滤 Φ_θ + 保留时间序）

**这条路径被异质审计（另一个 pro 模型 + Codex）定理级否证。三个收敛的失败原因：**

【失败原因 1：包含关系处理改变底层时间序列】
缠论在建模前先做「K线包含关系处理」——相邻 K 线若一根的高低点完全含于另一根，则合并为
一根（取方向相关的高低点）。这个合并**改变了底层时间序列本身**，因此 merge tree 随之改变。
这不是「在固定 S 上做遗忘投影」——缠论和 PH 作用在**不同的底层序列**上（缠论作用在包含
处理后的序列，PH 作用在原始序列）。F_Chan 因此不是 S 的干净遗忘函子。

【失败原因 2：特征序列法（段的判定）是非局部操作】
线段的划分用「特征序列 + 缺口 + 分型」：以向上笔开始的段，取其所有向下笔构成特征序列，
对特征序列元素做包含处理后找分型，分型是否有缺口决定段在何处结束（可能需要构造第二特征
序列）。这是一个**序列级的非局部二元关系**（哪些笔属于同一特征序列、相邻元素是否有缺口、
是否需要第二特征序列），**不在 merge tree 的局部 birth/death 信息里**。barcode 没有典范全序，
而特征序列包含是序列级全序关系——单位也不可换（persistence 是价格差，最小跨度是时间）。

【失败原因 3：「走势必完美」要求全局截面存在，PH 没有这个约束】
缠论「走势必完美」原则（缠师原文，一级权威）：任何走势可**唯一地、完全地**分解为笔/段/
中枢——价格曲线每一段都被归类，无遗漏、无重叠、无歧义（exhaustive partition + 唯一性）。
PH 的 barcode **不强制完全分解**：persistence < 阈值的区域被当噪声丢弃，barcode 对这些
「无结构区」保持沉默——它**列举**特征，不主张特征**覆盖**全曲线，也不要求唯一解析。

否证的总结论：缠论的「语法层」（包含处理、特征序列、最小跨度、完全分解）部分在标准
装饰 merge tree S **之外**——标准 S 统一框架的有效域被否证为空。存活的开放问题是：
是否存在一个 **enriched S**（携带临界点全序、时间长度、缺口标记、甚至语法）使统一成立。

# 你的任务：评估三条新的候选拓扑数学路径

下面三条路径试图绕过上述失败。对**每一条**，请严格回答四个子问题，不要泛泛而谈：

(a) **形式化草图**：把缠论对象（笔/段/中枢/走势 + 包含处理 + 特征序列 + 走势必完美）
    如何精确地映射到该数学框架的对象上？给出尽量具体的定义，指出哪些是直接的、哪些是
    勉强的、哪些做不到。
(b) **能否克服三个失败原因**：逐条对照失败原因 1/2/3，说明该框架是**真正解决**了它、还是
    只是**换了一种语言重新表述**同一个困难（即障碍是否被搬运而非消除）。这是最关键的一问——
    我不要语言上的重新包装，我要知道困难是否在数学结构上被真正吸收。
(c) **现成文献**：是否有现成的数学文献支撑（TDA + 离散 Morse、sheaf theory + 时间序列 /
    signal processing、Reeb graph + formal grammar / 带文法的拓扑、Mapper + 语法约束等）？
    给出具体的研究方向名称、关键作者或结果（如 Forman 离散 Morse、Curry/Ghrist/Robinson 的
    sheaf-theoretic signal processing、de Silva/Munch/Patel 的 Reeb graph 范畴化、Mapper 等）。
    如果某方向在文献里基本是空白，明确说「这是空白，需要原创」。
(d) **致命障碍**：这条路径最可能在哪里失败？给一个具体的反例或结构性论证。

## 路径 1：Discrete Morse Theory（离散 Morse 理论）
- 在价格序列的 CW 复形（1维：顶点=时间点，边=相邻 bar）上定义离散 Morse 函数 f。
- 笔 = Morse 配对（相邻 critical cells 的配对，即 V-path / gradient path）。
- 包含关系处理 = 某种 Morse cancellation（消除不必要的 critical pairs）。
- 段 = 更高维或更粗粒度的 Morse 配对（cancellation 后的残余 critical cells 再配对）。
- 中枢 = Morse 复形上的某种 homological feature（持续的 1-cycle？或 critical cell 的聚集）。
- **核心问题**：Morse cancellation 的偏序（cancellation tree）能否自然导出「走势必完美」的
  唯一完全分解？离散 Morse 的 cancellation 是否能编码缠论的「先包含处理、再特征序列」的
  两段式语法？还是 cancellation 只能消除而不能像缠论那样「归并 + 重新分类」？

## 路径 2：Sheaf Theory on Price Curves（价格曲线上的层论）
- 在价格序列的时间轴 [0,T] 上定义一个层 F。
- F(U) = 区间 U 上的走势类型分类（盘整 / 上涨趋势 / 下跌趋势 / 未定）。
- 层公理（局部截面唯一粘合为全局截面）= 「走势必完美」（局部走势唯一粘合为全局走势）。
- PH 给出 cosheaf 的同调（Borel–Moore 同调 / cellular cosheaf homology）。
- 缠论给出 sheaf 的全局截面（全局走势分类）。
- 统一 = sheaf–cosheaf 对偶（Verdier 对偶 / 元胞层的 sheaf-cosheaf 配对）。
- **核心问题**：缠论的「走势必完美」真的满足层的粘合公理吗？层公理要求局部截面在重叠上
  一致就能**唯一**粘合——但缠论的级别递归（笔粘成段、段粘成走势）在重叠区的「一致性」是
  什么？包含处理（失败原因 1）改变底层拓扑空间，层是定义在固定空间上的——层论能否容纳
  「底空间本身被语法改写」？特征序列（失败原因 2）是非局部的，而层是局部到全局的——
  非局部约束能否塞进层的限制映射（restriction map）里？

## 路径 3：Reeb Graph with Grammar（带文法的 Reeb 图）
- Reeb graph = merge tree 的推广（允许环，对应 H1）。
- 在 Reeb graph 上装备一个形式文法 G。
- G 的产生式规则 = 缠论语法约束（包含关系、特征序列、最小跨度、方向交替）。
- 语言 L(G) = 所有合法的走势分解。
- 「走势必完美」= L(G) 对任何价格序列**恰有一个**解析（无歧义文法 unambiguous grammar，
  甚至唯一解析 → 类似确定性可解析）。
- PH barcode = Reeb graph 的拓扑签名（遗忘文法后的残余）。
- **核心问题**：把语法**显式外挂**在拓扑对象上（Reeb graph + G）是不是「承认失败原因 2/3
  无法被纯拓扑吸收，于是把它们打包进一个并列的文法层」？如果是，那这是「统一」还是
  「拓扑 × 文法的乘积（两个并列模块）」？无歧义文法对任意价格序列恰有一个解析——这个
  性质对缠论文法是否真的成立（缠论本身有「古怪线段」「待定方向」等歧义情形）？包含处理
  （失败原因 1）改变 Reeb graph 的底层，文法 G 是作用在改写前还是改写后的图上？

# 最后：横向排名与裁决

1. 给三条路径排名（最可能成功 → 最不可能），每条一句话理由。
2. 三条路径里，有没有哪一条**真正**把三个失败原因之一吸收进了数学结构（而不是换语言）？
   如果有，是哪条、吸收了哪个？如果三条都只是搬运困难，明确说出来——否定性结论同样有价值。
3. 你认为最有希望的「enriched 结构」长什么样？是否需要把缠论语法当作**第一类数学对象**
   （而非拓扑的附属），即统一框架可能本质上是「拓扑 + 语法」的**纤维化 / 分层**而非单一拓扑？
4. 一个诚实的元判断：缠论与 PH 的统一，究竟是「还没找到对的数学」，还是「缠论语法携带了
   PH 原理上无法捕获的信息（如交易者的非局部决策、时间方向性、人为规约的完全分解），
   因而不存在单一拓扑统一，只能是拓扑 + 外部语法层的并置」？请给出你的立场和理由。
"""


def call_pro(prompt: str) -> tuple[str, str]:
    from openai import OpenAI

    if not os.environ.get("OPENAI_API_KEY"):
        print("ERROR: OPENAI_API_KEY not set", file=sys.stderr)
        sys.exit(1)

    client = OpenAI()
    last_error: Exception | None = None
    reasoning_models = {"gpt-5.5", "o3", "o4-mini"}

    for model, api in MODEL_CHAIN:
        try:
            print(f"Calling {model} ({api}, reasoning=high)...", file=sys.stderr)
            t0 = time.time()
            if api == "resp":
                resp = client.responses.create(
                    model=model, input=prompt, reasoning={"effort": "high"}
                )
                text = resp.output_text
                usage = getattr(resp, "usage", None)
            else:
                kwargs: dict = {"model": model, "messages": [{"role": "user", "content": prompt}]}
                if model in reasoning_models:
                    kwargs["reasoning_effort"] = "high"
                resp = client.chat.completions.create(**kwargs)
                text = resp.choices[0].message.content
                usage = resp.usage
            dt = time.time() - t0
            print(f"Success {model} in {dt:.0f}s. usage={usage}", file=sys.stderr)
            return model, text
        except Exception as e:  # noqa: BLE001 — surface any failure, then 降级
            last_error = e
            print(f"Model {model} failed: {e}", file=sys.stderr)

    return "none", f"ERROR: all models failed. Last error: {last_error}"


def main() -> None:
    prompt = build_prompt()
    print(f"Prompt length: {len(prompt)} chars", file=sys.stderr)

    model_used, content = call_pro(prompt)

    ts = datetime.now().strftime("%Y%m%d-%H%M%S")
    out = REPO_ROOT / ".chanlun" / "review-results" / f"gpt-pro-three-paths-{ts}.md"
    header = f"""# GPT Pro 三条统一拓扑路径探索

> 异质探索（L0）：{model_used} 对 Discrete Morse / Sheaf / Reeb+Grammar 三条路径
> 能否统一缠论形态学与 PH 的独立判断。
> 背景：§18 标准装饰 merge tree 路径已被 §18.8 定理级否证。
> 生成：`analysis/unified_topology_three_paths.py`，{ts}

---

"""
    out.write_text(header + content, encoding="utf-8")
    print(f"Output: {out}", file=sys.stderr)
    # 同时打到 stdout 供主流程消费
    print("===CONTENT-BEGIN===")
    print(content)
    print("===CONTENT-END===")


if __name__ == "__main__":
    main()
