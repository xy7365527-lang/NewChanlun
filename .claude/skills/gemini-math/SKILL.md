# Gemini Mathematical Derivation (derive)

## 概述

构造性数学推导引擎。接收命题，输出形式化证明。不限领域，通过公理集注入实现领域特化。

## 触发条件

- 遇到需要形式化证明的数学命题
- 关键词：证明、推导、几何性质、递归极限、拓扑、收敛、不变量、公理、定理、引理
- CLI: `python -m newchan.gemini_challenger derive "命题" --domain "领域"`

## 领域支持

| 领域 | 标识 | 公理集来源 |
|------|------|-----------|
| 通用数学 | `general` (默认) | 内置 ZFC 基础 |
| 拓扑学 | `topology` | 点集拓扑公理 |
| 递归论 | `recursion` | Church-Turing 框架 |
| 集合论 | `set-theory` | ZFC / NBG |
| 数论 | `number-theory` | Peano 公理 |
| 形式化缠论 | `chanlun` | definitions/ 目录动态读取 |
| 类型论 | `type-theory` | Martin-Lof / HoTT |
| 自定义 | `custom:<path>` | 用户指定公理文件 |

任何可公理化的数学领域均可通过提供公理集文件接入。

## 缠论特化

缠论不是特殊模式，而是通过 context 注入公理集：

- 包含关系、笔、线段、中枢的形式化定义
- 从 `definitions/` 目录读取最新定义（版本感知）
- 与通用数学使用同一推导引擎
- 定义处于生成态时，derive 输出自动标记为 provisional

```
# 示例：缠论命题推导
python -m newchan.gemini_challenger derive \
  "三个连续线段的重叠区间构成中枢" \
  --domain chanlun
```

## 输出格式

四段式结构，每段不可省略：

### 1. 形式化重述 (Formal Restatement)
将自然语言命题转化为形式化符号表达。明确量词、变量域、前提条件。

### 2. 定义与公理 (Definitions & Axioms)
列出推导所依赖的全部定义和公理。每条标注来源（内置/definitions/用户提供）。

### 3. 推导链 (Derivation Chain)
逐步推导，每步标注所用规则。格式：

```
Step N: <结论>
  By: <规则/公理名称>
  From: Step X, Step Y, ...
```

### 4. 结论 (Conclusion)
重述命题成立/不成立的判定，附 Q.E.D. 标记。如果命题不成立，给出反例或说明推导链断裂点。

## 与其他 Gemini 模式的关系

| 模式 | 方法论 | 产出 |
|------|--------|------|
| `challenge` | 辩证法 | 寻找矛盾 |
| `verify` | 形式逻辑 | 检查闭环 |
| `decide` | 价值判断 | 选择方案 |
| `derive` | 构造性数学 | 产出新命题 |

协作流：`derive` 产出 → `verify` 二次校验（双盲测试）→ 通过则结晶为定理。

## 质量保证

- derive 的输出必须经过 verify 模式二次校验
- 校验采用双盲：verify 不知道 derive 的推导路径，独立验证结论
- 如果 verify 发现推导链缺陷，返回 derive 修正（最多 3 轮）
- 3 轮后仍未通过，标记为 `unresolved` 并上浮

## 边界条件

- 公理集不完备时：明确标注哪些步骤依赖了未声明的假设
- 定义处于生成态时：输出标记 `[provisional]`，不写入 settled
- 推导链超过 50 步时：建议拆分为引理链
- 遇到不可判定命题时：输出判定并说明原因，不强行构造证明

## 债务集成

当 derive 产出被多次复用（>=3 次引用），触发结晶债务：
- 自动写入 `.chanlun/.crystallization-debt.json`
- 债务描述包含命题摘要和引用计数
- 结晶为 skill 文件后标记 resolved
