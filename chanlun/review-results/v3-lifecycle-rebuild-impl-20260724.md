# V3 NestLifecycleBook 重建实装说明（issue #231，spec #232）

- 日期：2026-07-24
- 工位：实装 subagent（worktree `/tmp/kimi-nest-mainline`，分支 `kimi-nest-mainline-20260717`）
- 规格：`chanlun/review-results/spec-v3-lifecycle-rebuild-20260724.md`（spec #232，下称「spec」；ID-n 指 spec 节号）
- 票据：issue #231（map #221 子票；#229 定案【丢失】后的重建票）
- 纪律自检：**零 git mutation**（commit/branch/checkout/reset/stash 全禁，提交由主 session 验收后执行）；主仓 `/Users/silencehan/Projects/NewChanlun` 只读零写入；禁止 tracker 写入（未 comment、未关票）；v3 硬禁令合规（全部判据 = 确定性结构/力度谓词，无概率/统计推断、无回测验证、无 EMH；测试全为确定性合成序列，T5/T11/T14 用真实 provider 夹具）；implement skill 流程（TDD 接缝 → 常跑 typecheck → 末尾全量 → 两轴 code-review 收尾，commit 步骤按主 session 指令禁用）。

## 1. 改动面（恰好两处 + 本说明）

| 文件 | 改动 | 说明 |
|---|---|---|
| `rust/src/theta_v0/classifier/nest_lifecycle.rs` | **新建**（1804 行，含 T1–T8/T11/T14/T15 共 11 测试） | 唯一实装落点 |
| `rust/src/theta_v0/classifier/mod.rs` | +2 行（:76-77，注册注释 + `pub mod nest_lifecycle;`） | 模块注册恢复 |

`git diff HEAD --stat` = `mod.rs | 2 ++`（唯一 tracked diff）；`git status` 另列本说明与 spec 两个 untracked 文档。**既有行零改动**：level_view.rs / nest.rs / divergence.rs / signal.rs / bins / Cargo.toml 全不触碰——judge_at 一个 bit 不动、`divergence_confirmed` 布尔口径不动、N^δ 装配与 `d_parent_interval_snapshot/terminal` 不动（#64 §2(a) 消费边界零侵蚀，`git diff` 可证）。

新文件结构一句话：模块头 090 登记（四项强制 + 口径与契约 + 出切片项 + v3 合规）→ 身份键 `LifecycleKey`/白名单桥 `bridge_identity` → 三态/原因码/`ForceCheck`/`ForceEvidence` → 修订/`NestLifecycleEntry`（`push_revision` 计数留档同步）→ 观察契约（`PanLiveWindow`/`LifecycleObservation`/`ForceMaterial`/`RetrogradeRejection`）→ `NestLifecycleBook`（`advance` 八步推进 + `assert_invariants` 公开不变量 + 读面）→ `provide_pan_live_windows` 活窗产出机 → 测试族 11 个。

## 2. spec #232 对照（ID-1 ~ ID-7 逐条）

