# p101 接入裁定 DRAFT 缺口分析（task #101 配套·纯文档）

日期：2026-07-17　性质：纯文档缺口分析——未跑 cargo、未改 `chanlun/escalate/` 任何文件、未改生产源码。
对象：`chanlun/escalate/cert-bsp-binding-ruling-DRAFT-20260717.md`（下称 DRAFT，行号引其条款）。
输入材料：DRAFT、`p103-truncation-replay-doubletest-20260717.md`、`nest-cert-bsp-recon-20260717.md`（#100 报告）、
`p92-postfix-regression-20260717.md`、`nest-migration-ruling-20260716.md`（#91 裁定）、
探针 `rust/src/bin/p100_cert_bsp_recon.rs` / `p101_cert_bsp_tag.rs`、`/tmp/p92_ckpt_dump.txt`（只读，下称 dump）。

## 0. 口径总表：两个证书口径并存（一切缺口之源）

| 口径 | 张数 | 出处 | 用途现状 |
|---|---|---|---|
| nest-91（A=46/B=45） | 91 | #100 报告输入（nest-cert-bsp-recon-20260717.md:4） | DRAFT 全部实证数字（bound=91、strong=39/39、distinct_bsp=31、multi_bsp=22，DRAFT:41-45）均出自此口径 |
| p92 #105 修复后（A=41/B=25） | 66 | dump 实测 CERT 行 66 条；bucket pan=51/mixed=9/trend=6，与 P92_CERT 逐字一致（p92-postfix-regression-20260717.md:18） | 当前唯一在盘的权威证书集合；**尚未跑过 p100/p101 对账** |

- 差异来源已记载：末端 `2:4613084` 簇（39 张、judge=4613104）在 p92 装配集合为 0 条——p92 走 `terminal_bits_new` 且无 intake fallback（p103 报告 :40-42；探针侧 `p92_nest_replay_postruling.rs:728,786,854,931`）。
- 算术推论：91−39=52 < 66 ⟹ 除「91 有而 66 无」的 39 张外，**至少 14 张为 66 口径独有**（两方向都有差集，非单向缺失）。逐键双向差集目前无报告——列入 §3 待 #100 接口。
- dump 实测方向分布：A Long=17/Short=24、B Long=12/Short=13；judge_at 逗号钟 1–4 个（48/12/5/1）；exec/top 分布 1:1=42、1:2=12、1:3=5、1:4=1、2:2=4、3:3=2。
- **结论：DRAFT 条款的实证数字全部待 #105 口径复跑重建；在复跑落地前，DRAFT:41-45 的「物化校验」段只能声明为 91 口径有效。**

## 1. DRAFT 逐条款可执行性核对

状态图例：【已验证】物化且在指定口径下复核一致；【部分】物化但语义/口径有缺口；【未物化】仅有规则文本。

### 条款 1（sidecar 打标，判据层不可见，DRAFT:14-15）——【部分】
- 已物化：p101 探针以只读 sidecar 产出 `P101_TAG`（p101_cert_bsp_tag.rs:218-221），不动判据、不写主路径（头注 :18-19）。
- 缺口：生产侧无任何打标存储/消费点；「判据层不可见」目前靠纪律维持，非代码强制——正是待主人裁 #1（DRAFT:49）。本报告建议升格（见 §2 共同前提与 §4 清单 1）。

### 条款 2（绑定规则：judge_max + 最近同侧 + forward 消歧，DRAFT:16-18）——【已验证（91 口径）】
- 物化：`nearest_dt_tie`（p101_cert_bsp_tag.rs:100-121）与 p100 `nearest_dt`（p100_cert_bsp_recon.rs:97-113）逐字同规则；judge_max = judge_at 逗号钟 max（p101 :76-82）。
- 验证：91 口径 bound=91 / unbound=0 / ties=0（DRAFT:41）。注意 **ties=0 表示 forward 消歧从未被触发**，该分支只有代码路径、无实证覆盖。
- BSP 侧语义同源：探针 = 生产 `IncrementalClassifier::classify_at` + seen-set append-only diff（p101 :157-178），与 runner `newly_confirmed_step`（runner.rs:726-752）同语义；`bsp_bits_disc` 逐字相同（runner.rs:702-709 ↔ p101 :33-40）。
- 待复跑：66 口径。

