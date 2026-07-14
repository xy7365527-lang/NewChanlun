# C2 组装器生产消费层接线与例2严格重放（task #71）

- 日期：2026-07-14
- 工作区：隔离 worktree，基线 `d01d9b73ed66e9dd1757b6ca0f80182e2252d15b`
- 性质：生产消费层实现 + 只读调研 + 隔离重放
- 纪律：不修改生产主线，不修改既有裁决或谱系；本文只提出复核建议，不执行裁决

## 0. 结论先行

**建议维持 `p55-recursion-consistency-20260713.md` §5.3 对例2的“否”倾向，但必须修订其证据理由。**

1. #54 的 `L3#273..#277` 是可变长窗口/ownership 对象宇宙，不能搬进 C2 exact-three 主树当作同一对象。p55 自己也明示二者不可混算：外部只读材料 `p55-recursion-consistency-20260713.md:90-94,113-128`。
2. p55 原探针从 history-end 构塔后再按 `end_index<=judge_at` 过滤，仍泄漏了后来才冻结的对象身份。真实 raw-prefix 重放显示：
   - `judge_at=2906047` 时，目标 `L3#274 [2896743..2906047]` 尚不存在；同 ordinal 的开放对象只到 `2905681`；
   - `judge_at=2943378` 时，目标 `L3#277` 尚不存在；`L3#276` 仍是开放对象 `[2927357..2942773]`，不是后来的 `[2927357..2934387]`。
3. 在 #54 对象宇宙的真实前缀里，四种适配均没有得到“目标回抽段已完成 Move[3]”；在 exact-three 主树中按目标坐标外部投影，四种适配同样均未得到两个不同、相邻且 Completed 的 Move。
4. 因而两组 pair 都没有进入严格 C2 合格域。这里的“否”是 `NotEligible/Pending/Unassigned`，不是对 `firstRetrace` 几何或语义作了否定裁决。后续几何 `SUCCESS` 不能单凭 replay 成功推翻 firstness/identity 要求。

## 1. 材料完整性与对象宇宙

### 1.1 必读材料状态

当前 HEAD 只含 `chanlun/review-results/assembler-spec-20260712.md`。以下四份指定材料不在当前 worktree 的 HEAD/Git 对象中；本次从 `/Users/silencehan/Projects/NewChanlun/chanlun/review-results/` 的未跟踪文件只读恢复，未复制、未修改，引用时均标为“外部只读材料”：

- `c2-architecture-reassessment-20260713.md`
- `p55-recursion-consistency-20260713.md`
- `seed-failure-geometry-20260713.md`
- `adopt-overlay-spec-20260713.md`

这四份材料给出的共同约束是：C2 seam 与划分策略独立版本化、A4 consumer-first、真实前缀可见域、零回退、同版本 CompletedFreeze，以及 seed 身份与 host membership 分离。锚：外部 `c2-architecture-reassessment-20260713.md:13-20,88-124,326-347,361-369,388-400`；外部 `seed-failure-geometry-20260713.md:12-16,196-212`；外部 `adopt-overlay-spec-20260713.md:57-84,127-165,266-312,394-406`。

### 1.2 两个对象宇宙，禁止混算

| 标签 | 构造口径 | 本报告用途 |
|---|---|---|
| `MainExactThreeV1` | #58/C2 的 exact-three `WindowUnit` seed 主树 | 严格 C2 CompletedMove 与 pair 资格 |
| `V54VariableWindowOwnership` | #54 的可变长窗口与 ownership 对象 | 保留 `L3#273..#277` 身份做敏感性/前缀核查 |

生产 API 把宇宙做成显式枚举；query 与 material 必须携同一宇宙标签且均为 exact-three。每个输入窗口还必须恰有三个 `sub_moves`，否则 fail closed：`rust/src/theta_v0/classifier/level_view.rs:19-26,74-100,162-219`。

当前 `d01d9b73ed` 的 `recursive_tower` 实码已经是 seed+extension 可变长扫描，并在总段数达到 9 时重切；compose 使用 `seed 三段 + 延伸段` 的完整切片：`rust/src/theta_v0/classifier/recursive_tower.rs:693-797,800-852`。因此，不能把当前塔输出口头标成旧 exact-three 主树。旧 exact-three 隔离重放使用 #58 原型提交 `d676feafb2` 的塔快照；本次生产 seam 对当前可变长输入明确拒绝，而不是暗中适配。

