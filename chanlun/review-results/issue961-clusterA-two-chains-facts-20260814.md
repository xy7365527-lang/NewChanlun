# #961 簇 A 两套跨级确认链代码级事实调查

- 日期：2026-08-14
- 执行：子代理（代码级事实调查，不做裁定）
- HEAD：main @ `9fc5570d0c`（`chore: roster 2026-08-14`）
- 方法：静态读（grep / sed 逐条自证行号），**未跑 cargo build**（参照 #849 先例；承重断言 = 行号与引用逐条 grep 核实）
- 产出用途：供 #961 簇 A 裁定（退役时点与唯一链定谁）作事实底座；本文件只陈述「代码里有什么、谁消费谁、开关在哪」，不回答「该留哪套」

## 一、簇 A 的问题（#961 原文）

> **簇 A**：两套跨级确认链并存生产（#106 身份桥 vs #529 TowerChainCertificate），退役时点与唯一链定谁（乙层雾「证书链归属」随迁）。

代码级的事实校正（本调查首条结论）：**#106 线的「身份桥」已全部退役**（T4 #173 进场侧四件 + T5b #208 出场侧四件，「终态无桥达成」）。#106 线现役的产物是**严格链**（`NestChainGate::chain_lookup`），不是桥。故「两套跨级确认链」的代码指涉是：

1. **链 1（#106 线现役）**：`backtest/admission.rs` 的 `NestChainGate` 严格链（chain_lookup）——π 进出场准入门的唯一 nest 判定源（env 开关，默认关）。
2. **链 2（#529 线现役）**：`classifier/chain_cert/` 的 `TowerChainCertificate`（+ `bsp_bridge` 的 `BspBridgeEdge`）——塔内原生链谱系，纯产出零消费（N7 Consume_at 未实装）。

另须防误读：`classifier/nest.rs` 的 `NestCertificate`（GUARD-ROLE: nest-pipeline，ADR-0005 对照臂）是链 1 **内部的上游件**（其证书装配/判定核被 `nest_index.rs` 消费，即链 1 自身的谓词来源），不是第三套并存链——与簇 A 的「两套」不是同一划分轴。

## 二、链 1：#106 线现役产物（NestChainGate 严格链）

### 2.1 票链沿革（行号核对自 ticket 描述与代码注释）

#106（map）→ T1 #170 键域重锚（三元→T5a 两元锚）→ T2 #171 投影层三元锚索引 → T3 #172 严格链判定（typed_lookup_multi 重写为 chain_lookup）→ T4 #173 旧桥退役（进场侧）→ T5a #207 方向退役（键域去方向位）→ T5b #208 出场侧迁链 + 旧出场桥删除。终态：#106 自述「进场侧无桥 + 出场侧迁链后无桥」。

### 2.2 构件清单（文件:行号，全部 grep 自证）

