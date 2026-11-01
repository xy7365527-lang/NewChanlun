# #1598 受影响代码清单（事实补件）

- **基线**：`origin/main = 13ed4d45c8ff85175819ca987690328ab2a247a4`。
- **范围**：A-01（δ/impact_form）、A-09（capacities.source/overflow_trigger）、A-10（pairing/second_layer.rule）、B-05（model_id/seed/capacity_source）。只列实际代码事实，不改裁定、不改代码。
- **分类**：**生产**＝当前生产/回放可读路径；**探针/golden**＝测量或独立测试证据，不能当生产实现；**旧臂**＝ADR 已要求收编/退役的入口；**未实现**＝未找到本票字段或消费点。
- **核对方式**：直接打开当前文件复核行号；相关调用关系另用 code graph 查询。主仓本地 HEAD 与 `origin/main` 在本清单承重文件上无内容差异；`tb03c_funding_tests.rs` 只核 `origin/main` 版本。

## 1. 总结与真实承接

| 条目 | 当前事实 | 真正下游 |
|---|---|---|
| A-01 δ / impact_form | 生产仍是 `depth_weights` 和 `/3` 等旧量纲；`impact_form` 未实现。`power(δ)`/ROUNDTRIP 只在探针与正本中明确。 | 新量档来源及消费归 **#1444**；旧入口终态归 **TB-10（#1385/#1386）**；探针窗口与不确定性带归 **#1444**。 |
| A-09(a) capacities.source | `tb03c-book/2` 已读 `capacities`，但只有 `level/C/source/valid_through` 四字段；`source` 目前只作非空文本检查。`ProbeAuto` 未实现。 | **#1406** 现有资金链，**#1444** 容量域、来源、窗口与准入。 |
| A-09(b) overflow_trigger | `tb03c_transfer` 接受外部 `trigger_evidence_ref`，不计算 units/capital/mark_value 触发；`overflow_trigger` 未实现。 | **#1446**（资金溢出换档）。不能拿 `growth_eligible` 当溢出触发器。 |
| A-10(a) pairing | `p907`/`p915` 有 ROUNDTRIP 测量；没有生产 `pairing` 字段。 | **#1441**（成本停止及证据）；旧成本门入口归 TB-10。 |
| A-10(b) second_layer.rule | 未找到 `DescentBottomEvidence`、`share_lb_95/share_ub_95`、`second_layer` 或 `share_threshold` 实现。 | **#1441**。 |
| B-05 model_id/seed/capacity_source | 有确定性回放与统计种子，但没有本票 `model_id`/`capacity_source` 字段；`bar_volume_derived`、`stochastic-microevent` 未实现。 | 历史模型与输入 **#1381**，适配器 **#1382**，双跑 **#1411**，报告分层 **#1418**；seek/修订 **#1399**。**#1445 不是本条的直接前置**。 |

## 2. Rust 实际清单

### A-01：δ / impact_form

