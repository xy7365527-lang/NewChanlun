---
id: 249
title: L78 修复后真实数据重新验证
type: 经验验证
status: settled
depends_on: [247, 248, 246]
date: 2026-02-28
---

# 249号：L78 修复后真实数据重新验证

## 1. 结论

**段引擎 L78 修复（ca3958a）使 60min 的 D2 压缩从 71→1 恢复到 71→8，产生 2 个中枢和 1 个走势。T6 仍不可达（active_levels=1，需 >=2），但 60min 单 TF 层已突破走势产出瓶颈。**

### 对比表

| 指标 | v101（修复前） | v103（修复后） | 变化 |
|------|---------------|---------------|------|
| 60min segments | 1 | **8** | +7 |
| daily segments (daily_weekly) | 3 | 3 | 不变 |
| daily segments (60min_daily) | 1 | 1 | 不变 |
| weekly segments | 1 | 1 | 不变 |
| 60min zhongshu | 0 | **2** | +2 |
| daily zhongshu | 0 | 0 | 不变 |
| 60min moves | 0 | **1** | +1 |
| daily moves | 0 | 0 | 不变 |
| 60min active_levels | 0 | **1** | +1 |
| T6 reachable | false | false | 不变 |
| directional divergences | 0 | 0 | 不变 |
| amplitude divergences | 0 | 0 | 不变 |

### 60min 管线详细恢复

修复前：71 strokes → 1 segment（s0=0, s1=70, down, unconfirmed）→ 0 zhongshu → 0 moves

修复后：71 strokes → 8 segments（7 confirmed + 1 unconfirmed）→ 2 zhongshu → 1 move

8 段详情：
| 段 | s0-s1 | 方向 | confirmed | high | low | stroke_span |
|----|-------|------|-----------|------|-----|-------------|
| 1 | 0-12 | down | true | 667.34 | 634.92 | 12 |
| 2 | 13-17 | up | true | 673.95 | 654.41 | 4 |
| 3 | 18-34 | down | true | 689.70 | 650.85 | 16 |
| 4 | 35-39 | up | true | 688.39 | 650.85 | 4 |
| 5 | 40-46 | down | true | 689.25 | 671.20 | 6 |
| 6 | 47-53 | up | true | 696.09 | 671.20 | 6 |
| 7 | 54-56 | down | true | 696.09 | 676.57 | 2 |
| 8 | 57-70 | up | false | 697.84 | 675.78 | 13 |

2 个中枢：
- ZS1: seg 0-3, zg=667.34, zd=654.41, gg=689.70, dd=634.92
- ZS2: seg 3-6, zg=688.39, zd=671.20, gg=696.09, dd=650.85

1 个走势（盘整）：
- kind=consolidation, direction=up, seg 0-7, zs_count=1, settled=false

### 力度对比（首次产出）

60min 层首次产出力度值：
- 振幅力度（amplitude_force）：54.78
- 方向性力度（directional_force）：4.25
- move_kind: consolidation, move_direction: up, zs_count: 1

两种力度在数值上差异显著（54.78 vs 4.25），但因为只有一个 TF 层有走势，跨 TF 背驰检测无法执行（需要高低两个 TF 层都有走势）。

### daily_weekly 数据集

daily_weekly 数据集在 v101 和 v103 之间**完全不变**（daily 36→3, weekly 6→1）。这符合预期：L78 修复只影响初始方向 bug 触发的场景，daily 和 weekly 数据上 L78 的初始方向本来就可行。

## 2. 定义依据

### 248号：bug 精确定位

248号诊断确认 71→1 压缩是 L78 前置 reject 在初始方向不可行时的 bug。修复方式：reject → warning，让段引擎继续构建而非阻断。

对比实验（248号中从笔 1 开始）预测 70→7。实际修复结果 71→8——多出 1 段是因为修复后笔 0 也参与构建，提供了额外的初始段（s0=0, s1=12, down）。

### 246号：T6 可达条件

> recursive_levels_equivalent = 有走势输出的 TF 层数。当 >= 2 时，t6_reachable = True。

v103 结果：active_levels=1（仅 60min 有走势，daily 无走势）→ t6_reachable=False。

T6 不可达的原因从 v101 的"所有 TF 层都无走势"缩窄为"daily 层 9 strokes → 1 segment → 0 zhongshu → 0 moves"。Daily 层仍是瓶颈。

### 缠论中枢定义

中枢需要 ≥3 个**连续线段**的价格区间有公共重叠。60min 8 段满足此条件，产出 2 个中枢。Daily 1 段不满足。

## 3. 边界条件

### 3.1 结论翻转条件

