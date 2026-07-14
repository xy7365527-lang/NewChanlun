# L1->L2 归零代码实现层审计（任务 A，2026-07-11）

- 任务边界：只审 `theta_v0` 实现层 bug；定义谱系以 `chanlun/review-results/dr-l1l2-zero-genealogy-20260711.md` 为上游结论，不重复裁定义层。
- 代码图工具状态：`codebase-memory-mcp` 的 `list_projects` / `index_status` 本轮均返回取消，已按 `AGENTS.md` 降级为 `rg` + 带行号源码读取。
- 审计范围：`rust/src/theta_v0/classifier/divergence.rs`、`recursive_tower.rs`、`nest.rs`、`rust/src/bin/strict_nest_check.rs`，并核对 `#38` 的 `confirm_src` / `interval.end` 拆绑报告。
- 结论：本轮未发现能把 L1->L2 严格嵌套证书误杀为 0/0 的纯实现错误。现有 0/0 在代码路径上落在预注册结构门 `同方向 ∧ child.a_interval ⊆ parent.D_parent` 的 `Sub` 左界失败；这属于当前冻结定义落地的结果，不是索引、边界、方向、单位、过滤器误杀或条件永假的实现 bug。

## 1. 执行路径复原

### 1.1 事件生产

`CandDeltaEvent` 的字段语义在 `rust/src/theta_v0/classifier/recursive_tower.rs:725-744` 定义：

- `confirm_src`：算法确认时点。
- `interval`：结构定位区间，当前 D_parent 生产为 `[lambda_c, seg.end_index]`。
- `a_interval`：与当前 C 配对的前一中枢离开 episode，即冻结口径的 `I(A_child)`。
- `enter_src`：D_parent 左端，等于 `lambda_c`。
- `cand_delta`：趋势背驰确认 bit。

生产赋值在 `rust/src/theta_v0/classifier/recursive_tower.rs:862-889`：

```text
a_seg_entry   = locate_departure_move_a(...)
c_start_entry = departure_move_c_start(...)
confirm_src   = pf.source_index
interval      = (lambda_c, seg.end_index)
a_interval    = a_seg_entry
enter_src     = lambda_c
cand_delta    = pf.bits.buy1 || pf.bits.sell1
```

`departure_move_c_start` 本身在 `rust/src/theta_v0/classifier/divergence.rs:738-755`，窗口是 `start_index ∈ [c.end_index, until_start]`，并委托 `episode_start_in`。`episode_start_in` 在 `divergence.rs:718-735` 用最后一个回中枢反向段作为边界，再取边界后首个同向锚段。这里没有发现 off-by-one：相邻段共享端点时，用 `s.start_index >= r.end_index` 能取到回中枢段之后的下一段；测试 `departure_move_c_start_reentry_bounds_episode` 在 `divergence.rs:896-909` 覆盖了该场景。

### 1.2 递归塔输入单位

`cand_delta_tower_with_series` 在 `rust/src/theta_v0/classifier/mod.rs:505-540` 按级别生成事件：

- L0 使用 `l0.segments`。
- L>=1 使用 `project_to_units(&tower_snapshots[lvl], levels[lvl-1].moves)`，再经 `unit_to_segment` 还原成 `Segment`。
- `hist/dif/closes_tick/close_src` 全部沿用同一原始 `source_index` 坐标。

这排除了 L1->L2 审计中的一个常见实现错误：没有看到“高级别 unit 序号”和“原始 bar source_index”混用。`unit_to_segment` 的坐标保真约束在 `mod.rs:139-157`，`LeveledMove` 的 `start_index/end_index` 来源在 `recursive_tower.rs:103-115` 与 `:183-194`。

### 1.3 证书装配

严格装配在 `rust/src/theta_v0/classifier/nest.rs:394-409` 把事件转换成两个区间：

- `d_parent_interval(ev)`：`[ev.enter_src, ev.interval.1]`，并 debug 断言 `enter_src == interval.0`。
- `d_child_interval(ev)`：`[ev.a_interval.0, ev.a_interval.1]`。

装配递归在 `nest.rs:471-508`：