| 文件:行 | 当前事实与名分 | 承接 |
|---|---|---|
| `rust/src/theta_v0/config.rs:167` | `VoiceConfig.depth_weights: Vec<f64>`，旧权值字段，不是 δ 或 `impact_form`。 | #1444 供新量档来源；TB-10 登记旧入口。 |
| `rust/src/theta_v0/config.rs:177` | 注释明确 `leg_target` 仍按 `depth_weight × dir_weight × w_grade` 计算。 | #1444 消费侧迁移；保留调用契约由实施票重审。 |
| `rust/src/theta_v0/config.rs:187` | 默认 `depth_weights = [0.60, 0.30, 0.10]`，旧固定值。 | TB-10 旧默认退出；#1444 负责记录化替代。 |
| `rust/src/theta_v0/config.rs:512` | 测试仍断言旧三值，是 **golden/回归夹具**，不是新记录。 | 实施时改为记录等值/变异对拍，不当作生产默认值。 |
| `rust/src/theta_v0/strategy/voice.rs:195-208` | `depth_weight()` 直接读取 `config.voice.depth_weights`；`depth=0` 走第一项，越界为 0。 | #1444 改读当前生效量档；TB-10 处理旧入口。 |
| `rust/src/theta_v0/strategy/coverage/leg.rs:148-163` | `leg_target()` 将 `depth_weight` 与方向、级别权重相乘后 `units: base_units * w`。这是生产尺寸消费点，不是 δ 公式本身。 | #1444 把权重来源改成分级记录；`base_units` 是否等于固定 Q 须在实施票先核。 |
| `rust/src/theta_v0/strategy/coverage/leg.rs:171-187` | 双段版本同样调用 `depth_weight`，不能只改上一处。 | 同上，必须一起改并做同输入对拍。 |
| `rust/src/theta_v0/strategy/oscillation_campaign.rs:312-313` | `reduce_units()` 仍执行 `cost_basis().units() / 3`，是固定 `/3` 旧行为。 | #1444 迁移到 `w(d) × Q`；TB-10 旧 `/3` 终态。 |
| `rust/src/bin/p915_capacity_frequency.rs:10-11` | 探针明确 `capacity` 与 `w(d)=(A_{ℓ−d}/A_ℓ)^(1/δ)`，不是生产 `impact_form` 字段。 | #1444 用作 Measured 来源；补窗口、n、最大月簇和不确定性带。 |
| `rust/src/bin/p915_capacity_frequency.rs:70-78` | 只读探针计算四种日波动估计，RMS 与中位日并列，不能由本票偷换成 `log` 冲击模型。 | #1444 保留证据语义；不要与 `impact_form=log` 混为一谈。 |
| `rust/src/bin/p915_capacity_frequency.rs:499-509` | FULL=[0,n)是汇总窗，H1=[0,n/2)、H2=[n/2,n)是两个互不重叠半窗；FULL包含两者，不能算三独立窗，也不满足三窗门。 | #1444仍须补足所引evidence_gate要求的独立窗口；FULL只能汇总，不代替逐窗判据。 |
| `rust/src/bin/p915_capacity_frequency.rs:543-580` | 生产探针代码实际按同级首个反向信号计算 ROUNDTRIP 波幅和 GAPBARS。 | #1444/#1441 共用测量出处；不是 `impact_form=log` 的实现。 |

**未实现**：`impact_form` 在 `rust/src`、`src`、`s_session`、`formal`、`tests`、`trading_system`、`scripts` 均无直接命中。`log` 仅作为候选与反面证据存在，不能把 `powf()` 或 `.ln()` 的其他用途当作本字段实现。

### A-09(a)：capacities.source / ProbeAuto

| 文件:行 | 当前事实与名分 | 承接 |
|---|---|---|
| `rust/src/economic_session/tb03c_funding.rs:23` | 只有政策族 `#1355/quantity-tier-policy/v1` 标识，没有 `ProbeAuto`。 | #1406/#1444 扩来源域。 |
| `rust/src/economic_session/tb03c_funding.rs:35-50` | `POLICY_FIELDS` 含 `tiers/capacities/frequency_evidence_ref`；`CAPACITY_FIELDS` 只有 `level/C/source/valid_through`，无 `domain{粒度,δ,x,Y,σ}`。 | #1444 扩充并验证记录形状；这不是 `impact_form` 或 `overflow_trigger` 的现成字段。 |
| `rust/src/economic_session/tb03c_funding.rs:125-136` | 本级 `capacity()` 查条目，`C=Unknown` 被排除在已签容量之外。 | #1406 保持 Unknown 非无限；#1444 负责来源与测量。 |
| `rust/src/economic_session/tb03c_funding.rs:203-250` | 校验政策族、profile、级别和容量条目；`source` 仅要求非空文本，没有校验 `UserSigned` 或 `ProbeAuto`。 | #1444 加来源语义与 domain/profile 一致性，不默认接受自动探针。 |
| `rust/src/economic_session/tb03c_funding.rs:350-379` | 开局真正执行 `Q ≤ C(ℓ)`，并输出 `policy_record` 与 `capacity`。 | #1406 保留唯一容量门；#1444 新增来源时不得绕开此门。 |
| `rust/src/economic_session/tb03c_funding.rs:389-407` | E 侧在开局缺容量时返回 `capacity` 等待，B/E 拒绝口径一致。 | #1406/#1444 保留 typed AwaitingPolicy。 |
| `rust/src/economic_session/tb03c_funding_tests.rs:43-52`（按 `origin/main`） | golden 记录为 L0=1110000、L1=3200000、L2=9630000、L3=Unknown，`source=UserSigned(#917)`/`none`。 | #1406/#1444 用作等值夹具，不是 ProbeAuto 正例。 |
| `s_session/tests/fixtures/tb03c_funding/funding_oracle.py:21-33` | 独立 Python golden 与 Rust 共用容量和 w(1) 数值。 | #1406/#1444 只作独立对拍，不是生产读取器。 |
| `s_session/tests/fixtures/tb03c_funding/funding_oracle.py:43-70` | `derive()` 在容量/频率未签时停止，满装后向上溢出；这是测试 oracle，不证明 `ProbeAuto` 可自动准入。 | #1444 核等值与溢出，不得改成自动签容量。 |

