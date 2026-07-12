# #43 隔离重放：完整父级 `c` 左端（2026-07-12）

- 最终判定：**BLOCKED-CAPABILITY**
- 裁决权威：`chanlun/review-results/dparent-leftend-p0-review-20260711.md`
- 分支 / HEAD：`p0-replay-dparent` / `99ce554b8a`
- 作用域：只检查 `D_parent.left: c_episode_start -> c_start_full` 的可表达性；`D_child := child.a_interval` 保持 **provisional / pending separate ruling**。
- 写入边界：未修改 θ_v6 生产代码、既有重放脚本或任何冻结文档；本文件是唯一新增产物。

## 0. 结论

现有字段与对象图不能表达裁决 §2 定义的完整 `c_p`，因此在重放前硬停止。没有合法的 `c_start_full`，也不能证明现有 `seg.end_index` 等于 `c_end_full = end(c_p)`；继续执行只会把 `c_episode_start`、确认点或猜测的塔锚点冒充完整 `c`，违反裁决 §3.2(5) 与 §7.D。

本次没有生成“新父左端”候选、边或证书；没有对旧基线做新旧横比；预测 `P0-43-L1L2-CFULL` **未裁定**（既不是 `SUPPORTED-ON-THIS-REPLAY`，也不是 `FALSIFIED`）。

## 1. 能力冲突

| 层 | 现有能力 | 与裁决要求的冲突 |
|---|---|---|
| 事件字段 | `CandDeltaEvent` 只有 `level/side/confirm_src/interval/a_interval/enter_src/cand_delta/pan_div_diag`（`rust/src/theta_v0/classifier/recursive_tower.rs:725-745`）。 | 没有 `B_p` 身份、`c_p` 身份、`c_interval_full`、`c_episode_interval`、第三类离开证书或递归归属路径。 |
| 事件生产 | `level_cand_delta` 定位 `c_idx` 后，仍以 `departure_move_c_start` 产生 `lambda_c`，并同时写入 `interval.0` 与 `enter_src`；`c_idx`、`prev_center`、段序与归属证据未进入事件（同文件 `:819-891`）。 | 唯一可消费左端仍是局部 episode；不能只改字段名取得 `c_start_full`。 |
| episode 定界 | `departure_move_c_start` / `episode_start_in` 按最后一次 reentry 切分并返回其后首个同向锚段。 | 裁决已明确该值只能映射为 `c_episode_start`，一般情形不等于 `c_start_full`。 |
| 递归塔 | `LeveledMove::compose` 把“构成一个中枢的连续窗口”封为上级 move（`recursive_tower.rs:280-316`）；`sub_moves` 保留的是该中枢窗口的次级对象。 | 该对象不是“最后同级别中枢 `B_p` 之后、最终构成背驰段的完整 `c_p`”；没有从父事件到完整 `c_p` 的所有权边。 |
| 连接段归属 | 中枢扫描遇到首个 non-extension 单元 `j` 后，以 `i = j` 继续扫描（`recursive_tower.rs:451-510`）。 | 离开/连接单元可被后续中枢窗口重新吸收；扫描没有产出独立的 `c` 归属侧车。后置的 `MoveBlock` 无法恢复扫描阶段未保存的边界。 |
| 走势分解 | `MoveBlock` 只携中枢下标跨度 `start_center/end_center`、类型、方向、状态（`classifier/decompose.rs:41-52`）。 | 没有完整走势类型的 bar 首尾，也没有 `B_p -> c_p` 连接与第三类离开在 `c_p` 内的证明。用块首中枢或父趋势起点回填都会越过裁决禁区。 |
| 第三类证据 | 第三类 `BspPoint` 保存回试点与 `center`；第一类点的 `center=None`（`classifier/signal.rs:371-473`）。 | 第三类对象不保存完整离开走势区间，也不归属到某个父事件的 `c_p`；无法证明“其内第三类离开结构”。 |
| 右端 | 当前事件右端直接取候选破中枢 `seg.end_index`（`recursive_tower.rs:880-889`）。 | 没有完整 `c_p` 对象就不能证明该端点是 `end(c_p)`；裁决 §2.2 禁止无证明沿用。 |

因此，虽然递归塔有坐标与 `sub_moves`，它只证明“中枢构成窗口”的递归组成；它没有裁决所需的“父级最后中枢—完整 `c`—第三类离开”三者归属证书。把 `tower move.start_index`、`MoveBlock` 首中枢、`confirm_src`、`until_start`、父级 A 起点或整个趋势起点任一项代填，都不是现有能力的合法推出。

仓库另有 `rust/src/level.rs::zhongshu_from_components / moves_from_level_zhongshus` 参考实现，它显式保留突破组件并把非末组走势延伸到下一中枢之前；但该实现不在 θ_v0 的 `cand_delta_tower` 事件生产链上。将 θ_v0 的中枢构造/走势分解切换到该实现会改变 `B_p`、递归塔与上游候选身份，违反本轮“唯一变量只改父左端”的隔离要求，不能作为重放脚本中的临时代填器。