T6 将在以下条件下变为可达：
1. **daily 层段数增加**：如果 daily 层的 9 strokes 能产出 ≥3 段（其中 ≥3 段有价格重叠），则 daily 也产出中枢→走势→active_levels=2→T6 可达
2. **更长数据**：更多月份的 60min 数据（当前受 yfinance 6 个月限制）可能让 daily 层有更多笔
3. **不同市场阶段**：当前 daily 层只有 1 个 up 段（市场处于近 6 个月单边上涨+急跌），震荡市可能产出更多方向交替的段

### 3.2 ker(D) 验证条件

ker(D) 操作意义验证需要**两个 TF 层都有走势**才能执行跨 TF 背驰检测。当前只有 60min 有走势，不足。但 60min 层已首次产出力度值（amplitude=54.78 vs directional=4.25），当第二个 TF 层也产出走势时，两种力度的背驰检测差异将可观测。

### 3.3 数据时效

yfinance 数据受拉取时间影响。v101 和 v103 在同一天运行，数据一致性高。但 bar 数和价格可能因 yfinance 缓存/更新而有微小差异（实际观测到的差异在小数点后几位，不影响结论）。

## 4. 下游推论

### 4.1 247号部分翻转

247号结论"T6 在真实数据上不可达，所有 TF 的 segment → zhongshu → moves 全为 0"被部分翻转：

- **翻转**：60min 的 "71→1→0→0" 翻转为 "71→8→2→1"。这不是 D2 弱方向吸收的真实压缩，是初始方向 bug
- **保留**：daily 和 weekly 的结果不变（不受 L78 修复影响）。Daily 的 D2 压缩（36→3→0→0）仍然存在
- **修正**：247号"D2 压缩是独立的、根本性的瓶颈"需要区分——60min 的压缩是 bug（已修），daily 的 3 段未产出中枢可能是数据量不足（9 strokes），不是 D2 bug

### 4.2 229号仍不可关闭

T6 在真实数据上仍不可达（active_levels=1 < 2）。229号的"T6/T7 贡献率极低"结论不变。但瓶颈已从"所有层无走势"缩窄为"daily 层无走势"——229号关闭的距离缩短了。

### 4.3 248号完全确认

248号诊断"71→1 是 bug 不是 D2 弱方向吸收"被完全确认：
- 预测 70→7（对比实验从笔 1 开始）
- 实际 71→8（修复后从笔 0 开始，多出初始段）
- 偏差 +1 完全可解释：修复后笔 0 不再被 L78 阻断，产生了一个额外的初始段

### 4.4 力度值首次产出

60min 层首次产出了真实数据上的力度值。两种力度差异（54.78 vs 4.25）为 ker(D) 操作意义提供了**间接证据**：同一个走势在两种度量下的力度值差异达到一个数量级。但直接操作意义证据（跨 TF 背驰检测差异）仍需 daily 层走势产出。

## 5. 谱系引用

- **248号**（段引擎 71→1 压缩诊断）：直接前置。248号的 bug 诊断被本次修复+重验完全确认
- **247号**（真实数据多 TF 验证）：直接前置。247号的 60min 结论被翻转（bug 导致的虚假压缩），daily/weekly 结论保留
- **246号**（多 TF pipeline 集成）：间接前置。246号架构在修复后的段引擎上表现正确（60min 层走势产出），但 T6 可达仍未达到
- **229号**（T6/T7 贡献率分析）：间接前置。229号不可关闭，但瓶颈范围缩窄
- **233号**（ker(D) 离散化算子核）：间接相关。60min 层首次产出力度值，两种力度差异 54.78 vs 4.25 为 ker(D) 提供间接证据

## 6. 影响声明

### 新增文件

1. `scripts/real_data_revalidation_v103.py`：重新验证脚本（与 v101 脚本仅输出路径和文档字符串不同）
2. `tmp/real-data-revalidation-v103.json`：重新验证结果 JSON
3. `.chanlun/genealogy/settled/249-real-data-revalidation-post-l78-fix.md`：本谱系

### 状态变更

- **247号**：60min 相关结论被翻转（bug 导致的虚假压缩），daily/weekly 结论不变
- **248号**：完全确认（bug 诊断正确，修复有效）
- **229号**：不变。T6 仍不可达，但瓶颈缩窄为 daily 层
- **246号**：不变。架构正确，60min 层已产出走势

### 暴露的新瓶颈

**Daily 层（60min_daily 数据集中）的 9 strokes → 1 segment** 是 T6 可达的当前瓶颈。这不是 L78 bug（daily 层初始方向可行），可能是数据量不足（6 个月日线仅 126 bar → 9 strokes）。增加数据长度可能解决此问题。
