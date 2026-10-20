# #1355 考古

[grilling] 完整量档与容量经验参数 policy（#1323，FU-02）

归类：A 已裁 7 · B 有先例未正式裁 5 · C 确未设计 2。只有 C 类可列为真正待决；B 类以「默认值，确认或否决」呈现；A 类直接沿用。

## Q1 [A] 量档 policy 的记录形状（具名版本化、绑定 profile/标的/级别、固定 Q 只以 q_ref 引用资金授权、逐档/逐级字段）是否已裁？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | quantity-tier-policy | `gh search issues "quantity-tier-policy" --repo xy7365527-lang/NewChanlun` | 5 |
| tracker | 量档 policy | `gh search issues "量档 policy" --repo xy7365527-lang/NewChanlun` | 4 |
| repo | quantity-tier-policy | `rg -n "quantity-tier-policy" docs/adr .chanlun/definitions CONTEXT.md` | 2 |
| repo | quantity_policy\|tier_quantity（代码） | `rg -n "quantity_policy\|tier_quantity" rust/src s_session scripts tests` | 0 |

命中：
- docs/adr/0034-quantity-tier-policy-growth-equity-shift.md:14：政策 id 族 `#1355/quantity-tier-policy/v1`。记录 ⟨policy_id, version, profile_id, symbol, operation_level, q_ref（引用 ADR 0029 资金授权记录，不复制数值）, tiers[...]
- CONTEXT.md:35：具名版本化记录（`#1355/quantity-tier-policy/v1`），绑定 profile/标的/操作级别；固定 Q 只引用资金授权；逐档 w 为整数对并带来源域
- #1355 评论 6153710637：Q1 记录形状：量档 policy 是具名版本化记录（`#1355/quantity-tier-policy/v1`），绑定 profile/标的/操作级别

- 出处：ADR 0034 裁定一（正本层）；CONTEXT.md「量档 policy」词条；#1355 resolution 评论 6153710637，resolution commit 1f5cdf76 已在 main（git merge-base --is-ancestor 核过）。代码侧尚无实装（rust/src 0 命中），实施归 #1444（OPEN）。

