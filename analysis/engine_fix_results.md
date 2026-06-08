# 引擎修复验证报告（5 项修复）

> 生成时间：2026-05-31  
> 数据：QQQ 5y 日线  
> 认识论等级：L2（真实数据，可否证）

---

## 0. 修复清单

| # | 修复内容 | 状态 |
|---|---------|------|
| a | `reset_dir_on_fractal=True`（engine_fix_runner 显式传入；默认值保持 False 以维护缠论短序列正确性） | ✅ |
| b | MACD 传入 `build_recursive_levels`（`df_macd` 参数） | ✅ |
| c | 截窗口 600 根（默认） | ✅ |
| d | 递归层：`build_recursive_levels` 已实现 Segment→TrendTypeInstance 递归 | ✅ 已存在 |
| e | 修复后对比 TV 137 个买卖点 | ✅ 见下 |

---

## 1. 修复后（600 根日线窗口）

| 指标 | 数值 |
|------|------|
| 原始 K 线 | 600 |
| 合并后 K 线 | 485 |
| 分型 | 202 |
| 笔（总） | 44 |
| 笔（confirmed） | 43 |
| 线段（总） | 4 |
| 线段（confirmed） | 3 |
| 递归层数 | 0 |
| 背驰（总） | 0 |
| 背驰（confirmed） | 0 |

### 1.1 各递归层详情

| 层 | Moves | 中枢 | 走势 | 走势(conf) | 背驰(总) | 背驰(conf) | MACD背驰 |
|---|-------|------|------|-----------|---------|-----------|---------|

---

## 2. 原始对比（全量 1255 根，无截窗口）

| 指标 | 数值 |
|------|------|
| 原始 K 线 | 1255 |
| 合并后 K 线 | 1018 |
| 笔 | 87 |
| 线段（confirmed） | 8 |
| 背驰（confirmed） | 0 |

### 2.1 各递归层详情（全量）

| 层 | Moves | 中枢 | 走势 | 走势(conf) | 背驰(总) | 背驰(conf) |
|---|-------|------|------|-----------|---------|-----------|
| L1 | 8 | 1 | 1 | 0 | 0 | 0 |

---

## 3. TV 137 个买卖点对比

| 维度 | 修复后（600根） | 全量（1255根） | TV 基准 |
|------|--------------|-------------|---------|
| 线段（confirmed） | 3 | 8 | ~345+ (推算) |
| 递归层数 | 0 | 1 | — |
| 背驰（confirmed） | 0 | 0 | ~137 (BSP 代理) |
| MACD 背驰 | 0 | N/A | — |

## 4. 根本差距分析（修正估算）

**原比较报告估算「600 根→~200 线段」是错误的。**

实际笔→线段压缩比在各窗口中一致（≈10:1），与窗口大小无关：
- 全量（1255根）：87 笔 → 9 线段
- 截窗口（600根）：44 笔 → 4 线段

600 根窗口产出 3 个 confirmed 线段，不满足 L1 中枢形成条件（需 ≥3 个 confirmed 走势）。

TV 137 个 BSP 来自多时间框架 CZSC（Study1 + Study2 两个实例），不是单一日线级引擎的输出。
要达到 TV 的信号量级，需要：
1. 分钟级数据（更细粒度的笔/线段结构）
2. 单独实现 Type2/Type3 买卖点（不依赖背驰的结构判断）
3. 桥接 Center v0 和 Zhongshu v1 管线

---

## 5. 结果包六要素

**结论**：
- 3 项代码修复（reset_dir_on_fractal + MACD + 截窗口）已正确实装
- 但受限于日线级笔→线段 10:1 压缩比，600 根窗口仅产出 3 个 confirmed 线段
- 递归层无法触发（需要 ≥3 confirmed 走势才形成 L1 中枢）
- confirmed 背驰 = 0，与 TV 137 的差距依然存在
- d 项（递归层）已实现（`build_recursive_levels` L0 代数验证通过）

**定义依据**：
- `build_recursive_levels` 实现了 Segment → TrendTypeInstance 的自下而上递归（F_k 滤子构造）
- `divergences_from_level` 使用 MACD DIF/HIST 面积计算 force_a/force_c 对比
- `confirmed` 背驰 = C 段完成后确认（走势 settled=True）

**边界条件**：
- 日线引擎能触发 L1 递归的条件：confirmed 线段 ≥ 9（3 中枢 × 3 线段/中枢）
- 要达到该条件需要约 90 笔（90/10=9线段），对应约 ~900+ 根原始日线（≥3.5 年）
- 但 3.5 年全量数据又导致大级别结构主导（月线视角），形成矛盾

**下游推论**：
- 日线级别引擎的固有矛盾：短窗口→线段不足，长窗口→月线化
- 破局路径：用分钟级/小时级数据运行笔/线段引擎，日线级别作为二级递归输入
- 或桥接 v0 Center 与 v1 Zhongshu 接口，实现完整 Type1/2/3 买卖点提取

**谱系引用**：
- engine_vs_tv_comparison.md 的 5 项修复建议（L2 验证）
- 不确定是否有关于线段算法或买卖点分级的相关谱系记录

**影响声明**：
- `src/newchan/a_inclusion.py`：**未修改**（默认值保持 False 以保证缠论短序列正确性；`engine_fix_runner.py` 显式传入 True，原计划的默认值修改因测试失败而撤回）
- 新建 `scripts/engine_fix_runner.py`（本脚本，含 reset_dir_on_fractal=True 显式调用）
- 新建 `analysis/engine_fix_results.md`（本报告）
- 新建 `scripts/ph_level_analysis.py` + `analysis/ph_level_definition.md`（Task 1 输出）