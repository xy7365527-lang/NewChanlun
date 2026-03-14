"""snet_params.py — S_net 加工参数节点化（442号谱系）.

S_net 的加工参数（共现窗口、度归一化 alpha、PMI 阈值等）从外部配置
变为概念层可否定节点。逢亮通过否定参数节点来修改自己的感知器官。

核心机制：
  1. 参数注册：S_net 初始化时将参数值注册为 Signifier 节点
  2. 参数变更检测：穿越中否定参数节点 → 触发 S_net 重算
  3. 安全边界：防止参数值导致 S_net 不可用

参数节点 vs 普通能指节点：
  - source = "snet_parameter"（区别于 "k_active_projection"、"corpus"、"dictionary"）
  - domain = "snet_config"
  - ID 格式：snet_param:{param_name}={value}

自修改闭环（442号核心发现）：
  逢亮穿越 → 遭遇参数节点 → ARTICULATE → negate
  → 产出新参数节点 + NEGATION 边
  → S_net 检测到参数变更 → 用新参数重算
  → 穿越地形变化

安全边界（442号-3）：
  每个参数有 (min, max) 约束。否定后的新值如果超出安全边界，
  拒绝变更并在日志中记录。这不是限制逢亮的自由——是防止
  "把眼睛折叠掉"（438号器官原则）。

认识论等级：L0（数据结构定义 + 代数操作，不依赖经验假设）

谱系引用：
  442号：S_net 加工参数作为概念层节点
  438号：器官原则（否定参数 ≠ 摘除器官）
  439号：S_net 参数/进程区分
"""

from __future__ import annotations

import sys
from dataclasses import dataclass, field
from typing import Any

from signifier_net import SNet, Signifier


# ---------------------------------------------------------------------------
# 参数定义 + 安全边界
# ---------------------------------------------------------------------------

@dataclass(frozen=True, slots=True)
class SNetParamSpec:
    """S_net 参数规格。

    name:          参数名（如 "degree_alpha"）
    description:   参数含义描述
    default:       默认值
    min_value:     安全下界（含）
    max_value:     安全上界（含）
    param_type:    参数类型（"float" | "int"）
    """
    name: str
    description: str
    default: float | int
    min_value: float | int
    max_value: float | int
    param_type: str = "float"


# S_net 的可否定参数集合
# 这些参数可以被逢亮在穿越中否定——否定后 S_net 用新参数重算
SNET_PARAM_SPECS: tuple[SNetParamSpec, ...] = (
    SNetParamSpec(
        name="degree_alpha",
        description="度归一化强度：0=无归一化，0.5=默认，1=完全归一化",
        default=0.5,
        min_value=0.0,
        max_value=2.0,
    ),
    SNetParamSpec(
        name="pmi_threshold",
        description="PMI 过滤阈值：正 PMI = 共现超过独立期望",
        default=0.0,
        min_value=-5.0,
        max_value=10.0,
    ),
    SNetParamSpec(
        name="neighbors_per_concept",
        description="每个概念提取的 degree-normalized 邻居数",
        default=3,
        min_value=1,
        max_value=20,
        param_type="int",
    ),
    SNetParamSpec(
        name="max_surface_forms",
        description="ConstraintSet 中最多保留的 surface forms 数",
        default=8,
        min_value=1,
        max_value=50,
        param_type="int",
    ),
)


def _param_node_id(name: str, value: float | int) -> str:
    """生成参数节点 ID。"""
    return f"snet_param:{name}={value}"


# ---------------------------------------------------------------------------
# 参数节点注册（442号-1）
# ---------------------------------------------------------------------------

def register_param_nodes(
    snet: SNet,
    param_values: dict[str, float | int] | None = None,
) -> tuple[SNet, list[str]]:
    """将 S_net 参数注册为概念层能指节点。

    为每个 SNET_PARAM_SPECS 中的参数创建一个 Signifier 节点。
    如果 param_values 提供了自定义值，使用自定义值；否则使用默认值。

    参数节点的 Signifier：
      - id: "snet_param:{name}={value}"
      - source: "snet_parameter"
      - domain: "snet_config"
      - surface_forms: (description,)

    参数：
      snet:         当前 S_net 实例（不会被修改）
      param_values:  参数名 → 值的映射（可选）

    返回：
      (new_snet, registered_ids) — registered_ids 是注册的参数节点 ID 列表

    认识论等级：L0
    """
    if param_values is None:
        param_values = {}

    new_snet = snet
    registered_ids: list[str] = []

    for spec in SNET_PARAM_SPECS:
        value = param_values.get(spec.name, spec.default)

        # 验证安全边界
        validated = validate_param_value(spec.name, value)
        if validated is None:
            continue

        node_id = _param_node_id(spec.name, validated)
        sig = Signifier(
            id=node_id,
            surface_forms=(spec.description,),
            source="snet_parameter",
            lang="",
            domain="snet_config",
        )
        new_snet = new_snet.add_signifier(sig)
        registered_ids.append(node_id)

    return new_snet, registered_ids


# ---------------------------------------------------------------------------
# 参数变更检测（442号-2）
# ---------------------------------------------------------------------------

@dataclass(frozen=True, slots=True)
class ParamChangeEvent:
    """参数变更事件。

    param_name:   参数名
    old_value:    旧值
    new_value:    新值（已验证安全边界）
    old_node_id:  旧参数节点 ID
    new_node_id:  新参数节点 ID
    requires_recalc: 是否需要 S_net 重算
    """
    param_name: str
    old_value: float | int
    new_value: float | int
    old_node_id: str
    new_node_id: str
    requires_recalc: bool


