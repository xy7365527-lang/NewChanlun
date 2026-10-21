# #1586 自由度审计：既有裁定与实现中把合法候选排除或写死的情形

> 名分：工作草稿（宪法 §5 review-results），活期绑定 [#1586](https://github.com/xy7365527-lang/NewChanlun/issues/1586)。**只列不改**：本文件不改任何正本或代码，改动按条另开票或并入既有票。
> 基线：origin/main `d2f20aaca8`（2026-10-20）。通则正本：ADR 0041（PR [#1591](https://github.com/xy7365527-lang/NewChanlun/pull/1591)，分支 `codex/dof-configurable-principle-20261020`）。初稿输入：同 PR 的 `.chanlun/review-results/dof-audit-20261020/candidates.md`（下称 candidates）。

## 结论

共审 **76 条**：**应改可调（T）40 条**、**真错维持排除（E）27 条**、**原文语义不可调（O）8 条**，另 **1 条待补溯源标签**（B-01）后再归类。分层：正本层 40 条（T15/E21/O4）、决策票层 6 条（T3/E2/待补标签 1，另 4 条并入正本层不重复计）、代码层 30 条（T22/E4/O4）。

影响最大的问题集中在代码层：资金基数与执行成本仍是代码常数，其中 NT 真生产路径的下单基数是一个名为「占位」的 1,000,000（C-02），且 FFI 在 nav 非法时静默换成 1e6 继续算（C-03，属 #799 兜底形态，判 E）。正本层的主要缺口是两类：一是已标 `[新缠论:选择]` 却只给单值、没有字段的参数（冲击函数形式与 δ、验收统计参数、溢出分拆方向）；二是 ADR 0041 第三条「每个字段必须有默认值」与「签署前不给值」模式（ADR 0022 裁定五、ADR 0034–0036、#1354–#1357）相冲，本审计给出的解法是：运行态默认为显式状态 `AwaitingPolicy`，数值默认以「推荐值＋出处」写进签署单（B-10/A-28）。

与 candidates 初稿的分歧：candidates #2（几何衰减）、#3（溢出分拆方向）、#4 中的 C 按比例，本审计判 E（前两条违 ADR 0016 裁定三、后一条违 ADR 0017 裁定五，均为 `[新缠论:推论]`）；candidates #14（笔作 a₀）判 O；candidates #15（`resolve()` 锚为键）不是自由度，不列。其余候选全部复核纳入，映射见第四节。

## 口径

- **适用面**（ADR 0041 裁定一）：只有 `[新缠论:选择]` 与参数类、原文无定论的量可能是 T。`[旧缠论]` 与 `[新缠论:推论]` 为 O 或其另一侧为 E。（AGENTS.md「缠论教义正本」节「Lean 形式化管边界」一类，ADR 0041 裁定一原文未列；本审计不自行扩展，遇到此类按「标签缺失先补标签」处理，见 B-01。）
- **E 的判据**：违原文【正文】、违已裁推论、实测否证、#799 兜底形态（主值失败换备用值）、伪造数据。否决理由只是代价（要填数、重试、前置未就绪、缺标定）的，判 T。
- **可调的含义**：事前在版本化记录里单选、运行中只走一个值（ADR 0041 裁定二判别句）；诊断专用值不参与准入（#799 限定词②）。
- **影响**：高 = 进生产判定或资金路径；中 = 回测、验收口径或现役研究线；低 = 诊断、工具、旧线或调度。
- **原文引证**：只认 `docs/chanlun/text/blog/`，按 AGENTS.md 四步核（行号、逐字、界前后与行首注标记、行内注目视避开）；本文所引 `083:14/:16`、`064:34`、`040:14`、`035:30`、`084:54`、`025:16`、`026:80`、`033:16` 均已在本基线核过。
- **方法**：正本层由主控亲自考古；决策票层与代码层由两个只读分片（claude-opus-5-5 经 CLIProxyLocal）起草，主控复核行号并给终判。非作者独审见第五节。

## 应改可调项按影响排序（前 12）

全部 40 条 T 的受影响票与改法见各层条目表末两列。

| 排名 | ID | 自由度 | 现状 | 建议默认值 | 受影响票 | 改法 |
|---|---|---|---|---|---|---|
| 1 | C-02 | NT 生产下单基数 | `theta_strategy.rs:177`、`theta_pi_strategy.rs:194` 常数 `INITIAL_NAV_PLACEHOLDER = 1_000_000.0` | 无数值默认；读签署记录「总投入」，缺值 fail-loud | #1557、#1530、#1561 | 去常数，从签署记录或场所 profile 注入 |
| 2 | C-01 | 回放总投入 | `trading/types.rs:39` `INITIAL_CAPITAL = 100_000.0`（25 文件引用） | P0 回放 100,000；P1 500 USDC；验收回放 5,000,000（#1557 Q4） | #1557、#1386 | 读签署记录，输出带版本号 |
| 3 | B-10 | 签署类参数的默认值形态 | #1354–#1357、#1349 与 ADR 0034–0036「签署前不给值」，与 ADR 0041 第三条第 2 款相冲 | 运行态 `AwaitingPolicy`；签署单写推荐值＋出处 | #1557–#1561、PR #1591 | ADR 0041 第三条补一句；回写 ADR 0034/0035/0036 |
| 4 | C-06 | 执行成本（佣金、滑点、税、入场延迟、`uncalibrated_addon`） | `theta_v0/config.rs:336-345` `impl Default`；`venue_fee.rs:326` | 现行值，补出处 | #1559、#1561、#917 | 移进场所 profile / ADR 0035 成本记录，`Default` 只留测试 |
| 5 | C-08 | 风控与声部参数（rho、beta、gamma、kappa、z_alpha、cap 开关、max_depth） | `theta_v0/config.rs:238-252`、`:158-167` | 现行值，补出处 | #1557、#1558 | 并入 Θ_risk 签署记录 |
| 6 | A-01 | 冲击函数形式与 δ（决定 `w(d)`） | ADR 0017 `:138-160` 写死幂律、δ=0.5 | 幂律 δ=0.5 | #1444、#1355 | 量档记录 `Measured` 下加 `impact_form`、δ 值域与敏感性代价 |
| 7 | C-10 | MACD 参数 | `theta_v0/config.rs:106-113` 12/26/9 | 12/26/9（`025-第25课.md:16`【正文】「一般都取用12、26、9」） | #985、#1340 | 力度 policy 字段，写原文锚与快速走势代价 |
| 8 | C-04 | 线段特征序列尾窗 | `theta_v0/parser/segment.rs:79` `TAIL_WINDOW = 7`，未登记（正本撤票理由的订正在 #1493／PR #1582） | 7；在 theta_v0 上重测 7↔∞ 或耗尽标 `INCOMPLETE_RESOURCE_BOUND` | #1493、#1344、#1420 | 并入 `second-seq-scan-budget` 记录 |
| 9 | C-09 | 塔高上界 | `theta_v0/config.rs:87` `l_max: 6`，达上界当「自然终止」 | 6，达上界须标覆盖不完整 | #1360、TB-10 | 落具名资源预算字段（#1344 先例） |
| 10 | A-02 | 验收统计参数（95%、月块、n≥30、月块≥12、≥3 窗、≥50%） | ADR 0022 `:28-58` 正文写死 | 现行值 | #1559、#1398、#1410 | `evidence_gate` 记录化，读数前签版本（预登记） |
| 11 | A-04 | 空头扩量风险门 | ADR 0013 `:83` 写死「不设」 | 不设 | #1560 | 作为可选具名系统风险条目，默认未启用 |
| 12 | A-12 | 线段相切边界 `<` / `<=` | `xianduan.md:220-226`、`:473` 两实现各写死一种，待裁 | `<=`（theta_v0 生产） | #813、#1420 | 落 `tangency` 字段，全级同值（#804） |

## 一、正本层（docs/adr、.chanlun/definitions、CONTEXT.md）

### 1.1 应改可调（T）

| ID | 出处 | 当前处置 | 归类 | 理由与证据 | 建议默认值 | 受影响票与改法建议 | 影响 |
|---|---|---|---|---|---|---|---|
| A-01 | `docs/adr/0017-chong-quantity-numerator-shortdiff-amplitude.md:138-160`（裁定六 `w(d)=(A_{ℓ−d}/A_ℓ)^{1/δ}`，`:145` `δ = 0.5`）；`:315` 溯源表 | 写死：幂律形式与 δ=0.5 | T | 裁定六自标 `[新缠论:选择]`（`:37`、`:315`）；`:145` 自陈文献带 0.4–0.7；`:157` 起登记 Zarinelli 等 2015 对数形式拟合更优（E_RMS 6.70 vs 2.80）。形式与指数都是参数，另一取值无原文或实测否证。ADR 0034 裁定一的 `Measured` 来源已含 `δ` 字段，但冲击函数形式（幂律/对数）没有字段 | 幂律、δ=0.5（`0017:145`） | #1444（TB-06-C 量档记录）、#1355 后续：在 `#1355/quantity-tier-policy` 记录的 `source: Measured` 下补 `impact_form ∈ {power(δ), log}`，δ 进可选值集合并写敏感性代价；`log` 标「待标定」：对数形式不齐次、`Y` 约不掉（ADR 0017 `:161`），而 `Y` 在聚合 Bar 通道上标不了（`:149`，#917）；改签走 ADR 0034 裁定三新版本 | 高 |
| A-02 | `docs/adr/0022-acceptance-criterion-parameters.md:28-33`（单边 95%、日历月块）、`:45`（n≥30、月块≥12）、`:51-58`（≥3 窗、各窗过线、≥50% 覆盖） | 写死在 ADR 正文 | T | 溯源表自标裁定三 `[新缠论:选择]`（「原文留白且无统计法可援」）、裁定六 `[新缠论:选择]`；这些是统计参数，原文无数。ADR 0035 裁定一的 `evidence_gate` 只写「继承 ADR 0022 裁定六/七」，没有把数值落成记录字段。代价：验收参数可调会带来看数后改参的风险，故必须在读数前签版本（预登记），版本号随每份读数输出 | 95% / 日历月 / n≥30 / 月块≥12 / ≥3 窗 / ≥50%（现行） | #1559（成本门记录）、#1398/#1410（运营统计）、ADR 0036：在 `evidence_gate` 记录化为 `{confidence, block_unit, min_events, min_blocks, min_windows, min_coverage}`，读数前签署，读数带版本 | 中 |
| A-04 | `docs/adr/0013-partition-machine-bidirectional-form.md:83`、`:325`（「空头扩量不设风险门」） | 写死为「不设」 | T | `:325` 自标「不设风险门」为 `[新缠论:选择]`（编排者裁定），`:83` 自陈代价「空头每次扩量都在放大最大损失，规则层无门拦截」。扩量风险门属系统自设条件（ADR 0030 裁定一第二类来源同族），设与不设都不违原文 | 不设（现行） | #1560（退出授权）、ADR 0030 SR 清单：把「扩量门」作为可选的具名系统风险条目（默认未启用），启用须签阈值；注意它只能是非选择性动作（ADR 0030 裁定一） | 中 |
| A-05 | `docs/adr/0018-limit-order-fill-model.md:79-84`（被否「触及 + 穿透 k 个 tick 的缓冲」；`:81` 被否「触及即成交」） | 被否 | T | 否决 k 缓冲的理由是「k 无数据可标定」（`:82`），属缺标定的参数；「触及即成交」的否决理由是偏乐观（`:76`），属代价不是错误。裁定一「穿越才成交」（`:69`）可视为 k=1tick 穿越、触及即成交为 k=0 | 穿越才成交（现行） | #917（标定线）、#1386：fill 模型落 `fill_rule ∈ {cross(k≥1), touch}` 字段；生产与验收默认 cross，touch 只许诊断 profile（不参与准入，合 #799 限定词②） | 中 |
| A-06 | `docs/adr/0028-first-capability-verification-profile.md:50-55`（被否 A 回放、C 主网最小额、D IBKR paper、E Binance 只读） | 被否 | T | 都是首个能力验证的场所 profile 取值；否决理由是前置未就绪或代价（`:52`「保留为……退回范围」、`:54`「需本地 TWS」「执行器未实现」），无一是错误 | `P1-hl-testnet`（现行） | #1561、ADR 0021：把被否改写为「可选 profile，前置条件……」，保留 A 为回退值 | 低 |
| A-07 | `docs/adr/0030-exit-authorization-and-liability-retirement.md:48`「R2 观察期（要填数字，只延迟）」 | 被否 | T | 否决理由是代价（要填数、只延迟）；观察期是参数，不违原文。同行其余各项见 E 类 A-31 | 关（现行） | #1560：退休判据加可选 `observation_period`（默认 0/关），启用须签数值 | 低 |
| A-08 | `docs/adr/0029-common-cap-contention-policy.md:37`「E 超限即拒（不换来任何不变量，只制造重试）」「O3 有责任即禁取（过度）」 | 被否 | T | 两条的否决理由都是代价（重试、过度），不违 ADR 0014 无仲裁人，也不破坏 Σ责任≤cap（`0029:27`）；`0029:53` 四问全部 `[新缠论:选择]` | 现行（整 slot 等待；按可取额部分取出） | #1557、TB-06：争用政策记录加 `on_contention ∈ {await_slot, reject}`、`withdraw_rule ∈ {partial_up_to_free, deny_if_liability}`，默认现行 | 低 |
| A-09 | `docs/adr/0034-quantity-tier-policy-growth-equity-shift.md:39`「探针容量自动准入（x 未裁）」「按市值触发溢出」 | 被否 | T | 前者否决理由是 x 未裁（`0034:23` 裁定四「探针容量是依据不是裁定」），属待定参数；后者没有给出违原文的理由，资金溢出触发口径（数量/资金/市值）是 `[新缠论:选择]`（`0034:51`）。代价：市值触发会让价格波动单独引发分拆 | 用户签署容量；`Q + units_g` 数量口径触发（现行） | #1444、#1558：`capacities[].source` 已有 `UserSigned` 字段，把 `ProbeAuto` 列为可选值并写前置（x/Y/σ 口径签署后才可选）；触发口径加 `overflow_trigger ∈ {units, capital, mark_value}`，默认 units | 低 |
| A-10 | `docs/adr/0035-cost-stop-policy-instance-and-base-extension-evidence.md:27`「AMP_EXEC 配对」「第二层用占比阈值」 | 被否 | T | 波幅配对口径与第二层判据形式都是 `[新缠论:选择]`（`0035:8` 只写「#907 后裁：ROUNDTRIP 配对」，没有给出 AMP_EXEC 的错误证据）；记录已有 `amplitude.pairing` 字段（`0035:12`），被否只是把值域收成单值。约束：配对口径须与 ADR 0017 的 `A_ℓ` 同一取值（#804 同一判定） | ROUNDTRIP；序关系判据（现行） | #1559：`pairing` 值域恢复 `{ROUNDTRIP, AMP_EXEC}` 并写代价；第二层 `rule ∈ {order_relation, share_threshold(θ₂)}` | 低 |
| A-11 | `docs/adr/0003-live-hypothesis-structure-completion.md:7`（「锚在中枢破坏（三类点）」被否） | 被否 | T | 原文自述「**被否于代价而非教义**」「保留为后续可加的第二层」，教义依据（`092:16`、`018:64`）站在它这边 | 通道切换锚（现行） | 活假设账本后续票：记为可选完成锚 `completion_anchor ∈ {channel_switch, center_break}`，前置为接桥 | 低 |
| A-12 | `.chanlun/definitions/xianduan.md:220-226`、`:473`（X-14 相切边界 `<` vs `<=`） | 两处实现各写死一种，待裁 | T | `:222`「原文未表态 ⟹ `[新缠论:选择]`」；Lean `Claim10:471-474` 同型分歧「不擅自裁定」。两种边界都合法；现状是两套实现各持一种，属 #804 待收敛，收敛方式应是一个签署字段而非删掉一侧 | `<=`（theta_v0 生产 `rust/src/theta_v0/parser/segment.rs:116`） | #813 待裁项、#1420：落 `tangency ∈ {open, closed}`，全级同值（#804），Python P1 同读该字段 | 中 |
| A-13 | `.chanlun/definitions/qujiantao.md:832-840`（`Sel_Θ` 三键：end 最新 → start 最新 → idx 最小） | 写死 | T | `:834`「零原文对应，PDF 与实装都自陈是工程选择 ⟹ `[新缠论:选择]`」；唯一近似原文 `027:38`「最后中枢」 | 现行三键序（#778） | #1346 RD-05、#847：选择器落版本化字段 `selector_keys`，候选序列表与代价写入 | 中 |
| A-14 | `.chanlun/definitions/qujiantao.md:1097`、`:1271`（`TrendScope::Ancestor`，伪引文删除后降 `[新缠论:选择]`） | 可配置但无版本化记录与默认值出处 | T | 该判据唯一溯源 `positional.rs:458` 被核为伪引文（`026` 课「小转大」零命中，AGENTS.md「文档层名分」节在案），名分降为选择；它已经是可配置生产判据，缺的是记录与默认出处 | 现行默认（实现中的默认档） | #847：迁入版本化记录，默认值改写出处为「本仓选择」 | 中 |
| A-15 | `.chanlun/definitions/qujiantao.md:441-453`（塔底 ＝ L0，`econ_positive.rs:823-824`「构造选择，非客观无次级别」）；`zhongshu.md:138`、`level_recursion.md:108`、`beichi.md:459`（a₀ 合法备选 K 线三根 `017:40`、成交三笔 `035:16`） | 现行塔底写死为线段层 L0；备选在正本里保留但无字段 | T（正本已部分合规） | `084-第84课.md:54`【正文】（界 `:68`）「关于第一条，其实，可以随意设置任何的」；#830 已裁基底自由（ADR 0013 `:289`）。ADR 0035 记录已有 `tower_base_ref` 字段（`0035:12`）。缺：生产塔构造仍只有一个硬编码基底 | 线段（L0，现行） | #847 阶段二、#907、TB-02：`tower_base ∈ {segment, kline3(017:40), trade3(035:16)}` 进 profile 记录；笔不在值域（见 O 类 A-40） | 中 |
| A-16 | `.chanlun/definitions/bi.md:29`（`strict` 档 `min_strict_sep` 默认 5，`src/newchan/a_stroke.py:233`）；`bi.md:348-354`（S-3 gap 不足回退：族I `i` 前进 vs theta_v0 `i+=2`） | 写死 | T | 两条都自标 `[新缠论:选择]`、原文零出处（`:350`）。S-3 改值代价已写明（可能翻掉 #93 H1 的 O(n²) 修复，`:354`） | 5；`i+=2`（现行） | #813：仅在对应实现仍有消费者时落字段；族I 退役则随 TB-10 删除 | 低 |

### 1.2 维持排除（E）

| ID | 出处 | 当前处置 | 归类 | 理由与证据 | 影响 |
|---|---|---|---|---|---|
| A-03 | ADR 0016 `:151-159`（五(c) 原级别留满、溢出升上一级）；`:238` 溯源表 | 选定一种（另一分法未保留） | E | 自标 `[新缠论:选择]`，`:159` 明写与 `040-第40课.md:14`【正文】（界 `:24`）「升级为大级别操作基础上部分保持小级别操作」字面语序相反。但把「主体升级＋部分保持」用于**资金溢出**时，钱够而下层重规模 < C(ℓ)，违同 ADR 裁定三「规模＝容量」`[新缠论:推论]`（`:157`「超出容量的部分按定义就不属于这个重」；与 A-20 同一口径）。`040:14` 的「部分保持」形态在 ADR 0016 `:159` 本就归**行情触发**换档，已由 ADR 0034 裁定七（`0034:33`）落为用户签署的分拆份额，属已合规的可调落点，不另设 `overflow_split`。初稿 candidates #3 归改可调，本审计经独审改判 | 中 |
| A-20 | ADR 0016 `:52-66` 几何衰减 `Q(ℓ)=Q₀·f^ℓ` | 被否 | E | 与同 ADR 裁定三「规模＝该操作级别的容量」（`:68-89`，`[新缠论:推论]`，三条【正文】合成）直接冲突——容量是测出来的量，不是 Q₀ 的几何倍；`:63` 在案实测价格几乎不变时 deployed notional 塌 22.5 倍。初稿 candidates #2 归「待判」，本审计判维持：被否的是「量层形状」，`f` 作为参数的残余问题已在 ADR 0017 `:283-292`（#925「形式层待判」）保留，不重复。ADR 0016 `:54`/`:232` 的 `[新缠论:选择]` 标的是「否决」这一动作本身；判 E 的依据是裁定三 `[新缠论:推论]`，不是该标签 | 中 |
| A-21 | ADR 0016 `:125-131` 五(a) 否决「挪钱」 | 被否 | E | 冲突 ADR 0010 §一「专属筹码」要件与 ADR 0013 裁定一「各重的钱独立」（均已裁）；改它须先重开两条定义，不是同层可调值（candidates #16 同判） | 中 |
| A-22 | ADR 0029 `:37` B 具名优先序、D 覆盖最大、C 按比例 | 被否 | E | B、D 都是在重之间排序或按大小选人，违 ADR 0014 裁定二「重间无仲裁」（`[新缠论:推论]`）；`0029:18` 明写允许小 slot 先过「等于按大小选人，即被否的 B/D」。C 按比例改写每重动作量，违 ADR 0017 裁定五「幅度＝w(d)×Q」（`[新缠论:推论]`）。初稿 candidates #4 把 C 归改可调，本审计不同意 | 中 |
| A-23 | ADR 0029 `:37` I2、O2 | 被否 | E | I2「只对新申请生效（会永久等待）」是活性缺陷；O2「允许 cap < 责任」破坏 Σ责任≤cap 不变量（ADR 0024 裁定一/五） | 低 |
| A-24 | ADR 0030 `:48` X1、X2、系统选重减、限价未成自动转市价、R3、恢复时重新评估、一律等人工 | 被否 | E | X1 与 `026-第26课.md:26`【正文】相悖；系统选重减违 ADR 0014；自动转市价是 #799 兜底形态（ADR 0030 `:36`「不自动升级为市价」）；恢复时重新评估是第二次判断；R3 无释放触发；一律等人工把失败态当政策 | 中 |
| A-25 | ADR 0030 `:48`「一律 S1 / 一律 S4」「一律只撤 / 一律市价」 | 被否 | E（合通则） | 被否的是「写死一种」，现行裁定二/三已是条目声明 S1–S4、动作表可改（`0030:27`、`:36`「用户可以改表」）——这正是 ADR 0041 第三条的落法，属正面先例 | 低 |
| A-26 | ADR 0034 `:39` 常数表 `depth_weights`、运行时现算、增长自动并入 Q、G 记成本金、新重阶段特判、目标超时退款 | 被否 | E | 常数表与通则同向；增长并入 Q 违裁定五 `[旧缠论]`（`031:24/:26/:30`）；G 记成本金违 ADR 0031 裁定一；阶段特判是第二个判断（`growth_eligible` 唯一谓词）；超时退款发明时钟终结（`0034:36` 裁定八「无时钟超时」） | 中 |
| A-27 | ADR 0035 `:27` 中位数读法、缺项代 0、代理下界充 Known、引用 #852 旧读数；「θ 固定为 1」 | 被否 | E | 中位数作门违 ADR 0022 裁定三「门必须是下界」（N-6 判例）；缺项代 0 与代理下界充 Known 是伪造数据；#852 读数已过期。「θ 固定为 1」被否的是写死，θ 已是用户签署字段且「系统拒绝 θ < 1」（`0035:12`），属正面先例 | 中 |
| A-28 | ADR 0036 `:35` 只收成本下降事件、零时长加 ε、汇总作门、人工判定触达、现在给闸门后槽位默认值 | 被否 | E | 只收下降事件是选择偏差；加 ε 造数据；统计单位＝重（`0036:22`），汇总作门会掩盖逐重失败；触达是事实不是判断。「现在给闸门后槽位默认值」：ADR 0022 裁定五禁止零样本硬编数字。与 ADR 0041 第三条「每个字段必须有默认值」的张力（candidates #13）按此解：该类字段的默认值是显式状态 `Unset → AwaitingPolicy`，不是数值，建议 ADR 0041 补一句 | 中 |
| A-29 | ADR 0036 `:35` 按原件逐条或按自然日聚合、跨闸整条入闸前或整条剔除、空头用开空均价作基价 | 被否 | E | 采样单位＝(重, 稳定点) 由 ADR 0033「批内顺序不存在」推出（`0036` 裁定二），逐条拆分会引入不存在的批内顺序；跨闸整条处理与裁定四的分段记账矛盾；空头基价须与多头同一 `cost_remaining` 函数（裁定五） | 低 |
| A-30 | ADR 0032 `:39-43` 最大余数法改配额法 | 被否（修订） | E | 最大余数法违反本裁定自己声明的「A_i 单调不减」（Alabama 悖论，4,672 清单穷举 1,738 个反向） | 低 |
| A-31 | ADR 0014 `:46-51` (a) 大级别优先、(b) 撞车消耗总闸门额度 | 被否 | E | (a) 违 `044:48`（拿别级别信号处置本重筹码）；(b) 重复发明已有机制 | 低 |
| A-32 | ADR 0009 `:16-17` 底层分级闭合 (a) 松、(b) 紧 | 被否 | E | (a) 违 53 课「类别判定不可跳」；(b) 原句零命中且实测判死 37.5–50% 链 | 中 |
| A-33 | ADR 0001 `:279-280` T3-in-c「固定首对」 | 被 #1229 改判 | E | `037:18`「c **至少**包含对 B 的**一个**第三类买卖点」⟹ 固定首对比教义严，属同一判断的两档（#804），已统一为全窗后扫 | 中 |
| A-34 | ADR 0006 `:27-28` 力度确认进候选身份、I(A)⊆D_parent | 被否 | E | 前者把门焊进定义，与 #522 冲突（且其理由正是「剥夺各级门政策的选择自由」，属通则先例）；后者比较的是两种对象 | 低 |
| A-35 | ADR 0008（first-retrace）`:18-24` 右端可动桥、RetestReenters 记 Invalidated、未决候选不入消费面、成功后同中枢新一轮、缺席超时处死、快照当真相 | 被否 | E | 各条有原文或机制依据：唯一死法＝三类点（`018:64`/`101:76`）、`020:62`、`024:36`、补充十三/十四；超时处死发明教义外终结 | 低 |
| A-36 | ADR 0019 `:131` 按挂单价认单 | 被否 | E | 中枢换代时静默误判（不同中枢可同 ZD） | 低 |
| A-37 | ADR 0027 `:106-107` π 合并 owner、π 与 organic 双 owner；ADR 0011 `:112-113` 投影接口、第二条中枢链；ADR 0007 `:13-15`；ADR 0008（bsp）`:23-30`；ADR 0002 `:7-9`、`:31-32` | 被否 | E（架构，非自由度） | 均为架构/载体方案：双 owner 允许失败时切换判据（#799）；投影有损；`038:26` 已定塔；其余为对象建模取舍。不属参数或取值自由度，列入以示覆盖 | 低 |
| A-38 | `.chanlun/definitions/qujiantao.md:351` 价格投影被否 L、E | 被否 | E | L：「L 失败用 raw 兜底」是 #799 所禁；E：四点反例 index-subset 真而 price-subset 假 | 低 |
| A-39 | `.chanlun/definitions/qujiantao.md:790`、`:792`（②′「取最近同向段」、④ Weak 进 Cand）；`zhongshu.md:294-298`（Z-4 继承母核心）；`xianduan.md:210`、`:471`（族I `MAX_SECOND_SEQ_SCAN = 50`） | 判违规／被否 | E | ②′ 被 `061-第61课.md:26`【正文】「都可以比较力度」正面反对；④ 四路否掉；Z-4 继承口径实测 48.8% 子中枢核心落在母核心之外，与 Z-1 冲突；50 截断在第 51 项出现有效终结时出错（#1344 第五问） | 中 |

### 1.3 原文语义不可调（O）

| ID | 出处 | 当前处置 | 归类 | 理由与证据 | 影响 |
|---|---|---|---|---|---|
| A-40 | `.chanlun/definitions/zhongshu.md:136`、`:148`；`level_recursion.md:91-97`（缠论笔不作 a₀） | 排除 | O | candidates #14 待判项，本审计判 O。`083-第83课.md:14`【正文】（界 `:36`，行首无注标记、行内无娇注）「我们是可以设计这样的程序……但这样构造出来的系统，其稳定性极差」单看像带代价的合法值；但同课 `:16`【正文】给出结构性理由（一笔会被「打错单」「老鼠仓送货」改变，线段则不会），且 `064-第64课.md:34`【正文】（界 `:44`）「站在严格意义上，线段以下是没有中枢的，所以说是类中枢」——笔上三段重叠的对象按原文不叫中枢。可调的是「中枢以下用什么做 a₀ 的构件」（A-15），笔构件只能产出「类中枢」，不进 a₀ 值域。记 O 而非 E：被排除的不是同一对象的另一取值，而是按原文语义根本不叫「中枢」的另一对象（同 candidates 终版「移入维持排除」，结论一致，只是分类口径不同） | 中 |
| A-41 | `.chanlun/definitions/beichi.md:286-303`（次级别背驰保留 Extreme，#814 D-2）；对照 `qujiantao.md:791`（Extreme 硬门标 `[新缠论:选择]`） | 写死含 Extreme | O | D-2 以 `014-第14课.md:34`【正文】缠中说禅买点定律及四处 Extreme 必要条件原文为据，标 `[旧缠论]`（`beichi.md:303`），且明令不得宽严两档。`qujiantao.md:791` 的 `[新缠论:选择]` 标签早于 D-2，属待订正的旧名分（建议在 qujiantao 订正，不另开可调） | 中 |
| A-42 | ADR 0015 `:21` 否决净值型 `E[ΔR]>0`／Sharpe 作验收尺 | 被否 | O | 验收尺取自原文成本下降与利润率（`049-第49课.md:74`【正文】；ADR 0015 `:148` 明写「本 ADR 的尺子来源只能是缠师原文（第 31/49/75 课）」），不是参数 | 低 |
| A-43 | ADR 0016 裁定三/四（规模＝容量、频率门）；ADR 0017 裁定一/四（满仓基线、位置定方向） | 写死 | O | `[新缠论:推论]`／`[旧缠论]`，依据 `049:52`、`049:64`、`053:34`、`049:74`、`026:34` 等【正文】；容量与频率的**数值**是测量与签署对象（ADR 0034 裁定四），不在此列 | 高 |
## 二、决策票层（#1323 图内已关裁定票 resolution）

分片底稿由 claude-opus-5-5 起草（只读），主控逐条复核出处后在末列给出终判；终判与分片判断不同处以末列为准。计数按终判：B-04、B-06、B-07、B-08 并入 A 层条目，不重复计数。

### 2.1 条目表

| ID | 出处 URL | 当前处置 | 归类 | 理由与证据（逐字引句） | 建议默认值 | 受影响票与改法建议 | 影响 | 主控终判 |
|---|---|---|---|---|---|---|---|---|
| B-01 | [#1478 ic-6149640661](https://github.com/xy7365527-lang/NewChanlun/issues/1478#issuecomment-6149640661)（落码登记 [ic-6152239031](https://github.com/xy7365527-lang/NewChanlun/issues/1478#issuecomment-6152239031)）；正本 `.chanlun/definitions/zhongshu.md:234`、`:236` | Q2 延伸测试的末成员端点采 Lean 全单元口径（`extendWeak`），标 `[新缠论:选择]`。「只测 Zn」没有落成可选值，只出现在退场条件里 | **T**（**边界待判**） | zhongshu.md:234：「原文没讲它的归属 ⟹ 采 Lean 全单元口径（`extendWeak`，塔同）：**该回抽段为连接段（突破段）**」。:236：「**代价照实**：……`Zn` 口径会让该回抽成为前中枢末成员、下一中枢换向起扫，两者给出不同的下一中枢。缠论直觉站在哪边原文未定」。两种口径都不违原文，也没有实测否证，属于 ADR 0041 第四条所说的「另一种合法取值」。**待判点**：AGENTS.md 权威分层规定「原文未讲 ⟹ Lean 管」。如果这条分层让 Lean 口径算作已裁事实，本条应改判 O | 全单元口径（现行；出处为 Lean `extendWeak` 与 #1478 裁定二） | #1434、#1424、#1429；zhongshu.md §4.1 把「只测 Zn」从退场条件改写为可选值，写明代价「下一中枢不同」；在中枢构造的 policy 版本记录里加端点口径字段 | 高 | **待补标签**（主控终判，经独审修正；不计入三类）：`zhongshu.md:234` 本行无溯源标签；#1478 [ic-6149640661](https://github.com/xy7365527-lang/NewChanlun/issues/1478#issuecomment-6149640661) 依据链标 `[新缠论:选择]` 并注明「由 Lean 管边界」。按 ADR 0041 裁定一「标签缺失时先补标签」，建议经 zhongshu 裁定票补为 Lean 边界裁定；实质上倾向不落运行期字段（改口径须改 Lean 定义并同拍改塔，即 `:236` 退场条件），但补标签前不记 O |
| B-02 | [#1342 ic-6149622771](https://github.com/xy7365527-lang/NewChanlun/issues/1342#issuecomment-6149622771)；正本 `.chanlun/definitions/baohan.md:120`、`:124` | 初始包含方向选 W（用随后首个非包含严格对的方向折叠前缀）。固定 UP/DOWN 与 close/open 两种做法被否 | **T**（**边界待判**） | resolution：「固定 UP/DOWN 无源且偏向，close/open 引入原文没有的坐标」；「用随后首个非包含严格对的方向折叠此前的全包含前缀是 `[新缠论:选择]`」。baohan.md:124：「二者均无原文依据……不得作为授权」。三个候选都没有原文来源，被选中的 W 本身也只是 `[新缠论:选择]`。所以「无源」本身不足以把另两个判为真错；「偏向」与「引入新坐标」说的是代价。已有规则版本机制（「只改政策也须新规则版本」）。**待判点**：「引入原文没有的坐标（close/open）」算违原文（那就是 E），还是只是代价更高的合法选择（那就是 T） | W（现行） | #1393 TB-02-U、baohan.md §初始方向：被否的两种做法改写为「可选值 + 代价」，或者补一句理由，说明为什么它们违原文、而不只是没有原文来源 | 中 | **E**（主控终判）：固定 UP/DOWN 在有 raw 证据时仍断言一个方向，等于发明方向（baohan.md:120「无 raw 证据的方向是发明」同理）；close/open 把包含关系从高低点换到另一组坐标，改变的是被判对象本身。两者都不是同一对象上的另一取值。排除类别（ADR 0041 裁定四）：固定 UP/DOWN＝伪造数据；close/open＝违原文坐标（包含关系按原文只在高低点上判） |
| B-03 | [#1343 ic-6153028395](https://github.com/xy7365527-lang/NewChanlun/issues/1343#issuecomment-6153028395)；正本 `.chanlun/definitions/bi.md:327`、`:328` | Q2 同价极值取极值根集与极值区的集合语义，policy 名为 `same-price/keep-earlier+extreme-region/1`。「从并列根里指定某一根作端点」被否 | **E**（**边界待判**） | resolution 给出的理由是集合语义「等于对任意指定取最严，契合 `081:110`「不能放松」与 #799」。指定任一根会使条件 2 比集合语义更松，这正是 `081:110`「绝对不能放松」所禁止的方向，所以倾向 E。但 bi.md:327 自己写「原文（`081:110`「不包括这两K线」）预设唯一、Lean……未定义，故为新裁定」，并标 `[新缠论:选择]`。**待判点**：标签说是选择，理由说是违原文，两者自相矛盾。应当二选一：改标签为 `[新缠论:推论]`（那就是 E），或者承认指定根是合法的较松取值（那就是 T，但又会撞上 #799 的宽严两档） | 集合语义（现行） | #1405；bi.md:327 统一溯源标签与理由（建议改标 `[新缠论:推论]`，引 `081:110`） | 中 | **E**（经独审修正理由）：指定单根＝集合语义的宽档，属 #799／ADR 0041 判别句所禁的同一判断两档；`081:110` 只作「预设唯一」旁证（该行在 081 课 `↑正文` 界 `:78` 之后，归属同 C-11 待复核）。不建议据此改标签 |
| B-04 | [#1345 ic-6149626626](https://github.com/xy7365527-lang/NewChanlun/issues/1345#issuecomment-6149626626)；正本 `.chanlun/definitions/qujiantao.md:349`、`:351` | 独立选父 `NestInterval` 的价格挂法定为 R（共享原始序列闭窗外缘）。L 与 E 都写成「**被否**」 | **T**（**边界待判**；L 的兜底部分为 E） | qujiantao.md:351：「**被否**：L（……可得空集；「L 失败用 raw 兜底」是 #799 所禁的宽严两档）；E（两端端点价：忽略内部极值——四点例……index-subset 真而 price-subset 假；还须先定端点根并卷入 RD-02 同价身份；会重开裁定三）」。:349：「价格轴对独立选父链零额外严格度，它是一致性证据，不是闸门」。拆开看：「L 失败用 raw 兜底」是 #799 兜底形态，E，维持排除；单选 L（不兜底）只是可能得空集，属于代价。E 的四点例说明它是另一种更严的挂法，不是逻辑错，但会重开 #817 裁定三（qujiantao §3.4）。**待判点**：(a) 单选 L 得空集算代价还是算不可用；(b) E 重开已裁的裁定三，按 ADR 0041 第一条是否就足以判它为真错 | R（现行） | #1438、TB-04；qujiantao.md:351 把「被否」拆成两部分：「L+raw 兜底：E（#799）」，以及「单选 L、E：可选值 + 代价」，后者需先过裁定三 | 中 | **E，并入 A-38**：L 跨级非恒真（可得空集）、E 端点价有四点反例使一致性证书误报，二者作为「一致性证据」都不成立；「L 失败用 raw 兜底」另是 #799 形态 |
| B-05 | [#1353 ic-6149639079](https://github.com/xy7365527-lang/NewChanlun/issues/1353#issuecomment-6149639079)；ADR 0033 | 历史模拟采 v1 确定性模型 `history-sim/bar-batch-known-frontier/1`。被否两项：随机微事件模型；从 bar volume 推可用量 | **T** | 随机微事件被否的理由：「概率与延迟无数据标定，与 ADR 0018 裁定一否掉「穿透 k 个 tick 缓冲」同理」。另有「不从 bar volume 推可用量」。两者被否都是因为缺标定，不违原文，也没有实测否证，属于 ADR 0041 第四条的合法候选。承载字段已经存在：模型 id、`input_domain`、`capacity_source`。同票里 cancel-first 被否，依据 `049:62`，是 E，不入本条 | v1 确定性、无随机种子；`capacity_source=none` 时按 L0 处理（现行） | #1381、TB-07、TB-06；ADR 0033 的被否项改为「可选值（待标定）+ 代价」，挂在模型 id 与 `capacity_source` 下。与 candidates #8（ADR 0018 k tick 缓冲）同类，但不是同一条 | 中 | **T**（同意）：否决理由是缺标定，属参数；影响 中 |
| B-06 | [#1356 ic-6153715166](https://github.com/xy7365527-lang/NewChanlun/issues/1356#issuecomment-6153715166)；ADR 0036 :34–35「被否方案」 | Q5 选多空统一的 `cost_remaining` 作基价；「空头用开空均价作基价」被否 | **T**（**边界待判**） | ADR 0036 被否方案列有「空头用开空均价作基价」。该 ADR 只把采样单位与闸门两侧两域标为 `[旧缠论]`（`049:74`、`031:24`、`031:169`），其余都是 `[新缠论:选择]`，基价属于后者。初稿只列了同节的「槽位默认值」一项（candidates #13），这一项没有列。**待判点**：开空均价是否与 ADR 0031 裁定一的空头口径（保证金 M）冲突。若冲突就是 E | `cost_remaining`，多空统一（现行） | #1398、#1410；ADR 0036 把这一项移到「可选值 + 代价」 | 中 | **E，并入 A-29**：ADR 0036 裁定五要求多空用同一 `cost_remaining` 函数且闸门处同为 0（空头闸门＝累计利润≥保证金，ADR 0013 裁定四 `[新缠论:推论]`）；开空均价作基价在闸门处不归 0 |
| B-07 | 同 B-06（[#1356 ic-6153715166](https://github.com/xy7365527-lang/NewChanlun/issues/1356#issuecomment-6153715166)；ADR 0036 :34–35） | Q4 跨闸事件分段记录；「跨闸整条入闸门前或整条剔除」被否 | **T**（**边界待判**） | ADR 0036 被否方案列有「跨闸整条入闸门前或整条剔除」，属 `[新缠论:选择]`。两种做法是不同的统计归属取值，不是同一判断的宽严两档。**待判点**：闸门两侧两域是 `[旧缠论]`（`031:24`、`031:169`）。如果整条归一侧会跨越原文划定的两域，就违原文，是 E | 分段记录（现行） | #1398、#1410；同 B-06 | 中 | **E，并入 A-29**：闸门前后两域为 `[旧缠论]`（`031:24`、`031:169`），整条归一侧会让一条记录跨两域 |
| B-08 | 同 B-06（ADR 0036 :34–35） | 同节其余被否项合并为一行 | **E**（其中一项为 O，一项边界待判） | 「只收成本下降事件」：选择偏差，E。「零时长加 ε」：造数据，E。「人工判定触达」：违反 GateReachedEvent 的事实口径，E。「按原件逐条或按自然日聚合」：采样单位已标 `[旧缠论]` `049:74`，O，维持。「汇总作门」：**边界待判**，待判点是汇总量作门属另一种门槛构造（T），还是混淆了闸门前指标与门（E）。「现在给闸门后任何槽位默认值」：同 candidates #13，不在此重复 | —（维持排除） | ADR 0036 给每个被否项补上排除依据的类别（违原文、实测、#799、造数据），以符合 ADR 0041 第四条「写明排除依据」 | 低 | **E，并入 A-28/A-29** |
| B-09 | [#1348 ic-6149630088](https://github.com/xy7365527-lang/NewChanlun/issues/1348#issuecomment-6149630088)；`docs/adr/0031-principal-batches-phase-predicate-corporate-actions.md:28-30` | 裁定五把分红写死为减 K，没有列出可选值，也没有落成字段 | **T** | ADR 0031:30：「### 五、公司行动（第五问 A，`[新缠论:选择]`，原文留白）」，「现金分红净额 `dividend_cash_net: i64` 计已实现回收、减 K」。原文留白，标签是选择，但只给了一个值，没有备选值，也没有版本化字段，违反 ADR 0041 第三条。备选值举例：分红计入资本化 A，不减 K | 计已实现回收、减 K（现行） | #1406、T-19；在会计 policy 版本记录里加 `dividend_treatment` 字段，列出可选值与代价。加密标的不涉及分红，所以影响低 | 低 | **T**（同意）；影响 低 |
| B-10 | [#1354 ic-6153713798](https://github.com/xy7365527-lang/NewChanlun/issues/1354#issuecomment-6153713798)、[#1355 ic-6153710637](https://github.com/xy7365527-lang/NewChanlun/issues/1355#issuecomment-6153710637)、[#1356 ic-6153715166](https://github.com/xy7365527-lang/NewChanlun/issues/1356#issuecomment-6153715166)、[#1357 ic-6153712139](https://github.com/xy7365527-lang/NewChanlun/issues/1357#issuecomment-6153712139)、[#1349 ic-6149631601](https://github.com/xy7365527-lang/NewChanlun/issues/1349#issuecomment-6149631601) | 无默认值模式：签署类参数在用户签署前一律不给值 | **T**（**边界待判**） | 四票共用的根复核句：「全部 Q、量档、容量、θ、费率、滑点、份额、门槛由用户签署，签署前系统只输出 AwaitingPolicy / NotEvaluable / InsufficientEvidence」。#1355 Q2：「不以 0.32 或 1.0 代填」。#1349 Q4：「cap 数值不在本轮给出」。这与 ADR 0041 第三条第 2 款「每个字段必须有默认值，附推导依据」相冲，和 candidates #13（ADR 0036 :35）同类，但覆盖面更大（涉及五张票的全部签署参数）。**待判点**：资金路径上的 P 类签署参数，能否以「默认值必须由用户签」为由豁免 ADR 0041 默认值条款。也就是说，「签署前不可评估」本身算不算一种默认 | 若不豁免：每个字段写「推荐默认值 + 出处」，签署时确认（例如 θ 用 ADR 0035 在案读数）；若豁免：在 ADR 0041 第三条加例外句 | #1557、#1558、#1559、#1560、#1561；由 #1586 统一裁定后回写 ADR 0034/0035/0036 | 高 | **T**（主控终判，通则落法缺口）：与 A-28 同解——签署类字段在运行态的默认是显式状态 `AwaitingPolicy`，数值默认以「推荐值＋出处」写进签署单，由用户签（#1557 第 7 版签署单 [ic-6282040441](https://github.com/xy7365527-lang/NewChanlun/issues/1557#issuecomment-6282040441) 的形态；该版已于 [ic-6282686046](https://github.com/xy7365527-lang/NewChanlun/issues/1557#issuecomment-6282686046) 撤回，前提「按方向签投入额」被否，此处只引其「推荐值＋出处」写法）。建议 ADR 0041 第三条第 2 款补这一句；该款是用户同日确认的边界，补句须用户点头，#1586 不自行回写；影响 高 |

### 2.2 覆盖表（扫过的已关裁定票）


| 票号 | 标题 | 是否产出条目 | 一句话原因 |
|---|---|---|---|
| #1321 | [grilling] 完整生产程序声明边界：M1 的 π 批量链、organic 赋格/声部链与 paper 执行链如何归名 | 否 | Owner 选 B（[ic-5537011170](https://github.com/xy7365527-lang/NewChanlun/issues/1321#issuecomment-5537011170)，resolution [ic-5537418507](https://github.com/xy7365527-lang/NewChanlun/issues/1321#issuecomment-5537418507)）。A′ 被打回（[ic-5492624806](https://github.com/xy7365527-lang/NewChanlun/issues/1321#issuecomment-5492624806)），理由是与 #787 及 ADR 0010/0011/0013/0014 冲突，属违已裁教义，E |
| #1342 | [grilling] 初始包含方向策略及全包含前缀输出 | 是 | B-02 |
| #1343 | [grilling] 同价极值端点及对象身份的确定性实例 | 是 | B-03 |
| #1344 | [grilling] 第二特征序列扫描域与 continuation 边界 | 否 | Q1 依据 `078:52`，为 `[旧缠论]`，O。族 I 的 `MAX_SECOND_SEQ_SCAN=50` 属旧入口退役，不是自由度。Q4 续扫预算是「具名、版本化的调度参数，本票不填数值」（[ic-6149624668](https://github.com/xy7365527-lang/NewChanlun/issues/1344#issuecomment-6149624668)），属资源调度参数，不影响判定，N/A，可并入 B-10 的默认值讨论 |
| #1345 | [grilling] 独立选父 NestInterval 的价格投影 | 是 | B-04 |
| #1346 | [task] 已裁 Comparable/Sel 到条件候选全集的适用域桥 | 否 | task 票，只核已裁对象的适用域；resolution（[ic-6154425014](https://github.com/xy7365527-lang/NewChanlun/issues/1346#issuecomment-6154425014)）自述「没有未覆盖的新选择」 |
| #1347 | [grilling] Type2/3 N/A 在递归证书中的合成处置 | 否 | Q1 为 `[新缠论:推论]`，O。被否的「跳过」与 Kleene 三值属违已裁推论，E（[ic-6149628539](https://github.com/xy7365527-lang/NewChanlun/issues/1347#issuecomment-6149628539)） |
| #1348 | [grilling] 本金与真实回收的边界会计 policy | 是 | B-09（裁定五：分红减 K，初稿未列） |
| #1349 | [grilling] 共同资金争用、cap 与公用资金进出 policy | 部分 | 主体同 candidates #4。跨 slot 次序取结构顺序、不用 FIFO 作默认，是为了不引入仲裁人（ADR 0014），E。cap 不给数值一项并入 B-10 |
| #1350 | [grilling] 净额外效与部分成交费用的分配归属 | 否 | ADR 0032。B 类优先级规则被否，因违 ADR 0014，E。policy id 已版本化（[ic-6149633188](https://github.com/xy7365527-lang/NewChanlun/issues/1350#issuecomment-6149633188)） |
| #1351 | [grilling] 首个具名场所账户与验证权限 profile | 否 | 同 candidates #9。`execution_form` 已经是 profile 版本字段，合乎通则 |
| #1352 | [grilling] 完整退出授权与责任退休 policy | 否 | 同 candidates #5。动作表默认值「是执行偏好，用户可以改表」，属正面先例 |
| #1353 | [grilling] 完整历史模拟执行模型：多重并发、混合订单与同 slot 次序 | 是 | B-05。cancel-first 被否，依据 `049:62`，E |
| #1354 | [grilling] 成本停止 policy 实例与基底进阶证据 | 部分 | 同 candidates #6。θ<1 拒收是有效域下界，依据 `035:30`，O 或 E。签署前不给值一项并入 B-10 |
| #1355 | [grilling] 完整量档与容量经验参数 policy | 部分 | 同 candidates #7。「不以 0.32 或 1.0 代填」并入 B-10 |
| #1356 | [grilling] 经营统计完整事件样本与边界规则 | 是 | B-06、B-07、B-08。槽位默认值一项同 candidates #13 |
| #1357 | [grilling] 固定 Q 下增长换档的权利与会计衔接 | 部分 | 同 candidates #7（增长自动并入 Q）。签署前不给值一项并入 B-10 |
| #1463 | [task] 级别递归归一化现状核验 | 否 | 现状核验 task（[ic-5723053083](https://github.com/xy7365527-lang/NewChanlun/issues/1463#issuecomment-5723053083)）。`l_max` 默认 6 只是观察到的现状，不是裁定 |
| #1478 | [grilling] 中枢外缘 GG/DD 的遍历范围落码与延伸测试的末成员端点 | 是 | B-01 |
| #1489 | [grilling] S 内容编码让每代新增可达节点有界 | 否 | 工程布局（[ic-6159518677](https://github.com/xy7365527-lang/NewChanlun/issues/1489#issuecomment-6159518677)）。被否项（每单元一条 typed 记录、走 observations 族、读者派生）属实测容量不够或违 §1:17，E 或 N/A。`max_store_bytes`=64 GiB 是工程常数，N/A |
| #1490 | [grilling] 线段第一种情况的确认时点、转折点两侧包含与段端门失败去向 | 否 | 被否的「c 侧放宽」「门失败后再锚定重判」都是 #799 形态，E（[ic-6159626152](https://github.com/xy7365527-lang/NewChanlun/issues/1490#issuecomment-6159626152)）。其余为 `[旧缠论]`（071:30/32/42）或 `[新缠论:推论]` |
| #1530 | [grilling] FU-08 生产入口与唯一 owner | 否 | ADR 0039（用户批复 [ic-6281488966](https://github.com/xy7365527-lang/NewChanlun/issues/1530#issuecomment-6281488966)）。候选 B、C、D（含「多个 launcher」）违 ADR 0027 背景段及裁定三、五，E |
| #1537 | [grilling] FU-15 双跑确定性、恢复、seek、AsKnown | 否 | ADR 0038（[ic-6263365907](https://github.com/xy7365527-lang/NewChanlun/issues/1537#issuecomment-6263365907)）。候选 B（seek 看未来、批内排序）属前视且违 ADR 0014，E |
| #1541 | [ruling] FU-13 带仓迁移、回退、责任退休（裁定票） | 否 | 作废关闭（[ic-6262300922](https://github.com/xy7365527-lang/NewChanlun/issues/1541#issuecomment-6262300922)），无裁定内容 |
| #1542 | [ruling] FU-11 逐 bar 事件、身份、生命周期合同（裁定票） | 否 | 作废关闭，无裁定内容 |
| #1543 | [ruling] FU-10 逐 bar Classification→旁路/下钻→Chong（裁定票） | 否 | 作废关闭，无裁定内容 |
| #1544 | [ruling] FU-12 旧引擎/简化臂逐入口终态（裁定票） | 否 | 作废关闭，无裁定内容 |
| #1545 | [ruling] FU-09 严格判据唯一、级别同构锁（裁定票） | 否 | 作废关闭，无裁定内容 |
| #1546 | [ruling] FU-08 生产入口与唯一 owner（裁定票） | 否 | 作废关闭（[ic-6262306720](https://github.com/xy7365527-lang/NewChanlun/issues/1546#issuecomment-6262306720)），无裁定内容；实际裁定在 #1530 |
| #1565 | [grilling] S3 后继串如何发布 W 初始化对象 | 否 | schema 选择（用户批复 [ic-6276988200](https://github.com/xy7365527-lang/NewChanlun/issues/1565#issuecomment-6276988200)，落正本 [ic-6277671337](https://github.com/xy7365527-lang/NewChanlun/issues/1565#issuecomment-6277671337)）。方案 B 不可行（raw 不带 cut），且有 #804 身份风险，E 或 N/A |

## 三、代码层（rust/src、src/、.sandcastle 硬编码参数）

分片底稿由 claude-opus-5-5 起草（只读，自写扫描脚本），主控亲自复核了 C-01、C-02、C-03、C-04、C-05、C-06、C-10、C-20 的文件行号与原文锚（`025-第25课.md:16` 在界 `:54` 内、行内「(注：…)」已避开；`026-第26课.md:80` 在界 `:40` 之后，属解盘段，分片标【解盘】正确），并把 C-05 影响由高订为中。路径名分：theta_v0 为 π 引擎与 NT 真生产核心；trading/ 为现役研究回放；spiral、fugue_v3、recursive_t、segment.rs、buysellpoint.rs 与 Python `src/newchan` 为旧线，归 TB-10（#1368）/FU-12（#1532）。

### 3.1 条目表（按影响排序）


| ID | 出处 文件:行号 | 当前处置 | 归类 | 理由与证据 | 建议默认值（T 必填） | 受影响票与改法建议（T 必填） | 影响 |
|---|---|---|---|---|---|---|---|
| C-01 | `rust/src/trading/types.rs:39` `pub const INITIAL_CAPITAL: f64 = 100_000.0;`（`:38` 注「与 `fugue_alpha_diagnosis.INITIAL_CAPITAL` 逐字」；`rg -l` 共 25 个文件引用） | 写死 | T | **同 candidates #1**，行号已复核，没有漂移。总投入属资金参数，原文没有定论；#1557 Q4 已把「总投入」定为唯一的人签输入，回放侧却仍是常数 | 100,000（现行，P0 回放；出处是 `fugue_alpha_diagnosis` 的逐字对齐）。另有 P1 500 USDC、验收回放 5,000,000 两档可选，见 candidates #1 | #1557（数值签署）、#1386 TB-10-C（organic/positional 旧经济臂隔离）。改法：常数改为读 #1557 签署记录里的「总投入」字段，回放 bin 把所用版本号写进输出 | 高 |
| C-02 | `rust/src/theta_v0/nautilus/theta_strategy.rs:177`、`theta_pi_strategy.rs:194` `const INITIAL_NAV_PLACEHOLDER: f64 = 1_000_000.0;`（`theta_strategy.rs:162` 传给 `read_portfolio_snapshot(...)`；`theta_pi_strategy.rs:184` 传给 `decide(..., INITIAL_NAV_PLACEHOLDER)`，作为 sizing 基数） | 写死（名为占位符） | T | 在 NT 真生产路径上，直接决定下单量的基数。属资金参数，原文没有定论。两处常数逐字重复，candidates 初稿没有列 | 不设数值默认值（以 B-10 的 `AwaitingPolicy` 补句被采纳为前提）。生产路径从签署记录读取 #1557 的「总投入」，缺值 fail-loud；回放 profile 单独签一个 nav 字段，不作生产缺值时的备用值（ADR 0041 裁定二推论，与 C-03 同理） | #1557、#1530 FU-08（生产入口唯一 owner）、#1561 FU-04（场所 profile）。改法：去掉占位常数，改从签署记录或场所 profile 注入；缺值时 fail-loud，不能退回 1e6 | 高 |
| C-03 | `rust/src/theta_v0/ffi.rs:71` `initial_nav=1.0e6`、`:83-87` 传入值 ≤0 时改成 1.0e6、`:96` `let nav = if nav > 0.0 { nav } else { self.initial_nav };`（`:69` 文档：「`initial_nav` 仅在 … `nav<=0` 时作兜底」）、`:105` `let nav = self.initial_nav;` | 写死，并以兜底方式运行 | E | 列入的是**兜底行为本身**：调用方给的 nav 不合法（≤0）时，代码悄悄换成 1e6 继续算。这是 ADR 0041 第二条点名的「主值失败时的备用值」，也是 ADR 0034:39 被否方案「运行时现算（伪造默认）」的同一形态，按 #799 维持排除。资金数值本身另归 C-01/C-02（T） | —（应改成 fail-loud） | #1557、#1530。改法：nav≤0 直接报错；1e6 默认参数去掉，或改为必须由调用方显式传入 | 高 |
| C-04 | `rust/src/theta_v0/parser/segment.rs:79` `pub const TAIL_WINDOW: u32 = 7;`（`:827`、`:1178` 都是 `FeatureSeqState::new(seg_dir, ExtendMode::Strict, TAIL_WINDOW, config.second_seq_scan_window)`；`feature_seq.rs:649-655` 的 `scan_trigger` 用它作扫描下界 `tail_lo = n - tail_window`；`bin/s_session_v2/tb02c.rs:195` 把它作为 `"tail_cache_window"` 输出，同时写死 `"resource_truncated":false`） | 写死，未登记 | T | 线段特征序列的扫描窗口属预算类，原文没有定论。**与正本冲突**：`.chanlun/definitions/xianduan.md:212-214` 以「theta_v0 无对应的第二个 `TAIL_WINDOW`」为理由撤销 G-5b，但 theta_v0 里确实有这个常数，而且在 S 会话和 π 路径上生效；`:216` 又承认「7→∞」没有可跑的 harness。#1493 主控裁定 1（票内评论）写了「防 TAIL_WINDOW=7 挪出候选」，说明主控知道它存在，但它没有像姐妹量 `second_seq_scan_window` 那样落成具名版本（见 `config.rs:58-61`、`xianduan.md:206`「预算是具名、版本化的调度参数」）。tb02c 把截断固定报成 false，覆盖标记因此不可信 | 7（现行），需登记出处。`segment.rs:76-78` 已登记一次 L2 实测（OKLO 2000 笔，`analysis/segment_refsem_cert.py`，7→∞ 237→237），但为单数据集、非 theta_v0 harness（`xianduan.md:216`）；须在 theta_v0 上重测，或耗尽时标 `INCOMPLETE_RESOURCE_BOUND`。PR #1582 订正后它是「有语义后果的调度截断」，受粘性阻断约束（尾窗不得挪出未决候选） | #1493 TB-02-X（正在改这一段状态机）、#1344/#1420（`second-seq-scan-budget/1` 先例）。改法：并入 `second-seq-scan-budget` 记录，作为下一个具名版本的字段；xianduan.md §5.2 撤票理由的订正已由 #1493 主控裁定 [ic-6282247099](https://github.com/xy7365527-lang/NewChanlun/issues/1493#issuecomment-6282247099) 第 4 项与在途 PR #1582（xianduan v2.4 §5.2 订正段）覆盖，不另开票 | 高 |
| C-05 | `rust/src/theta_v0/strategy/oscillation_campaign.rs:313` `self.short_diff.cost_basis().units() / 3`（`reduce_units`，`:308-314` 注「#348 的『固定 1/3』……原型口径零发明」） | 写死 | T | **同 candidates #10**，行号已复核。减仓比例是参数。旁证 `026-第26课.md:80`「但仓位可以控制，例如用其中的1/3，慢慢养成好习惯以后，就可以更随心所欲一点。」该行在 ↑正文 界（:40）之后，属每日解盘段，应标【解盘】而不是【正文】，作者是缠师本人，而且原文写明这是举例、可以调。ADR 0034:48 已把「oscillation /3」登记到 TB-10。主控订正影响：`oscillation.rs:36-39` `CenterOscillationConfig` 默认 `enabled: false`（ADR 0016 受影响清单在案），至今未进主线读数，影响由高降为中 | 1/3（现行；出处是 #348 和 026:80【解盘】） | TB-10（#1368）、#1386、#1558（量档签署）。改法：落成量档 policy 记录里的减仓比例字段；ADR 0017 已裁由 `w(d)×Q` 取代，故字段只在旧入口保留期内存在 | 中 |
| C-06 | `rust/src/theta_v0/venue_fee.rs:326` `uncalibrated_addon: f64`（`:315-327` `FeeQuoter`，addon 恒等于 `slippage_bps/1e4`）；`rust/src/theta_v0/config.rs:336-345` `ExecConfig::default()`：`entry_delay_bars 1`、`commission_bps 1.0`、`slippage_bps 2.0`、`tax_bps 0.0`、`fee_schedule None` | 字段未标定，默认值写在代码里 | T | **同 candidates #11**，行号已复核。另补 ExecConfig 其他四项：费率、滑点、税、入场延迟都属场所和执行参数，原文没有定论。ExecConfig 已是 config 字段，但默认值写在 `impl Default` 里，没有签署出处，只部分满足 ADR 0041 第三条 | 现行值；出处待补（commission 和 slippage 的来源在代码里查不到） | #1559 FU-01/05（四项成本输入签署）、#1561（场所 profile）、#917（fill 标定）。改法：移进场所 profile 或 ADR 0035 成本 policy 记录，`impl Default` 只用于测试 | 高 |
| C-07 | `rust/src/theta_v0/config.rs:161` `depth_weights: vec![0.60, 0.30, 0.10]`（`VoiceConfig::default()`，`:158-167`） | 写死 | E | ADR 0034:39 已把「常数表 `depth_weights`（资金帽语义已废）」列为被否方案；ADR 0041 第三条又点名它和代码常数同向；ADR 0034:48 把 depth_weights 登记到 TB-10。语义作废的东西不是自由度，应维持排除，等 TB-10 删除 | — | TB-10（#1368）、#1558。改法：随旧入口终态删除，不要转成可调字段 | 高 |
| C-08 | `rust/src/theta_v0/config.rs:238-252` `RiskConfig::default()`：`rho 0.005`、`beta 0.5`、`gamma 1.0`、`kappa 2.0`、`default_lot 1`、`chi_z_alpha 0.0`、`enforce_gross_cap false`、`enforce_level_cap false`；`:158-167` VoiceConfig 的 `max_depth 3`、`w_grade [1.0,1.0]` | 已是 config 字段，默认值写在代码里 | T | 都是风控和声部参数，原文没有定论。`enforce_gross_cap` 依 #937/#890 是条件式开关。没有签署记录或版本号，只部分满足第三条 | 现行值；每个字段要补出处 | #1557（共同 cap）、#1558。改法：并入 Θ_risk 签署记录，带版本号；`impl Default` 降级为测试夹具 | 高 |
| C-09 | `rust/src/theta_v0/config.rs:87` `l_max: 6`（LevelConfig `:84-90`）；`classifier/pipeline.rs:398-399`「自然终止：某级单元数 < `min_parts_per_level` …，或达 `l_max` 上界」、`:507`、`:760` `for level_idx in 0..=l_max`。同族：`rust/src/recursive_t/rec_engine.rs:94` `MAX_LEVEL = 8`、`trading/types.rs:28` `MAX_LADDER = 11`、`bin/p940_struct_input_abc.rs:71` `MAX_LEVELS = 6`、Python `orchestrator/recursive.py:75` `max_levels: int = 6` | 写死上界，截断时不加标记 | T | 塔高属于资源上界，原文没有设上限：级别递归本身没有终点。达到 l_max 时停止被当作「自然终止」，与「单元数不足」的真正终止没有区分，输出里也没有覆盖标记。数值本身是合法的资源参数，问题在于静默截断 | 6（现行），并补「达上界即标 `INCOMPLETE_RESOURCE_BOUND`」 | #1463（级别递归现状核验，已关）、#1360 TB-02、TB-10。改法：落成具名资源预算字段，和 C-04 一样按 #1344 先例处理；旧线常数随 TB-10 终态处理 | 高 |
| C-10 | `rust/src/theta_v0/config.rs:106-113` `MacdConfig::default()` `fast 12, slow 26, signal 9` | 已是 config 字段，默认值写在代码里 | T | 原文明说参数可选：`025-第25课.md:16`【正文】（↑正文界在 :54）逐字为「MACD的灵敏度，和参数有关，一般都取用12、26、9为参数，这对付一般的走势就可以了」，紧接着说太快速的走势要另看；同行的「(注：…)」已避开。所以这是有原文锚的默认值，参数本身是自由度。力度判据走生产判定 | 12/26/9（出处 `025:16`【正文】） | #985（G4 力度订正）、#1340。改法：落成力度 policy 记录字段，写明原文锚，并说明「太快速走势」时的代价 | 高 |
| C-11 | `rust/src/theta_v0/config.rs:65-72` ParseConfig `new_stroke_min_gap: 3` | 写死 | O | 新笔条件 2「至少有3根（包括3根）以上K线」，出处 `081-第81课.md:110`，逐字录在 `.chanlun/definitions/bi.md:101-109`；生产默认切新笔见 bi.md §5（:176）。归属提醒：该行在 081 的 ↑正文 界（:78）之后，bi.md:127 判为「续排博文正文，请人复核归属标签」。**附带说明**：bi.md:263 记有 081 答疑「两档并存……允许并存」，所以新笔与老笔之选可能本身是原文许可的自由度（T 候选）。另外 strict 档（gap≥5，bi.md:29，`[新缠论:选择]`）与 wide 档的关系归 #799 判，交主控裁 | — | — | 高 |
| C-12 | `rust/src/theta_v0/config.rs:65-72` `seg_min_strokes: 3` | 写死 | O | `xianduan.md:32/:39`；`077:62`「线段必须至少有三笔」 | — | — | 高 |
| C-13 | `rust/src/theta_v0/config.rs:88` `min_parts_per_level: 3` | 写死 | O | 中枢至少由三个次级别部件构成（zhongshu 正本），属定义 | — | — | 高 |
| C-14 | `rust/src/theta_v0/classifier/recursive_tower.rs:84` `UPGRADE_TOTAL_SEGMENTS = 9`（`:553` 按 `n/3` 重切） | 写死 | O | `033-第33课.md:16`【正文】（↑正文界在 :52）：「每3段构成一个5分钟的中枢」「中枢的延伸不能超过5段，也就是一旦出现6段的延伸，加上形成中枢本身那三段，就构成更大级别的中枢了」 | — | — | 高 |
| C-15 | `rust/src/trading/types.rs:41` `STOP_FRAC = 0.02`（`:40`「核心止损比例（stop_mode A/B）」） | 写死 | E | 这是按入场价比例止损。`.chanlun/definitions/fengkong.md:99-101`：「**退出条件** = 买入程序的判断条件被否定……止损是价格偏离的度量，退出条件是买入逻辑的否定」；`:123`「外部止损：入场价 - N点 / 入场价 × (1-N%) → 与走势结构无关」。承重依据是 ADR 0030 裁定一：结构退出来源 `[旧缠论]` `031-第31课.md:26`【正文】（界 `:32`）「除非证明你买入的理由没有了，技术上出现严重的形态，否则都不能抛一股」，按价格偏离 2% 平仓直接违此句；系统风险条件只授权非选择性动作（ADR 0030 `:21`），逐重价格止损不属该类。`fengkong.md` 为生成态 v0.1，其中只有 `:34` 是 #659 用户裁定（「无结构止损直引，退出语义全由买卖点/力度比较承担」），作旁证。维持排除；如果只作消融对照，应标为诊断臂 | — | TB-10（#1368）、#1386。改法：v2 收编时去掉 stop_mode A/B，或标成仅限诊断 | 中 |
| C-16 | `src/newchan/backtest/full_pipeline.py:253` `current_price < self._held_entry_price * 0.9` → `BUY_POINT_NEGATED`；`src/newchan/backtest/orchestrator.py:435-436` `price < self._sm.fsm.entry_price * 0.9` → STOP_LOSS | 写死 | E | 理由同 C-15：用 10% 价格偏离冒充「买点被否定」，标签名和语义也对不上。依据 fengkong.md:99-101、:123、:34 | — | TB-10（Python 旧入口）。改法：退役，或改成结构否定判据 | 中 |
| C-17 | `rust/src/theta_v0/config.rs:30` TickConfig `tick_size: 1e-8` | 默认值写在代码里 | T | tick 大小是场所事实，不同场所取值不同；1e-8 只是通用占位，应放进场所 profile | 1e-8（现行，只作未知场所的保守值）；生产必须用 profile 值 | #1561。改法：放进场所 profile | 中 |
| C-18 | `rust/src/theta_v0/backtest/data.rs:297` `if max_dev > 0.80`（#922 坏 tick 闸，`:285-288` 注：依 issue922 扫描，空档 0.40 到 0.999，取 0.80） | 写死 | T | 数据清洗阈值，有在案读数，原文不涉及 | 0.80（出处 #922 扫描和 #1013 影子评审） | #922、#1013（已关）、#1536 FU-14（完整历史数据 profile）。改法：落成数据 profile 字段，附扫描读数 | 中 |
| C-19 | 统计和验收常数：`backtest/metrics.rs:132` `cvar(…, 0.05)`、`:401` `percentile_ci(…, 0.025, 0.975)`、`:562` `p ≤ 0.05`、`:677` `1.96`、`:718` `ANN = 252.0`；`backtest/highlow_mu.rs:113` `(1.645, 0.05)`；`backtest/decontam.rs:13`「§4 冻结（`z_α=1.645`、`perm_α=0.05`…）」；`backtest/l3_delta_r_alpha.rs:1317/:1601` `z_alpha_lcb = 1.645`、`:2228-2229` ICC 0.3/0.7；`backtest/l3_pi_probe.rs:38` `(first_px * 1000.0).max(1.0e6)` | 写死 | T | 置信水平、显著性门槛、年化天数、探针资金都属验收口径参数。其中 decontam 的冻结值是预注册，应保留，只补版本出处 | 现行值（出处：PDF §7.3、§12，decontam §4 冻结） | #1358（经营评价参数）、#1468。改法：合并成一份「验收口径」版本化记录；预注册值原样登记，改值即新版本 | 中 |
| C-20 | λ 三处：`rust/src/spiral/params.rs:40` `LAMBDA = 2.0`（`:47` `SUB_SPAWN_FRAC = 1/LAMBDA`）、`rust/src/fugue_v3/mod.rs:120` `LAMBDA = 3.0`（`:129` MOBILE_FRAC）、**新增** `rust/src/trading/positional_fusion.rs:102` `pub(crate) const LAMBDA: f64 = 2.0;`（`:122` SUB_SPAWN_FRAC） | 写死，两族互斥 | T | **同 candidates #12**，并补第三处（初稿漏列 positional_fusion，它在 v2 现役树里）。#925 已裁 λ=2 的值层作废；#943 处理过两个互斥 λ。π 和 theta_v0 里没有 LAMBDA | 不给数值默认值（#925 值层作废）；如需保留，就按族分开设字段，不合并 | TB-10（#1368）、#1386、#1532。改法：旧线随终态删除；positional_fusion 如留在 v2，就落成带出处的字段 | 中 |
| C-21 | 成本门旧族重复常数：`SUB_COST_K 2.0`（spiral/params.rs:51、fugue_v3/mod.rs:132、trading/positional_fusion.rs:89）、`SUB_FRICTION_RT 0.001`（spiral :54、fugue_v3 :135、positional_fusion :90；`axiom_voice.rs:71` 用 `2*SUB_FRICTION_RT`）、`SUB_COST_Q 0.5`（spiral :57、fugue_v3 :138、trading/config.rs:584）、`SUB_COST_MIN_OBS 10`（spiral :60、fugue_v3 :141、config.rs:587）、`DEPTH_REF_WINDOW 50`（spiral :63、trading/depth_ref.rs:25）、`SUB_LIQ_FACTOR 2.0`（spiral :69、fugue_v3 :145） | 写死，多处重复 | T | 注释标为「35课成本门参数」L2。成本门语义是 `[旧缠论]`，但阈值、分位、样本门槛属参数。现役正本已有 ADR 0035 的签署 θ，这几组旧常数是平行副本 | 现行值（只作旧线回放用） | #1559、TB-10（#1368）、#1386。改法：v2 留用的部分改读 ADR 0035 policy 记录；旧线随终态删除 | 中 |
| C-22 | `rust/src/trading/types.rs:36` `DEFAULT_OP_LADDER = FIRST_BSP_LADDER + 1`（`:33-35` 注「操作者参数，#1278 L0 裁定……= move(L1)」）、`:43` `SUB_EXPIRY = 60` | 写死 | T | 操作级别由操作者选择（注释自认是参数）。注意坐标系：这里的 ladder 3 = move(L1)，而 #1557 Q5 的 ℓ_start 默认 L0 是走势级别坐标，两者不能直接比，签署时要写换算。SUB_EXPIRY 是确认过期的 bar 数，原文没有定论。`FIRST_BSP_LADDER = 2` 和 `LADDER_MOVE = 3` 是结构坐标，不算自由度 | ladder 3（现行，依 #1278）；SUB_EXPIRY 60 | #1557 Q5、#1278、TB-10。改法：并入 ℓ_start 签署字段，写明 ladder 和 ℓ 的换算 | 中 |
| C-23 | `rust/src/trading/config.rs:641-700` `OrganicConfig` 的 Default（`theta_amp 0.01`、`theta_cost_k 2.0`、`friction_rt 0.001`、`sub_friction_rt 0.001`、`sub_cost_k 2.0` 等，还有几十个开关）；`trading/positional.rs:34` `EQUITY_SAMPLE_BARS 1440`、`:36` `MIN_FILL_FRAC 0.01` | 默认值写在代码里 | T | v2 现役研究和回放用的参数，没有签署记录 | 现行值 | #1386 TB-10-C、#1385 TB-10-B。改法：收编时整组落成版本化记录；开关里如有同一判断的宽严两档，按 #799 剔除 | 中 |
| C-24 | `rust/src/buysellpoint.rs:24` `const TYPE1_CONFIRM_RATIO: f64 = 0.9;`（用在 `:229`、`:803`，`force_c/force_a <= 0.9`）；Python `src/newchan/a_buysellpoint_v1.py:95` | 写死 | T | 背驰力度比较的附加裕度，原文没有给数值。按 ADR 0041 判别句，「同一判据取不同参数值不算」两档，所以归参数。beichi.md:794 对黄白线回抽 0 轴的结算是「完全无人为阈值」，对照之下这里是人为阈值 | 0.9（现行，出处待补）；也可取 1.0（纯比较） | TB-10（旧线）。改法：随旧入口终态处理；若迁到 theta_v0，落成力度 policy 字段 | 中 |
| C-25 | `src/newchan/a_stroke.py:235` `min_strict_sep: int = 5`、`:237` `new_raw_gap_min: int = 3`；`src/newchan/live_engine.py:42`、`live_pool.py:47` `min_strict_sep: int = 5`；`live_pool.py:45` `max_symbols: int = 10` | 默认值写在代码里 | T | strict 档（gap≥5）在 bi.md:29 标为 `[新缠论:选择]`，比原文老笔严一根；默认值和生产裁定「默认切新笔」（bi.md §5）不一致。`new_raw_gap_min 3` 的取值属 O（同 C-11），但新/老/严的档位选择是可调的档位字段 | 依 bi.md §5 的新笔档；min_strict_sep 5 作为可选值保留并写代价 | #813（笔教义正本，已关）、TB-10。改法：Python 旧入口退役，或改读同一份笔档位记录 | 中 |
| C-26 | Python 回测资金和比例：`backtest/orchestrator.py:108` `initial_capital 1_000_000.0`、`:110` `short_diff_ratio 0.1`；`backtest/engine.py:90` `operation_level 1`、`:94` `position_fraction 0.1`；`cost/slippage.py:85` `slippage_pct 0.001`、`:132` `y 0.9`（Donier & Bonart 文献标定）；`trading/ph_fugue.py:306`、`trading/cost_reduction_fsm.py` `sub_ratio 0.3`；`orchestrator/recursive.py:75` `max_levels 6`；`a_level_detection.py:141` `min_gap_ratio 1.8` | dataclass 默认值 | T | 资金、比例、滑点、级别都属参数，原文没有定论；和 Rust 侧同类量取值不一致（资金 1e5、1e6 各有） | 现行值（只限 Python 旧回放） | TB-10。改法：随 Python 旧入口退役；保留的部分改读 #1557、#1559 签署记录 | 中 |
| C-27 | Python 模块级权重常数：`trading/position_sizing.py:71-73` `W_CONFIG 0.50`、`W_EDGE 0.25`、`W_SINGLE 0.08`；`strategy/k4_path_c.py` `W_*` 权重；`equivalence.py:59` `MIN_OVERLAP 5`；`topology/fiber_bundle.py:32-33`；`decomposition_tree.py:250/:253` `THETA_INCOMPAT 0.5` | 写死 | T | 仓位权重和研究阈值，没有出处 | 现行值 | TB-10。改法：随旧入口退役，不单独改造 | 低 |
| C-28 | trading/ 诊断和消融：`sublevel_confirmation_ablation.rs:28`、`consolidation_ablation.rs:25`、`trade_behavior.rs:31` `FLOOR_LADDER 2`；`gate_state_dump.rs:66` `L2_LADDER 4`、`:253` PREREG7 品种表 | 写死 | T | 诊断工具里的级别下限和预注册品种池 | 现行值（预注册表原样保留） | #1532 FU-12。改法：诊断 bin 输出时写出所用值即可 | 低 |
| C-29 | `.sandcastle` 模型档位和并发：`main.mts:27-29` `IMPLEMENTER_MODEL`、`REVIEWER_MODEL = "claude-opus-5-5"`、`EFFORT = "medium"`；`main-swarm.mts:17-23` 四个 k3 模型、`MAX_PARALLEL = 4`（#1007 裁定第 2 条：「并发上限 = 官方上游自用值 MAX_PARALLEL=4」）；`codex-child.ts:14-15` `MODEL`、`EFFORT = "xhigh"`；`decision-claimer.mts:26-27` 两档模型、`:31` `DEFAULT_MAPS = [1323, 1340]`、`:32` `MAX_CONCURRENT = 3` | 写死 | T | 调度选择，不涉及缠论语义 | 现行值（MAX_PARALLEL 出处 #1007） | #1007（已关）。改法：集中到 sandcastle 的一份配置记录里，改值留痕即可 | 低 |
| C-30 | `.sandcastle` 超时、轮询、陈旧阈值和重试：`main.mts:32` `MAX_ITERATIONS 3`、`:34` `MAX_TICKETS_PER_RUN 1`、`:36` `IMPLEMENTER_IDLE_TIMEOUT_SECONDS 1800`（#1066）、`:114-115` worktree 重试 2 次、间隔 5000ms；`claim-loop.mts:19` `DEFAULT_POLL_SECONDS 60`、`:23` `CLAIM_ROLLBACK_WATCHDOG_MS` 15 分钟（#1259）；`health-monitor.mts:19-21` `WAYFINDER_STALE_MS` 10 分钟、`CLAIM_STALE_MS` 48 小时、`DISK_MIN_GIB` 100（可用环境变量覆盖）；`decision-claimer.mts:33` `LOOP_INTERVAL_MS` 10 分钟 | 写死（部分可用 env 覆盖） | T | 运维参数，大多已在注释里写了票号出处 | 现行值 | #1066、#1259。改法：同 C-29 | 低 |


### 3.2 扫描方法与覆盖说明


**rg 和脚本模式**（自写辅助脚本 `/tmp/c_scan.py`：遇到 `#[cfg(test)]` 截断，跳过注释、assert、`tests/`、`bin/` 和文件名含 test 的文件）：

1. Rust 常量：`rg -n '^\s*(pub(\(crate\))? )?(const|static) [A-Z_0-9]+' rust/src` 共约 840 条，包括 &str 和环境变量名。限定为数值类型、非测试、非 bin 的顶层 const 后约 150 条，逐条看过。
2. `rg -l 'impl Default for' rust/src` 共 21 个文件，重点读了 `theta_v0/config.rs` 的全部 Default 和 `trading/config.rs` 的 OrganicConfig。
3. theta_v0 非测试浮点字面量 `[0-9]+\.[0-9]+` 共 684 条；去掉格式化、探针输出和诊断打印后剩 32 条，逐条看过（得到 C-17 到 C-19）。
4. 整数比例 `/ [2-9]\b` 共 48 条，得到 C-05 和 C-14。
5. `rust/src/economic_session/` 数值字面量 34 条，都是工程常数；`op_level` 和 `fixed_q` 来自输入，没有语义自由度。
6. S 会话 bin（`bin/s_session_v2|v3`、`s_structure_session.rs`）的 const 都是工程常数，只有 `tb02c.rs:15` `ROUND_BUDGET = 0`，它已合规，属于 `second-seq-scan-budget/1`。S 会话的语义参数全部来自 `ThetaConfig::default()`，所以并入 theta 条目。
7. Python：模块级大写常数 `^[A-Z_]+\s*(:\s*\w+)?\s*=\s*[0-9]` 共 16 条；dataclass 和函数签名数值默认值 `^\s+\w+:\s*(int|float)\s*=\s*[0-9]` 共 329 条。抽查了资金、比例、阈值、级别、笔档几类（C-16、C-25 到 C-27），其余作为同类归入这几条，没有逐条列。
8. `.sandcastle`：`rg -n '^(export )?const [A-Z_0-9]+' -g '*.mts' -g '*.ts' -g '!*.test.*'` 共 104 条，其中有数值或档位语义的合并为 C-29、C-30；其余是路径、URL、标签、环境变量名和 schema。
9. 对照正本：`docs/adr/`（0016、0018、0034、0035、0041）、`.chanlun/definitions/{bi,xianduan,beichi,fengkong,zhongshu}.md`；原文锚只引 `docs/chanlun/text/blog/` 下的 025、026、033、081、077，按四步核过。

**排除的类别（不是自由度，不列）**：
- 结构计数：`theta_v0/strategy/mutex.rs:73` `M=10`、`channel.rs:71` `CHANNEL_M=8` 是谓词个数；`trading/types.rs:30/:32` `FIRST_BSP_LADDER`、`LADDER_MOVE` 是结构坐标。
- 定点量化和数值容差：`coverage/sizing.rs:195` `J_SCALE 1000`、各类 `FLAT_EPS`、`EPS`。
- 工程安全窗和容量：`segment_layers.rs:177/:278/:280`、`buysellpoint.rs:987/:989` 窗口 256；`trading/types.rs:69` `MAX_REV_DEPTH 4`。
- 已合规的：`config.rs:58-61` `second_seq_scan_window: 0`，属 `second-seq-scan-budget/1`，`xianduan.md:206`。
- 已裁的旧线：`rust/src/segment.rs:410` `TAIL_WINDOW 7`、`:413` `MAX_SECOND_SEQ_SCAN 50`（xianduan.md:208-214、X-12/X-13，归 TB-10）；Python `a_segment_v1.py:290/:369`（X-20）。
- 测试代码：`venue_fee.rs:971-972` 的 ADDON/FALLBACK 在 `#[cfg(test)]`（:518）之后；`runner_tests.rs`、`econ_positive/tests.rs` 里的 `nav`、`1.645`。
- `.sandcastle` 的路径、URL、launchd 标签、密钥变量名：只记名字，不读值，也不输出值。

**覆盖缺口**：
- Python 329 条默认值没有逐条核，只按类别抽样。
- `rust/src/bin/` 下研究 bin 的 const 只抽查了 S 会话和 `p940_struct_input_abc.rs:71`，其余研究探针没有逐个看。
- `src/` 之外的 `scripts/`、`analysis/` 不在本分片范围内。
- C-11 附带的「新笔与老笔之选是否属原文许可的自由度」需主控另裁（见第六节）；C-04 指出的 §5.2 不符已由 #1493 裁定与 PR #1582 覆盖，不另开票。


## 四、与 candidates 初稿的映射

| candidates | 本审计 | 结论 |
|---|---|---|
| #1 INITIAL_CAPITAL | C-01 | T，同 |
| #2 几何衰减 | A-20 | **E**（初稿：改可调待判） |
| #3 五(c) 溢出分拆 | A-03 | **E**（初稿：改可调；「部分保持」已在 ADR 0034 裁定七行情换档下可签） |
| #4 ADR 0029 C/D/B | A-22（B、C、D）、A-08（E 超限即拒、O3） | **C 判 E**（初稿 C 改可调）；新增 E、O3 为 T |
| #5 ADR 0030 | A-07（R2）、A-24、A-25 | R2 为 T；「一律 S1/S4」判 E（现行已是可配置，属正面先例） |
| #6 ADR 0035 | A-10（AMP_EXEC、占比阈值）、A-27 | 中位数读法判 E（违 ADR 0022 裁定三） |
| #7 ADR 0034 | A-09、A-26 | 按市值触发与探针自动准入为 T；增长并入 Q 为 E |
| #8 ADR 0018 k 缓冲 | A-05 | T，同；touch 仅诊断 |
| #9 ADR 0028 profile | A-06 | T，同 |
| #10 `/3` | C-05 | T，影响订为中 |
| #11 `uncalibrated_addon` | C-06 | T，扩为整组执行成本 |
| #12 两处 LAMBDA | C-20 | T，补第三处 `positional_fusion.rs:102` |
| #13 ADR 0036 槽位默认值 | A-28、B-10 | 被否项维持 E；张力按 B-10 解 |
| #14 笔作 a₀（终版已移入维持排除） | A-40 | O |
| #15 `resolve()` 锚为键 | — | 非自由度，不列 |
| #16 挪钱 | A-21 | E，同 |

## 五、非作者独审（抽查复跑）

审查员：claude-fable-5-1（经 CLIProxyLocal `127.0.0.1:8318`，只读，与起草方均非同一会话）。记录原样存同目录 `review-round1-fable-5-1.md`、`review-round2-fable-5-1.md`。

- **第 1 轮**抽 13 条（A-01、A-03、A-20、A-22、A-40、B-01、B-02、B-10、C-02、C-03、C-04、C-10、C-15）逐条复跑出处与归类：**BLOCK**。出处 FAIL 3（A-03 行号漂一行；B-01 把票评论里的标签当成正本标签；C-04 改法未查到 #1493 裁定与在途 PR #1582 已覆盖），归类不同意 2（A-03 应判 E，与 A-20 同一口径；B-01 在补标签前不得记 O），另 7 项补证或措辞要求。
- 作者按 12 项逐条修订：A-03 改判 E 并同步计数与排名；B-01 改「待补标签」不计入三类；口径节撤回自加的「Lean 管边界」类；C-02 删去「回放基数备用值」（与 C-03 同形）；C-04 改引 #1493/PR #1582 与 `segment.rs:76-78` 已有 L2 实测；C-15 改以 ADR 0030 裁定一 + `031:26`【正文】承重；B-10 补注 #1557 第 7 版已撤回、改通则须用户点头。
- **第 2 轮**针对性复审：12 项全部落实，新增出处行号与逐字全部命中，计数、排名、映射独立点数一致；另新抽 3 条（A/B/C 各一）出处 3/3 PASS、归类 3/3 同意 ⟹ **PASS-with-notes**。所余 1 项残句与 1 项非阻塞措辞（B-03 理由）已改。
- 合计复跑 **16 条**（≥10 条门槛），第 1 轮出处失败率 3/13；针对这三类错误（行号漂移、票内标签冒充正本标签、改法未查在途票），作者另对正本层全部行号与决策票层的标签来源做了自查，订正 B-09 行号一处；代码层未抽条目没有逐条重核，可信度以分片自述与 16 条抽查为限。

## 六、遗留与限制

- 代码层 Python 329 条 dataclass 默认值按类别抽样，`rust/src/bin/` 研究探针只抽查 S 会话与 `p940_struct_input_abc.rs`；推论层 `analysis/` 不在票面范围内。
- `xianduan.md:212-214` 撤票理由与 `theta_v0/parser/segment.rs:79` 不符（C-04），已由 #1493 裁定与 PR #1582 覆盖。需另票处理的一处正本标签：`qujiantao.md:791` 的 Extreme `[新缠论:选择]` 标签早于 #814 D-2，应订正（A-41）。
- C-11 附带的问题未裁：081 答疑「两档并存」是否使新笔/老笔之选成为原文许可的自由度。本审计按现行正本（生产默认新笔，`bi.md` §5）判 O，留待笔正本复议。
- 各 T 项的改法只是建议；按 ADR 0041 代价第 4 条，既有裁定不追溯自动改写。
