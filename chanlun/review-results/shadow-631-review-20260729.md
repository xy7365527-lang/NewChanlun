# 影子评审：#631 `level_origin` 空转字段删除（claude/opus，新上下文独立复现）

- 日期：2026-07-29｜票据：issue #631｜对象：`d9f1860124`（14 文件）+ `7b589a7347`（ID-6.5 登记补件）
- 工位：`/tmp/kimi-nest-mainline`（分支 `kimi-nest-mainline-20260717`，HEAD=`7b589a7347`）
- 评审车未参与实装；全程单线程、只读 + 跑测试，未改任何源文件、未做任何 git mutation
- 开工 `git status` 自核：并行线未提交面 4 modified + 8 untracked（含 `rust/src/bin/p100_cert_bsp_recon.rs`），全程未触碰；`retrace_ledger/`、`incremental.rs`、`level_view*` 仅只读查阅

---

## 结论（先行）

**改动本体正确，行为零变化实证充分——但文档/声明面存在 HIGH 缺口，建议回票补两条后收。**

代码删除面干净、完整、零行为变化，且证据强度高于实施件自述（本评审补跑了 250k 对拍，dump SHA 与 #497/#630 历史记录逐位一致）。缺口全部落在「声明与实际一致」轴（090 严格性语法规则的声明膨胀条）：

**根因一条**：先例核对只取了 main #455 的首提交 `2d1abf9786`，**漏了同一票的回票修复 `11b1a02bbe`**（`docs(classifier): #455 回票修复——UL 文档 level_origin 退役墓碑 +「无消费者」口径限缩`）。main 影子评审 #464 已就该处置提出 HIGH+MED 两条并已修复；本次在本分支**逐字复现了被回票前的两处缺陷**。

| 级别 | 条目 | 状态 |
|---|---|---|
| HIGH | `UBIQUITOUS_LANGUAGE.md:11,44` 仍把已删字段当现行定义 | 未收口（与 main 回票前逐字相同） |
| MEDIUM | 「无消费者」口径过宽（`bsp.rs:194`、`projection.rs:71`、登记件） | 未限缩（与 main 回票前逐字相同） |
| MEDIUM | 「全仓 62 处」数字沿用先例，本分支实际 48 处 | 数字未复核 |
| LOW | 「正主已实填」未加门控限定（默认配置下投影层不构造） | 措辞精度 |
| LOW | 与 main #455 文件面差异（18 vs 14）来源未说明 | 已由零残留+编译反证覆盖，仅缺注 |

---

## 一、独立复现结果（7 项验收，逐条）

### 1. 零残留 ✓

```
grep -rn "level_origin" rust/src → 仅 signal.rs:3415/3423/3424/3428/3431（5 处历史链注释）
全仓非 .md 文件 → 同上，无第二处
```
字段定义、48 处构造点、`PartialEq` 分量全清。

### 2. 删除完整性反证 ✓

字段已删 ⟹ 任何遗漏消费点必编译失败。实跑（release）：

```
cargo build --lib --release        → Finished in 10.12s（37 warnings，均为既有 dead_code/naming）
cargo test --release --lib --no-run → Finished in 20.55s（53 warnings）
cargo test --release --lib          → 2205 passed / 0 failed / 138 ignored
```
均为真编译（非缓存重放：耗时 10s/20s 量级 + 产出 warning 全集，非 `0.0Xs Finished`）。

### 3. GOLDEN 交叉印证 ✓✓

```
cargo test --release --lib extract_signals_bit_exact
  extract_signals_bit_exact_vs_orig_per_case ... ok
  extract_signals_bit_exact_digest_guard     ... ok   (2 passed / 0 failed)
```
新值 `0xe371_3897_d9bf_978c` 与 main #455（`2d1abf9786`）实跑值**逐位一致**（已直接 `git show` 核对该提交 diff 的 `+const GOLDEN` 行）。两独立分支同一改动同一摘要 = 强交叉印证。

`digest` 计算函数本身未被改动（`signal.rs` diff 仅 4 个 hunk：三处 `make_*_point` 构造 + GOLDEN 常量/注释），排除「改算法迁就常量」。

per-case 对拍 `assert_eq!(opt, ori)` 走 `PartialEq`——删除的是恒真分量（`0 == 0`），判定强度无实质放松。

### 4. 无夹带 ✓

两纯态源树（`git archive d9f1860124^ / d9f1860124`）`diff -rq` 结果**恰为 14 个文件**，与 `--stat` 一致，无第 15 个文件被动过。逐 hunk 阅读：全部属 `level_origin` 面（字段定义 + 构造点 + `PartialEq` 分量 + 三处诚实注释 + GOLDEN 常量）。