```text
if !ev.cand_delta || ev.side != side { continue; }
iv = d_parent_interval(ev)
if !is_sub(child_iv, &iv) { continue; }
next_child_iv = d_child_interval(ev)
```

`is_sub` 是闭包含，定义在 `nest.rs:61-66`：`inner.start_time >= outer.start_time && inner.end_time <= outer.end_time`。这与漏斗诊断的 `diagnose_pair` 完全同构，见 `rust/src/bin/strict_nest_check.rs:722-727`。

### 1.4 `confirm_src` / `interval.end` 拆绑

`#38` 报告 `chanlun/review-results/unbind-confirm-src-38-20260710.md` 说明 `confirm_src` 与结构区间右端已经拆字段承载。代码中：

- 事件生产分别赋值：`confirm_src = pf.source_index`，`interval_end = seg.end_index`，见 `recursive_tower.rs:880-887`。
- 证书保存确认时点但不回填区间：`NestRung::assembled(ev.confirm_src, ...)` 与 `base_confirm_src: Some(base.confirm_src)`，见 `nest.rs:459-463`、`:501`。
- 测试 `assemble_preserves_confirm_src_independent_from_interval_end` 在 `nest.rs:870-884` 明确构造 `confirm_src != interval.end` 并要求证书仍成立。

D_parent 复议后，确认时序不再是装配门：`dparent_accepts_child_a_inside_and_ignores_confirm_order` 在 `nest.rs:887-897` 覆盖了 `I(A_child)⊆D_parent` 成立时即使确认顺序逆序也应装配。漏斗中 `confirmation_lag` 只在 `strict_nest_check.rs:730-731` 计算并进入诊断分布，未进入 `passes()`。

## 2. 三个近失样本反推

来源：`chanlun/review-results/dparent-funnel-20260710.md:45-49`。

| 样本 | 父级 | 子级 | 代码谓词路径 | 失败原子谓词 |
|---:|---|---|---|---|
| 1 | L2 Short, `D=[352003,354036]` | L1 Short, `I(A)=[340499,341236]`, `confirm=345518` | `closest_pair_misses` -> `diagnose_pair` | `same_side=true`; `sub_end=true`; `sub_start=false`，因 `340499 < 352003`，缺口 11504 bar |
| 2 | L2 Short, `D=[2180264,2182560]` | L1 Short, `I(A)=[2160411,2161133]`, `confirm=2168349` | 同上 | `same_side=true`; `sub_end=true`; `sub_start=false`，因 `2160411 < 2180264`，缺口 19853 bar |
| 3 | L2 Short, `D=[2194856,2197213]` | L1 Short, `I(A)=[2160411,2161133]`, `confirm=2168349` | 同上 | `same_side=true`; `sub_end=true`; `sub_start=false`，因 `2160411 < 2194856`，缺口 34445 bar |

对应源码：

- `diagnose_pair`: `strict_nest_check.rs:722-727`。
- 缺口计算：`pair_gap_bars` 的 `parent.enter_src.saturating_sub(child.a_interval.0)`，见 `strict_nest_check.rs:734-739`。
- 近失排序：`NearMiss::rank_key`，见 `strict_nest_check.rs:810-819`。
- 近失枚举只看可达子链与 `cand_delta=true` 父子事件，见 `strict_nest_check.rs:884-921`。

因此三例都不是：

- `cand_delta=false` 过滤；
- 方向错；
- `child.confirm_src` 早于/晚于父窗导致否决；
- `interval.end` 被误用作 `confirm_src`；
- 右界越界；
- terminal 查无。

三例唯一失败点都是 `child.a_interval.0 >= parent.enter_src`。

## 3. 已排除的实现层疑点

### 3.1 `enter_src` 与 `interval.0` 漂移

生产上 `enter_src=lambda_c` 且 `interval=(lambda_c, interval_end)`，见 `recursive_tower.rs:882-889`。release 复跑还显式统计 `cand_delta=true` 中 `enter_src != interval.0`，函数在 `strict_nest_check.rs:959-965`；`dparent-funnel-20260710.md` 报告该计数为 0。

判定：未发现字段拆分后左端漂移 bug。