| 构件 | 位置 | 角色 |
|---|---|---|
| `nest_cert_gate_enabled()` | `backtest/admission.rs:62` | 运行层开关：`THETA_NEST_CERT_GATE=1` |
| `chain_driven_level_projection()` | `admission.rs:80` | T3 并门：链活⟹投影层必载（config 派生单点） |
| `NestGateStats` | `admission.rs:101` | 观测统计（不进判定） |
| `exit_candidate_would_close()` | `admission.rs:228` | 出场归因只读 helper |
| `nest_gate_admit()` | `admission.rs:261` | L2 旧臂双读对照（`build_gate_certificate`） |
| `NestGateObs` | `admission.rs:357` | 链观测（三态+谱系+对照格） |
| `ChainLevelStatus` / `ChainGapKind` / `ChainLevelGenealogy` / `ChainVerdict` | `admission.rs:427/443/455/474` | 链谱系逐级状态与缺/断极性 |
| `ChainProbe` | `admission.rs:486` | chain_lookup 返回体 |
| `NestChainGate` | `admission.rs:536` | 链门本体（结构体） |
| `NestChainGate::new` | `admission.rs:597` | 门开一次性构建（O(n)） |
| `sync_events` | `admission.rs:669` | 每 bar 增量喂法（值指纹跳过未变级） |
| `derive_level_events` | `admission.rs:708` | 级别事件派生（p92 `collect_snapshot_candidates` 同族骨架） |
| `absorb_exts` | `admission.rs:794` | first-wins 追加 + `by_triple_anchor` 键域登记 |
| `sync_index` | `admission.rs:831` | 索引按需重建（懒，决策逐字节不变） |
| `resolve_foot` | `admission.rs:880` | 脚身份解析：(极值价, 组锚) 两元锚 |
| `chain_key_hint` | `admission.rs:921` | 索引重建唯一前置（保守超集） |
| **`chain_lookup`** | `admission.rs:950` | **严格链查询（判定本体）** |
| `admit` / `admit_inner` | `admission.rs:1101/1112` | 门开单候选裁决（三态→放行/拒/Xzd 回退） |
| `t5a_chain_dump` | `admission.rs:1426`（cfg(test)） | shadow dump 装置 |
| `NestCertificateIndex` / `build_nest_certificate_index` | `classifier/nest_index.rs`（317 行全文件） | 事件→typed 证书索引（唯一读者 = chain_lookup） |
| `TypedNestCertificate` / `n_delta()` / `assemble_typed_certificate` / `terminal_bits_at_event_measured` | `classifier/nest.rs:433/335/894/850` | 证书装配 + 判定谓词单一来源 |
| `LevelProjectionLayer` / `triple_anchor_index` / `from_level` / `cross_level_query` / `entry_at` / `anchor_resolver` | `classifier/projection.rs:111/131/144/192/214/34` | T2 层索引（层间联动基座） |
| `NestCandidateEventExt` / `provide_nest_candidate_events_ext` | `classifier/level_view/pan_provider.rs:96/186` | 事件 provider（携两元锚） |
| `build_gate_certificate` / `build_xzd_fallback` | `backtest/econ_positive.rs:1732/1770` | L2 旧臂（对照）+ Xzd 回退单一来源 |
| gate 接线 | `backtest/fill.rs:4473`（nest_gate_hist）、`:4489`（NestChainGate::new）、`:4898-4950`（filter）、`:6189`（stats 输出） | π fill loop 消费点 |
| env 登记 | `env_registry.rs:77,212`（语义：「π 开仓准入区间套证书门」；default_arm：「未设/非"1" ⟹ π 门整体跳过（全路径 bit-exact 不变）」） | 46 键注册表（#746） |

### 2.3 判定语义（chain_lookup 四步，admission.rs:950 起文档逐字）

1. 脚身份 = (极值价, 组锚 a*)（`resolve_foot`，**不携带方向**——T5a，ADR 20260723 裁定 1）；
2. 恰好存在扫描：逐级查 T2 层 `cross_level_query(极值价)`，链顶 = x 为拐点的最高账本级；
3. 链区间 [L0, 链顶] 逐级查 `by_triple_anchor`(事件级=账本级+1, 价, 锚) → 证书身份集 → 因果守卫（`judge_at ≤ anchor`，越界整证剔除）→ 级内 T7 合并 `n_delta()`（nest.rs 递归核，判定谓词唯一来源）;
4. 三态：全闭合=Pass；≥1 闭合但有缺/断=Reject；零闭合/存在性全无/锚不可解=NoChain（⟹ Xzd 回退通道，复用 L2 旧臂 `build_xzd_fallback` 单一来源）。

### 2.4 开关状态（簇 A 裁定的关键事实）