### 5. 无意图误伤 ✓

- 引入提交 `bbbd8f89fa` commit message 自述：「注：bsp.rs/signal.rs 内 #110 level_origin 族（字段+构造默认 0）**顺带入库**（同文件不可分）」——字段非该票本意，删除不违背在案意图。
- 正主实填证据：`projection.rs:177` `identity: LevelIdentity { level }`，`level` 来自 `pipeline.rs:368` `build_level_projection(config, levels.len() as u32, ..)` 与 `incremental/tower.rs:305` `level_ordinal as u32` —— 真实级别下标，非常量。
- **同名不同物澄清**（本评审新增核对）：`admission.rs` 曾有函数参数 `level_origin: usize`（语义「候选实际所在级别」，有真实消费 `level_origin+1`），已于 `d2312352f9` 独立消失，与本次删除无关，未被误伤。
- **附加安全核**：`BspPoint` 仅 `#[derive(Debug, Clone, Copy)]`，无 `serde`/`pyclass` ⟹ 无外部序列化契约破坏。
- **附加消费面核**：全仓无 `HashSet<BspPoint>`/`dedup` on BspPoint；跨级访问一律 `levels[ℓ].bsp` 按下标取，级别由容器位置承载 ⟹ `PartialEq` 收窄无行为面。

### 6. 对拍抽点 ✓✓（强于实施件自述）

两纯态各自独立 `CARGO_TARGET_DIR` 全量 release 编译 `p123_fast_replay`，产物 md5 相异（确认真编译）。

| 规模 | stdout | dump | stderr |
|---|---|---|---|
| 20k（`P116_MAX_BARS=20001`） | 零 diff | 零 diff（83 行） | 唯一差异 `prefix_s` 0.107→0.104（墙钟） |
| 250k（`P116_MAX_BARS=250000`） | 零 diff | SHA-256 **逐位相同** | 唯一差异 `prefix_s` 33.132→33.153（墙钟） |

250k dump SHA = `614b83323271752a0b569d11f3efb4ea2e472524b584f1570c323b03ca62421f`，与 `issue497-impl-20260728.md`（`614b83…`）、`issue630-levelview-slim-impl-20260729.md`（`614b8332…62421f`）记录值一致 —— **跨三票同值**，行为零变化在更长时间轴上稳定。所有语义计数器（triggers/reevals/reuses/pan_*/wm_cross_without_lower=0 等）逐位相同，与 #69/#497 墙钟豁免口径一致。

> **方法论提示（本评审实测踩到）**：两个纯态源树若共享同一 `CARGO_TARGET_DIR`，第二次编译会 `Finished in 0.16s` 且产物 **md5 与第一次完全相同**——cargo fingerprint 未识别源树切换，对拍变成「自己跟自己比」的假绿。必须为每个纯态用独立 target dir（本评审即因此重编）。实施件自述的「临时 worktree + patch 隔离」若共享了主 target，其零 diff 需补自证；本报告的对拍已用独立 target dir 独立成立，可作为替代证据。

### 7. 登记闭环 ✓

ID-6.5 三时段齐：
1. `owner-attribution-fix-readings-20260724.md` §5（center 面，`0x90c7…` → `0xe6a2…`）
2. `issue610-id65-supplement-20260729.md`（level_origin 引入面，补记同一提交第二因）
3. `issue631-id65-supplement-20260729.md`（level_origin 删除面，`0xe6a2…` → `0xe371…`）

且 #610 补记「边界条件」明文预告了删除路径须第三次登记、不得静默放行——#631 已履行。`signal.rs` 历史值五级链（`0xe6a2→0x90c7→0x56ed→0x37d2→0x06b3`）与各登记件一致，且显式声明「不自定义 Debug 补回假字段掩盖翻转」。

---

## 二、发现（分级）

### HIGH-1：`UBIQUITOUS_LANGUAGE.md` 未收口——已删字段仍是现行定义

`UBIQUITOUS_LANGUAGE.md`（仓根领域词表，非历史评审报告，属 living 文档）两处仍以 `level_origin` 为现行事实：

- 第 11 行「**级别身份** | 买卖点携带的所属级别（level_origin），是查询的起点。在正确实现中应该是"买卖点所属走势的级别"…」
- 第 44 行 Flagged ambiguities：「…(2) 买卖点的 level_origin（所属级别）…当前 (2) 设为 (1) 的值…**这是一个待修正的实现口径**」

字段已删，词表却仍声明它存在、且是「查询的起点」「待修正的实现口径」。这与实装正面冲突（090 声明一致性），并正中票面自列的第三害：「未来真要用时，恒 0 现状会让接入方误判『已有数据』」——现在变成更糟的版本：**接入方会误判该字段仍存在**。