## 2. 生产消费层接线

### 2.1 唯一 seam 与版本元组

新增并公开：

```text
assemble_level_view(query, ExactThreeMaterialSnapshot)
```

`LevelViewQuery` pin：

```text
(level, coordinate_window, as_of, object_universe,
 assembly_seam_version, partition_policy_version, rule_version,
 direction_provider_version, divergence_pair_provider_version)
```

实现锚：

- 模块生产公开：`rust/src/theta_v0/classifier/mod.rs:71-75`；
- 版本与方向 provider：`rust/src/theta_v0/classifier/level_view.rs:19-69`；
- 四适配及同宇宙 ownership fallback：同文件 `:127-151`；
- 生产 seam、exact-three 门与显式 A/C provider 对齐：同文件 `:161-228`；
- 原 C2 组装、下层账本孤儿恢复、缺 A/C hook 时趋势 Pending：`rust/src/theta_v0/classifier/move_view.rs:222-257,275-406`；规范锚 `assembler-spec-20260712.md:54-73,88-164`。

塔、信号和 `judge_third_cert` 均未被改写；C2 是只读 consumer view，不反向写塔。

### 2.2 actual-prefix 硬门

`ExactThreeMaterialSnapshot.observed_at` 必须严格等于查询的 `as_of`。history-end 物料即使内部按 end 坐标过滤也不能进入 seam；这堵住“后来冻结的同 ordinal 对象身份”倒灌：`rust/src/theta_v0/classifier/level_view.rs:74-98,162-199`。防御性坐标可见域仍由 `move_view` 二次裁剪：`rust/src/theta_v0/classifier/move_view.rs:183-185,275-305`。

### 2.3 A4、shadow replay 与 CompletedFreeze

- `PartitionPolicyVersion` 独立于 C2 seam 与 `rule_version`；当前只开放 `GREEDY_V1`，未知 policy fail closed。
- `assess_shadow_replay` 只比较同宇宙、同 as-of、同 level、同固定规则/provider 元组的基线与候选；硬拒旧 membership 释放及 Completed 变化：`rust/src/theta_v0/classifier/level_view.rs:422-480`。
- `CompletedFreezeGuard` pin 完整 `ViewVersion` 与宇宙。首次 Completed 记录 `judge_at`、`entry_bar=judge_at+1`、shape、chain/membership；以后若缺失、降级、改 shape 或改 chain 一律拒绝：同文件 `:244-388`。
- 新完成对象只追加 `CompletedFreezeEvent`；重复观察不覆盖、不重复追加，违规轮次原子失败：同文件 `:289-388`。
- strict pair 必须精确命中两个不同、相邻且 Completed 的 Move：同文件 `:482-542`。

## 3. 例2严格 as-of 重放

### 3.1 输入

数据：`analysis/data_cache/btc_1m_full.json`（符号链接）。目标来自外部只读 p55 `:113-120`：

| pair | leave | retest | #54 原子 | judge_at |
|---|---|---|---|---:|
| 首对 | `L3#273 [2884261..2896661]` | `L3#274 [2896743..2906047]` | `RETEST_REENTERS` | 2906047 |
| 后续对 | `L3#276 [2927357..2934387]` | `L3#277 [2935555..2943378]` | `SUCCESS` | 2943378 |

每个 `judge_at=t` 均独立读取 raw bars `[0..=t]` 重构塔；没有从 history-end 塔反向过滤。两份隔离探针各双跑，stdout byte-identical。

### 3.2 #54 可变长/ownership 宇宙矩阵

本表保留 #54 对象身份，只回答“目标对象在该时点是否已存在/完成”；不把结果当 exact-three CompletedMove 真值。

