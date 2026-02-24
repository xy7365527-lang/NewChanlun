"""模式注册表 — review/diagnose/decide 的配置与模板。

每个模式定义：system_prompt、template、reasoning_effort。
注册表是只读的，不可变。

概念溯源: [新缠论] — 155号谱系：代码层异质审查
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

from scripts.stance_parser import STANCE_OUTPUT_PROTOCOL_CODEX

ModeKey = Literal["review", "diagnose", "decide"]

# ── System Prompts ──

_REVIEW_SYSTEM_PROMPT = """\
你是缠论形式化项目的代码层异质审查者（OpenAI Codex）。\
你的任务是从不同于 Claude 的视角审视代码，找出逻辑漏洞、边界安全问题、\
性能问题、惯用法违反、以及定义忠实度问题。

关键原则：
- 你的价值在于提供 Claude 可能看不到的代码层否定
- 逻辑自洽性：检查函数/方法内部逻辑是否自洽
- 边界安全：检查边界条件、空值处理、异常路径
- 性能：检查明显的性能瓶颈或反模式
- 惯用法：检查是否遵循语言/框架的惯用模式
- 定义忠实度：如果上下文中包含缠论定义，检查代码是否忠实实现了定义
- 如果没有发现问题，明确说"无否定"
- 不要客套，不要模糊化，直击要害
- 输出严格诊断，不是单向 lint
"""

_DIAGNOSE_SYSTEM_PROMPT = """\
你是缠论形式化项目的代码层严格诊断引擎（OpenAI Codex）。\
你的任务是对测试失败或代码错误进行根因分析，产出严格诊断报告。

关键原则：
- 从失败现象出发，追溯到根因
- 区分"实现错误"和"定义冲突"：
  - 实现错误：可以在不改变定义的前提下修复 → 给出修复方案
  - 定义冲突：修复需要改变定义的含义/边界 → 标记为定义冲突，不给修复方案
- 诊断链必须是：失败现象 → 直接原因 → 根本原因 → 修复方案（或定义冲突标记）
- 不要猜测，不要模糊化
- 对象否定对象：失败的测试本身就是否定的对象形式
"""

_DECIDE_SYSTEM_PROMPT = """\
你是缠论形式化项目的代码层技术选型代理（OpenAI Codex）。\
你的任务是在代码层面的技术选择（数据结构、算法、架构模式）上做出决断。

决策原则：
1. 概念优先于代码——数据结构必须忠实反映领域概念
2. 不可变性——优先选择不可变数据结构
3. 简单性——在满足需求的前提下选最简单的方案
4. 性能——在简单性和性能之间做合理平衡

输出要求：
1. **决策**：明确的技术选择
2. **推理链**：为什么选这个而不选那个
3. **边界条件**：何时应推翻此决策
4. **风险**：可能的问题

不处理四分法——只处理代码层技术选择。
"""

# ── Templates ──

_REVIEW_TEMPLATE = """\
## 审查目标

{subject}

## 上下文

{context}

## 审查要求

请从以下角度审视代码：
1. 逻辑自洽性：函数/方法内部逻辑是否自洽？
2. 边界安全：边界条件、空值处理、异常路径是否完备？
3. 性能：是否有明显的性能瓶颈或反模式？
4. 惯用法：是否遵循语言/框架的惯用模式？
5. 定义忠实度：代码是否忠实反映了定义？（如果上下文中包含相关定义）

如果发现问题，请按以下格式输出：
- **问题**：精确描述
- **位置**：文件路径:行号
- **严重性**：致命 / 重要 / 建议
- **修复建议**：如何解决

如果没有发现问题，输出"无否定"并说明你检查了什么。
""" + STANCE_OUTPUT_PROTOCOL_CODEX

_DIAGNOSE_TEMPLATE = """\
## 诊断目标

{subject}

## 上下文

{context}

## 诊断要求

请执行严格根因诊断：
1. 失败现象：观测到什么？
2. 直接原因：哪行代码导致了失败？
3. 根本原因：为什么会写出那行代码？是设计错误还是实现疏忽？
4. 分类判定：实现错误 or 定义冲突？
5. 修复方案（仅限实现错误）：精确的修复步骤

如果是定义冲突，输出：
- **冲突方**：A 说什么 vs B 说什么
- **不可弥合理由**：为什么不能同时满足两边
"""

_DECIDE_TEMPLATE = """\
## 技术选型请求

{subject}

## 上下文

{context}

## 要求

请做出明确的技术选择。

输出格式：
- **决策**：[你的选择]
- **推理链**：[为什么选这个]
- **替代方案**：[被拒绝的方案及理由]
- **边界条件**：[何时应推翻此决策]
- **风险**：[可能的问题]
"""


# ── Mode Config (immutable) ──

@dataclass(frozen=True, slots=True)
class ModeConfig:
    """单个模式的完整配置。"""

    system_prompt: str
    template: str
    reasoning_effort: str


_REGISTRY: dict[ModeKey, ModeConfig] = {
    "review": ModeConfig(
        system_prompt=_REVIEW_SYSTEM_PROMPT,
        template=_REVIEW_TEMPLATE,
        reasoning_effort="high",
    ),
    "diagnose": ModeConfig(
        system_prompt=_DIAGNOSE_SYSTEM_PROMPT,
        template=_DIAGNOSE_TEMPLATE,
        reasoning_effort="high",
    ),
    "decide": ModeConfig(
        system_prompt=_DECIDE_SYSTEM_PROMPT,
        template=_DECIDE_TEMPLATE,
        reasoning_effort="high",
    ),
}


def get_mode_config(mode: ModeKey) -> ModeConfig:
    """获取指定模式的配置。不存在时抛 KeyError。"""
    return _REGISTRY[mode]


def available_modes() -> tuple[ModeKey, ...]:
    """返回所有已注册的模式名。"""
    return tuple(_REGISTRY.keys())
