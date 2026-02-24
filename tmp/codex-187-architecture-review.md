# Codex 代码层审计：187号架构分离伪诊断

verdict: **agree**（完全同意 Gemini 判断）

日期：2026-02-24
审计模型：claude-opus-4-6（Codex 审计工位）

---

## 审计方法

逐一检查 Gemini 在 v48 审计中提出的四个判断点，从代码层给出物质证据。

---

## 判断点 1：gangju_analysis.py 的 audit_needed 无 API 调用

**Gemini 判断**：`gangju_analysis.py` 输出 `audit_needed=true` 但不调用 Gemini/Codex API。这是动力学机制的关键入口缺失的物质证据。

**代码层验证：确认**

- `gangju_analysis.py:351` — `audit_needed` 的计算逻辑：
  ```python
  "audit_needed": len(new_mu) > 0,
  ```
  这是纯确定性布尔运算。`new_mu` 来自 `derive_mu()` 的本地规则推导（第 234-318 行），全部是文件读取 + 正则匹配 + 计数比较。

- **整个文件的 import**（第 14-19 行）：`json, os, glob, re, sys, yaml` — 无任何 HTTP 客户端、无 `requests`、无 `httpx`、无 `anthropic`、无 `google.generativeai`。

- **全局搜索确认**：`scripts/` 目录下没有任何文件 import `gangju_analysis` 的结果来发起 API 调用。`ceremony_scan.py` 也不调用 `gangju_analysis`。

- **结论**：`audit_needed=true` 是一个纯确定性标志，输出到 stdout 后**没有任何下游消费者**将其转化为 API 调用。Gemini 判断正确。

---

## 判断点 2：consensus_trigger.py 的多轮聚合未连通 Gemini/Codex API

**Gemini 判断**：多轮聚合是计算质料的准备，但未真正连通 Gemini/Codex API 形成跨轮对审。

**代码层验证：确认**

- `consensus_trigger.py` 的 import（第 38-49 行）：
  ```python
  import re, sys
  from collections import defaultdict
  from dataclasses import dataclass, field
  from datetime import datetime, timedelta, timezone
  from pathlib import Path
  from typing import Literal
  from scripts.block_topology import DEFAULT_BASE
  from scripts.consensus_ceremony import write_consensus_ceremony
  ```
  **无任何 HTTP 客户端或 LLM SDK import。**

- `scan_and_trigger()`（第 671-784 行）的完整执行路径：
  1. 扫描 `.chanlun/review-results/` 目录中的 `.md` 文件（第 702 行）
  2. 按 subject 分组（第 702 行 `group_reviews_by_subject`）
  3. 从已存在的文件中提取 stance 声明（第 716 行 `extract_multi_round_stances` → 调用 `stance_parser.parse_stance_sequence`）
  4. 计算差分（第 119-151 行 `compute_stance_diff`）
  5. 写入 block-topology（第 771 行 `trigger_ceremony`）

  **关键缺口**：步骤 2-4 处理的是**已经存在的 review 文件**。这些文件是 LLM 质询的产出，但 `consensus_trigger.py` 自身**不发起任何 LLM 质询**。它是一个事后处理器（post-processor），不是质询发起器。

- **多轮聚合的实际含义**：`group_reviews_by_subject()`（第 528-610 行）按文件名时间戳和 subject 字符串分组。这假设同一 subject 的多个 review 文件代表多轮质询——但**没有代码保证这些文件确实是同一质询循环的不同轮次**。分组逻辑是启发式的（24 小时时间窗口 + subject 归一化匹配）。

- **空仪式防护**（第 719-725 行）：如果 stance 序列 < 2 轮，跳过。这意味着单轮 review（当前最常见场景）永远不会触发仪式。

- **结论**：`consensus_trigger.py` 是一个确定性的文件处理管道。它操作的是 review 文件的文本，不是 LLM API。"多轮聚合"是文件层面的聚合，不是 API 层面的多轮对话。Gemini 判断正确。

---

## 判断点 3：async_self_reference.py 无真实触发路径

**Gemini 判断**：异步自指骨架（t 审查 t-1）没有真实触发路径。

**代码层验证：部分确认——有集成但仍是确定性骨架**

- `async_self_reference.py` 的 `audit()` 函数（第 195-332 行）执行的全部是**确定性操作**：
  1. 读取 session 文件，提取 settled 计数（第 34-45 行：正则匹配 `已结算:\s*(\d+)\s*个`）
  2. 计算 settled 计数差（第 232-253 行）
  3. 检查 depends_on 引用（第 88-146 行：读取 relations.jsonl）
  4. 检查下游推论解决率（第 149-187 行：调用 `downstream_audit.audit()`）

