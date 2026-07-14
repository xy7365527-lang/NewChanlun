# P45：父级完整 `c_p` 归属证书能力实装（2026-07-12）

- 分支：`p0-replay-dparent`
- 基线 HEAD：`99ce554b8a6bfbe2e5ce601585192616b1fc3d8c`
- 权威：`p43-replay-cfull-20260712.md` §6；`dparent-leftend-p0-review-20260711.md` §2/§3
- 范围：只实装 θ_v6 递归塔生产路径的 `B_p / c_p / 第三类 / CandDeltaEvent→c_p` 能力。
- 明确未做：#43 正式重放、旧基线横比、三个预注册样本裁定、冻结/裁决文档回写。

## 1. 结论

`BLOCKED-CAPABILITY` 的对象表达缺口已解除：递归中枢扫描现产出并持久保存与每个中枢 1:1 的 `CpScanOwnership`；`CandDeltaEvent` 可同时承载 `B_p` 身份、完整 `c_p` 结构身份、`c_p` 内第三类离开/回试结构，以及事件到 `c_p` 的确定性归属边。

能力是严格部分函数：扫描尚无 non-extension 离开、事件前没有合法第三类结构、或不能证明 `end(c_p)` 时，相应字段保持 `None`。这不再是“对象无法表达”的全局阻断，而是逐事件可观察、可登记的结构证书未闭合。

## 2. 字段与结构图

```text
detect_centers_windowed_resume
  ├─ Center B_p
  ├─ WinMeta.win_exit ──────┐
  └─ input LeveledMove IDs  │
                            ▼
                    CpScanOwnership       （与 Center/upper_move 1:1）
                    ├─ b_center_index
                    ├─ b_center_id + B 坐标/核心
                    ├─ departure_move_id
                    └─ departure_interval = c_start_full 的结构来源
                            │
                            ▼
level_cand_delta ── judge_third_cert（复用既有第三类唯一判据）
  └─ CandDeltaEvent
     ├─ c_episode_start / c_episode_interval
     ├─ b_parent: ParentCenterIdentity
     ├─ c_structure: CpStructureIdentity
     │  └─ [departure_move_id .. terminal_move_id] + source 首尾
     ├─ third_class_in_c: ThirdClassInCp
     │  └─ leave/retest move IDs + 区间 + B/c 引用
     ├─ cp_ownership: CandDeltaCpEdge
     └─ c_interval_full: Option<(start(c_p), end(c_p))>
```

生产字段：

| 对象 | 新结构/字段 | 确定性来源 |
|---|---|---|
| `B_p` | `ParentCenterIdentity { center_index, center_id, source_interval, zd, zg }` | 中枢输出序 + compose `ElementId` + `Center` 坐标 |
| 扫描侧车 | `LevelState.cp_ownership: Rc<Vec<CpScanOwnership>>` | `WinMeta.win_exit` 对应的首个 non-extension 递归单元 |
| 完整 `c_p` | `CpStructureIdentity { departure_move_id, terminal_move_id, source_start, source_end }` | 同级递归元素的连续 ID 首尾；右端仅在证书闭合时写入 |
| 第三类 | `ThirdClassInCp` | `signal::judge_third_cert`；与原 BSP 第三类判据同一函数体 |
| 事件归属边 | `CandDeltaCpEdge` / `CandDeltaEvent.cp_ownership` | `B_p ID + c 首尾递归元素 ID + source 首尾` |
| episode | `c_episode_start / c_episode_interval` | 原 `departure_move_c_start`，语义不变 |
| 完整区间 | `c_interval_full` | 仅完整结构证书闭合时为 `Some` |

`interval / enter_src` 为旧重放消费者保留的 episode 兼容别名，不再声明为规范 `D_parent`。`nest::d_parent_interval_full` 是完整区间的严格读取入口：无 `c_interval_full` 时返回 `None`，不回退 episode。

## 3. 硬约束逐条核对

1. **`c_start_full` 只由递归结构分解产生**：唯一来源是扫描时 `WinMeta.win_exit -> input LeveledMove` 保存的 `departure_interval.0`。未读取父 `A` 起点、整趋势起点、`confirm_src`、`until_start` 或产量锚点。
2. **离开单元被后续中枢吸收仍可追溯**：侧车在扫描产出时即写入，并在全量/增量的 pop、truncate、cascade、extend 路径与 `centers/upper_moves` 1:1 同步；不从最终 `MoveBlock` 反猜。
3. **episode 与完整 `c` 分离**：`departure_move_c_start` 函数体及语义未改；其值写入 `c_episode_start`/`c_episode_interval`。代码没有 `c_episode_start == c_start_full` 全局公理。
4. **第三类属于 `c_p`**：第三类证书保存 leave/retest 的递归元素 ID、source 区间、`B_p ID` 与 `c_p departure ID`；只有二者落在 `[start(c_p), end(c_p)]` 内才闭合事件证书。
5. **右端不回填**：只有 `cand_delta=true`、第三类证书存在、终端递归元素 ID 存在三者同时成立，才写 `source_end/c_interval_full/cp_ownership`；否则 `None`。没有用 `confirm_src` 回填。
6. **未切换参考实现**：未调用或接入 `level.rs::zhongshu_from_components / moves_from_level_zhongshus`；原 θ_v6 canonical scanner、塔、候选身份保持不变。
7. **未执行 #43 裁定**：独立冒烟不装配区间套、不横比旧基线、不读取或重判三个预注册样本。