### 条款 3（强度分档 240/1440 + unbound escalate，DRAFT:19-20）——【部分】
- 物化：`strength()`（p101 :123-129）。91 口径 A strong=39/weak=7、B strong=39/weak=6，与 #100 的 w240=39/39 一致（#100 报告 :11-12）。
- 缺口 a（探针语义）：|dt|>1440 的证书计入 `P101_SUMMARY unbound`（p101 :205-207,234-239），**但仍被插入 `per_bsp` 绑定表**（p101 :212）——当前 unbound=0 无实害；一旦出现，distinct_bsp/multi_bsp 会被未绑定证书污染。「不静默丢弃」由 P101_TAG 行可见性满足，但 per_bsp 插入语义复跑前宜修正或注明。
- 缺口 b：240 固定阈值 vs 级别自适应 = 待主人裁 #2（DRAFT:50）。91 口径 240–1440 档 13 张「多为 mixed bucket Short 侧」（#100 报告 :31）；级别自适应需要 lvl↔bar 换算依据，目前无数据支撑。建议先固定 240 落地，把 |dt| 按 near_lvl 分层出报告后再议。

### 条款 4（时钟纪律：judge_max + 确认 bar；created_at 禁入，DRAFT:21-22）——【已验证】
- 物化：探针只解析 judge_at（p101 :76-82），CERT 行本无 created_at 字段；BSP 确认 bar = 因果塔 diff 的循环序（p101 :161-170）。
- 外部一致性：#91 裁定 ④「created_at 禁用」（nest-migration-ruling-20260716.md:17）；P92_SNAPSHOT created_at_reads=0（p92-postfix :10）。

### 条款 5（层级不硬映射，near_lvl 仅归因，DRAFT:23）——【已验证】
- p100 仅在报告字段携带 near_lvl（p100 :203-211）；p101 绑定键为 (bsp_bar, side)，完全不带 lvl（p101 :191-221）。设计即如此，无缺口。

### 条款 6（打标不回改，append-only，DRAFT:24）——【已验证（探针层）/ 未物化（生产层）】
- 探针层结构性成立（只读；644 meta-rule，p100 头注 :14-15）。
- 生产层尚无接入，属待保持的设计不变量。可复用的不回改先例：OpsemDump env-gated no-op ⟹ 生产路径 bit-exact（runner.rs:1015-1017）；#103 侧信道「只写不判，不触碰 book/seen 主路径状态」（p103 报告 :7-9）。

### 条款 7（同 BSP 多证书合并：布尔 + 最强档，张数不线性放大，DRAFT:25-28）——【部分】
- 已物化：簇「可见性」（`P101_MULTI`，p101 :226-232）。**未物化**：合并语义本身（布尔化 + 取最强档）在消费侧，探针与生产均未实装。
- 实证基础在 91 口径：末端簇 39 张（DRAFT:27-28,45）。#105 口径下该簇不存在（p103 :40-42），但链级多证书现象仍在：dump 终端同 base `1:706241` 出现在 ≥2 条 exec 链（/tmp/p92_ckpt_dump.txt:795 与 :807）。66 口径的 multi_bsp / 最大簇待 #100 复跑重建（§3）。

### 条款 8（稳定主键 = exec-base 判定事件，DRAFT:29-31）——【依据已验证 / 键未物化】
- 依据：#103 测 1 unfold_exact=66、vanish=1 ghost=1（p103 报告 :17-26）。
- dump 直接可核：幽灵链 `3:725489:724160-725489|2:704358:...|1:706241:...`（dump:35，CKPT 750000，pan）；其终端形态 `3:689893:688090-689893|2:704358:...|1:706241:...`（dump:807，mixed，`judge_at=795426,736245,707523`）——top 身份迁移、bucket pan→mixed、**base 事件与 base judge（707523）不变**，与 #103 逐字吻合。
- 缺口：p101 仍以完整 `ids` 串（含 top 身份）作证书键输出（p101 :49,64,219），未按条款 8 发射 `(caliber, exec, base 事件, judge_max, side)` 幂等主键。物化可行：dump ids 以 `|` 分隔、末段即 base；judge_at 高→低排列含基例（nest.rs:407-413 注释），末元素 = base judge。复跑时建议增列主键字段。