**未实现**：`ProbeAuto` 在生产代码无直接命中；当前 `source` 只是文本，没有自动探针准入或签署域绑定。`capacity` 在此是级别容量，不能与 B-05 的 `capacity_source` 混用。

### A-09(b)：overflow_trigger

| 文件:行 | 当前事实与名分 | 承接 |
|---|---|---|
| `rust/src/economic_session/tb03c_transfer.rs:142-177` | transfer 只核验外部 `growth_source`、`trigger_evidence_ref`、`authorization_ref`、`quantity_policy_ref`，不计算 units/capital/mark_value 触发量。 | #1446 负责真实溢出触发与证据；不是在这里直接塞字段名。 |
| `rust/src/economic_session/tb03c_transfer.rs:148-155` | 目前只接受 `tb03c-transfer/1` 的 TestOnly `capital_overflow` 金额片。 | #1446 扩大正式语义前，先明确测试与生产边界。 |
| `rust/src/economic_session/tb03c_c5_tests.rs:341-343` | 测试 `capital_overflow` 使用 `testonly-completed-split` 作为触发证据，是 golden，不是触发计算。 | #1446 用作等值夹具，补 units/capital/mark_value 各自负控。 |
| `rust/src/economic_session/tb03c_book.rs:283-303` | `growth_eligible` 只判断回本/费后实现是否具备扩量资格，**不是 overflow_trigger**。 | 不得用该函数冒充 units/capital/mark_value 三口径触发；#1446 分责。 |
| `rust/src/economic_session/tb03c_book.rs:699-709` | `/2` 的 q_rebase 仍要求新签，缺容量时返回 `AwaitingPolicy:capacity`。 | #1446 保留版本签署门，不自动改 Q。 |

**未实现**：`overflow_trigger` 无直接命中。当前 transfer 是“接收已存在的转移声明”，不是“根据 G 和 C(ℓ) 计算溢出”的触发器。`mark_value` 也没有现成消费点。

### A-10(a)：pairing / ROUNDTRIP

| 文件:行 | 当前事实与名分 | 承接 |
|---|---|---|
| `rust/src/bin/p907_cost_gate_level.rs:339-369` | 只读探针明确以同级最近反向信号构成 ROUNDTRIP，并输出波幅。 | #1441 作为成本测量来源；不是生产 `pairing` 字段。 |
| `rust/src/bin/p915_capacity_frequency.rs:543-580` | 同一 ROUNDTRIP 规则的另一只探针，带窗口、级别和 GAPBARS 输出。 | #1444/#1441 共用证据，避免两份算法漂移。 |
| `rust/src/theta_v0/venue_fee.rs:58-63` | 现执行按 bar close 成交且生产角色恒 Taker，是旧成本输入边界。 | #1441 的 `fee_profile` 引用来源，不用 Taker 常量代替费率簿。 |
| `rust/src/theta_v0/venue_fee.rs:112` | `PRODUCTION_LIQUIDITY_ROLE=Taker`。 | 同上，只是角色事实，不是成本记录或费率来源。 |
| `rust/src/theta_v0/venue_fee.rs:403-420` | 生产报价调用经该角色取费率，缺在册档时可走已声明 fallback。 | #1441 改从具名 `fee_schedule_ref` 读取并核缺项状态，不填默认。 |
| `rust/src/theta_v0/backtest/econ_positive.rs:823-825` | `cand_delta_base_gate` 是塔底候选基础门，不是 ROUNDTRIP 配对实现，也不是第二层序关系判据。 | #1441 只在成本门职责内评估，勿与 `second_layer.rule` 混合。 |
| `rust/src/recursive_t/divergence.rs:506-508` | 旧 `recursive_t` 的“成本门 ∧ 塔底”停止规则，是旧影子/退役域，不是新 second_layer。 | TB-10 终态；不得把它当作新规则已实现的证据。 |

