# D_parent / D_child 重放前冻结（2026-07-10）

- 状态：**FROZEN-BEFORE-REPLAY**
- 适用分支：`p0-replay-dparent`
- 基线：`1dcdb95a09` + confirm_src 拆绑定提交（本分支 cherry-pick）
- 裁决：`chanlun/escalate/p0-jparent-dparent-ruling-20260710.md`
- 考据输入：`chanlun/review-results/nest-lag-deep-research-20260709.md`（本机权威副本 SHA-256：`43b56ab127d40639c8c9c740d90a9de6608c20a898852e0e8063c05234677b36`）

本文件在实现和重放前冻结。后续不得因产量、延迟分布或测试结果调整本定义；若实现发现字段能力与本冻结冲突，只能在最终报告记录冲突，不得回改本文件。

## 1. 原文契约

1. **[旧缠论] 背驰段**：第 27 课把“某级别的某类型走势，如果构成背驰或盘整背驰”的该段走势类型称为该级别背驰段（`docs/chanlun/text/blog/027-第27课.md:26`）。
2. **[旧缠论] 区间套对象**：第 27 课要求低级别背驰段位于高级别背驰段内、区间更小，并以不同级别背驰段逐级收缩定位转折（同文件 `:38-46`）。因此包含对象是闭区间“段对段”，不是确认时刻对确认窗。
3. **[旧缠论] 进入背驰段后下钻**：第 27 课盘中说明“在 5 分钟进入背弛段后”寻找 1 分钟相应背驰段（同文件 `:332`）。父级最终确认不是下钻前置门。
4. **[旧缠论] C 段判定语境**：第 24 课以 A、B、C 描述趋势背驰，C 是第二个中枢后的同向离开走势；C 走势类型完成且力度弱于 A 时构成标准背驰（`docs/chanlun/text/blog/024-第24课.md:22-28`）。MACD 只作辅助，冻结边界取结构字段，不引入数值容忍带。

## 2. 坐标与同名消歧

全部区间均使用 `source_index` 闭区间 `[left, right]`。

代码历史注释把 C 离开 episode 起点称为 `lambda_c`，而本轮裁决把“子级确认时刻”写作 `λ_C`。为避免把进入坐标与确认坐标再次绑定，本文件采用两个不同记号：

- `λ_enter(e) := e.enter_src = e.interval.0`：事件 `e` 进入当前 C 离开 episode 的结构起点；
- `λ_conf(e) := e.confirm_src`：事件 `e` 的算法确认时点，即裁决第 3 条所称子级 `λ_C`。

二者没有相等不变量；`λ_conf` 不得由任一区间右端回填。

## 3. D_parent 冻结定义

**[旧缠论:选择]** 对任一 `cand_delta=true` 的父级 `CandDeltaEvent p`：

```text
D_parent(p) := [ λ_enter(p), right_C(p) ]
             := [ p.enter_src, p.interval.1 ]
             =  p.interval
```

左端点冻结为 `p.enter_src`，其生产语义是：最后一个已确认父级中枢之后，当前 C 离开 episode 中**第一个与破中枢方向同向的 Segment 的 `start_index`**。代码唯一来源为 `departure_move_c_start(...)`；并要求运行时诊断不变量 `p.enter_src == p.interval.0`。

右端点冻结为父级背驰对应转折段的结构端点 `p.interval.1`，当前生产者取 `seg.end_index`。它不是 `p.confirm_src`，也不由 `p.confirm_src` 派生。

明确排除以下左端口径：父级确认窗左端、`confirm_src`、最后一根破中枢 Segment 的临时起点、按产量回看后选择的任意锚点、任何 `ε` 外推。

## 4. D_child 字段映射与包含判据

**[新缠论]** 对相邻低一级的 `cand_delta=true` 子级事件 `c`，裁决中的 `I(A)` 冻结映射为：

```text
D_child(c) := I(A_child) := c.a_interval
             = [ c.a_interval.0, c.a_interval.1 ]
```

`a_interval` 的生产语义是与子级当前 C 离开相配对的、前一中枢同向离开 episode 全区间 `[λ_A, ρ_A]`。它不是 `c.interval`（后者是 `I(C_child)`），也不是 `enter_src` 或 `confirm_src` 的点区间。

相邻级唯一结构门冻结为闭包含：

```text
I(A_child) ⊆ D_parent
⇔ child.a_interval.0 >= parent.enter_src
∧ child.a_interval.1 <= parent.interval.1
```

这与 `nest::is_sub(child, parent)` 的闭区间契约同构。方向 `side` 必须一致，父子事件都必须显式满足 `cand_delta=true`；盘整背驰诊断事件不得入链。

## 5. 仅登记的诊断量

**[新缠论]** 对每个接受或被结构门考察的相邻父子候选，登记：

```text
lag_conf := λ_conf(child) - right(D_parent)
          := child.confirm_src - parent.interval.1
```

登记使用有符号整数，保留负值、零与正值的完整分布。`λ_conf(child)` 只要求字段存在；`lag_conf`、`λ_conf` 与 `ε_conf` 均不参与候选否决、排序或包含判定。`ε_conf` 仅可作为报告中的诊断标签，不得成为参数门。

## 6. 重放前不可变验收表

| 项 | 冻结值 |
|---|---|
| `J_parent` | `D_parent = parent.interval = [parent.enter_src, parent.interval.1]` |
| D_parent 左端 | 当前 C 离开 episode 首个同向 Segment 的 `start_index` |
| `D_child` / `I(A_child)` | `child.a_interval` |
| Sub | `is_sub(D_child, D_parent)`，闭包含 |
| 子级确认时刻 | `child.confirm_src`，仅登记 |
| 延迟 | `child.confirm_src - parent.interval.1`，有符号完整分布 |
| `ε_conf` | 纯诊断，不作闸门 |
| 事件过滤 | 父子均 `cand_delta=true`，方向一致；`pan_div_diag` 不入链 |