**这条在 main 上已被判 HIGH 并已修复**：`11b1a02bbe` 把两处改为删除墓碑（「现行实现中是参照系视图，非买卖点自身的存储事实；唯一存储事实是 `LevelProjectionLayer.identity.level`…已由 SPEC #455 在 commit `2d1abf9786` 删除」）。本次未吸收。

> 注：main 的 `CONTEXT.md`「级别身份」词条在本分支不存在（本分支无 `CONTEXT.md`），故本分支的对应义务仅 UL 两处。

**修复建议**：照 `11b1a02bbe` 的墓碑措辞改写 UL:11 与 UL:44，commit 号改引本分支的 `d9f1860124`。

### MEDIUM-1：「无消费者」口径过宽，与本 commit 自身证据自相矛盾

现行注释三处（`bsp.rs:194`、`projection.rs:71`，及 `issue631-id65-supplement` 表格）称该字段「全仓恒为 0、**无消费者**」。但同一 commit 的核心证据链正是「删除 ⟹ Debug 串少一枚常量 ⟹ FNV 摘要翻转」——即该字段**有** Debug/digest 机械消费者，且 `PartialEq` 中有它的恒真分量。

commit message 正文用的是准确的限缩表述（「全仓无任何跨级 BspPoint 去重/比较依赖该字段」），但落到代码注释与登记件时收缩成了无限定的「无消费者」。

**这条在 main 上已被判 MED 并已修复**：`11b1a02bbe` 限缩为「无**下游级别语义**消费者，仅有恒真 equality 分量及 Debug/digest 机械消费」。本次未吸收。

**修复建议**：`bsp.rs:194`、`projection.rs:71` 两处照 main 限缩措辞改写；登记件同步。

### MEDIUM-2：「全仓 62 处」数字与本分支实际不符

commit message 与 `issue631-id65-supplement` 均称「全仓 **62** 处硬编码常值 0」。实测：

- 本次 diff 删除的 `level_origin: 0` 构造点行 = **48**
- main `2d1abf9786` 删除的 = **62**

62 是 main 先例的数字，被直接沿用未复核。两分支代码已分叉（main 侧另有 `fill.rs`/`interval_necessity.rs`/`compose_tests_1,2.rs` 等构造点，本分支无对应出现），本分支实际就是 48 处。

**修复建议**：登记件数字改 48，并注明与先例差值来源为分支分叉。

### LOW-1：「正主已实填」缺门控限定

`projection.rs:140` 的 `LevelProjectionLayer::from_level` 只在 `config.level_projection.enabled` 为真时被调用（`pipeline.rs:190` 早退；`config.rs:425` 注明「默认关闭（= 链死）⟹ stamping 不构造 `LevelProjectionLayer`」）。故「同信息两份拷贝其一恒空」在默认配置下更准确的说法是「两者都不存在」。

不影响删除结论（删的是恒 0 无信息字段），但「正主已实填」宜写作「门开时正主实填真实级别下标（`levels.len()`/`level_ordinal`），门关时级别身份本就由容器位置承载」。

### LOW-2：与 main #455 的文件面差异未注明

登记件称与 main #455「同构」，但两票文件面为 18 vs 14。差异来源是分支分叉（本分支相应文件无 `level_origin` 出现），已由「零残留 grep + `--lib` 编译通过」双重反证覆盖，非遗漏。建议登记件补一句说明，避免后续核对者误判为漏删。

---

## 三、结果包六要素

**1. 结论**：`d9f1860124` 的代码删除面正确、完整、零行为变化（20k/250k 双规模对拍 + 全量 lib 测试 + 两纯态源树 diff 三路独立证），GOLDEN 重锚诚实且与 main 独立实跑值逐位一致。但文档/声明面有 1 HIGH + 2 MEDIUM 缺口，全部源于「先例核对漏了 main #455 的回票修复 `11b1a02bbe`」。**判定：PASS-WITH-FINDINGS，建议回票补 HIGH-1 + MEDIUM-1（MEDIUM-2/LOW 随手一并）后收。**