### A-10(b)：second_layer.rule

- `rust/src`、`src`、`s_session`、`formal` 中均未找到 `DescentBottomEvidence`、`share_lb_95`、`share_ub_95`、`reached_bottom`、`divergence_not_judged`、`second_layer` 或 `share_threshold` 的直接实现。
- **承接**：#1441。需要新增第二层证据数据形状、读取和 `order_relation` 默认；`share_threshold{theta2}` 只能在前置完成并签署后启用，不能把 `p` 值、现有 cost gate 或塔底函数换名充当。
- `formal/Tlayers/Signal.lean:464-491` 虽有“配对结构”字样，但它是另一套信号定理，不是成本门 ROUNDTRIP 或 second_layer，不能作为实现证据。

### B-05：model_id / seed / capacity_source

| 文件:行 | 当前事实与名分 | 承接 |
|---|---|---|
| `rust/src/theta_v0/backtest/fill.rs:4777` | `pi_theta_fill_loop_overlay` 是旧模拟臂，不是 ADR 0033 的正式 `model_id`/`capacity_source` 合同。 | TB-10/C11 旧臂终态；正式历史模型归 #1381/#1382。 |
| `rust/src/theta_v0/backtest/fill.rs:4827,4858,5054-5055` | 旧臂仍持 `entry_cost` 和 `pending` Vec，按 `exec_index` 取出订单，不是“重间不排序”的新批模型。 | TB-10/C11 收编/退役，不能把 `pending` 遍历当 `intra_batch_order: Unknown` 的实现。 |
| `rust/src/theta_v0/strategy/exec.rs:145,160-175` | `ConflictKey` 是旧规划冲突/排序键，不是 N 重同 slot 并发模型。 | TB-10/C11；不得挪作新模型种子或容量函数。 |
| `rust/src/theta_v0/strategy/coverage/sizing.rs:538` | `schedule_order` 是单净单调度，不证明 N 重可并行，也没有 `capacity_source`。 | TB-10/C11；新模型归 #1381/#1382。 |
| `rust/src/theta_v0/backtest/replay_dump.rs:142-173` | 真实单次回放逐 bar 推流并输出 action/qty/exec_index；未读 `model_id`、`capacity_source` 或模型 seed。 | #1381 输入与回放；#1399 获知时间；#1418 报告溯源。 |
| `rust/src/theta_v0/backtest/theta_pi_diff.rs:371-385` | 有批量/stream 三面对拍与信号差异报告，是双跑比对证据，不含本票模型字段。 | #1411 双跑确定性与恢复；#1418 回归报告。 |
| `rust/src/theta_v0/backtest/perm_test.rs:768-781` | `PERM_XPROC_SEED` 是**置换检验**种子，供跨进程统计复现，不是 `history-sim/*` 的模型 seed。 | 不随 B-05 改写；若动它须另开统计口径票，不能混用。 |
| `rust/src/theta_v0/backtest/metrics.rs:154,166-173` | `PREREG_SEED=20260625` 是统计显著性检验的预注册种子，不是历史执行模型 seed。 | 同上，不属于 B-05 的 `model_id/seed`。 |
| `rust/src/lav_seal/bundle.rs:41`、`rust/src/lav_seal/samples.rs:209` | `model_id` 是 LLM bundle 身份，与历史交易模型无关。 | 排除同名异物，不能拿来证明 B-05 已有模型 id。 |

**未实现**：`capacity_source`、`scenario_contract`、`bar_volume_derived`、`stochastic-microevent`、`history-sim/bar-batch-known-frontier` 均无直接实现。`capacity` 在 tb03c funding 中是级别容量，不是本字段。

## 3. Lean 实际清单

### A-01