### 3.2 `confirm_src` 回流进结构门

装配门只读 `cand_delta`、`side`、`d_parent_interval`、`d_child_interval` 和 `is_sub`，见 `nest.rs:490-503`。漏斗门只读 `same_side`、`child.a_interval.0 >= parent.enter_src`、`child.a_interval.1 <= parent.interval.1`，见 `strict_nest_check.rs:722-727`。`confirmation_lag` 只用于 `lag_considered/lag_accepted` 诊断向量，见 `strict_nest_check.rs:867-873`。

判定：未发现确认时点误杀 bug。

### 3.3 盘整背驰诊断事件误入链

盘整背驰诊断事件在 `recursive_tower.rs:846-855` 被写成 `cand_delta=false, pan_div_diag=true`。装配层和漏斗层均显式过滤 `cand_delta=true`，见 `nest.rs:528-532`、`strict_nest_check.rs:861-865`。

判定：未发现 `pan_div_diag` 混入链导致误杀或误纳。

### 3.4 条件永假 / 过滤器误杀

L0->L1 已有 3 条可达证书，L1->L2 同向可达候选对有 22 个，见 `dparent-funnel-20260710.md:17-31`。这说明装配谓词不是全局条件永假；归零发生在 L1->L2 的具体 Sub 左界。

判定：不是全局死条件；若要改变 0/0，必须改变被比较的对象或 D_parent 左端口径，这属于定义/字段映射复议，不是当前代码把既定谓词写错。

### 3.5 `first_match_idx` 反查风险

风险点：`recursive_tower.rs:806-864` 与 `signal.rs:856-862` 使用 `(end_index, zd, zg)` 的首匹配下标 `pos` 来取 `prev_center=centers_sorted[pos-1]`，而最近中枢本体仍是 `c_idx`。`signal.rs:192-207` 明确说明重复三元组时首匹配可能不同于最近中枢下标。

审计判断：

- 这是一个真实实现风险面，因为 A 段定位声明上依赖“相邻前一中枢”，见 `divergence.rs:671-675`。
- 但本轮不能把它列为 L1->L2 0/0 的已确认 bug：当前代码注释将其作为旧 `.position()` bit-exact 兼容路径，且 `signal.rs:930-957` 的 resume 路径说明生产中枢 `end_index` 严格递增时 `pos==c_idx`，on2w3 实测重复三元组为 0/7.17M。
- 本轮近失样本报告没有提供 `pos/c_idx/prev_center` 诊断列，无法证明三例的早起 `child.a_interval` 来自首匹配误锚。

建议：不改代码；若后续要坐实该风险，给 `strict_nest_check` 增加只读诊断列 `parent/child c_idx,pos,prev_center.end_index,a_interval`。只有发现三例或可达 L1 事件存在 `pos != c_idx`，才可升级为实现 bug。

## 4. Bug 列表

本轮确认的纯实现 bug：无。

未列为 bug 的定义层/口径问题：

- `D_child := child.a_interval` 是否应改为低级别背驰段或 `span(A,C)`：任务 B 已归为定义/符号映射问题，不在本轮代码 bug 范围。
- `D_parent` 左端是否应从当前 C episode 起点改为完整父级 `c` 走势类型起点：任务 B 已归为定义落地复议，不在本轮代码 bug 范围。

## 5. 修复建议

由于没有确认的实现 bug，本轮不建议修改代码。建议的后续动作仅限诊断增强：

1. 在复跑漏斗中增加 `child.interval`、`child.a_interval`、可选 `span_ac`、`parent.enter_src`、`parent.interval` 的并列表，避免把定义对象错配误判为实现过滤错误。
2. 对 `first_match_idx` 风险增加非门诊断：统计 `pos != c_idx` 的事件数，并在近失样本中打印 `pos/c_idx`。该项只用于证伪/证实实现风险，不应改变当前判据。
3. 若定义层最终裁定 `D_child` 不再等于 `child.a_interval`，再改 `CandDeltaEvent` 字段与 `d_child_interval`，并保留当前三例作为回归样本，确认变化来自字段映射裁决而不是隐式放宽 Sub。