| ID | spec 要求 | 落实 | 证据 |
|---|---|---|---|
| ID-1 模块形状 | 新文件 + mod.rs 注册一行；既有行零改动；`LifecycleKey` 六元组 / `NestLifecycleEntry`（key/state/revision/五钟）/ `NestLifecycleBook(BTreeMap)` + `advance` + `assert_invariants` 公开 | **符合** | mod.rs:76-77；`LifecycleKey`（:79-95，键不含状态/钟/行进中区间——E2E §6.1:248）；book 内部 `BTreeMap`（:496-503）；范式锚照 spec（persistent.rs Pi 注册表 / recursive_tower `CpScanOwnership` book 内对象 + 显式推进 / p92 YieldBook first-write-wins——模块内 :490-495 注明复用出处） |
| ID-2 转移表 | ∅→Provisional（observed 只写一次）；自环 first_provable 只写一次（Unavailable 不写）；→Invalidated（Verified(false) 反超 / 身份消失，原因码入载荷，身份消失 invalidated 独立于 first_provable）；→Confirmed（first_provable ∧ 结构完成 ∧ 完成时复核仍弱）；终态吸收禁复活；完成但不背驰诚实滞留 Provisional | **符合** | `advance` 第 1/2/5/6/7/8 步（:560-746）；完成但不背驰 :715-717；T1/T6/T7 锁定 |
| ID-3 ForceCheck 三值化（#78 修复 1 全量） | `ForceCheck { Verified(bool), Unavailable(UnavailReason) }`；`MissingForceSeries / CoordinateMapFailed`；活窗现算三值化；事件通道恒 `Verified(divergence_confirmed)`；仅 `Verified(false)` 进 Invalidated(ForceOvertake)；`Unavailable` 只记 `ForceUnavailable` 注记 + `force_unavailable_at`，不判 Invalidated、不写 first_provable；等力亦失效（严格 <）登记模块头 + 测试子场景 | **符合** | 枚举 :173-196；`force` :407-433（事件通道恒 Verified :409；活窗三值化 :411-431，divergence.rs `segments_diverge_or` 单一引擎禁第二查法）；advance 第 5/6 步；模块头「等力亦失效」登记 + T2 等力子场景（等值三通道全假 ⟹ Invalidated） |
| ID-4 as_of 单调守卫（#78 修复 2 全量） | 倒退 prefix 显式拒绝（非静默吸收）；同 as_of 幂等零 delta；倒退不制造 IdentityVanished；observed_at 建仓写一次；终态吸收含桥匹配 | **符合** | advance 第 1 步桥匹配倒退拒绝（:576-583）+ 第 2 步守卫（:633-644，`RetrogradeRejection` 注记，零 revision、entry 零改动）；第 8 步 `last_as_of <= as_of` 过滤（:723-728）；T15 全项锁定 |
| ID-5 白名单工程桥（卡 §6.3 原样） | 除 seg_c 右端外全等判同身份 ⟹ `Supersedes`（链留痕、钟不动、无 Invalidated）；seg_a 改变 ⟹ 越界 ⟹ IdentityVanished；模块头标注禁入证书真值路径 | **符合**（一处字面张力登记见 §5 Spec 轴 1） | `bridge_identity` :131-140；advance 第 1 步迁移（唯一 remove 点，仅 Provisional 可达——终态已先被吸收）；T8 正负对照锁定；模块头 090 登记 1 |
| ID-6 消费边界（#64 裁定） | book 是观察记录：不进 `d_parent_interval_snapshot/terminal` 输入、不改 `divergence_confirmed` 布尔口径、N^δ 装配仍只消费已闭合 c_p；§5 字面矛盾按 §2(a) 执行（可查账、不开放消费）并登记 | **符合** | 本模块对既有代码零行改动（git diff 可证）；`consumable_closed` 只放 Confirmed（:533-538）；`lineage_nodes` 红线注释（:541-548）；模块头 090 登记 4 |
| ID-7 验收 | 全量 cargo test --lib 绿（既线 #110 失败除外）；T1–T8+T11+T14/T15 全绿；cargo check --lib 零新增警告；mod.rs 注册恢复；模块头登记齐全 | **符合** | §3 测试输出；§4 验收明细；模块头 090 登记 1-4 齐全 |

## 3. 测试输出（TDD：红 → 绿）

**红阶段**（测试先行：完整测试族 + API 骨架，`advance`/`provide_pan_live_windows` 等 6 处 `unimplemented!()`）：

```text
test result: FAILED. 0 passed; 11 failed; 0 ignored; 0 measured; 1935 filtered out; finished in 0.00s
# 11 个测试全部 panic 于 unimplemented（nest_lifecycle.rs:501 advance / :539 provide_pan_live_windows）
```

**绿阶段**（实装函数体后，`cargo test --lib nest_lifecycle`）：

```text
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 1935 filtered out; finished in 0.00s
```

**全量**（`cargo test --lib` 尾行）：

```text
test result: FAILED. 1813 passed; 1 failed; 132 ignored; 0 measured; 0 filtered out; finished in 6.40s
```

- 基线（本票开工前同命令）：`1802 passed; 1 failed; 132 ignored`——本票 +11 测试全绿，**零既有测试变红**。
- 唯一失败 = 既线 `theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`（#110 线在案，left/right 数值与基线逐字一致 16618955402698307653 vs 10432481772907336594）——**勿修勿归因**，与本票无关。

**警告**（`cargo check --lib`）：全 crate 33 条 warning，全部位于既有文件（含 mod.rs:1740 `unused_mut`，系 +2 行注册导致的行号漂移——1738 处的既有增量失效代码）；**nest_lifecycle.rs 与注册行零警告 = 零新增警告**。

## 4. 验收明细（spec Testing Decisions 逐条）