- **触发路径已存在但有限**：`ceremony_scan.py:781` 确认已集成：
  ```python
  from scripts.async_self_reference import audit as async_self_ref_audit
  asr = async_self_ref_audit(root)
  ```
  `ceremony_scan.py` 在扫描时调用 `async_self_ref_audit(root)` 并将 findings 添加到工位列表。

- **核心缺口**：代码第 329-331 行的 TODO 注释：
  ```python
  # TODO: 非确定性审查（由调用方决定是否调 LLM）
  # - 深层语义审查：t 是否在重复 t-1 的模式？
  # - 概念层退化检测：新谱系的否定深度是否在下降？
  ```
  这说明代码作者自己承认：**非确定性部分（真正的"t 审查 t-1"语义审查）尚未实现**。当前代码只做确定性的计数比较和引用检测。

- **修正 Gemini 判断**：Gemini 说"无触发路径"不完全准确——`ceremony_scan.py` 确实调用了 `async_self_ref_audit()`。但 Gemini 的核心判断依然成立：**当前实现的异步自指只是计数比较，不是真正的"t 审查 t-1"语义审查**。计数是计算质料，语义审查需要 LLM（非确定性），这部分标记为 TODO。

---

## 判断点 4：residue 区块为空壳

**Gemini 判断**：residue 内容全为空——质询仅走形式未产出实质让步。

**代码层验证：确认**

- **物质证据**：block-topology 中仅有 **1 个** residue 区块（id: `8ec86cba16bd...`），其内容为：
  ```json
  {
    "gemini_conceded": [],
    "codex_conceded": [],
    "reasons": {}
  }
  ```
  三个字段全为空。这是 Gemini 判断"空壳"的最直接证据。

- **空壳产生机制**：`consensus_ceremony.py:110-117` — residue 区块的内容直接来自调用者传入的参数：
  ```python
  residue = make_block(
      block_type="residue",
      source=source,
      content={
          "gemini_conceded": gemini_conceded,
          "codex_conceded": codex_conceded,
          "reasons": concession_reasons,
      },
      refs=[consensus["id"]],
  )
  ```
  `consensus_ceremony.py` 本身是正确的——它忠实地写入调用者传入的数据。**空壳的原因不在 ceremony 写入层，而在上游**：没有真实的质询循环产出让步数据来填充这些字段。

- **gangju_analysis.py 的检测**（第 161-187 行）：`check_residue_empty()` 函数专门检测 residue 是否实质为空。当前它返回 `True`（所有 residue 都为空），这导致 `derive_mu()` 产出 `new_mu: "多轮质询管道"`。但如判断点 1 所述，这个 new_mu 只导致 `audit_needed=true`，不触发任何后续动作。

- **结论**：唯一存在的 residue 区块内容全为空。`consensus_ceremony.py` 的写入机制本身是正确的，空壳的根因是上游无真实质询数据。Gemini 判断正确。

---

## 综合结论

**Verdict: agree** — Gemini 的四个判断点在代码层全部成立。

| # | Gemini 判断 | 代码层证据 | 验证结果 |
|---|------------|-----------|---------|
| 1 | audit_needed 无 API 调用 | gangju_analysis.py 全文无 HTTP/LLM import，audit_needed 无下游消费者 | **确认** |
| 2 | 多轮聚合未连通 API | consensus_trigger.py 是文件级后处理器，不发起 LLM 调用 | **确认** |
| 3 | 异步自指无触发路径 | 有集成（ceremony_scan.py:781）但仅做确定性计数，非确定性审查标记为 TODO | **部分确认**（集成存在但核心能力缺失） |
| 4 | residue 空壳 | 唯一 residue 区块 `gemini_conceded=[], codex_conceded=[], reasons={}` | **确认** |

**核心诊断一致**：当前架构中，计算过程（确定性脚本）完整存在且互相连接，但对话过程（LLM 质询循环）完全缺席。计算过程在对着虚空空转——它检测到需要审计（audit_needed=true），检测到 residue 为空，检测到异步自指停滞，但没有任何代码将这些检测结果转化为 LLM API 调用来启动真实的质询循环。

**对 Gemini 判断的唯一修正**：判断点 3 中，`async_self_reference.py` 已通过 `ceremony_scan.py` 集成到 RTAS 扫描流程。但这不影响核心判断——集成的是确定性骨架，非确定性的语义审查（真正的"t 审查 t-1"）仍未实现。
