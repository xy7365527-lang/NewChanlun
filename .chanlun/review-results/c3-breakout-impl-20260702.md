# C3 判据重设计实装：新中枢+突破方向（task #47，codex #44 终局裁定(c)）

**认识论等级**：L0（结构判据实装+单测）+ L2（350K bar 真实 BTC 命中率复测，产出否定性结果）。

## 结论

按 codex #44 终局裁定(c) 精确规格，在 `rust/src/theta_v0/backtest/econ_positive.rs` 实装「背驰后新中枢+反向突破」（第43课语义）判据，替换 #41 的 `same_side_same_center` 匹配口径：

- 新增 `XzdC3BreakoutDiag { new_center_exists, new_center_breakout_ok }` + `xzd_c3_new_center_breakout(source_index, confirm_index, side, sub_centers, sub_moves)`：
  新中枢 = `sub_centers` 中 `start_index >= source_index && end_index <= confirm_index`；突破 = 该中枢之后、confirm 之前的次级走势满足 `Side::Long ⟹ m.rmove.hi() > z.zg` / `Side::Short ⟹ m.rmove.lo() < z.zd`。
- `XzdEvidence` 新增 `c3_new_center_exists`/`c3_new_center_breakout_ok` 两字段；`xiaozhuanda_confirm` 新增 `sub_moves: &[LeveledMove]` 参数（调用侧 `build_gate_certificate` 内部从 `tower.get(lvl-1)` 取只读 slice，`checked_sub` 防 lvl==0 下溢，不引入缓存/索引结构，线性扫描）。
- `gate_pass()` 改为 `type2_confirmed && (level != 1 || c3_new_center_breakout_ok)`——**仅 level==1 收紧为 C2∧C3 硬门，level>=2 维持既定 C2-only**（本次翻案范围严格限于 level==1，未越权改动 lvl>=2）。
- 旧 4 项诊断字段（`sub_last_zs_type3`/`same_side_l0_type3_any`/`same_center_any`/`same_side_causal_ok`）+ `sub_bsp_type3_count` 全部原样保留，不参门。
- 报告标签由 `C2-only xzd` 改为 `C2+C3(breakout) xzd`。

单测新增 3 个（`xzd_c3_new_center_breakout_cases` 覆盖存在+突破/存在未突破/无新中枢/off-by-one `start_index==source_index` 四态；`xzd_gate_pass_level1_c3_breakout_hard_gate` 覆盖 level==1 硬门 vs level!=1 C2-only 两侧共 5 组断言）。`cargo test --release econ_positive`：**31 passed，0 failed，7 ignored**（含既有 Nest/bit-exact/P7/small-to-large 全部用例）。

## ⚠️ L2 复测触发裁定明文边界条件——不得静默接受，需 codex 复审

`ECON_L2_MAX_BARS=350000 cargo test --release acc_classification_level_hole_dx -- --ignored --nocapture` 真实 BTC 复测结果：

| 指标 | 值 |
|---|---|
| lvl==1 routed（路由到 Xzd） | 84 |
| c3_new_center_exists（新中枢存在，忽略 confirm 上界后仍验证为 0，见根因诊断） | 0/84 = **0.00%** |
| c3_new_center_breakout_ok（level==1 硬门参门项） | 0/84 = **0.00%** |
| level==1 通过门总数（新硬门下） | 0/84（**全部被拒**） |
| 小转大通道总通过（含 lvl>=2 C2-only） | 30 条（全部来自 lvl 2/3/4/5） |

裁定原文（task #47 边界条件）：「若 L2 复测显示 post-source 新中枢在模型中根本不可表达，应先修 center/tower 输出，而不是退回 (a)」+「若命中率仍为 0% 或 100% 不得静默接受，回报 Lead 转 codex 复审」——**本次复测命中此条款，命中率恰为 0%**。已在代码中把该检查固化为 `assert!`（`acc_classification_level_hole_dx` 内，触发时 panic 并打印 `breakout_ok=0/84`），防止未来静默通过。

### 根因诊断（已定位，非笼统"命中率为0"）

对 10 个采样 lvl==1 信号做临时诊断（诊断代码已移除，不留在最终 diff 中）：

| src(source_index) | confirm(confirm_index) | gap | sub_centers.len() | post_source（**去掉 confirm 上界后**，仅 `start_index>=source_index`）|
|---|---|---|---|---|
| 2365 | 2452 | 87 | 6 | 0 |
| 3745 | 3781 | 36 | 9 | 0 |
| 4878 | 4927 | 49 | 12 | 0 |
| 10450 | 10508 | 58 | 27 | 0 |
| 15461 | 15518 | 57 | 38 | 0 |
| 18167 | 18253 | 86 | 44 | 0 |
| 18089 | 18293 | 204 | 44 | 0 |
| 19984 | 20021 | 37 | 49 | 0 |
| 21712 | 21774 | 62 | 53 | 0 |