- **编译层**：`backtest` 模块整体 `#[cfg(any(test, feature = "backtest_bin"))]`（`theta_v0/mod.rs:110-111`）。默认 cdylib（Python 扩展）构建两者皆不开 ⟹ **链 1 不进默认生产构建**；`theta_backtest` CLI（backtest_bin feature）才含。nautilus 实盘路径够不着（模块门控 + `NestChainGate` 为 `pub(super)`）——即 #961 卡口④「nautilus 够不着门（pub(super)）」的代码事实。
- **运行层**：`THETA_NEST_CERT_GATE=1` 才激活（admission.rs:62-71）；**默认关**（`strategy/level_clock.rs:61` 明文「THETA_NEST_CERT_GATE 默认关」）。门关 ⟹ `nest_gate_hist=None` ⟹ filter 整体跳过，全路径 bit-exact 不变（fill.rs:4473-4492）。
- **门开时的覆盖面**：fill.rs:4908-4950 的 filter 作用于 `step_gamma_trade` 全部候选（进出场同一 filter 点）——T5b 的「出场迁链」实现形态 = 出场候选与进场候选同经 `gate.admit`（chain_lookup），另以 `observe_exit_candidate` 归因。门关时（默认生产路径），进出场共同的证书门 = fold 规则2 消费的本地 `nest_confirmed`（装配层 `strategy/interp.rs:569` 逐候选设位，`nest::chi_bool` **本级单级**确认，非跨级；`interp.rs:1541` 未确认者归 record、不能落规则2/3），跨级链不参与。
- **门开的在案运行记录**：treasury verify 臂（`THETA_NEST_CERT_GATE=1` + `VOICE_EXEC=1`，`chanlun/review-results/treasury-reverify-20260727.md`）；wf7/wf8 链读数见 `chanlun/review-results/chain-attribution-current-caliber-20260725.md`（表 A：pass ~1.9%/2.2%，reject ~5.8%/2.2%，no_chain ~92.3%/95.6%）。

### 2.5 数据来源链

tower 窗口（`tower_i` + `confirmed_lens`）→ `derive_level_events`（level_view provider 族）→ `NestCandidateEventExt`（provider 构造点携两元锚，禁第二查法）→ `absorb_exts`（`by_triple_anchor` + `events_by_level` + `anchor_by_id`）→ `sync_index`（`build_nest_certificate_index`，生产区间口径 B / 终端背书 CWindow）→ `chain_lookup` 查表判定。上游结构事实 = 递归塔 + `Classification`；本链不自造结构判据。

## 三、链 2：#529 线现役产物（TowerChainCertificate + BspBridgeEdge）

### 3.1 票链沿革

map #529（塔内原生化长线，长驻台账）→ N0 #537（Cand/Sub 边语义）→ N1 #540/#550/#551（候选事件对象）→ N2 #544/#552（C⊆C 谓词 `candidate_is_sub`）→ **N3 #636/#641（TowerChainCertificate）** → **N4 #666/#668（BspBridgeEdge）** → N5 #669/#671（钟口径）→ N6 #778（Sel_Θ，留雾 frontier）→ **N7 Consume_at 未实装（留雾）**。子票完成度 42/45。

### 3.2 构件清单

| 构件 | 位置 | 角色 |
|---|---|---|
| `CandidateKey` / `CandidateEvent` / `CANDIDATE_RULE_VERSION` | `classifier/cand_event/key.rs:40/143/21` | N1 候选事件身份与本体 |
| `CandidateEventBook` / `advance` / `streams` | `classifier/cand_event/book.rs:17/36/32` | 每级 append-only 修订簿 |
| `observations_for_level` | `classifier/cand_event/observe.rs`（`pub(crate)`） | 单次塔扫描→观察 |
| `candidate_is_sub` | `classifier/cand_sub.rs:38` | N2：C⊆C 跨级区间包含谓词 |
| `ChainStatus` / `ChainKey` / `TowerChainCertificate` / `chain_paths` / `ChainCertificateBook` / `advance` / `summarize` / `digest` | `classifier/chain_cert/mod.rs:96/288/334/508/802/867/928/995` | N3：链证书塔对象（1199 行全模块） |
| `BspStructuralKey` / `BridgeKey` / `BspBridgeEdge` / `BspBridgeBook` / `edges_for_bsp_point` / `advance` | `classifier/bsp_bridge.rs:149/163/185/617/646/661` | N4：事件↔BSP 稳定身份边（724 行） |
| 产出点（生产侧） | `classifier/mod.rs:677`（逐级 `observations_for_level`）、`:733`（`candidate_book.advance`）、`:818`（`classify_with_tower_events`）、`:1907`（`classify_with_tower_events_incremental`） | 事件随 classify 内联产出（主路径内联） |
| 事件通道入口 | `backtest/incremental.rs:119`（`classify_at_events`）、`classifier/streaming.rs:92`（`append_bar_events`） | 三元通道（classification, tower, streams） |