| judge_at | 适配 | leave 的前缀 view | retest 的前缀 view | 目标 retest 已完成 Move[3] | strict C2 pair |
|---:|---|---|---|---|---|
| 2906047 | `ownership_fallback` | `g132 Consolidation/Completed [2884261..2896661]` | `g133 Consolidation/Pending [2896743..2905681]`，目标终点未形成 | 否 | 否 |
| 2906047 | `first_leaf` | `g136 Consolidation/Completed [2884261..2896661]` | `g137 Consolidation/Pending [2896743..2905681]`，目标终点未形成 | 否 | 否 |
| 2906047 | `first_last_envelope` | `g140 Undetermined/Pending [2864469..2896661]` | `g141 Consolidation/Pending [2896743..2905681]`，目标终点未形成 | 否 | 否 |
| 2906047 | `sequence_envelope` | `g132 Consolidation/Completed [2884261..2896661]` | `g133 Consolidation/Pending [2896743..2905681]`，目标终点未形成 | 否 | 否 |
| 2943378 | `ownership_fallback` | `g134 Consolidation/Pending [2927357..2942773]`，目标 leave 边界未冻结 | 目标 `L3#277` 不存在 | 否 | 否 |
| 2943378 | `first_leaf` | `g138 Undetermined/Pending [2906049..2942773]` | 目标 `L3#277` 不存在 | 否 | 否 |
| 2943378 | `first_last_envelope` | `g142 Consolidation/Pending [2927357..2942773]` | 目标 `L3#277` 不存在 | 否 | 否 |
| 2943378 | `sequence_envelope` | `g134 Consolidation/Pending [2927357..2942773]`，目标 leave 边界未冻结 | 目标 `L3#277` 不存在 | 否 | 否 |

原始锚：`/private/tmp/c71-p55-prefix-proof.log:1-10`，SHA-256 `1bcac5ca26304f51d75e69564fa8b47b6cd4d01c1da61adaa35d197d63b6aeb5`。旧 history-end-filter 结果见 `/private/tmp/c71-p55-asof-rerun-1.log:1-11`，SHA-256 `240f38c27a6511f6b81c8a6d8ca3839fa696a693ea9da1466c56d82aed5ee357`；它只能解释 p55 旧表，不能作为本次严格 as-of 结论。

### 3.3 exact-three 主树矩阵（坐标外部投影）

exact-three 主树在相同区域使用 `L3#634..#645`，不是 #54 的 `L3#273..#277`：

- `t=2906047`：可见 `L3#634 [2884261..2887580]`、`#635 [2887599..2890852]`、`#636 [2899772..2902514]`；
- `t=2943378`：相关可见对象为 `L3#641..#645`，边界见 `/private/tmp/c71-c58-exact-object-universe.log:304-316`。

对象证据锚：`/private/tmp/c71-c58-exact-object-universe.log:86-91,304-320,560-580`。因此下表是“用 #54 目标坐标查询 exact-three C2 view”的外部投影，不是 ID 转换。

| judge_at | 适配 | leave 投影 | retest 投影 | 目标 retest 已完成 Move[3] | strict C2 pair |
|---:|---|---|---|---|---|
| 2906047 | `ownership_fallback` | Unassigned | Unassigned | 否 | 否 |
| 2906047 | `first_leaf` | Unassigned | Unassigned | 否 | 否 |
| 2906047 | `first_last_envelope` | Unassigned | Unassigned | 否 | 否 |
| 2906047 | `sequence_envelope` | Unassigned | Unassigned | 否 | 否 |
| 2943378 | `ownership_fallback` | `g310 Undetermined/Pending [2921148..2937987]` | Unassigned | 否 | 否 |
| 2943378 | `first_leaf` | Unassigned | Unassigned | 否 | 否 |
| 2943378 | `first_last_envelope` | Unassigned | Unassigned | 否 | 否 |
| 2943378 | `sequence_envelope` | `g310 Undetermined/Pending [2921148..2937987]` | Unassigned | 否 | 否 |

说明：旧 exact-three 快照没有 #54 ownership API；生产 `ownership_fallback` 在本宇宙 ownership 缺失时按版本化规则退回 `sequence_envelope`，故对应行与 sequence 相同，而不是导入 #54 ownership。实现锚：`rust/src/theta_v0/classifier/level_view.rs:127-151`。

三种独立方向输出的原始锚：`/private/tmp/c71-c58-exact-c2-asof.log:1-8`，SHA-256 `231a1fff0c5b1fcfc50a6f6c5be700e1c674c442bc810c6fb206f9da56774308`。ownership 行由上述同宇宙 fallback 规则确定。

## 4. 无 repaint 门核验