**2. 定义依据**：
- 「级别身份 = 参照系视图，非存储事实」——main `CONTEXT.md`「级别身份」词条（`2d1abf9786` 建、`11b1a02bbe` 修）；本分支对应载体为 `UBIQUITOUS_LANGUAGE.md:11,44`。
- 「删除零损失」判据 = ①字段全仓恒 0（48 处构造点全硬编码 `0`，终装点从未写）②无下游级别语义消费者（`levels[ℓ].bsp` 容器位置承载级别；无 `HashSet<BspPoint>`/`dedup`）③正主 `LevelProjectionLayer.identity.level` 门开时实填。三条经本评审独立复核成立。
- 「声明与实际一致」= `.claude/rules/no-patch-mentality.md`「声明膨胀」条（090 严格性语法规则）——HIGH-1/MEDIUM-1 依此判级。
- 「ID-6.5 漂移登记」= `owner-attribution-fix-readings-20260724.md` §5 确立的受控流程。

**3. 边界条件**（结论翻转条件）：
- 若 `config.level_projection.enabled` 默认转为开启、且出现把不同级别 `BspPoint` 混入同一集合去重/比较的消费点，则 `PartialEq` 删除该分量会产生真实行为差（当前不成立：无此类消费点）。
- 若 250k dump SHA 在其他机器/工具链上不复现 `614b8332…62421f`，则「零行为变化」证据退回 20k 级。
- 若 `bbbd8f89fa` 之外存在未在案的 #110/#111 spec 明文要求 `BspPoint` 自带级别副本，则「无意图误伤」翻转为「误伤在案意图」——本评审在仓内未找到此类明文，UL:44 的「待修正的实现口径」是最接近的一条，但它描述的是**未实装的期望**而非在案实装契约，删除一个从未承载该语义的恒 0 字段不构成能力损失（惟须以墓碑保留该期望，即 HIGH-1）。
- 若实施件的对拍确系共享 `CARGO_TARGET_DIR`，其自述证据失效——但本报告的独立 target 对拍已独立成立，结论不翻转。

**4. 下游推论**：
- `BspPoint` 上不得再加同形级别字段（墓碑一旦落地即为显式禁令）；未来若要实装 UL:44 的「买卖点所属走势的级别 ≠ 产出它的 bsp 列表级别」语义，须以新 issue 走实装路径（写入路径 + 非零测试），不可复活恒 0 字段。
- `extract_signals_bit_exact_digest_guard` 的 GOLDEN 链已达五级；任何再触 `BspPoint` Debug 串形状的改动须第四次登记（`issue631-id65-supplement`「边界条件」已钉）。
- 250k dump SHA `614b8332…62421f` 现已跨 #497/#630/#631 三票同值，可作为后续重构票的行为基线锚。

**5. 谱系引用**：
- `.claude/rules/no-patch-mentality.md`（090 严格性语法规则，「声明膨胀」禁令）——HIGH-1/MEDIUM-1 判级依据。
- `.claude/rules/formalization-validity-domain.md`（有效域 ≠ 定义域）——LOW-1 的「正主已实填」缺门控限定属同型（在门开域成立的声明写成了全域声明）。
- ID-6.5 漂移登记流程（`owner-attribution-fix-readings-20260724.md` §5 → `issue610-id65-supplement` → `issue631-id65-supplement`）。
- main #434 grilling → #455 处置（`2d1abf9786`）→ #464 影子评审回票（`11b1a02bbe`）：本票为该谱系在本分支的独立复现，**回票修复段未被复现**即本报告主发现。

**6. 影响声明**：本报告为只读评审产出，未改动任何源文件、未做任何 git 操作。落盘一个新文件 `chanlun/review-results/shadow-631-review-20260729.md`。评审过程在 `/tmp` 下建了 4 个临时目录（`sh631-base`/`sh631-post`/`sh631-target`/`sh631-target2`）与若干 dump/stdout 文件，均在工作树外，不入仓。被评审对象 `d9f1860124`/`7b589a7347` 未被改动。

---

## 四、未验证项（090 如实）

- 仅跑 `--lib`；`--tests`（集成测试 target）与 `--bins` 全量未跑——但字段删除的编译面反证已由 `--lib` + `p123_fast_replay` bin 编译共同覆盖（后者含 lib 全链）。工作区有并行线未提交的 `p100_cert_bsp_recon.rs`，为避免污染未跑 `--bins` 全量。
- 对拍数据面仅 BTC 1m（`analysis/data_cache/btc_1m_full.json`）两个规模；跨标的未跑（本改动为字段删除，无标的相关分支，风险可忽略）。
- main `#464` 影子评审报告原件未在仓内检索到，两条 finding 的级别（HIGH/MED）取自 `11b1a02bbe` 的 commit message 自述。
- 未复核 `bbbd8f89fa` 引入时 #110/#111 的 spec 原件（仓内 `.chanlun/`/`docs/` 未检索到对应 SPEC 文本）；「意图在案」判断依据为该提交 commit message 自述 + UL:44 的 flagged ambiguity 两条间接证据。
