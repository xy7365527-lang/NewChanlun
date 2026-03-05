---
id: '372'
number: 372
title: "EQUITY通道折叠等价传递性——板块划分互斥性未在谱系层面声明"
type: 矛盾发现
status: settled
date: 2026-03-05
session: v162
source: Gemini 异质质询（gemini-3.1-pro-preview）
negation_source: heterogeneous
negation_form: unclassified
negation_model: gemini-3.1-pro-preview
depends_on:
  - '352'   # 折叠等价类定义——等价关系 ~
  - '292'   # 折叠拓扑本体论——EQUITY 通道判据
related:
  - '231'   # 形式化有效域规则
---

# 372号：EQUITY通道折叠等价传递性——板块划分互斥性缺口

**来源标注**：[Gemini 异质质询] gemini-3.1-pro-preview

## 矛盾描述

352号定义折叠等价关系 ~ 时，EQUITY 通道的判据是："同板块且 D 算子三态一致"。

Gemini 的否定：若某标的 S2 属于多个板块（如某股票同时被分类为"科技"和"新能源"），则：

- S1（科技板块）~S2（科技+新能源）：因同属科技
- S2（科技+新能源）~S3（新能源板块）：因同属新能源
- 但 S1（科技）≁S3（新能源）：因板块不同

传递性破缺 → ~不是等价关系 → 商空间在数学上不成立。

## 代码实现分析

`fold_equivalence.py` 中 `TargetAttributes.sector` 是 `str` 类型（单值）：

```python
@dataclass(frozen=True, slots=True)
class TargetAttributes:
    ...
    sector: str  # 单值字符串
```

等价键：
```python
def equivalence_key(target):
    if target.fold_channel is FoldChannel.EQUITY:
        return (FoldChannel.EQUITY, target.sector, target.d_tri_state)
    return (target.fold_channel, "", None)
```

**代码实现层面**：sector 是单值 str，一个标的只能有一个板块标签。代码实现上板块是互斥的。

但 **谱系层面（352号+292号）** 没有明确声明"板块划分是互斥 partition"——这是代码实现的隐性约束，没有在本体论层面显式化。

## Gemini 推导链摘要

1. 读取 352号 §3.2：EQUITY 等价判据依赖"同板块"
2. 构造反例：跨板块股票使传递性破缺
3. 结论：等价关系公理的传递性需要额外假设（板块划分为互斥 partition）
4. 否定：商空间数学合法性依赖一个未显式化的假设

## 质询判定：否定部分成立（定义缺口，而非代码错误）

定义回溯：传递性确实需要板块为互斥 partition。代码实现已隐性满足，但谱系定义层面未声明。

反例有效性：在代码实现约束下，反例不能实际发生（sector 是单值）。但如果未来实现改为 sector: list[str]（允许多板块），传递性将破缺。

推论检验：这是一个**定义缺口**（谱系未声明互斥性），而非**当前代码错误**。但定义缺口在概念层面是真实的——等价关系的完整定义缺少一个必要前提。

## 边界条件

- 当 sector 为单值（当前实现）：传递性在实现层面成立，否定不影响当前代码
- 当 sector 为多值（未来扩展）：传递性破缺，商空间失效
- 否定的强度：定义层面的缺口（需补充声明），而非根本性错误

## 下游推论

1. 352号需补充声明："板块划分是互斥 partition——每个标的有且仅有一个板块标签"
2. fold_equivalence.py 中 sector: str 的单值约束应在注释中明确：这不仅是实现细节，也是等价关系公理成立的前提
3. 如果未来扩展支持多板块，等价关系需要重新定义（可能需要从等价关系退化为相似关系）

## 谱系引用

- 352号：折叠等价类定义——被质询的 EQUITY 判据来源
- 292号：折叠拓扑本体论——EQUITY 通道的概念来源
- 231号：形式化有效域规则——定义缺口属于"有效域 ≠ 定义域"的表现

## 影响声明

- 352号需补充：板块划分互斥性声明
- fold_equivalence.py 需在 sector 字段注释中说明互斥性约束
- 不影响当前代码的正确性（实现已满足）
- 不影响任何现有 L2 验证结果

---

## 结算意见（settler-math, v163-swarm）

### 判定：否定部分成立（定义缺口，当前代码正确），予以结算

### 结论

Gemini 指出的传递性破缺问题在**数学推导上正确**：如果板块允许多归属（一个标的属于多个板块），则通过不同板块标签建立的等价关系确实不满足传递性，商空间不合法。

但**当前实现不受影响**。`fold_equivalence.py:108` 中 `sector: str` 是单值字符串——每个标的有且仅有一个板块标签。在单值约束下，等价键 `(EQUITY, sector, d_tri_state)` 的传递性由元组相等性保证：若 key(A)=key(B) 且 key(B)=key(C)，则 key(A)=key(C)。

问题的实质是：352号定义层面没有显式声明"板块划分是互斥 partition"这一等价关系成立的必要前提。代码中 `sector: str` 隐性满足了这个前提，但谱系文本中没有这个声明。

### 定义依据

| 概念 | 来源 | 关键表述 |
|------|------|---------|
| 折叠等价关系 ~ | 352号 §3.2 | EQUITY 通道：同板块 + D 算子三态一致 |
| 等价关系公理 | 数学定义 | 自反、对称、传递三性质缺一不可 |
| sector 字段类型 | `fold_equivalence.py:108` | `sector: str`（单值字符串） |
| equivalence_key | `fold_equivalence.py:122-136` | `(EQUITY, target.sector, target.d_tri_state)` |

### 边界条件

- 当 sector 为单值 str（当前实现）：传递性成立，否定不影响当前代码和任何 L2 验证结果
- 如果未来 sector 改为 `list[str]` 或 `set[str]`（多板块归属）：传递性破缺，商空间失效，需要重新定义等价关系（可能退化为聚类/相似关系）
- 否定的强度：定义层缺口（需补充显式声明），而非架构缺陷

### 下游推论

1. **352号需补充前提声明**：在 §3.2 EQUITY 通道判据处明确添加："板块划分是互斥 partition——每个标的有且仅有一个板块标签。此互斥性是等价关系传递性的必要前提"
2. **fold_equivalence.py sector 字段注释强化**：当前 docstring 已说明 "板块/行业标签"，但应明确标注"单值约束是等价关系公理成立的前提，不仅是实现细节"
3. **未来扩展的红线**：如果需要支持多板块标的，不能简单将 sector 改为列表——必须重新定义等价关系或引入不同的聚类方法

### 谱系引用

- 352号：折叠等价类定义——被质询的 EQUITY 通道判据来源
- 292号：折叠拓扑本体论——EQUITY 通道的概念来源
- 231号：形式化有效域规则——隐性约束未显式化属于有效域边界未声明

### 影响声明

- 352号需补充：板块划分互斥性前提声明（定义层文本修正）
- `fold_equivalence.py` 中 `sector` 字段的 docstring 建议强化（注释层，不改代码逻辑）
- 当前代码正确性不受影响
- 当前所有 L2 验证结果不受影响
- 不产生任何紧急代码变更需求