### 开放条款（DRAFT:33-37）核对
- B 口径 Long 稀少：91 口径 B Long=11/45（#100 报告 :20-21）；#105 口径实测 B Long=12/25；p92 postfix 记「B 口径 Long 侧偏少」、B_zero=false（p92-postfix :18,23）。**任务台账 #102 题名「B 口径 Long 侧 0 张」与两个口径的实测均不符**——「0 张」的观察窗/口径待 #102 归因时澄清（§3）。在归因落地前，方向不对称权重禁令（DRAFT:35-36）继续有效。
- strong/weak 权重数值属策略层（DRAFT:37）：本报告 §2 候选 C 只给接入点与门，不给数值。

## 2. 实装范围草案：接入点候选三选

共同前提（三选共享）：
- 打标物 = 证书（条款 8 主键）→ BSP 身份键 `(lvl, source_index, bits)` 的映射 + 强度档。BSP 身份键单一来源已存在：`seen_bsps: HashSet<(usize, usize, u8)>`（runner.rs:978,745）；绑定发生在确认 bar 部署点（runner.rs:1181）。
- 绑定逻辑若进生产必须单一实现源（探针/生产复用同一函数），否则违反 644 meta-rule（p100 头注 :7-8）。
- 证书 → (bsp_bar, side) 绑定表的装配走 #103 侧信道同款「只写不判」模式（p103 报告 :7-9），不进 classify 主路径。

### 候选 A：影子模式（shadow tag，只写不判）——推荐第一实装
- 接入点：runner.rs:1181 `newly_confirmed_step` 之后，对本 bar 新确认 BSP 查绑定表，命中经 env-gated writer 写 sidecar（OpsemDump 同款，runner.rs:1015-1017）。
- 回归门：writer 未启用 ⟹ 全路径逐字节不变（先例：runner.rs:934-936 overlay=None bit-exact；测试先例 `disabled_pan_div_production_keeps_book_bit_exact`，pan_div.rs:567）；`cargo test --lib` 绿；p92 复跑 P92_BIT_EXACT 全 0（p92-postfix :7）。
- 价值：零风险产出 66 口径打标分布实证（强度档、簇、near_lvl 分层），供候选 C 定标。
- 局限：不产生策略价值，只产证据。

### 候选 B：过滤模式（tag 作准入谓词）——不推荐作入场过滤
- 接入点：χ 过滤臂（runner.rs:1243-1251 → selector.rs:357 `filter_gamma_with_admission`）；z 扩展维有先例——ZExt 已载 cand_channel/nest_depth/origin_level（selector.rs:63-92），π 路径候选这些维目前诚实 None（runner.rs:1202-1205 注释）。
- 回归门：chi=None 臂 bit-exact 不变；开启臂改变订单流 ⟹ 须预注册 OOS 评估。
- 致命约束：反向覆盖 0.17%–0.97%（#100 报告 :25，91 口径）——打标 BSP 仅 ~1/300，「有标才准入」会掐断订单流，经济语义不成立；非对称用法（未打标加门槛）证据不足。
- 结论：入场侧否决；仅保留风控减仓侧（对未打标持仓降档）作为远期评估项，且须 #102 闭环后议。

### 候选 C：加权模式（tag 调仓位权重）——第二实装
- 接入点：候选 sizing 处——`base_units = equity_nav / px`（runner.rs:1182）与 `PiThetaWeights::from_risk`（runner.rs:966），对命中打标的候选按 strong/weak 档乘系数。
- 回归门：系数=1.0 ⟹ 逐字节等于影子模式（先 1.0 上线等价 A，再梯度开）；系数≠1 臂须：① #100 复跑绑定表落地；② #102 归因闭环（DRAFT:35-36 方向不对称禁令）；③ 预注册评估。
- 与 DRAFT 条款 1「下游仓位/风控可用」逐字对齐；不掐订单流。
- 证书入信号流已有生产先例可参考（非复用）：econ_positive 的 Nest/XZD 二通道准入门（econ_positive.rs:358-377）与 z 维装配（:388-398）、#145 盘背承接（:429-449）——均为「证书不冒充 B1/S1、经独立门入流」的既有形态。

### 推荐序列
A（影子）→ C（加权，1.0 起）→ B 仅风控侧远期评估。依赖：A 不依赖 #100/#102（但消费 #100 复跑的绑定表口径）；C 依赖 #100 复跑 + #102 闭环；B 依赖 C 的实证。