- 未找到 `impact_form`、δ 幂律/对数冲击形式或 `depth_weights` 的直接定义。现有 Lean 的笔、段、级别递归不能当量档公式。
- **承接**：当前没有必须新增的 Lean 定义；若实施票要求形式化量档/取整与版本生效，应在 #1444 下另列明确义务，不在本裁定中虚构“已有证明”。

### A-09

- `formal/Origin/TotalWealth.lean:97-108` 定义 `TWState.free/holding/withdrawn/notionalIn/stage/units`，承载资金与持仓状态，不含 `capacities.source` 或 `overflow_trigger`。
- `formal/Origin/TotalWealth.lean:140-141` 的 `advanceTo` 只做阶段 rank 推进，不是资金溢出触发。
- **承接**：#1446 如要补 G/Transfer 的 Lean 守恒证据；不能把现有 stage 定理当作三口径触发证明。

### A-10

- `formal/Origin/LedgerReconciliation.lean:456-476` 证明成本分配合计一致，属于账簿守恒，不决定 ROUNDTRIP 配对或 second_layer 判据。
- 未找到 `pairing=ROUNDTRIP`、`DescentBottomEvidence`、`share_lb_95/share_ub_95` 或 `share_threshold`。
- **承接**：当前无直接 Lean 实现；#1441 只在需要形式化成本证据时承接，不能以账本定理冒充成本门模型。

### B-05

- `formal/Origin/TotalWealth.lean:140` 与 `formal/Origin/LedgerReconciliation.lean:467` 均不决定撮合；没有 `model_id` 或 `capacity_source`。
- `formal/Origin/TraceProjection.lean:56-75` 的 `same_seed_same_future_trace` 证明相同投影 seed 产生相同轨迹，是**确定性证据**，不是历史执行模型 seed 字段或模型实现。
- **承接**：#1411 可复用该类确定性证据；正式模型/适配器归 #1381/#1382，不据此声称 Lean 已实现批模型。

## 4. Python 实际清单

| 文件:行 | 当前事实与名分 | 承接 |
|---|---|---|
| `s_session/tests/fixtures/tb03c_funding/funding_oracle.py:21-33` | 容量、频率和 w(1) 的独立 golden 数据。 | #1406/#1444 对拍；非生产读取。 |
| `s_session/tests/fixtures/tb03c_funding/funding_oracle.py:43-70` | `derive()` 的容量等待、逐层容量分配、向上溢出和 w(1) 取整，是独立测试 oracle。 | #1444 验来源与溢出；ProbeAuto 仍需单独正例。 |
| `s_session/tests/tb03c_oracle.py:1-15` | 独立账簿 oracle，包含 growth_eligible 与未签 post-gate AwaitingPolicy；不是 overflow_trigger 或模型 id。 | #1446 对账，不当作触发器。 |
| `s_session/tests/tb03c_oracle.py:130-141` | `view()` 输出 growth_eligible/growth_enabled 等 golden 视图，不是生产 `capacity_source`。 | #1446 对拍；B-05 不归它。 |
| `src/newchan/server.py:432` | 旧 HTTP `include_nested_divergence` 默认 false 的下钻开关，不是 second_layer 证据，也不是历史批模型。 | TB-10 旧入口终态；#1441 不继承这个开关当 second_layer。 |

**Python 未实现面**：

- A-01 未找到 δ/`impact_form` 的生产读取或 `depth_weight` 对应实现。
- A-09(a) 未找到 `ProbeAuto`；A-09(b) 未找到 `overflow_trigger`。
- A-10(b) 未找到 `second_layer`/`share_threshold`。
- B-05 未找到获准的历史执行 adapter、`model_id`、`capacity_source`、`scenario_contract` 或 `bar_volume_derived`。`s_session` 的测试驱动与查询工具不能自动升格为生产 adapter；正式实现归 #1381/#1382。

## 5. 承接票状态（2026-10-25 实时查询）

全部以下游票当前状态为准，不把已关闭票当实施承接：

