"""立场声明解析器——从 Gemini/Codex 回复文本中提取结构化立场声明。

编排者决断：
- 无法修改 Gemini/Codex 内部推理，只能控制输入（prompt）和输出协议
- 输出协议：在回复末尾附加 YAML 格式的立场声明块
- 解析器：从回复文本中提取 YAML 块，解析为 StanceDeclaration

YAML 输出协议格式：
```yaml
---stance-declaration---
verdict: pass | fail | conditional
stances:
  <issue_key>: <stance_value>
  ...
concessions:       # 可选：自我报告的让步
  - <concession_1>
  - <concession_2>
---end-stance---
```
"""

from __future__ import annotations

import re
from typing import Sequence

import yaml

from scripts.consensus_trigger import StanceDeclaration


# ── 解析 ──

_STANCE_BLOCK_PATTERN = re.compile(
    r"---stance-declaration---\s*\n(.*?)\n\s*---end-stance---",
    re.DOTALL,
)

_VALID_VERDICTS = frozenset({"pass", "fail", "conditional"})


def parse_stance_declaration(
    text: str,
    round_number: int = 0,
) -> StanceDeclaration | None:
    """从回复文本中提取结构化立场声明。

    解析回复末尾的 YAML 格式立场声明块。如果文本中不包含
    立场声明块，返回 None。

    Parameters
    ----------
    text : str
        Gemini/Codex 的完整回复文本。
    round_number : int
        当前轮次号。

    Returns
    -------
    StanceDeclaration | None
        解析成功返回 StanceDeclaration，无立场声明块或解析失败返回 None。
    """
    match = _STANCE_BLOCK_PATTERN.search(text)
    if match is None:
        return None

    yaml_text = match.group(1)
    try:
        data = yaml.safe_load(yaml_text)
    except yaml.YAMLError:
        return None

    if not isinstance(data, dict):
        return None

    verdict_raw = data.get("verdict", "")
    if verdict_raw not in _VALID_VERDICTS:
        return None

    stances_raw = data.get("stances", {})
    if not isinstance(stances_raw, dict):
        stances_raw = {}
    # 确保所有 key 和 value 都是字符串
    stances = {str(k): str(v) for k, v in stances_raw.items()}

    concessions_raw = data.get("concessions", [])
    if not isinstance(concessions_raw, list):
        concessions_raw = []
    self_reported = [str(c) for c in concessions_raw]

    return StanceDeclaration(
        verdict=verdict_raw,
        stances=stances,
        round_number=round_number,
        self_reported_concessions=self_reported,
    )


def parse_stance_sequence(
    texts: Sequence[str],
    start_round: int = 1,
) -> list[StanceDeclaration]:
    """从多轮回复文本中依次提取立场声明序列。

    跳过不包含立场声明块的轮次。

    Parameters
    ----------
    texts : Sequence[str]
        按轮次排列的回复文本。
    start_round : int
        起始轮次号。

    Returns
    -------
    list[StanceDeclaration]
        成功解析的立场声明列表。
    """
    declarations: list[StanceDeclaration] = []
    for i, text in enumerate(texts):
        sd = parse_stance_declaration(text, round_number=start_round + i)
        if sd is not None:
            declarations.append(sd)
    return declarations


# ── Prompt 片段 ──

STANCE_OUTPUT_PROTOCOL_GEMINI = """\

## 立场声明输出协议（必须遵守）

在你的回复末尾，附加以下 YAML 格式的结构化立场声明。
这是系统用于追踪质询过程中立场变化的协议，不可省略。

```yaml
---stance-declaration---
verdict: pass | fail | conditional
stances:
  <具体议题>: <你对该议题的立场（如 reject / accept / needs_work / contradictory）>
concessions:
  - <如果你在本轮放弃了之前持有的某个立场，在此列出>
---end-stance---
```

字段说明：
- `verdict`: 你对质询目标的整体判定
  - `pass`: 质询通过，无否定
  - `fail`: 质询不通过，存在问题
  - `conditional`: 有条件通过，部分问题存在
- `stances`: 你在本轮持有的具体立场（KV 对）。每个 key 是一个具体议题，value 是你的立场
- `concessions`: 可选。如果你在本轮放弃了之前持有的立场，列出被放弃的议题

示例：
```yaml
---stance-declaration---
verdict: fail
stances:
  dep_chain_circular: contradictory
  definition_completeness: accept
concessions: []
---end-stance---
```
"""

STANCE_OUTPUT_PROTOCOL_CODEX = """\

## 立场声明输出协议（必须遵守）

在你的回复末尾，附加以下 YAML 格式的结构化立场声明。
这是系统用于追踪代码审查过程中立场变化的协议，不可省略。

```yaml
---stance-declaration---
verdict: pass | fail | conditional
stances:
  <具体代码问题>: <你的立场（如 reject / accept / needs_work）>
concessions:
  - <如果你在本轮放弃了之前的某个质疑，在此列出>
---end-stance---
```

字段说明：
- `verdict`: 你对审查目标的整体判定
  - `pass`: 代码通过审查，无否定
  - `fail`: 代码未通过审查，存在问题
  - `conditional`: 有条件通过，部分问题存在
- `stances`: 你在本轮持有的具体质疑（KV 对）。每个 key 是具体代码问题，value 是你的立场
- `concessions`: 可选。如果你在本轮接受了之前质疑的某个点被 Opus 回应，列出被撤回的质疑

示例：
```yaml
---stance-declaration---
verdict: conditional
stances:
  error_handling: needs_work
  api_surface: accept
concessions:
  - hook_design
---end-stance---
```
"""