### 3.3 判定语义（chain_cert/mod.rs 模块头文档逐字）

- 节点 = N1 候选事件（身份沿用 `CandidateKey`，模块不新造身份）；边 = C⊆C 闭区间包含谓词（`candidate_is_sub`，相切算包含，#246 全域）+ E2E-L 谱系；
- 边取包含序的**覆盖关系**（Hasse 边：`c ⊆ p` 且无存活 `m` 居中）——教义依据 027:46 逐级收缩；skip 边只在中间级真缺/真断时出现，`SkippedLevel` 逐级记数；
- 链 = 覆盖关系图上的**极大路径**（root 无存活父、leaf 无存活子）；单节点路径不是链；
- 终态三态：`Closed` = 链头 Confirmed + 不可再扩展 + 全链段谓词判过 + **至少一条有效链段**（#641 地板条款：链头独活永远 Open）；`Invalidated` = 谓词判不过或链头 Invalidated（成因两档，无连坐支）；否则 `Open`。终态不复活；
- 证伪节点不判死上级（留痕被跨过，链段改由两侧存活端点直接判）；
- 未做项照实：路径分叉不选（`selection_policy = null`，全分支各成一条链）；44 课小转大替代必要条件留 fog；floor 完整口径（FormalFloor/QuasiFloor/UnresolvedFloor）只落地「至少一条有效链段」一格。

### 3.4 消费状态（簇 A 裁定的关键事实）

**纯产出零消费**（`chain_cert/mod.rs` 头注自述：「不被 BSP / 级别形成 / 门 / admission / 订单路径调用，调用点 = 本模块单测 + 诊断 bin + p123 侧信道 dump（只写不判）」；`bsp_bridge.rs` 同：「零消费接线（裁定④）……消费并轨归 N7」）。

全部消费点（grep 全仓核实的闭合清单）：

- `bin/p123_fast_replay.rs:616`（`book.advance(&cache.candidate_streams(), as_of)`，只写不判侧信道）
- `bin/p127_skip_impact.rs:200,215`
- `bin/p129_44ke_strict_skip.rs:198`
- `bin/issue550_event_battery.rs:190,511,621`
- `bin/p126_d3_descending_clock.rs:236`（读 streams，不建链簿）
- `bin/p_issue668_bsp_bridge_battery.rs` / `bin/p_issue668_bsp_key_truth.rs`（N4 验收）
- `classifier/mod.rs` 单测（`chain_book_over_prefixes:5018` 等，真双路径锁）

生产路径零消费的构造性事实：

- π 回测 runner（fill.rs）逐 bar 走 `IncrementalClassifier::classify_at`（`backtest/incremental.rs:107`，**二元签名**，不含 streams）；`classify_at_events`（三元）全仓唯一调用点 = `issue550_event_battery.rs:505`；
- nautilus 实盘路径 `ThetaCore::plan_for_bar`（`nautilus/strategy.rs:87`）走 `OwnedIncrementalClassifier::append_bar`（二元），`append_bar_events` 无生产调用点；
- N7 Consume_at（含签名/类型冻结、LEE M3/M4 对接门、E2E-S8 基线）在 #529「Not yet specified」留雾。

### 3.5 编译状态

`classifier` 模块不受 backtest cfg 门控 ⟹ 链 2 的全部模块（cand_event/chain_cert/bsp_bridge/cand_sub）**编译进所有构建**（含默认 cdylib），与链 1 相反；但任何生产决策路径都不调用其推进/读出口。

## 四、两链并存的代码级事实（簇 A 裁定的直接输入）