| 票 | 状态 | 本清单中的职责 |
|---|---|---|
| #1444 TB-06-C | OPEN | δ/量档来源、capacity domain、ProbeAuto、窗口和不确定性带；消费侧量档迁移。 |
| #1446 TB-06-F | OPEN | G、资金溢出与 units/capital/mark_value 触发、transfer 及恢复。 |
| #1441 TB-02-S | OPEN | ROUNDTRIP 成本证据、成本门、second_layer 及费用来源。 |
| #1381 TB-07-A | OPEN | 完整历史输入推动权威域并生成可追溯回放；正式模型字段。 |
| #1382 TB-08-A | OPEN | 历史/paper 共用受控 adapter。 |
| #1399 TB-07-B | OPEN | seek、修订重算与真实获知时间。 |
| #1411 TB-07-C | OPEN | 双进程、双跑确定性与崩溃恢复。 |
| #1418 TB-07-D | OPEN | 回放报告、分层读数与生产改动回归锁。 |
| #1398 TB-06-I | OPEN | 完整经济事件样本和闸门前统计，与 capacity_source 场景容量相关。 |
| #1406 TB-03-C | OPEN | 当前资金合同与唯一应用链； capacities 既有承载。 |
| #1385/#1386 TB-10-B/C | OPEN | 旧策略、多重装配及 Organic/positional 旧入口终态。 |
| #1387/#1388 TB-10-D/E | OPEN | positional/fusion 与混合旧π入口终态。 |
| #1389/#1390 TB-10-F/G | OPEN | 严格构造费用、形式锁、探针与封存支持的退出准备。 |

**只作历史引用，不作承接**：#1677（evidence_gate 正本已关）、#917（数据可得性研究已关）、#1350（投影分配已关）。**禁止承接**：#1394 已关闭；#1699/#1700 已是其他在飞 PR，不能复用号码。

## 6. blocking 边评估

1. **#1598 → #1444：保留。** `ProbeAuto` 的窗口纪律、n/最大月簇和不确定性带是其启用前置，不是文字修饰。
2. **#1598 → #1418：保留。** 随机模型与 `bar_volume_derived` 需要在报告中单列合成/假设计数，不能混入真实路径读数。
3. **#1598 → #1411：保留。** 若启用随机模型，seed 是双跑确定性的必要输入；同 seed 必须逐位复现，且需要恢复边界证据。
4. **#1598 → #1445：不建议接。** #1445 是“行情换档与原重持续经营”，不是历史模拟模型、随机 seed 或 `capacity_source` 的验收票。历史模型与双跑应由 #1381/#1399/#1411/#1418 承担；报告分层由 #1418 承担。把它当成 B-05 的直接 blocker 会错配依赖。
5. **#1677 只引用版本规则，不再作为 blocker。** `share_threshold{theta2}` 可引用其 evidence_gate 签署/预登记先例，但 #1677 已关闭。
6. **#917 只作数据缺口证据。** 它证明当前没有逐笔/盘口数据，不是等待它完成才能写裁定；待标定项应保持 Unknown/AwaitingPolicy，而不是伪装已可运行。

## 7. 检索式与零命中边界

实际执行过（不是只看文件名）：

```bash
rg -n 'impact_form|overflow_trigger|capacity_source|scenario_contract|bar_volume_derived|stochastic-microevent|history-sim/bar-batch-known-frontier|second_layer|share_threshold' \
  rust/src src s_session formal tests trading_system scripts

rg -n 'DescentBottomEvidence|share_lb_95|share_ub_95|reached_bottom|divergence_not_judged' \
  rust/src src s_session formal

rg -n 'model_id' rust/src src s_session formal tests trading_system scripts
```

前三组对本票直接字段均为零命中；`model_id` 只命中 `lav_seal` 的 LLM 身份，已单列排除。另用 `depth_weights|depth_weight|ROUNDTRIP|AMP_ROUNDTRIP|capacities|capital_overflow|seed|model_id` 找出现有载体，再逐个打开复核，未把相似词当实现。

**不能证明的事项**：本补件不声称已经实现这些字段，也不替用户选择默认值。它只证明当前代码的真实承载、缺口和承接关系。`depth_weight` 的调用链另经 code graph 核到 `build_open_order`、`pi_theta_fill_loop_overlay` 等生产调用方；`pi_theta_fill_loop_overlay` 的调用方包括 `run_theta_v0_pi_overlay` 与 `pi_theta_fill_loop`，这些关系不改变上述旧臂名分。