| 门 | 状态 | 证据/边界 |
|---|---|---|
| 固定 `t,r` 的完整 view 不受未来 `H'` 影响 | PASS（fail-closed 接口） | material 必须实际观察于 `t`；history-end material 被拒。`level_view.rs:74-98,162-199,672-693`。真实数据也实证 history-end filter 与 raw-prefix 在两时点均不相等，见 §3.2。 |
| `entry_bar >= judge_at+1` | PASS（consumer event） | 首次 Completed 固定 `entry_bar=judge_at+1`；后续观察保持首次值，测试覆盖。`level_view.rs:250-275,733-771`。 |
| 旧行不覆盖，变化追加表达 | PASS（seam 内存事件）；持久化待接 | `CompletedFreezeEvent` 只追加，新观察不改写/重复追加；chain 变更原子拒绝。`level_view.rs:294-388,733-771`。尚未接项目持久化 event store。 |
| CompletedFreeze（附加第四门） | PASS | 同一完整 `ViewVersion`/宇宙下，Missing、Completed→Pending、shape/chain/membership 变化均拒绝。外部规范锚 `c2-architecture-reassessment-20260713.md:114-124`、`adopt-overlay-spec-20260713.md:266-284`。 |

## 5. 测试与回归

定向门：

```text
cargo test --release classifier::level_view --no-fail-fast
7 passed; 0 failed
```

覆盖：actual-prefix、对象宇宙隔离、exact-three 门、四方向 provider、孤儿恢复、缺 hook Pending、CompletedFreeze、append-only event、A4 shadow no-rollback、strict pair。

全量回归：

```text
cd rust && cargo test --release
exit=0
lib: 1578 passed; 0 failed; 127 ignored
其余 bin / integration / doc-test targets: 0 failed
```

`git diff --check` 同轮通过。仓库既有 warnings 不作为新增失败。`cargo fmt --check` 会被大量本任务外既有格式差异拦截，因此只对本次新增 Rust 文件执行了 `rustfmt`，未改写无关文件。

## 6. 遗留缺口

1. **当前 HEAD 的塔口径冲突。** `d01d9b73ed` 已非旧 exact-three 塔。生产 seam 选择诚实 fail closed；若要在当前塔上生产 CompletedMove，必须先由裁决明确“扩展窗口如何投影为 immutable exact-three seed”，不能在本任务暗设。
2. **A/C 自动配对仍缺。** 没有显式 provider 时 Trend 只能 `Pending(MissingDivergencePair)`，与 `assembler-spec-20260712.md:156-164` 一致。
3. **方向真值仍未裁决。** 四 provider 已独立版本化并并列重放；本任务不把任何一项升级为唯一方向定义。
4. **PartitionPolicy 候选尚未晋级。** API 与 shadow gate 已就位，但只有 `GREEDY_V1` 可用；替代 policy 仍须满足零回退、Completed 不降级、全边界 prefix equality 等门，外部 `c2-architecture-reassessment-20260713.md:388-400`。
5. **事件持久化 adapter 未接。** seam 内 append-only/CompletedFreeze 已实现；尚未写入项目正式 event store、缓存键或 migration/reopen reducer。
6. **四份必读材料未进入当前 worktree 权威链。** 本次只读恢复足以做调研，不足以把它们冒充 HEAD 内可复现输入；后续应由编排者决定是否归档，不在本任务代行。
7. **严格 C2 不等于 firstRetrace 裁决。** 本报告只判 pair 资格；Pending/Unassigned 不能被重述为几何“回试失败”，也不能据此结算 #56。

## 7. 复核建议（不执行裁决）

建议 #56 例2继续保持挂起，复核意见写为：

> 维持 p55 §5.3 的“否/未进入严格 C2 合格域”倾向；撤回其“history-end 对象按 end 截断即可代表 as-of”的证据方法。#54 两个目标 pair 在各自 raw-prefix 时点均没有已冻结的目标 retest CompletedMove；exact-three 主树又不存在可与 #54 ID 等同的 pair。故当前证据不能推翻 firstRetrace 口径，也不能把几何 `SUCCESS` 裁成严格 C2 反例。待 exact-three↔当前扩展塔的显式 projection、方向 provider 与 A/C hook 都版本化后，再以同一对象宇宙重开复核。