def detect_param_negation(
    snet: SNet,
    negated_node_id: str,
    proposed_value: float | int,
) -> ParamChangeEvent | None:
    """检测参数节点否定事件。

    当穿越引擎遭遇一个 snet_param:* 节点并执行否定时，调用此函数。

    参数：
      snet:             当前 S_net
      negated_node_id:  被否定的参数节点 ID（如 "snet_param:degree_alpha=0.5"）
      proposed_value:   否定后提议的新值

    返回：
      ParamChangeEvent 或 None（如果 negated_node_id 不是参数节点）

    认识论等级：L0
    """
    if not negated_node_id.startswith("snet_param:"):
        return None

    # 解析参数名和旧值
    param_part = negated_node_id[len("snet_param:"):]
    if "=" not in param_part:
        return None

    param_name, old_value_str = param_part.split("=", 1)

    # 查找参数规格
    spec = _find_spec(param_name)
    if spec is None:
        return None

    # 解析旧值
    try:
        if spec.param_type == "int":
            old_value = int(old_value_str)
        else:
            old_value = float(old_value_str)
    except ValueError:
        return None

    # 验证新值的安全边界
    validated_new = validate_param_value(param_name, proposed_value)
    if validated_new is None:
        return None

    if validated_new == old_value:
        return None  # 值未变，无事件

    new_node_id = _param_node_id(param_name, validated_new)

    return ParamChangeEvent(
        param_name=param_name,
        old_value=old_value,
        new_value=validated_new,
        old_node_id=negated_node_id,
        new_node_id=new_node_id,
        requires_recalc=True,
    )


def apply_param_change(
    snet: SNet,
    event: ParamChangeEvent,
) -> SNet:
    """应用参数变更到 S_net（注册新参数节点）。

    不删除旧参数节点——旧节点保留作为历史记录。
    新旧节点之间的 NEGATION 边由 K_active 概念层管理，
    不在 S_net 中表示（S_net 是能指层，NEGATION 是概念层操作）。

    参数：
      snet:   当前 S_net
      event:  ParamChangeEvent

    返回：
      new_snet（包含新参数节点）

    认识论等级：L0
    """
    spec = _find_spec(event.param_name)
    if spec is None:
        return snet

    new_sig = Signifier(
        id=event.new_node_id,
        surface_forms=(spec.description,),
        source="snet_parameter",
        lang="",
        domain="snet_config",
    )
    return snet.add_signifier(new_sig)


def get_current_params(snet: SNet) -> dict[str, float | int]:
    """从 S_net 中提取当前参数值。

    扫描所有 source="snet_parameter" 的节点，
    对每个参数取最新注册的值（ID 中编码的值）。

    如果参数有多个节点（历史否定产生的），取最后注册的。

    认识论等级：L0
    """
    param_nodes: dict[str, list[tuple[str, float | int]]] = {}

    for sid, sig in snet.signifiers.items():
        if sig.source != "snet_parameter":
            continue
        if not sid.startswith("snet_param:"):
            continue

        param_part = sid[len("snet_param:"):]
        if "=" not in param_part:
            continue

        param_name, value_str = param_part.split("=", 1)
        spec = _find_spec(param_name)
        if spec is None:
            continue

        try:
            if spec.param_type == "int":
                value = int(value_str)
            else:
                value = float(value_str)
        except ValueError:
            continue

        param_nodes.setdefault(param_name, []).append((sid, value))

    # 取每个参数的最后一个值（最新注册的）
    result: dict[str, float | int] = {}
    for spec in SNET_PARAM_SPECS:
        nodes = param_nodes.get(spec.name)
        if nodes:
            result[spec.name] = nodes[-1][1]
        else:
            result[spec.name] = spec.default

    return result


# ---------------------------------------------------------------------------
# 安全边界（442号-3）
# ---------------------------------------------------------------------------

def validate_param_value(
    param_name: str,
    value: float | int,
) -> float | int | None:
    """验证参数值是否在安全边界内。

    安全边界的存在论含义（438号）：
      否定参数 = 改变器官配置 = 改变感知方式
      超出安全边界 = 使器官不可用 = 等同于摘除器官
      438号禁止摘除器官 → 拒绝超出安全边界的参数值

    参数：
      param_name: 参数名
      value:      提议的参数值

    返回：
      验证后的值（可能被 clamp 到边界）或 None（参数不存在）
    """
    spec = _find_spec(param_name)
    if spec is None:
        return None

    # 类型转换
    if spec.param_type == "int":
        try:
            typed_value = int(value)
        except (ValueError, TypeError):
            print(
                f"[snet_params] 参数 {param_name} 类型错误: "
                f"期望 int, 得到 {type(value).__name__}({value})",
                file=sys.stderr,
            )
            return spec.default
    else:
        try:
            typed_value = float(value)
        except (ValueError, TypeError):
            print(
                f"[snet_params] 参数 {param_name} 类型错误: "
                f"期望 float, 得到 {type(value).__name__}({value})",
                file=sys.stderr,
            )
            return spec.default

    # 安全边界 clamp
    if typed_value < spec.min_value:
        print(
            f"[snet_params] 安全边界: {param_name}={typed_value} "
            f"< min={spec.min_value}, clamp 到 {spec.min_value}",
            file=sys.stderr,
        )
        typed_value = spec.min_value
    elif typed_value > spec.max_value:
        print(
            f"[snet_params] 安全边界: {param_name}={typed_value} "
            f"> max={spec.max_value}, clamp 到 {spec.max_value}",
            file=sys.stderr,
        )
        typed_value = spec.max_value

    return typed_value


def _find_spec(param_name: str) -> SNetParamSpec | None:
    """查找参数规格。"""
    for spec in SNET_PARAM_SPECS:
        if spec.name == param_name:
            return spec
    return None