## 2. 重放硬停止位置

1. 数据文件与既有重放二进制入口存在；`cargo check --bin strict_nest_check` 通过；`cargo test departure_move_c_start_reentry_bounds_episode --lib` 为 1/1 通过，确认现函数确实按 reentry 切分 episode。
2. 在任何字段变更和全量因果重放之前执行定义/能力门。
3. 能力门无法产出逐父事件的 `(B_p, c_p, third-class-inside-c_p)` 证书，故按裁决 §7.D 停止。
4. 未调用旧 episode 逻辑生成“新”结果，避免制造 `FAIL-DEFINITION-MAPPING` 型伪重放。

## 3. L1→L2 漏斗状态

旧冻结基线仅作为预注册参照保留：L1 可达 partial chain = **3**，L2 `cand_delta=true` = **13**，同向可达候选对 = **22**。

由于 `c_start_full` 不可表达，本轮没有合法的新候选数、边数、完整证书数或差分；没有进入横比，因此也不宣称“基线未漂移”。新增 L1→L2 边集合未定义，不能逐条输出 `parent B / c_interval_full / c_episode_interval / child.a_interval / direction / Sub`，更不能给出结构归属证明。

## 4. 三个预注册近失样本

事件身份按裁决原样保留，未以新排序替换：

| 样本 | 旧父窗 | `D_child := child.a_interval`（provisional） | 旧左界缺口 | 新 `c_start_full` | `child.a_interval.0 - c_start_full` | Sub 左/右 |
|---:|---|---|---:|---|---|---|
| 1 | `[352003,354036]` | `[340499,341236]` | 11504 | **不可表达** | **不可计算** | **未裁定 / 未裁定** |
| 2 | `[2180264,2182560]` | `[2160411,2161133]` | 19853 | **不可表达** | **不可计算** | **未裁定 / 未裁定** |
| 3 | `[2194856,2197213]` | `[2160411,2161133]` | 34445 | **不可表达** | **不可计算** | **未裁定 / 未裁定** |

三例没有合法通过或不通过结果，故不得登记 `SUPPORTED-ON-THIS-REPLAY` 或 `FALSIFIED`。

## 5. 裁决 §7 清单逐项状态

### A. 重放前定义/字段检查

- [ ] **BLOCKED** — 无法只把 `D_parent.left` 切换到合法 `c_start_full`；其余字段未改。
- [ ] **BLOCKED** — 无法同时输出真实 `c_start_full / c_episode_start / c_end_full`；只能输出 episode 值。
- [ ] **BLOCKED** — 无法为每个父事件输出 `B_p`、完整 `c_p` 首尾及其内第三类离开归属证据。
- [ ] **BLOCKED** — 无合法 `c_start_full`，无法统计与 `c_episode_start` 的 `== / < / >`。
- [ ] **BLOCKED** — 无法以现有对象证明左端没有扩到父级 A 或整个父趋势；因此没有选择任何替代锚点。
- [ ] **未进入重放** — `confirm_src / lag_conf / epsilon_conf` 的既有代码未改，但不存在合法新左端结果可验证新旧隔离横比。

### B. L1→L2 漏斗

- [ ] **未横比** — 旧基线 3 / 13 / 22 已保留为参照；能力门先于基线复跑失败，未声明基线稳定或漂移。
- [ ] **BLOCKED** — 新旧候选数、相邻边数、完整证书数及差分不可生成。
- [ ] **BLOCKED** — 无合法新增边，不能打印要求的七类字段。
- [ ] **BLOCKED** — 缺少子对象属于父级完整 `c_p` 的递归结构证明能力。

### C. 三个预注册近失样本

- [x] **已保持身份** — 三个旧父窗/子对象逐字保留，未换排序。
- [ ] **BLOCKED** — 新 `c_start_full`、有符号差及 Sub 左右界不可计算。
- [ ] **未触发** — 无样本获得合法通过结果，不能标 `FALSIFIED`。
- [ ] **未触发** — 无样本获得合法不通过结果，不能标 `SUPPORTED-ON-THIS-REPLAY`。

### D. 最终判定

- [ ] `PASS`
- [ ] `FAIL-DEFINITION-MAPPING`
- [ ] `FAIL-EVIDENCE`
- [x] **`BLOCKED-CAPABILITY`** — 现有字段/对象图不能表达完整 `c_p` 及其递归归属证书；没有使用局部 episode、确认点或产量锚点代填。

## 6. 解除阻断所需的最小能力（非本轮实装）

后续若要重启 #43，生产路径至少必须以可追溯对象而非数值猜测提供：父事件所归属的最后同级别 `B_p` 身份；完整 `c_p` 的结构身份与 source-index 首尾；`c_p` 内离开/回试构成的第三类结构及其对 `B_p` 的引用；以及 `CandDeltaEvent -> c_p` 的确定性归属边。只有这些字段能从递归分解产生并通过事件级证书校验，才可计算 `c_start_full` 并恢复 §7.B/§7.C 重放。