## 3. 与 #100 / #102 的接口占位（待回填清单）

### 待 #100 复跑填入（输入 = dump 66 键，探针 p100/p101）
| 占位 | 91 口径现值 | 66 口径 |
|---|---|---|
| P100_INPUT certs_total / A / B | 91 / 46 / 45（#100 报告 :4） | 待填（dump 实测 66/41/25，待探针确认） |
| P100_BSP events_total / buys / sells | 28,417 / 14,762 / 13,655（DRAFT:43） | 待填——BSP 侧与证书输入无关，**理论必须逐字复现**；若不同即暴露 BSP 提取口径漂移，escalate |
| P100_SUMMARY w0/w5/w30/w240/w1440/median | A 28/28/28/39/46/0；B 31/31/31/39/45/0（#100 报告 :11-12） | 待填 |
| P100_REVERSE w=240 / w=1440 | 49≈0.17% / 275≈0.97%（#100 报告 :25） | 待填 |
| P101_SUMMARY bound/unbound/ties/distinct_bsp/multi_bsp | 91/0/0/31/22（DRAFT:41） | 待填 |
| P101_COUNT strong/weak（A、B） | A 39/7、B 39/6（DRAFT:42） | 待填 |
| 条款 7 簇实证：最大簇张数/bar/是否单一顶层事件组合 | 39 张 @4613104（DRAFT:45） | 待填（末端簇在 66 口径不存在，p103 :40-42） |
| 两口径逐键双向差集（91\66 与 66\91） | —— | 待填（算术下界：39 / 14，见 §0） |
| ties 分支实证覆盖 | 0 例 | 待填 |
| 条款 8 主键列（caliber,exec,base,judge_max,side） | 未发射 | 待填（建议复跑增列） |

### 待 #102 填入
- 「B 口径 Long 侧 0 张」的观察窗/口径澄清：91 口径实测 11、#105 口径实测 12，台账题名「0 张」无对应实测——须说明 0 出自哪个 replay/阶段。
- 方向不对称是否结构性：结论直接门控候选 C 能否对 Long/Short 同参（DRAFT:35-36 禁令的解除条件）。

## 4. 铁律核对清单（接入实装时逐条过闸）

1. **不放宽判据**：打标不得进入 classifier/* 判据路径（条款 1）；BSP 产生/确认/撤销零影响（条款 6）；divergence.rs / bsp.rs / signal.rs / level_view.rs / nest.rs 保持只读。待主人裁 #1 若批「升格硬约束」，实装 = 打标产物置于 classifier 不可达的命名空间 + CI grep 门（classifier/ 禁 import 打标模块）。
2. **bit-exact 门**：任何接入的默认臂逐字节不变——writer 关 / 系数 1.0 / chi=None 三臂各自成立。门检先例：pan_div.rs:567、runner.rs:4426（`run_theta_v0_pi_overlay_reconciles_and_bit_exact_net`）、P92_BIT_EXACT 全 0（p92-postfix :7）。
3. **cargo test --lib 须绿**：每次接入改动后全量跑绿，红即停。
4. **无前视**：绑定只用 judge_max 与 BSP 确认 bar（条款 4）；created_at 与任何终态字段禁入（#91 裁定 ④，nest-migration-ruling :17；P92_SNAPSHOT created_at_reads=0，p92-postfix :10）。
5. **644 meta-rule**：探针必走生产路径、无影子分叉（p100 头注 :7-8）——绑定逻辑单一实现源，探针与生产共函数。
6. **异常不静默**：|dt|>1440 即 escalate（条款 3，并先修 p101 :212 per_bsp 插入语义）；多证书簇必须 P101_MULTI 可见（条款 7）；张数不得线性放大权重。
7. **幂等主键**：对外发布/打标用条款 8 键 (caliber, exec, base 事件, judge_max, side)；top 链身份仅归因字段（#103 幽灵实证，dump:35 vs :807）。
8. **R7 纪律**：复跑若出现 0 产量/0 绑定，按 #91 裁定 R7 三项（provider 完整 + 定义忠实 + snapshot 无前视）全过后方可写「正确市场答案」，否则只能写能力边界（nest-migration-ruling :55）。