**关键排除项**：去掉 `end_index <= confirm_index` 上界、只保留 `start_index >= source_index` 后仍为 0/10——证明这**不是 confirm_index 窗口过窄的时序假象**（gap 已到 36~204 bar，sub_centers 已累积 6~53 个）。真实现象是：**截至采样的所有 lvl==1 Type2/3 信号，其 source_index 之后从未有任何 level-0 中枢的 `start_index` 落在 source_index 及之后**——即当前 tower/center 输出中，「背驰点之后新确认的次级中枢」这一结构在本窗口从未被观测到过。这与旧 C3（`last_zs_exists=84/84=100%`，用 `s.start_index~s.end_index` 整段走势窗口而非 `source_index~confirm_index` 窄窗）形成对照：旧口径窗口宽（覆盖整个 level-1 走势跨度，含 source_index 之前的旧中枢），新口径窗口严格限定"背驰点之后"——这正是 codex #44 裁定判据换代的本意（不复用"归属链"，改用"新形成"），但真实数据显示这一更严格的时间约束在 level==1 子集上目前恒假。

**不是实现 bug**：unit test 已钉住 off-by-one（`start_index==source_index` 边界含入）、突破几何（`hi>zg`/`lo<zd`）、三态穷举，`xzd_c3_new_center_breakout` 逐行对照 codex 规格实现，无逻辑偏差。已用「暂时中和 assert 后重跑」验证：除本条 assert 外，本测试其余全部断言（含 lvl>=2 死门真封 `sub_bsp_type3_count==0`、Nest depth 直方图、配对后 decomps 分布）**全部照常通过**，证明本次改动未破坏任何既有不变量，问题精确定位在「level==1 硬门在本数据窗恒假」这一个点。

## 定义依据

第43课「背驰后新中枢+反向突破」原文语义；codex #44 终局裁定(c)（`.chanlun/review-results/codex-decide-20260702-193853-5bbe.md`，judge_third 归属链 vs last_zs 选择链结构性不重合）；task #47 描述给出的函数签名/过滤条件/gate_pass 公式（照施工实现，未改设计）。

## 边界条件（会翻转结论的条件）

1. 若后续在**更长历史窗**（当前仅 350K/全量 461 万的 7.6%）或**跨标的**复测中，level==1 命中率转为非 0（哪怕个位数百分比），当前"全域 0%"结论需重新评估——但本次是首次真实数据复测，非 0% 无先验假设。
2. 若 codex/编排者裁定「新中枢」窗口锚点应改为 confirm_index 之后而非 source_index 之后（即"背驰确认后"而非"背驰点后"），本判据需重新实装——本次实装严格照 task #47 给定规格（`start_index >= source_index`），未预判此翻转。
3. 若 center/tower 输出本身被判定为「post-source 新中枢结构性不可表达」（根因诊断支持此假设），则修复方向在 center/tower 构造层，不在本次 gate_pass 逻辑层——这一判断需 codex 复审，本报告不越权代为裁定。

## 下游推论

- **task #13（W-VERIFY alpha 全量重测）当前仍被阻塞**：新 gate_pass 已实装，但 level==1 分支在真实数据下等价于「永久拒绝」，与 codex 裁定"若命中率退化需回到 codex 复审"直接冲突——**不能** 用当前状态作为 #13 的最终验收口径。
- level>=2 通道不受影响（维持 #41 的 C2-only，30 条通过门信号全部来自 lvl 2-5）。
- 若 codex 复审后维持当前"新中枢+突破"定义不变，则 level==1 的小转大通道在实践中等价于关闭（84/84 信号全部被拒）——这是否可接受是价值判断，需编排者/codex 裁定，非本报告可单方面决定。

## 谱系引用

- 606（区间套有效域=Type1）、673（Cand^δ 三分拆）、知识库 L410（小转大补充定位）。
- #41（C3 硬门退化：gate_pass 仅 C2 + 诊断字段降级）、#44（C3 center 匹配口径重设计裁决：判据换代动机）。
- genealogist 待评估：本次「新判据本身在 level==1 子集恒假」是否与 606/673 系列同属"level 塔空洞/判据错位"模式家族（team-lead 消息已预告此项，本报告的根因诊断数据可直接喂给该评估）。

## 影响声明

- 改动文件：仅 `rust/src/theta_v0/backtest/econ_positive.rs`（未碰 `recursive_tower.rs`/`signal.rs` 并发域）。
- 另修复 2 处因task #40（05c Rc化迁移）已落地但未及时同步的编译断裂（`cand_predicate.rs` 由该任务owner在本次会话中自行修复，`econ_positive.rs` 内 5 处 `subs: sub_rmoves` 测试夹具由本任务补 `Rc::new()` 包裹，无逻辑变更，纯类型适配）——`recursive_tower.rs` 本身未触碰。
- `gate_pass()` 语义变化：level==1 的 Type2/3 小转大信号门槛从「仅 C2」收紧为「C2∧C3(新中枢突破)」，**真实数据下此收紧在 350K 窗口内等价于全拒（0/84 通过）**——这是本次实装最重要的下游影响，已在上方"⚠️"章节详述，未静默接受，报告 Lead 转 codex 复审。