| 测试 | spec 要求 | 实装与断言 | 结果 |
|---|---|---|---|
| T1 | trend 反超可触发（合成保留，标注真实不可达） | observed=149 →（收束 Supersedes）first_provable=155（t*）→（终假回扩）invalidated=169/ForceOvertake + ForceEvidence 载荷（60→150/5→6/2→3）→ t=179 终态吸收零输出；测试注释标注「合成路径，真实不可达（勘误 20260721）」 | ok |
| T2 | pan 活窗反超 + 等力边界 | as_of=129 三通道成立（first_provable=129）→ as_of=139 三通道全假 ⟹ Invalidated；等力子场景：三通道恰好等值（20==20、2.0==2.0、5.0==5.0）严格 < 全不成立 ⟹ 等力亦失效（卡 §4.3 口径登记） | ok |
| T3 | 单调不复活 | 谓词层：翻假后延展窗灌弱柱 `segments_diverge_or` 仍假（可执行证伪器）；状态机层：延展窗与构造性「复活」弱窗（右端回缩）均桥匹配到终态 ⟹ 吸收，零新 revision、revision 数零增长 | ok |
| T4 | 钟不变量 | observed/first_provable 写入不后移（125 写入后 130/135 桥迁移不变）；同 as_of 重复 advance 幂等零 delta；`assert_invariants` 公开调用全列通过 | ok |
| T5 | bit-exact 护栏（PartialEq + Debug 双判） | 真实夹具（复刻 level_view extended_windows + R1 全合取 MACD + retest 块满足 structural_pair_span）：挂/不挂 book 重跑 provider 输出 PartialEq 相等 + `format!("{:?}")` 逐字节相等；advance 后入参事件流逐字节不变 | ok |
| T6 | 身份消失（与 ForceOvertake 可区分） | pan 窄锚 seg_a=(50,69) → A′ 回退 seg_a=(20,39) 切换 ⟹ 旧 key Invalidated(IdentityVanished)@99、entry 保留（禁删除）、`force_evidence` 恒 None、原因码入 revision 载荷 | ok |
| T7 | 完成时复核（防「曾经弱过」） | (a) 完成窗仍弱 ⟹ Confirmed（structure_end=confirmed=139，first_provable=129 不后移）；(b) 完成窗已反超 ⟹ Invalidated(ForceOvertake) 而非 Confirmed（confirmed_at=None，024:24 机械表达，E2E §4.1:151 禁项防护） | ok |
| T8 | trend 身份迁移豁免 | 收束 [120,149]→[120,155] 记 Supersedes（superseded_from 留痕、observed=149 钟不动、无 Invalidated）；负面对照 seg_a 改变 ⟹ 白名单不越界 ⟹ IdentityVanished | ok |
| T11 | pan 活窗真实反超（勘误真实通道，真实 provider 夹具，必过） | `pan_real_fixture`（真实 `center_block_kind` + `locate_pan_div_structure` 窄锚锚定 A=(50,59)/c_start=70 + `self_anchors`）：as_of=79 first_provable → as_of=99 活窗延展真实现算三通道全假 ⟹ Invalidated(ForceOvertake)；ForceEvidence（20→65、5→6、2→3）可查账；终态留档不可消费；as_of=109 终态吸收（含桥）零输出 | ok |
| T14 | Unavailable 不判 Invalidated（#78 锚定） | 缺 hist/dif ⟹ ForceUnavailable(MissingForceSeries) 注记、无 Invalidated、不写 first_provable；同 as_of 注记幂等（revisions 不增长）；CoordinateMapFailed 子场景（close_src 截断）；数据补齐 + 结构完成 ⟹ 恢复推进至 Confirmed 可消费（无残留状态阻塞）；全程零 Invalidated | ok |
| T15 | 倒退显式拒绝（#78 锚定） | 倒退 as_of=90 喂同 key ⟹ 零 revision、entry 零改动（PartialEq 全等比对）、`RetrogradeRejection{last_as_of:100, rejected:90}` 注记在案；倒退空投喂不产生 IdentityVanished；同 as_of 幂等零 delta；合法前进照常（Supersedes，无新注记） | ok |

## 5. 两轴 code-review 结论（Standards + Spec #232 逐条，并行子代理静态评审）

**Standards 轴：通过，无硬违规。** 090 声明=能力（模块头四条登记与实装逐条相符）、v3 硬禁令（决策路径全确定性谓词、测试全确定性）、原则 8/14、与 persistent.rs/level_view.rs 风格范式一致。判断题 4 项，处置：(1) advance 内 revision 推送四步形状重复 ×8 —— **已修复**（提取 `NestLifecycleEntry::push_revision`，计数与留档同步单一来源；回归 11/11 绿 + 全量 1813/1/132 + 零新增警告）；(2) Side→u8 判别投影两处 —— **已修复**（共享 `side_tag`）；(3) `(usize,usize)` 段窗元组 Primitive Obsession —— 不改（与 `NestCandidateEvent` 既有口径同源，刻意保持）；(4) `revision` 计数为派生状态 —— 保留（卡 §2.3 字段表在案，`assert_invariants` 兜底一致性）。