## Q2 [A] 未定高档如何处置——是否只阻该深度短差并输出 AwaitingPolicy{tier:d}，不以 0.32/1.0 代填？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | AwaitingPolicy tier | `gh search issues "AwaitingPolicy tier" --repo xy7365527-lang/NewChanlun` | 3 |
| repo | AwaitingPolicy\{tier | `rg -n "AwaitingPolicy\{tier" docs/adr CONTEXT.md` | 2 |
| repo | AwaitingPolicy（代码） | `rg -n "AwaitingPolicy" rust/src s_session` | 0 |

命中：
- docs/adr/0034-quantity-tier-policy-growth-equity-shift.md:17：`AwaitingPolicy{tier:d}` 只阻深度 d 的短差动作；基线与已定档照常；不以 0.32/1.0 代填。
- CONTEXT.md:36：_Avoid_: 把 `depth_weights` 当量档；用底档 0.32 代填高档；用账户权益或市值当 Q 或容量。

- 出处：ADR 0034 裁定二；CONTEXT.md「量档 policy」_Avoid_；#1355 评论 Q2。

## Q3 [A] 经验值更新条件与生效规则——更新是否即新版本、触发仅三类、版本只对生效后投影的命令起作用、AsKnown 不回填？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | AsKnown 不回填 in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 200 --search "AsKnown 不回填 in:comments"` | 16 |
| tracker | valid_through in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 200 --search "valid_through in:comments"` | 7 |
| repo | AsKnown 不回填 | `rg -n "AsKnown 不回填" docs/adr` | 4 |
| repo | valid_through | `rg -n "valid_through" docs/adr CONTEXT.md` | 3 |

命中：
- docs/adr/0034-quantity-tier-policy-growth-equity-shift.md:20：触发：新探针读数（满足 ADR 0022 裁定七窗口纪律并带不确定性带）、用户签署、到期回 Unmeasured。版本对生效后投影的命令起作用；在途与已冻结 L 不重测；回本前同量回补按原卖出冻结的版本与数量；旧版本不删，AsKnown 不回填。
- docs/adr/0022-acceptance-criterion-parameters.md:53-57：凡「比例/速率超阈值即触发动作」的验收判据，必须同时写清四件事…用下界不用点估计…≥ 3 个互不重叠窗…每窗各自过线

- 出处：ADR 0034 裁定三；窗口纪律挂 ADR 0022 裁定七；回放口径挂 ADR 0038（AsKnown）。valid_through 的具体长度属数值，由 #1558 签署（见 Q8）。

## Q4 [A] 绝对容量 C(ℓ) 与投入总额 Q_total 的来源门——C(ℓ) 是否须用户签署、Q_total 是否只来自资金授权、容量 Unknown 是否≠无限？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | Unknown 不是无限 in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 200 --search "Unknown 不是无限 in:comments"` | 12 |
| tracker | q_ref in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 200 --search "q_ref in:comments"` | 3 |
| repo | AwaitingPolicy\{capacity | `rg -n "AwaitingPolicy\{capacity" docs/adr CONTEXT.md` | 1 |
| repo | q_ref | `rg -n "q_ref" docs/adr CONTEXT.md` | 2 |

命中：
- docs/adr/0034-quantity-tier-policy-growth-equity-shift.md:23：探针容量是依据不是裁定（x/Y/σ 口径未裁）。Q ≤ C(ℓ) 在开重与换档激活时校验；C(ℓ) Unknown ⟹ `AwaitingPolicy{capacity}`，未知不是无限
- docs/adr/0029-common-cap-contention-policy.md:33：cap 数值不在本 ADR 给出（根补充）：随具体能力验证或生产激活，由用户签署一版政策记录；本 ADR 不填任何默认金额
- docs/adr/0029-common-cap-contention-policy.md:49：[#1355] 不受影响

- 出处：ADR 0034 裁定四；ADR 0029 裁定四（资金授权记录形状与「不填默认金额」）；#1349 已关（本票 blocker 已清）。

## Q5 [B] 完整量档成功与不支持边界的验收表（S1–S9、U1）落在哪里，取整规则是什么？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | S1–S9 in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 50 --search "S1–S9 in:comments"` | 4 |
| tracker | DECISION §Q5 in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 50 --search "DECISION §Q5 in:comments"` | 8 |
| repo | S1–S9 | `rg -ln "S1–S9\|S1-S9" . ; rg -n "S1–S9" docs/adr .chanlun` | 0 |
| repo | num/den | `rg -n "num/den" docs/adr` | 2 |
| repo | 1355 DECISION 文件（git 全史） | `git log --all --oneline --diff-filter=A -- '*1355*DECISION*' '*1355-DECISION*'` | 0 |

命中：
- #1355 评论 6153710637：Q5 验收表：见 DECISION §Q5（S1–S9、U1）；取整 `⌊Q·num/den⌋`，整数运算，残量不累积。
- docs/adr/0034-quantity-tier-policy-growth-equity-shift.md:23：取整 `tier_quantity = ⌊Q·num/den⌋`（quantum 单位），残量不累积，整数运算。
- #1444 评论（回填表）：来源裁定：#1355（ADR 0034，resolution）。以下义务回填到本票，作为本票验收范围的一部分

- 出处：取整规则：ADR 0034 裁定四、数值口径段（正本）。验收表 S1–S9/U1：仅 #1355 resolution 评论提到「DECISION §Q5」，仓内（docs/adr、.chanlun、git 全史）无该文件；#1444 回填评论也只带受影响代码表，不含 S1–S9。
- 推理链：[新缠论:选择] 推理链：① resolution 评论声称验收表在「DECISION §Q5」，而 PR #1483（commit 82a9e543）只落了 ADR 0034–0036、CONTEXT 词条与五份 AFFECTED-CODE 副本，无 DECISION 文件；② ADR 0034 只收了取整与 AwaitingPolicy 两条边界，S1–S9 的逐条成功/不支持实例未入正本；③ 关票判据要求「具体规则、实例或适用域证据…按名分落正本」，验收表属工作草稿名分（宪法 §5）本应落 review-results 活期绑定票；④ 故 S1–S9 目前只有先例声明、无仓内载体，不能算 A。
- 默认值：由 #1444 实施会话按 ADR 0034 裁定一~四 + #1558 验收条款重建验收表（S：各档 Measured/UserSigned 成功路径、tier_quantity 取整、版本冻结回补；U：AwaitingPolicy{tier}/{capacity}、0.32 域外拒收、w 整数对 <1 拒收），落 `.chanlun/review-results/issue1444-*/` 并在 #1444 评论链接；若起草者手上仍有 DECISION 原稿，先入仓再引用。
- 冲突：#1355 resolution 评论「见 DECISION §Q5（S1–S9、U1）」指向的文件不在 main（rg 全仓 0 命中、git 全史无新增）；关票声明「本评论各项在此刻均已为真」与之不符。

## Q6 [A] 固定 Q 边界不重开、底档 0.32 只属 1m/δ=0.5/中位/L0→L1 不推广——是否已裁且本票是否原样继承？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | 固定 Q in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 200 --search "固定 Q in:comments"` | 50 |
| tracker | 0.32 in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 200 --search "0.32 in:comments"` | 6 |
| repo | 0\.32 | `rg -n "0\.32" docs/adr .chanlun/definitions CONTEXT.md` | 17 |
| repo | 0\.323（推论层/草稿） | `rg -n "0\.323" analysis .chanlun` | 40 |

命中：
- docs/adr/0017-chong-quantity-numerator-shortdiff-amplitude.md:243：站得住的那档是 **0.32**，不是 0.26；**剩下两档不是「另一个数」，是「测不出数」。**
- docs/adr/0017-chong-quantity-numerator-shortdiff-amplitude.md（后果段）：唯一可用于生产的是底档 `w(L0→L1) ≈ 0.32`…全部读数为 **1 分钟单粒度、δ=0.5、中位数口径**，换任一项都要重测。
- docs/adr/0034-quantity-tier-policy-growth-equity-shift.md:14：w(0)≡1 不入档；来源域与 profile 不一致即 Unmeasured；0.32 不写入记录，只可作 Measured 来源被引用。
- #915 评论 5209250256：几何平均 0.323，max/min = 1.34×。再劈成 14 个互不重叠半窗，全落在 0.253–0.441

- 出处：ADR 0017 裁定五/八与后果段；ADR 0016 裁定三（Q＝容量、重生命周期内不变）；ADR 0034 裁定一；#914/#915 已关 resolution。

## Q7 [C] 高档 w(d≥2) 的有效量实际数值从哪来（现有探针读数是否够格作 Measured 来源）？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | 上档没有判据 in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 200 --search "上档没有判据 in:comments"` | 5 |
| tracker | w(d) 高档 探针 | `gh search issues "w(d) 高档 探针" --repo xy7365527-lang/NewChanlun` | 0 |
| tracker | w(2) 无判据 in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 200 --search "w(2) 无判据 in:comments"` | 0 |
| repo | w\(2\)\|w\(3\) | `rg -n "w\(2\)\|w\(3\)" docs/adr/0017-chong-quantity-numerator-shortdiff-amplitude.md` | 1 |
| repo | 不确定性带（探针码） | `rg -n "不确定性带" rust/src` | 0 |

命中：
- #915 评论 5209250256：L1→L2：…劈成两个半窗，同一标的自己前后就对不上——BRN 0.132↔0.688（5.2×）…L2→L3：跨标的差 18.7 倍。L3 以上：出现 w = 35.7 废数
- docs/adr/0017-chong-quantity-numerator-shortdiff-amplitude.md:333：`w(0)=1.0`；`w(1) ≈ 0.32`（8 标的实测，裁定八）…w(2)、w(3) 当前无判据
- #1558 票面：在案唯一读数是底档 0.32，只能作 Measured 来源被引用…未定高档只阻该深度短差（`AwaitingPolicy{tier:d}`）

- 推荐值：按 wayfinder 反面判据③派探针票：在 #1444 把 p915 输出补齐（n、最大月簇、逐窗、不确定性带）之后，按 ADR 0022 裁定七窗口纪律（≥3 互不重叠窗、每窗各自过线、下界口径）重测 L1→L2/L2→L3；测不出则该档继续 AwaitingPolicy{tier:d}，或由用户在 #1558 以 UserSigned 来源签署。grilling 票（#1558）只消费读数，不代填。
- 已搜关键词：上档没有判据、w(d) 高档 探针、w(2) 无判据、不确定性带、0.323
- 缺口去向：派探针票测数

## Q8 [C] 逐级绝对容量 C(ℓ) 的数值、冲击门槛 x 与 σ 统计口径（RMS/中位日）、valid_through 长度，由谁按什么口径签署？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | C(ℓ) 数值签署 | `gh search issues "C(ℓ) 数值签署" --repo xy7365527-lang/NewChanlun` | 0 |
| tracker | x 未裁 in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 200 --search "x 未裁 in:comments"` | 58 |
| tracker | depth_weights in:comments（命中 #1558） | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 50 --search "depth_weights in:comments"` | 6 |
| repo | x 未裁\|x/Y/σ | `rg -n "x 未裁\|x/Y/σ" docs/adr` | 3 |
| repo | σ RMS/中位日（探针码） | `rg -n "RMS\|中位日" rust/src/bin/p915_capacity_frequency.rs` | 4 |

命中：
- docs/adr/0017-chong-quantity-numerator-shortdiff-amplitude.md:185：`x` 是 #917 为给出一个数而选的，本 ADR 不裁 `x`
- rust/src/bin/p915_capacity_frequency.rs:26：`σ_日` 取中位日还是 RMS 未裁，**本探针两个都报，不替裁定方选**
- docs/adr/0034-quantity-tier-policy-growth-equity-shift.md:23：探针容量是依据不是裁定（x/Y/σ 口径未裁）
- #1558 票面（OPEN，needs-approval）：规则已裁（#1355/#1357，PR #1483 合入 main `1f5cdf76`），**数值全部空缺**。
- #1558 评论（主控退回重写，用户 2026-10-20）：用户签署项收缩到：总投入…以及推出重清单所必需而正本尚未裁定的口径（容量的冲击门槛 x 或逐级 C(ℓ)、频率门阈值）

- 推荐值：已有承接票 #1558（OPEN，P 类，needs-approval，草稿在仓外 /Volumes/AgentStorage/decisions/1557/draft.md）——本票不另开。推荐答案：不裁 x，直接按 ADR 0034 裁定四形状逐级签 C(ℓ)=UserSigned，domain 引 #915 L1–L2 干净区间读数为依据；L0、L3+ 标 Unknown ⟹ AwaitingPolicy{capacity}；σ 口径随 C(ℓ) 的 domain 字段记「中位日」（与底档 0.32 同口径）而非另裁；valid_through 按资金授权记录同周期。#1558 草稿新浮出的 U3（同 (S,ℓ) 多空两重共用还是各占容量）与 U5（保证金倍数）正本未裁，留在 #1558 一并签。注意 ADR 0016 裁定三「不是分配出来的，是测出来的」与 ADR 0034「须用户签署」之间是来源门收紧（探针＝依据），不是定义更改。
- 已搜关键词：C(ℓ) 数值签署、x 未裁、x/Y/σ、RMS|中位日、valid_through
- 缺口去向：交用户决定

## Q9 [B] 代码里仍在生效的 depth_weights=[0.60,0.30,0.10] 资金帽与短差 /3 常数，与「w(d)×固定 Q」量档的关系如何处置？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | depth_weights 资金帽 | `gh search issues "depth_weights 资金帽" --repo xy7365527-lang/NewChanlun` | 1 |
| tracker | depth_weights in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 50 --search "depth_weights in:comments"` | 6 |
| repo | depth_weights（代码） | `rg -n "depth_weights" rust/src` | 18 |
| repo | 资金帽 | `rg -n "资金帽" docs/adr` | 4 |
| repo | / 3（短差常数） | `rg -n "/ 3\b" rust/src/theta_v0/strategy/oscillation_campaign.rs` | 1 |

命中：
- rust/src/theta_v0/strategy/voice.rs:12：资金帽深度权重 `config.voice.depth_weights`（w=[0.60,0.30,0.10]）；未用部分保留现金
- rust/src/theta_v0/strategy/coverage/leg.rs:159：let w = depth_weight(depth, config) * dir_weight(&role, depth, config) * w_grade(&role, config);
- rust/src/theta_v0/strategy/oscillation_campaign.rs:313：self.short_diff.cost_basis().units() / 3
- docs/adr/0017-chong-quantity-numerator-shortdiff-amplitude.md（裁定五）：**同时废掉资金帽语义**：…资金帽语义作废。
- docs/adr/0034-quantity-tier-policy-growth-equity-shift.md:39：被否方案：常数表 `depth_weights`（资金帽语义已废）；运行时现算（伪造默认）
- .chanlun/review-results/issue1323-grilling-20261013/1355-AFFECTED-CODE.md：depth_weights 资金帽语义已由 ADR 0017 作废；登记为旧入口参数，不作量档来源（TB-10 收编/退役）

- 出处：ADR 0017 裁定五（资金帽作废）、ADR 0034 被否方案；处置先例＝1355-AFFECTED-CODE.md 路由 TB-10（#1368 OPEN）。
- 推理链：[新缠论:选择] 推理链：① ADR 0017 裁定五把「和为 1 的分一份钱」语义作废，理由是与 ADR 0016 裁定三「理想是满仓」冲突；② 但 leg.rs:159/183 仍以 depth_weight() 算 w、oscillation_campaign.rs:313 仍以 /3 算短差量，生产路径在用；③ ADR 0034 没有新增 Lean/代码义务，只把两处登记为「旧入口」交 TB-10 退役；④ 在 TB-10 完成前，旧入口与新量档并存，新量档记录对旧入口无约束力——这是代码与正本不一致的过渡态，不能归 A。
- 默认值：维持 AFFECTED-CODE 路由：TB-10（#1368）退役 depth_weights/depth_weight/短差 /3；#1444 新建 quantity_policy 记录与查询时不读 depth_weights；在 TB-10 前把 voice.rs:12-13/195 注释改为「旧入口，量档来源见 ADR 0034」。同时修正 1355-AFFECTED-CODE.md 的行号：/3 常数在 oscillation_campaign.rs:313（基线 3da0b2c8 与 HEAD 均同），不是 :589（:589 是 close() 的 debug_assert）。
- 冲突：正本 ADR 0017 裁定五宣布资金帽作废，但 rust/src/theta_v0/config.rs:162 default 仍 [0.60,0.30,0.10]、leg.rs:159/183 仍据此算量、oscillation_campaign.rs:313 仍 /3；且 1355-AFFECTED-CODE.md 引用的 oscillation_campaign.rs:589 行号与实际 /3 位置（:313）不符。

## Q10 [B] Measured 来源必填项（n、最大月簇、逐窗、不确定性带）现有探针 p915 是否能供给，谁来补？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | p915 in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 200 --search "p915 in:comments"` | 5 |
| tracker | p915 不确定性带 | `gh search issues "p915 不确定性带" --repo xy7365527-lang/NewChanlun` | 0 |
| repo | 不确定性带 | `rg -n "不确定性带" analysis docs .chanlun rust/src` | 4 |
| repo | 月簇\|month_cluster\|uncertainty（探针码） | `rg -n "0\.32\|uncertainty\|不确定\|月簇\|month_cluster" rust/src/bin/p915_capacity_frequency.rs` | 0 |

命中：
- docs/adr/0034-quantity-tier-policy-growth-equity-shift.md:14：source: Measured{probe_run_ref, 粒度, δ, 统计口径, n, 最大月簇, 窗} | UserSigned
- rust/src/bin/p915_capacity_frequency.rs:163-164：P915_SIGMA est={tag} n_days={} rms_pct={:.4} median_pct={:.4} p25_pct={:.4}
- .chanlun/review-results/issue1323-grilling-20261013/1355-AFFECTED-CODE.md：探针输出补：n、最大月簇、逐窗、不确定性带（作为 Measured 来源的必填项） | #1444 或探针票
- #1444 评论（回填表）：| #1355 | `rust/src/bin/p915_capacity_frequency.rs` | 探针输出补：n、最大月簇、逐窗、不确定性带（作为 Measured 来源的必填项） |

- 出处：ADR 0034 裁定一（Measured 字段）与裁定三（新探针读数须带不确定性带）；归属先例＝#1444 回填评论（已把 p915 扩展列入 #1444，消解了 AFFECTED-CODE 里「#1444 或探针票」的二选一）。
- 推理链：[新缠论:选择] 推理链：① ADR 0034 把 Measured 来源的合格条件定为 n/最大月簇/逐窗/不确定性带；② p915 现只打 n_days、RMS/中位/分位，#915 报告里的「14 半窗」「月簇>25% 标记」是报告层手工聚合，探针二进制不产出结构化字段；③ 因此当前任何 p915 运行都不能直接作 Measured 来源入记录；④ 这是「代码落后于正本」的实施缺口，已有票承接，不是新决策。
- 默认值：归 #1444：扩 p915 输出结构化 Measured 块（probe_run_ref、粒度、δ、口径、n、最大月簇、逐窗读数、下界/不确定性带），并把输出 schema 锁进夹具；在此之前 tier.source 只能是 UserSigned 或引用 #915 报告（域限 L0→L1）。
- 冲突：ADR 0034 裁定一要求 Measured 来源带 n/最大月簇/逐窗/不确定性带，而 rust/src/bin/p915_capacity_frequency.rs 对这四项 0 命中，只报 RMS/中位/分位。

## Q11 [B] ADR 0013 裁定四「闸门之后…下一段用更大的量」与 ADR 0016 裁定三/ADR 0017「Q 固定」如何并存？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | ADR 0013 裁定四 in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 200 --search "ADR 0013 裁定四 in:comments"` | 25 |
| tracker | q_rebase | `gh search issues "q_rebase" --repo xy7365527-lang/NewChanlun` | 8 |
| repo | q_rebase | `rg -n "q_rebase" docs/adr CONTEXT.md` | 2 |
| repo | ADR 0013 里是否有 0034/q_rebase 指针 | `rg -n "q_rebase\|ADR 0034\|0034" docs/adr/0013-partition-machine-bidirectional-form.md` | 0 |

命中：
- docs/adr/0013-partition-machine-bidirectional-form.md:77：**闸门之后**：每段结束把已实现利润并进本金，下一段用更大的量。依据 `031-第31课.md:169`
- docs/adr/0017-chong-quantity-numerator-shortdiff-amplitude.md:118：`Q` 在该重的整个生命周期里不变（ADR 0016 裁定三：`Q` ＝ 容量…ADR 0013 裁定四的闸门后扩量走的是**换档**，不是改 `Q`）
- docs/adr/0034-quantity-tier-policy-growth-equity-shift.md:26：Q 只能由 `q_rebase` 政策版本改写，前件 `growth_eligible ∧ 新 Q ≤ 已签 C(ℓ)`。G 不是本金
- #1355 评论 6153710637：ADR 0013 裁定四「下一段用更大的量」按 `q_rebase` 显式政策版本读

- 出处：ADR 0034 裁定五（q_rebase）与 CONTEXT.md「q_rebase」词条；ADR 0017 裁定五的读法（扩量＝换档）。
- 推理链：[旧缠论] 031:169「成本为0后……股票才会越来越多」只说筹码变多；[新缠论:推论] ADR 0013 把它读成「下一段用更大的量」，ADR 0017 读成「走换档不改 Q」，ADR 0034 再读成「多出的筹码进 G、Q 只经 q_rebase 显式改写」——三份正本对同一句各给一种机制，后两份互不排斥（换档与 q_rebase 都是显式事件），但 ADR 0013 原文「每段结束把已实现利润并进本金」与 ADR 0031 裁定一「G 不是本金」字面相反；ADR 0013 正文无任何被收窄的指针。正本层允许改，不受 ADR 0012 裁定三「不追溯更新」约束。
- 默认值：按 ADR 0034 裁定五读：闸门后筹码增量入 G，不自动并本金、不自动改 Q；要扩量须 q_rebase 政策版本（前件 growth_eligible ∧ 新 Q ≤ 已签 C(ℓ)）或换档。建议在 ADR 0013 裁定四加一行「本条的『并进本金/更大的量』已由 ADR 0031 裁定一与 ADR 0034 裁定五收窄为 G 分账 + q_rebase 显式事件」。
- 冲突：ADR 0013 裁定四「把已实现利润并进本金，下一段用更大的量」vs ADR 0031 裁定一「G 不是本金」+ ADR 0034 裁定五「Q 只能由 q_rebase 改写」；ADR 0013 文本未标被收窄。

## Q12 [A] 频率门与量档 policy 的关系——频率是否独立于容量、按买/卖分列、policy 内只引用证据不合成？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | 买/卖分列 in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 200 --search "买/卖分列 in:comments"` | 87 |
| tracker | 频率门 买卖分列 | `gh search issues "频率门 买卖分列" --repo xy7365527-lang/NewChanlun` | 0 |
| repo | 买/卖分列 | `rg -n "买/卖分列" docs/adr` | 2 |
| repo | frequency_evidence_ref\|频率门独立 | `rg -n "frequency_evidence_ref\|频率门独立" docs/adr` | 2 |

命中：
- docs/adr/0022-acceptance-criterion-parameters.md:37-39：频率**不乘进**验收标量…ADR 0016 裁定四「开重准入门」…报告读数按 #915 落地要求**买/卖分列**
- docs/adr/0016-chong-quantity-layer-capacity-frequency-shift.md（裁定四表）：容量 | 这个重**多大** …频率 | 这个重**开不开**
- docs/adr/0034-quantity-tier-policy-growth-equity-shift.md:23：频率门独立。
- docs/adr/0036-operating-statistics-sampling-and-post-gate-parameter-protocol.md:30：`frequency.min_evidence_ref`（独立门）…`shift_threshold_ref`（只引用 #1355 容量）

- 出处：ADR 0016 裁定四；ADR 0022 裁定四（两道独立门、买/卖分列）；ADR 0034 裁定一（frequency_evidence_ref 只引用）与裁定四「频率门独立」；ADR 0036 槽位只引用。频率门阈值数值归 #1558 签署（见 Q8）。

## Q13 [B] CONTEXT.md「短差」词条是否已按 ADR 0017 后果补「d≥1 与 w(0)≡1.0」一行，与 ADR 0034「w(0)≡1 不入档」对齐？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | 短差 w(0) | `gh search issues "短差 w(0)" --repo xy7365527-lang/NewChanlun` | 0 |
| tracker | w(0)≡1 in:comments | `gh issue list --repo xy7365527-lang/NewChanlun --state all --limit 200 --search "w(0)≡1 in:comments"` | 0 |
| repo | w\(0\) | `rg -n "w\(0\)" docs/adr CONTEXT.md` | 9 |
| repo | w(0)\|w(d)\|幅度判据\|d≥1（仅 CONTEXT） | `rg -n "w\(0\)\|w\(d\)\|幅度判据\|d≥1\|d ≥ 1" CONTEXT.md` | 0 |

命中：
- docs/adr/0017-chong-quantity-numerator-shortdiff-amplitude.md（后果段）：「短差」条目补一行（`d≥1` 与 `w(0)≡1.0`）…**三处均由主控改，本 ADR 不动。**
- docs/adr/0034-quantity-tier-policy-growth-equity-shift.md:14：w(0)≡1 不入档
- CONTEXT.md:403-405：**短差（ShortDiff）**: 父声部持仓期间，由子级反向证书**显式**开启的子级反向对冲声部（`σ_u = −σ_{p(u)}`，父仓不动）

- 出处：ADR 0017 后果段（承诺补词条）、ADR 0034 裁定一；CONTEXT.md「量档 policy」词条已载 tier 语义但「短差」词条未补。
- 推理链：[新缠论:选择] domain-modeling「Challenge against the glossary」：ADR 0017 于 2026-08-06 把「短差幅度＝w(d)×Q、d≥1、w(0)≡1.0」交主控补进 CONTEXT「短差」词条，至今 rg 0 命中；ADR 0034 再立「w(0)≡1 不入档」。词条缺这一行，读者看「短差」看不出它与量档 d 的对应，而「量档 policy」词条又只讲 tier 不讲短差——两词条间没有桥。属正本层文本欠账，不是新裁定。
- 默认值：在 CONTEXT.md「短差」词条补一行：「短差动作量＝tier_quantity(d)，d≥1；d=0 是基线本仓位 w(0)≡1，不进量档」，指向 ADR 0017 裁定五 / ADR 0034 裁定一；走 triage debt，不进图。
- 冲突：ADR 0017 后果段承诺的 CONTEXT「短差」词条补行（d≥1、w(0)≡1.0）在 CONTEXT.md 中 0 命中，词汇表与正本 ADR 不同步。

## Q14 [A] TB-03/TB-06/TB-07 子义务（AT-06/AT-14）的实施/证明义务是否已回填到具名实施票？

| 路 | 查询词 | 命令 | 命中数 |
|---|---|---|---|
| tracker | TB-06-C | `gh search issues "TB-06-C" --repo xy7365527-lang/NewChanlun` | 30 |
| tracker | #1444/#1446/#1445 状态 | `for n in 1444 1446 1445; do gh issue view $n --repo xy7365527-lang/NewChanlun --json number,state,title; done` | 3 |
| repo | 1355 回填/下游票 | `rg -n "1444\|1446\|下游票" docs/adr/0034-quantity-tier-policy-growth-equity-shift.md .chanlun/review-results/issue1323-grilling-20261013/1355-AFFECTED-CODE.md` | 6 |

命中：
- docs/adr/0034-quantity-tier-policy-growth-equity-shift.md:47：下游票：#1444 TB-06-C、#1446 TB-06-F、#1445 TB-06-E、#1402 TB-10-H、#1406、#1394 FIN（quantity_policy_id 引用、transfer_escrow 类目）、TB-10
- #1444 评论（回填表）：#1323 资金/经营 policy 组 grilling 已裁（main `1f5cdf76`，PR #1483）。以下义务回填到本票，作为本票验收范围的一部分
- #1444 票面：全高档有效量、绝对容量、投入额来源及经验更新版本可查，不能把历史0.32窄域外推。

- 出处：ADR 0034「下游票」段；#1444（OPEN，被 #1355 阻塞的 blocker 已清）、#1446（OPEN）、#1445（OPEN）回填评论在案；数值签署另由 #1558（OPEN，P 类）承接。本票关票判据所需的回填已落，未完成的是实施本身。

## 机械校验

通过。