## 4. 单元测试与回归

新增能力测试：

- `cp_parent_center_identity_locates_last_center_deterministically`：`B_p` 下标、确定性 ID、坐标首尾与核心。
- `cp_scan_sidecar_survives_departure_unit_absorbed_by_next_center`：离开单元成为下一中枢 seed 首单元后，前 `B` 侧车仍保留其 ID/区间。
- `cp_third_class_structure_references_b_and_lies_inside_c`：第三类 leave/retest 的递归 ID、区间及 `B/c` 引用。
- `cand_delta_event_edge_uniquely_identifies_full_cp`：事件边唯一定位 `B ID + c 起止递归 ID + source 首尾`。
- `cp_end_remains_none_without_third_class_proof`：缺第三类时禁止用 `seg.end/confirm_src` 回填。
- `full_d_parent_accessor_never_falls_back_to_episode`：完整父区间读取入口不回退 episode。

验证结果：

| 命令 | 结果 |
|---|---|
| `cargo check --all-targets` | PASS |
| `cargo test cp_ --lib` | 4 passed |
| `cargo test cand_delta_event_edge_uniquely_identifies_full_cp --lib` | 1 passed |
| `cargo test departure_move_c_start_reentry_bounds_episode --lib` | 1 passed |
| `cargo test -- --test-threads=1` | PASS；lib 1551 passed / 0 failed / 127 ignored；其余 bin/integration/doc tests 全通过 |

补充：第一次默认并行 `cargo test` 为 1549 passed / 1 failed / 127 ignored，唯一失败是既有 `opsem_dump_env_gated_bit_exact` 在并发临时目录中读到空 `trades.jsonl`；该项单独复跑通过，随后单线程全量通过。未修改该测试逻辑。

## 5. 100k 前缀能力冒烟

命令：

```text
CP_SMOKE_MAX_BARS=100000 cargo run --release --bin cp_capability_smoke
```

输入：既有 BTC 1m 重放数据前 100,000 bar。输出只取终态 `cand_delta=true` 父事件。

| level | confirm_src | `B_p`（index / ID / source interval） | `c_start_full` | `c_episode_start` | `c_end_full` | 关系 | 完整证书 |
|---:|---:|---|---:|---:|---:|:---:|:---:|
| 0 | 42704 | `73 / (L1,73) / [42118,42315]` | 42339 | 42503 | None | `<` | 否 |
| 0 | 42841 | `73 / (L1,73) / [42118,42315]` | 42339 | 42503 | 42841 | `<` | 是 |
| 0 | 42998 | `73 / (L1,73) / [42118,42315]` | 42339 | 42503 | 42998 | `<` | 是 |
| 0 | 46888 | `81 / (L1,81) / [46318,46813]` | 46813 | 46813 | None | `==` | 否 |
| 0 | 81187 | `147 / (L1,147) / [80607,80880]` | 80928 | 81012 | None | `<` | 否 |
| 0 | 84446 | `155 / (L1,155) / [83527,84283]` | 84283 | 84374 | None | `<` | 否 |
| 0 | 89295 | `169 / (L1,169) / [88990,89241]` | 89241 | 89267 | None | `<` | 否 |

分布：

- 父事件：7；`c_start_full` 可表达：7；缺失：0。
- `c_start_full == / < / > c_episode_start`：`1 / 6 / 0`。
- 完整 `B_p + c_p + 第三类 + 事件边` 证书：2。
- `c_end_full=None`：5；均未回填。

## 6. 遗留能力冲突与边界

1. 100k 冒烟的 5 个事件在各自确认时点之前没有可由现有第三类唯一判据证明的 `c_p` 内第三类结构。因此 `c_end_full/c_interval_full/cp_ownership=None`。这些事件仍是既有 `Cand^δ` 事件，但不能冒充完整 `c_p` 父对象；后续 #43 正式重放必须拒绝其作为完整父证书，而不是回填。
2. 历史 `nest::d_parent_interval` 与 `strict_nest_check` 仍保留 2026-07-10 episode 口径，避免本能力任务暗中执行 #43 正式重放。新严格入口 `d_parent_interval_full` 已提供；正式重放迁移应在 #43 单独任务中完成并按裁决 §7 验收。
3. `c_end_full` 当前的闭合证明域是“背驰确认事件 + 已保存第三类内含 + 终端同级递归元素 ID”。其它走势完成形态若未来需要纳入，必须新增独立结构证明；不得扩大本实现的 `Some` 域。

## 7. 能力判定

**CAPABILITY-IMPLEMENTED，EVENT-CERTIFICATE-PARTIAL。**

四类必需对象已进入 θ_v6 全量与增量生产路径并可被事件级校验，故 #43 不再因“字段/对象图完全不能表达 `c_p`”而全局 `BLOCKED-CAPABILITY`。逐事件证书未闭合仍是合法 `None`，并在正式重放前构成明确的过滤/报告条件；本报告不把它们裁定为 #43 的 PASS/FAIL。