| 维度 | 链 1（#106 线，NestChainGate） | 链 2（#529 线，TowerChainCertificate） |
|---|---|---|
| 事件族 | `level_view::NestCandidateEvent`（塔窗口派生，p92/p123 provider 同族；`NestInterval` 时间坐标） | `cand_event::CandidateEvent`（classify_impl 逐级扫描 + `signal::first_structural_gates` 结构门；`source_index` 坐标） |
| 证书对象 | `TypedNestCertificate`（nest.rs：rungs 逐级装配 + `n_delta` 递归核） | `TowerChainCertificate`（chain_cert：Hasse 覆盖边 + C⊆C 谓词 + 谱系三态） |
| 身份键 | `NestEventIdentity`（基例身份）+ `by_triple_anchor`：(ℓ, 极值价, 组锚@ℓ) 两元锚（T1/T5a） | `CandidateKey`（事件身份）+ `ChainKey`（路径身份，`extends` 簿内最长 proper-prefix） |
| 判定谓词 | 逐级 `n_delta()`（背驰确认递归核）+ 恰好存在层间联动 + 因果守卫 | `candidate_is_sub`（C⊆C 闭区间包含）+ 覆盖关系 + 链头 Confirmed |
| 链形态 | 查询式：以候选脚为底，[L0, 链顶] 逐级查证，三态裁决 | 谱系式：极大路径自根至叶，逐 as_of 推进，终态三态 |
| 生产角色 | **决策门**（env 开时进出场唯一 nest 判定源；默认关） | **纯产出**（零消费；N7 Consume_at 待接） |
| 编译面 | backtest cfg 门控（默认 cdylib 不含） | 无条件编译（默认 cdylib 含模块，无调用方） |
| 上游结构事实 | tower + Classification（门自持 provider 域派生） | classify_impl 内联产出的候选观察（与 BSP 共用结构门） |

互不调用：两链代码零相互引用（grep 核实：admission.rs 不 import chain_cert/cand_event/bsp_bridge；chain_cert 不 import backtest/admission/nest_index）。共同上游只有递归塔/Classification 的结构事实，无数据共享。

「并存生产」的精确含义（照实口径，不替裁定预答）：

- 链 1 = **生产决策链，但开关默认关**：π 的进出场准入门里唯一有跨级确认判定权的是它；`THETA_NEST_CERT_GATE` 未设时它一个 bit 都不参与。生产默认路径上的跨级确认 = 无（进出场只剩本地 `chi_bool` 单级 `nest_confirmed`，interp.rs:569）。
- 链 2 = **生产产出链，但零消费**：N1 候选事件随 classify 主路径内联产出（classify_impl:677/733，逐级 `observations_for_level` + `CandidateEventBook::advance`）；N3 链证书/N4 桥接边对象仅在**有人推进链簿/桥接簿时**产生（推进调用点 = 单测 + 诊断 bin，无生产调用点）。没有任何决策消费它们；消费并轨 = N7，未实装。
- 乙层图锚点：判定 13 准入门（χ/nest/k_Θ，现役落点 `theta_v0/`）= 链 1 的域；判定 10 区间套下钻（`.chanlun/definitions/qujiantao.md` #817）= 两链共同教义正本；「证书链归属」= 乙层雾条目，待本簇随迁。

在案运行读数（供退役时点裁定作量级参考，非本调查产出）：

- 链 1（门开，wf7/wf8，2026-07-25 口径）：pass 32/33（~2%），reject 100/34，no_chain 1592/1451（~92-95%）；T5a 后多级全链闭合首次非零（4 例 top=1）。
- 链 2（BTC 三窗，N3 收口读数）：链 18/89/293，skip 边 39-41%；N4 一类 29/29、二类结构性不可解 0/280。

## 五、边界与照实声明

- 未跑编译/测试（静态读，参照 #849 先例）；行号以本调查 HEAD `9fc5570d0c` 为准，main 前进后行号会漂。
- 本调查**不覆盖**（防越界）：`trading/`、`spiral/`、`fugue_v3/`、`recursive_t/` 四族收敛候选区的区间套/背驰变体（乙层图已单列，属簇 C 与其他线）；nest.rs `NestCertificate` 对照臂的身份归属（ADR-0005，链 1 上游件）；「唯一链定谁 / 退役时点 / N7 接链 2 时的桥接关系」= 留给 #961 裁定，本文件不预答。
- 本文件为工作草稿（`chanlun/review-results/`），活期绑定 #961，不具名分。