**Spec 轴：ID-1~ID-7、US-01~US-07、T1–T8/T11/T14/T15、Out of Scope 主体全部落实**；出入 4 项，处置：
1. **ID-5「新 interval_b ⊆ 旧」字面张力**（spec:77 vs spec:28 三形态/T1 回扩/T3 延展）：spec 内部张力——右端若按字面只收不扩，则 spec 自己的 Solution 与 T1/T3 锚定自相矛盾。实装取三形态侧（右端双向），`bridge_identity` 注释登记，归编排者澄清（本票不替裁）。
2. **T11 夹具 spec:106 字面**「复刻 extended_windows + R1 全合取 + retest 块」：extended_windows 是 trend 夹具（实用于 T5）；T11 按 #78 核验锚名 `pan_real_fixture` 自构 pan 结构（真实 `locate_pan_div_structure`/`center_block_kind`/`segments_diverge_or`，非纯 mock）——满足 issue #231「真实 provider 夹具」口径与 #78 锚定，spec 字面系夹具风格泛指。如实登记。
3. **`provide_pan_live_windows` pub 可见性**（spec 轴建议降 pub(crate) 或登记）：取**登记**分支——该函数是交付「消费契约」的喂入参照实装；`pub(crate)` 在非 test 构建无调用方会触发 dead_code 警告，违反零新增警告线。函数文档已登记可见性理由。
4. **Unavailable ∧ structure_completed 同 prefix 时 StructureCompleted 留痕缺失**（语义边缘，spec 未明文）：行为保持 #78 核验原语义（Unavailable 分支「entry 保持原态、continue 不进第 7 步」——重建忠实度）；`StructureCompleted` 文档注释已收窄为该语义（数据补齐后重发信号时留痕），此处如实登记为 spec 外发现。

无 HIGH/Critical 项；已修复 2 项（Standards 异味 1/2）回归证据见 §3 尾行（修复后重跑：模块 11/11 绿、全量 1813 passed/1 failed（既线 #110）/132 ignored、`cargo check --lib` 33 警告零新增）。

## 6. 边界与诚实登记（090：声明 = 能力）

1. **模块头四项强制登记齐全**：①白名单工程桥限制（工程桥非 E2E WireV1 EventKey，并轨前不得进入证书真值路径——卡 §6.3/§9、裁定 #64 §2(c)）；②Unresolved 出切片（卡 §2.4，三态链非 E2E-D5 全四态）；③trend T1 合成保留不代表真实可达（勘误 20260721，真实通道 = pan 活窗 T11）；④#64 §5 验收线字面矛盾登记（按 §2(a) 执行：可查账、不开放消费，归编排者澄清）。
2. **交付物 = 状态机核心 + 消费契约**，不是生产 bin 接线（卡 §6.2 prefix 循环投产出切片）：活窗由调用方按 `PanLiveWindow` 契约喂入（`provide_pan_live_windows` 为参照实装），trend 结构完成信号经 `structure_completed` 通道喂入（判据属调用方）。
3. **出切片项维持**（卡 §9 原样，零夹带）：Unresolved 四态、WireV1 全量 EventKey/StateKey/修订链、谱系两钟 opened/closed、跨级证伪（043:30）、024:28 面积乘 2 外推、postcondition 诊断钟、DeferOrphan 重判、Lean 侧 ActiveTail↔OpenTailSystem 桥、白名单桥与两元锚（`NestCandidateEventExt.extreme_price/group_anchor`，#110/#206 线已入库）并轨。
4. **feed 契约**（模块头明载）：feed-every-prefix、同一身份每 prefix 至多一只观察、同身份 c 窗左端不动右端单调延展；跳 prefix 回填可构造 first_provable < observed 的越约输入，出切片（T8 注释明载）。
5. **#43/#64 边界零侵蚀**：N^δ 装配仍只消费已闭合完整 c_p（nest.rs:802-810/:987-989 未触碰）；`judge_at` 字段/写入点/回填/CERT 主键/D3 统计逐 bit 不动；本 book 不进 `d_parent_interval_snapshot/terminal` 输入。
6. **未验证项（如实）**：①`cargo test --release --lib` 未跑（spec ID-7 验收线为 debug 全量，已跑并留档；release 构建未验证）；②T5 护栏证明的是「advance 不反流改 provider 输出」的结构性事实，非未来改动的永久保险；③spec 外发现三项（§5 Spec 轴 1/2/4）已登记未自裁——ID-5 字面张力、T11 夹具字面、Unavailable∧completed 留痕语义，均归编排者澄清。
7. **与原实装的可核验性边界**：原 nest_lifecycle.rs（1283 行 + #64 续作至 2326 行）全库无副本，本重建按 spec/卡/勘误/#78 核验的语义锚逐条复建；与原文件的逐行一致性强声明不可证（无 diff 对象），声明 = 「规格语义全符合 + 测试族全绿 + 边界零侵蚀」，不声明「与丢失文件逐行相同」。
