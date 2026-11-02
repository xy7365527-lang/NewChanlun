> 正本：本文件是旧引擎与简化臂模块级终态的正本。
> 最后裁定：#1532（2026-10-22）
> 受影响代码清单：见「一、模块级终态清单」与「四、名分锚点与注释订正」节

## 问题

#1323 Destination ③ 要求：「每条旧引擎／简化臂逐一落为收编、冻结影子臂或删除；冻结臂不得参与生产准入和交易决策；#1096 不再图外悬空」（#1323 票面 :33）。ADR 0039（#1530）已经在**入口级**定了终态，并在裁定五分工表里把下列事项明文划给本票（`docs/adr/0039-production-entry-inventory-and-unique-launcher.md:306-309`）：

- **模块级终态**：旧引擎和简化臂的内部函数、FFI 导出，逐项定（:306）。
- **产出**：每个入口的替代入口细节、逐函数收编去向（落进哪个 owner）、退场条件与时间表、删除执行（含 `live/runner.py`），以及 `generation-constitution.md:35` 锚点和 `Cargo.toml` 「CLI 生产入口」注释的更新（:307）。
- **改动权限**：「只能把『冻结影子』收窄为『删除』，不得让任何入口重新取得 (a)(b)(c)」（:308）。
- **默认启动不触发旧核**：本票提供逐条清单，由 #1413 按清单逐条核（:309）。

SPEC C11.4 要求每个条目「附替代入口、失去的生产写权、保留用途和退场条件；用途未知先调查，不直接删除」（#1340 票面 :353）。

**本票的边界**：
- 不改入口级终态。
- 不裁名分轴：ADR 0039:299 已把代码谱系名分和生产 owner 分开处理。
- 不碰带仓迁移：归 FU-13 #1535（ADR 0039:310）。
- 不替 #1096/#1304 做退役取舍：#1323:194 写明「#1096/#1304 义务不提前删除」；主控重写评论 6310613611 已裁定 `recursive_t` 走路径 (a)，回到冻结影子。

**考古标出、本次取证坐实的冲突**（裁定段逐条处置）：

1. **Q2 名分轴自相矛盾**。三处说法互相打架：
   - `rust/src/recursive_t/DEPRECATED.md:1`「# recursive_t 已退役」、`:3`「**状态**：DEPRECATED（已废弃，仅保留历史参考）」。这份文件由 commit `97ec16d8ed`（2026-10-17）引入，没有挂裁定票；`:28` 引用的 `.chanlun/review-results/recursive-t-retirement-plan-20261017.md` 在仓内找不到。
   - 同一目录的 `mod.rs:92` 写「名分：现役」。
   - `lib.rs:29` 写「spiral/fugue_v3/recursive_t/trading 四族（#761/#762/#763 已」（下一行接「判现役」）。

   推论层的 `old-engine-retirement-20261017.md:125`「方案 A：**删除** `recursive_t`」，又与 #951 resolution 5291240738「recursive_t 本体冻结不删」、主控路径 (a) 方向相反。另外，#1654 票面 :14 把「保留还是随退场删除，在本票内定并留证据」写进了自己的范围，和本票的模块级终态重叠。AGENTS.md:36 规定「分歧只能存在于已开的票里，不能只存在于注释里」。
2. **Q2/Q16 与 #808 判死条件冲突**。#808 resolution 评论 5138174069 第「③ 退场条款正文」第 1 条（:39）原文：「**死于『消费者一个不剩』，不死于『`theta_v0` 更好』。** D1 继任者 PyO3 出口暴露目标净敞口 + per-leg 账本（现 **0/2**）；D2 九个消费者逐个有归宿；D3 一组对拍留档、**不要求 bit-exact**」。同一评论「未做」第 3 条（:84）写：「§2.C『没人要了』全是【推断】……已写进条款 D2 要求人显式表态关闭，**未代答**」。#808 后续评论 5224166201 写：「本票的判死条件、接不了 4 项、没人要 4 项、冻结口径全部原样有效」。上一稿默认「analysis/ 研究脚本不计入任何退场判据的调用面、依赖退场后不可运行是预期」，与 D2 直接冲突：`recursive_t` 当前的 Python 消费者共 10 件，其中 9 件就在 `analysis/` 下。
3. **Q3 冲突**。#1385 票面 :12「SUPPORT_BY_ACTUAL_CALLER先核用途，不自行改成删除或任意三选一」如果读成「禁止 #1385 执行任何删除」，就和 ADR 0039:230「删除由 FU-12 逐依赖验收后执行（X6）」相撞。
4. **Q9 冲突**，共四处：
   - **行号漂移**：ADR 0039:299/:307 写的是「`Cargo.toml:54`」，main `66c409e7e3` 上实际在 `rust/Cargo.toml:68` 与 `:85`。
   - **漏点名**：`rust/src/theta_v0/mod.rs:118` 是第三处「CLI 生产入口」，ADR 0039 没有点名。
   - **正本层词汇冲突**：`CONTEXT.md:364` 仍把「**最小生产路径 CLI** 指 `theta_backtest`……」列为「生产」三义之一，而 ADR 0039:209 已把 `theta_backtest` 定为冻结影子，:268 写明「`theta_backtest`……都不是 launcher」。
   - **过时表述**：`generation-constitution.md:35`「v1/dual = deprecated 待退役，处置见 #499」已过时：#499 已 CLOSED，`theta_v0/backtest/runner.rs:188/:190` 注明 `run_theta_v0`/`run_theta_v0_dual`「已按 #499 裁定退役删除」。
5. **Q10 冲突**。AUDIT.md:189 和评论 6285963138 把 C-28 四个文件称作「诊断与消融 bin」，但代码里它们是 `#[cfg(test)]` 模块（`rust/src/trading/mod.rs:66-67/:71-72/:89-90/:93-94`），`rust/Cargo.toml` 里也没有对应的 `[[bin]]`。
6. **Q19 冲突**。ADR 0039:214/:296 引用的 `nautilus/mod.rs:69-83` 在 main 上没有 `rust/src/nautilus/mod.rs` 这个文件，实际门控在 `rust/src/theta_v0/nautilus/mod.rs:69-83`（经 `theta_v0/mod.rs:127` `pub mod nautilus;` 挂载）。
7. **行号基线**。ADR 0039 清单以 `b63a3fd586` 为基线（ADR 0039:192）。当前 PyO3 导出块在 `rust/src/lib.rs:2815-2841`：
   - organic 五项：:2815-2819
   - spiral：:2821-2822
   - fugue_v3：:2824-2825
   - recursive_t 三项：:2826-2827、:2832-2833、:2835-2836，各带 `#[cfg(not(test))]`
   - `PyThetaStream`：:2838
   - `PyBiEngine`/`PyOnlineMacdState`/`PyRecursiveOrchestrator`：:2839-2841

   本票引当前行号，终态不变。
8. **Q12 考古命中数与调用面分类有误**。考古记 `rg -ln "RecursiveOrchestrator" src/ scripts/` 命中 10、「全部是研究或回放用途」。main `66c409e7e3` 实跑命中 **63**（src/ 22、scripts/ 41），其中有服务路径：`gateway.py`、`live_engine.py`、`live_pool.py`、`replay.py`、`orchestrator/timeframes.py`。按查询壳 import 闭包求（种子 gateway/server/realtime_server/live_engine/live_pool，含延迟 import），闭包内引用旧判定核的文件为 10 个引用者加本体 `orchestrator/recursive.py`。处置：B-Q12 与裁定一·E 的退场条件改为对闭包内 10 个引用者加种子 server.py 做 rg，并把 #1384 OE03（REST/WS replay 与 TFOrchestrator）、OE04 一并加进合取。

## 权威依据

1. **#1323 票面**
   - :33 Destination ③（原文见「问题」段）。
   - :41「N-1 技术类决策预授权……T 类 grilling 票经代理起草+独审通过即裁定」。
   - :194「#1096/#1304 义务不提前删除」。
2. **#1340 票面（SPEC C11）**
   - :351 C11.2「唯一结构核及可复用的严格构造收编；旧 Python 回放/session/API 外壳迁为同核 adapter，内部第二套判定冻结为无生产权影子或删除；旧 π fill/organic/简化策略按对象功能逐项收编或退役，不能整体更名后继续产许可。」
   - :353 C11.4「每个条目仅有收编、冻结影子、删除之一的终态，附替代入口、失去的生产写权、保留用途和退场条件；用途未知先调查，不直接删除。冻结影子只输出明确带 provenance 的比较读数，不能影响准入、资金或订单。」
   - :354 C11.5「切换先禁止旧程序新发送……新旧不双写」。
   - :355 C11.6「#1096 回到其完整票面核合法终态，必须证明相关旧回路失去生产权且现存责任有去向。未读部分、影子仍有下游写权或默认 launcher 仍触发旧核时，本组不通过。」
3. **ADR 0039**（`docs/adr/0039-production-entry-inventory-and-unique-launcher.md`）
   - :174「只读观察、研究读数、测试驱动本身不是生产入口。但凡能触到 (a)(b)(c) 之一的路径，清单里都要有终态。」
   - :186-188 三态口径：
     - 「**收编**：进入唯一 owner、X 或只读 adapter，失去独立写权。」
     - 「**冻结影子**：保留文件，只产带 provenance 的比较读数，不得触及 (a)(b)(c)。」
     - 「**删除**：执行由 FU-12 逐依赖验收，见 X6；本 ADR 不自动授权删文件。」
   - :190「入口内部『正确部分』逐函数收编进哪个 owner（ADR 0027:62），属于模块级判定，归 FU-12（见裁定五）。模块级收编不改变入口终态，也不给入口带回写权。」
   - π 线表：
     - :208 parser/classifier 收编 → S
     - :209 `theta_backtest` 冻结影子
     - :210 `pi_theta_fill_loop{,_voice,_overlay}` 冻结影子，`pub(super)` 不扩
     - :211 其余 20 个 `backtest_bin` 门内探针 bin 冻结影子
     - :212 42 个无 required-features 的 bin 冻结影子
     - :213 `ThetaPiStream`/`PyThetaStream` 冻结影子
     - :214 nautilus 旧决策链冻结影子
   - organic 线表：
     - :220 organic 五项冻结影子
     - :221 spiral/fugue_v3 冻结影子
     - :222 recursive_t 三项冻结影子
     - :223 `PyRecursiveOrchestrator`/`PyBiEngine`/`PyOnlineMacdState` 冻结影子，「不得成为 S 之外的第二份 Classification」
   - Python 线表：
     - :229 `backtest/runner.py` 冻结影子
     - :230 `live/runner.py`「**删除**……删除由 FU-12 逐依赖验收后执行（X6）」
     - :231 `databento_live_runner.py` 冻结影子
     - :232 LMT-only 和 maker 撤单逻辑「能否逐函数收编进 X adapter，归 FU-12」
     - :233 `theta_rec_strategy.py` 冻结影子
     - :234 `broker_config.py`/`risk_controls.py`「可收编资产……逐函数收编去向归 FU-12」
     - :235 研究回测脚本冻结影子
   - service 表：
     - :241 查询壳收编为只读 adapter
     - :242/:244 外壳内部第二套判定冻结影子
     - :243 `ws_live` 收编
     - :245 OE41/42/45 不纳分母
   - 直发表：:251-253 三行冻结影子。
   - :268「全仓只有这一个 launcher。`theta_backtest`、`trading_system/live/runner.py`、`newchan` CLI、任何 `*_launcher.py` 都不是 launcher。」
   - :272「`evidence_class` 只决定 adapter 装配：`simulation` 装受控撮合」。
   - :277「launcher 从 profile 的 `credential_scope` 指向的来源读取凭据，**只**写进 X 子进程的启动环境」。
   - :292-297 三层锁。
   - :298「标记载体是本 ADR 清单加 FU-12 的模块级清单，不另建台账」。
   - :299「锚点 `pi_theta_fill_loop_overlay` 的名分更新，和 `Cargo.toml:54`『CLI 生产入口』旧注释的改写，归 FU-12 落模块级终态时一并提交，注释须带票号」。
   - :300「影子臂不接在生产路径的任何 if-else 上（#799 判别式），失败只能是失败（ADR 0027:32），不设 legacy 开关」。
   - :306-310 裁定五分工表。
   - :314-318 裁定六迁移步骤，其中 :316「FU-12 #1532 落模块级终态清单；TB-10-I #1413 按清单逐条核默认启动只到新核」，:318「任何一步失败都按失败处理，不回退旧 writer」。
   - :334-337 验收 A4 写权闭包 diff。
   - :355-361 验收 B10-B13（#1523 交付 launcher，#1383 两条测试）。
4. **ADR 0027**（`docs/adr/0027-classification-chong-operating-program-ownership.md`）
   - :32「任何结构事实或操作状态都只能有一个生产 owner。失败必须作为失败处理，不能切到另一套判据、旧入口或宽松路径重新判定。」
   - :46「历史回放、paper 与 live 必须共用同一个结构判定域实现和同一个重操作程序实现。」
   - :62「现有 organic 赋格实现是可收编资产，不因本裁定自动成为正本。正确部分进入唯一重操作程序后，旧独立入口必须失去生产决策权或退役。」
   - :64「π 内重复拥有交易、成交或账本状态的入口同样必须迁移或退役。迁移期间不得用双 owner、回写结构主干或失败兜底维持兼容。」
5. **ADR 0028**（`docs/adr/0028-first-capability-verification-profile.md`）
   - :46「失败即拒绝发送，不回退旧 writer」。
   - :72「旧 Python `trading_system/config/broker_config.py` 与 `trading_system/execution/risk_controls.py` 为可收编资产（ADR 0027 裁定五），不自动成为正本。」
6. **ADR 0030**（`docs/adr/0030-exit-authorization-and-liability-retirement.md`）:44「无法连接场所是失败态（保留最大责任、重试、不切判据、不回退旧 writer），不是另一种政策。」
7. **ADR 0012**（`docs/adr/0012-inference-layer-document-naming.md`）:65「**不追溯更新**：一条推论被裁定覆盖之后，推论层那份原文**不动、不改、不加批注**。」
8. **ADR 0021**（`docs/adr/0021-venue-selection-deferred.md`）:13「三条下单链**没有一条 live 跑过**；`trading_system/live/runner.py` 自述『阶段 4+ 骨架，当前不可运行』」。
9. **ADR 0041**（`docs/adr/0041-degrees-of-freedom-configurable-not-excluded.md`）
   - :9「本通则只管**自由度**：正本标为 `[新缠论:选择]`、或属参数类且缠师原文无定论的量」。
   - :21「签署门字段的默认值 ＝ `AwaitingPolicy`」。
10. **CONTEXT.md**
    - :14-15 重操作程序词条「_Avoid_: 设置第二个重操作 owner；以旧入口或另一套判据接管失败」。
    - :363-364「生产」拆三义词条（待订正，见「问题」4）。
11. **AGENTS.md**
    - :36「注释可以写，但**必须带票号**；分歧只能存在于已开的票里，不能只存在于注释里」。
    - :55 #799 限定词②「诊断口径可更宽……但**不得参与准入**」。
    - :58「看两套判据**是不是接在同一个 if-else 上**」。
    - :68 #804「① 规则落在语义，不落在代码形状」。
    - :89「**写得出退场条件**——举证书**必须回答『这条缝在什么条件下消失』**」。
    - :100「代码注释只留一行指路且必须带票号」。
12. **`docs/agents/generation-constitution.md:35`**「**π 是唯一现役引擎**。锚定方式 = 家族根函数名（`pi_theta_fill_loop_overlay`），不锚行号（行号随版本漂移）。v1/dual = deprecated 待退役，处置见 #499。」
13. **`.sandcastle/docs/graph-engineering.md:23`**：T 类 =「数据结构、接口与 seam、owner 落点、状态机与事件 schema、测试锁、迁移步骤、恢复语义」。
14. **OLD-ENTRY-DISPOSITIONS.md**（`.chanlun/review-results/issue1339-r2-spec-20260909/view/sources/newchanlun-1339-architecture-20260908/author/`）
    - :25 X6「旧观察/隔离对照可长期保留；文件删除是后续逐依赖验收事项，不由本候选自动授权」。
    - :183「冻结为有独立身份的隔离对照；从生产装配撤除旧权威，不自动删文件」。
    - :192「旧账户历史、未完成 T/voice 与可能外部尾债先做身份映射和可回放记录，再撤销旧发送 epoch」。
15. **`.chanlun/definitions/beichi.md:486`**「嵌套深度 `recursive_t/divergence.rs:72-81` ｜ **冻结**，移交 #859；解冻前不许出现在任何生产判定路径上」。
16. **`.chanlun/review-results/dof-audit-20261020/AUDIT.md:189`** C-28 行，列值依次为：写死 ｜ T ｜ 现行值（预注册表原样保留） ｜ #1532 FU-12；改法：诊断 bin 输出时写出所用值即可。
17. **票评论**
    - #808 resolution 5138174069（xy7365527-lang，2026-07-31）③ D1-D3 判死条件、「未做 3」（原文见「问题」2）。
    - #808 5224166201（xy7365527-lang，2026-08-08）「判死条件……冻结口径全部原样有效，#951 逐条引用不重推」。
    - #951 resolution 5291240738（xy7365527-lang，2026-08-14）「recursive_t 本体冻结不删（设计稿 §4.3/§5.6 推荐）」。
    - #1286 裁定 5458813825（xy7365527-lang，2026-08-28）「**不删代码**：与旧引擎逐位等价，保留为回归参照系；analysis/ 分析脚本消费不受影响」。
    - #1532 主控重写 6310613611（2026-10-21）「`recursive_t` 改回**冻结影子**，配机械退场条件（#1096 CLOSED ∧ #1385 对应条款满足），不在本票删除；其余条目沿用已 PASS 内容」。
    - #1532 分级 6284962135 / 6311051879。
    - #1586 指针 6285963138。
    - #1323 N-1 原话 6261281111。
18. **其他票面**
    - #1096 :8-9：「勘察……4 处调用者逐处承重」「整体退役，或按 #951 执刀线口径登记收编（明确退场条件），不许静默并存」。
    - #1385 :12（SUPPORT_BY_ACTUAL_CALLER）、:15 OE07、:16 OE08、:17 OE09、:18 OE10、:19 OE11、:20 OE12、:21 OE13、:22「#1096 rec_t_strategy/recursive_t/旧engine残余全查」。
    - #1384 :15-20 OE01-06。
    - #1386 :15-17 OE15-17。
    - #1388 :17 OE32。
    - #1390 :15 OE14、:17 OE35。
    - #1413 :7 默认 launcher 只装配新完整程序、:11 实际部署运行默认启动、:13「源码形状检查不能替代真实调用阻断」、:14「默认启用须明确批准」。
    - #1654 :14、:21「删除须附 #808 消费者归宿证据」。
    - #1550 :4「#1304 外壳如何调用完整生产程序（正式 launcher）」。
19. **代码事实**（main `66c409e7e3`）：见裁定一各表「文件:行」列。

## 候选

各待决点都不涉及缠师原文语义，也没有 `[新缠论:选择]` 或参数类自由度：
- 三态是 SPEC 合同（#1340:353）。
- 冻结还是删除，属于治理归属（ADR 0039:308、#1323:194），不是版本化记录里的可调值。
- C-28 的常量在 `#[cfg(test)]` 套件里，不在生产判定路径上。
- `risk_controls` 阈值的取值不在本票裁，未签署前按 ADR 0041:21 为 `AwaitingPolicy`。

所以 ADR 0041:9 的「可调字段 + 默认值」不适用。下面 B 类点按「默认值——确认或否决」呈现，推理链照考古重过。

**B-Q2 `recursive_t` 模块级终态**：默认值 **冻结影子，本票不收窄为删除**。删除前置：(i) #1096 CLOSED；(ii) #1385 :19 OE11 与 :22 两条勾选；(iii) #808 ③ D1-D3 由 #1096 路径逐条举证，其中 D2 须人显式表态，本票不代答。（出处：ADR 0039:222/:308；#1323:194；#1340:355；#951 5291240738；#808 5138174069/5224166201；OE11 :183；主控 6310613611）——确认或否决。

推理链：
- [新缠论:推论] ADR 0039:308 只**允许**收窄，不强制收窄。是否删除，等于在 #1096:9 的「整体退役 / 登记收编」之间二选一，这是 #1096 自己的义务；#1323:194 又要求不提前删。
- [新缠论:推论] #808 ③ 把判死写在消费者上（D2），执刀给了写 `theta_v0` PyO3 出口的票（5224166201 指 #951）。#951 已关，留下的是「本体冻结不删」（5291240738）。剩下退役还是收编的选择权在 #1096，所以 D1-D3 是 #1096「整体退役」分支的前置，由 #1096 承接。本票只在 #1096 留评论指针，不改它的票面，也不替它判 D2。
- [新缠论:选择] #1654 只能在冻结影子终态内改 `A0Source::Stroke` 的注释和名分；如果要删 `Stroke` 档，以本条删除前置为前提（与 #1654:21「删除须附 #808 消费者归宿证据」同向）。
- `DEPRECATED.md` 的「已退役」属于名分轴，本票不改它的文字。只新增一行，带票号写明生产终态（ADR 0039:299 名分与 owner 分开；AGENTS.md:36）。
- 考古给的默认是改写 `DEPRECATED.md` 措辞，本稿取「只加一行」，理由同上。

**B-Q3 `live/runner.py` 删除的执行票与依赖清单**：默认值 **执行票 = #1385（OE13，票面 :21）**。依赖清单：代码 import 零命中（`rg -n "live/runner|live\.runner|from trading_system\.live|from \.live" rust src tests trading_system scripts --glob '!*.md'` 退出码 1），文字依赖为 `trading_system/README.md:18/:67/:100` 三处和 `trading_system/live/__init__.py:1` docstring。（出处：ADR 0039:188/:230/:307；ADR 0021:13）——确认或否决。

推理链：[新缠论:推论] 终态已由 ADR 0039:230 定，本票只定由谁执行、依赖有哪些。#1385:12「不自行改成删除」约束的是终态未定、用途未知的条目；`live/runner.py` 的终态已裁为删除，不在该约束之内。`live/runner.py:61` 只 print「实盘入口为阶段4骨架，尚未实装。」，`:69` `return 1`，没有尾责面。

**B-Q5 替代入口映射**：默认值 **按功能写七值域之一**：
- 结构事实 → S
- 账与放行判据 → E
- 重状态 → Bᵢ
- 下单/撤单/凭据 → X adapter
- 回放/回测/runner 壳 → `production_session`（`simulation` profile）
- 查询 → 只读 adapter
- 纯研究对照 → 「无（非生产职责）」

（出处：ADR 0039:186/:194-202/:208/:241/:272；#1340:351；AGENTS.md:68）——确认或否决。

**B-Q7 退场判据形式**：默认值 **每条只由三种二值判据合取而成**：
- 票状态：`gh issue view N --json state --jq .state` 输出 `CLOSED`
- 勾选：`gh issue view N --json body --jq .body | grep -c '^- \[x\] <条目首词>'` 输出 `1`
- 调用面：`rg` 输出为空或退出码 1

不写日期，不写「视情况」。（出处：AGENTS.md:89；ADR 0039:328-364 验收形式）——确认或否决。

**B-Q8 时间表形式**：默认值 **顺序闸门 T0-T5，不写日期**。（出处：ADR 0039:312-318；#1413:14「不因测试通过或归档合入自动切换」）——确认或否决。

**B-Q9 注释与锚点订正的措辞和落点**：默认值 **随本 ADR 落地 PR 一并提交**（ADR 0039:299「一并提交」）。订正对象为 `rust/Cargo.toml:68/:85`、`rust/src/theta_v0/mod.rs:118`、`docs/agents/generation-constitution.md:35`、`CONTEXT.md:364`；`recursive_t` 的两处指路行也一并加。新文字一律不锚 ADR 行号，写「ADR 0039 裁定二」。（出处：ADR 0039:299/:307；AGENTS.md:100；domain-modeling SKILL.md:44「Challenge against the glossary」）——确认或否决。

推理链：上一轮独审 R1 指出，把行号写进正本和注释会带入漂移错误；generation-constitution.md:35 自己也写「不锚行号」。`generation-constitution.md:35` 管的是谱系名分（ADR 0039:299），所以保留「π 是唯一现役引擎」这句，只补「不是生产 owner 裁定」的指路，并订正 v1/dual 的状态。

**B-Q10 C-28 处置**：默认值 **不进旧入口分母**，改称「trading/ 消融守卫测试套件（`#[cfg(test)]`）」。处置为：留用；`FLOOR_LADDER = 2`、`L2_LADDER = 4` 及 `PREREG7` 原样保留；各套件输出须写出所用值；不参与准入。（出处：ADR 0039:174；AUDIT.md:189；AGENTS.md:55；`trading/mod.rs:66-94`）——确认或否决。

**B-Q11 `ThetaPiStream`/`PyThetaStream`**：默认值 **冻结影子，本票不授权删除**。退场条件 = #1385 :16 OE08 勾选 ∧ #1413 CLOSED ∧ #1550 CLOSED；删除由 #1304 路径定。（出处：ADR 0039:213；#1323:194；`rust/src/bin/theta_replay.rs:1/:4`；#1550:4）——确认或否决。

推理链：`theta_replay.rs:1` 自述「#1305 ①件（#1304 第一批）」，`:4` 以 `ThetaPiStream::push_bar` 作回放 driver，属于 #1304 的义务件。

**B-Q12 查询壳内第二套判定**：默认值 **冻结影子，承 #1286 不删代码**。查询壳的服务路径替代入口 = S 只读 adapter（#1384）。退场条件 = #1384 :17（OE03 REST/WS replay 与 TFOrchestrator）、:18（OE04）、:19（OE05）、:20（OE06）四条全部勾选 ∧ #1413 CLOSED ∧ 查询壳 import 闭包内不再有任何文件引用旧判定核。两条命令：
- `gh issue view 1384 --repo xy7365527-lang/NewChanlun --json body --jq .body | grep -cE '^- \[x\] OE0[3456] '` 输出 `4`；
- `rg -l "newchan\.orchestrator\.recursive|RecursiveOrchestrator" src/newchan/gateway.py src/newchan/server.py scripts/realtime_server.py src/newchan/replay.py src/newchan/orchestrator/timeframes.py src/newchan/live_engine.py src/newchan/live_pool.py src/newchan/ab_bridge_newchan.py src/newchan/nested_pipeline.py src/newchan/a_level_fsm_adapter.py src/newchan/a_nested_divergence.py` 输出为空。

文件清单 = 以 `gateway.py`、`server.py`、`scripts/realtime_server.py`、`live_engine.py`、`live_pool.py` 为种子，沿 `newchan.*` 的 import（含函数内延迟 import）求传递闭包，在 main `66c409e7e3` 上闭包共 78 个文件，其中引用 `RecursiveOrchestrator` 或 `newchan.orchestrator.recursive` 的 11 个（10 个引用者加冻结影子本体 `orchestrator/recursive.py`）。命令列的 11 个文件 = 这 10 个引用者，加种子 `server.py`（它本身不引用旧核，列入是为了挡住它将来新增直接引用）。间接路径 `gateway.py:57-58` → `orchestrator/timeframes.py`、`replay.py`（`replay.py:13-15` 直接构造 RecursiveOrchestrator）因此在册；只删 `gateway.py:34-35` 的直接 import 不会让判据通过。#1384 实施若改动查询壳 import 结构，须按同一种子重求闭包并把新增文件补进本条命令（补列不改判据）。

（出处：ADR 0039:223/:242/:244；#1286 5458813825）——确认或否决。

**B-Q13 新增原生 blocking 边**：默认值 **新增 #1532→#1385、#1523→#1413、#1383→#1413；不新增 →#1096、→#1304**，改在 #1096、#1654 留评论指针。（出处：ADR 0039:309/:316/:355-361；实查边）——确认或否决。

**B-Q14 fugue_v3/spiral/organic 退场顺序**：默认值 **三族都冻结影子，退场按 `use crate::` 依赖链串写：recursive_t → fugue_v3 → spiral → trading**。（出处：ADR 0039:220-222；ADR 0027:62）——确认或否决。依据的代码依赖：
- `recursive_t/` 六个文件以 `use crate::fugue_v3` 开头引入 fugue_v3；
- `fugue_v3/` 五个文件引入 `crate::spiral`；
- fugue_v3（10 个文件）、spiral（9 个）、recursive_t（6 个）都引入 `crate::trading`。

**B-Q15 `backtest_t_fugue.py`**：默认值 **与 B-Q2 同命：冻结影子，不单独删**；`trading_system/data_cache/` 的 24 个 `t_fugue_*.json` 同批处理。（出处：ADR 0039:235；`backtest_t_fugue.py:1` FROZEN(#951)）——确认或否决。

**B-Q16 `analysis/` 研究脚本**：默认值 **不进旧入口分母，本票不改不删**（ADR 0012:65；ADR 0039:174）。但它们是 #808 意义上的**消费者**：
- 对 `recursive_t`：9 件 `analysis/*.py` 计入 D2 的「逐个有归宿」，归宿须人在 #1096 路径中显式表态，本票不代答（#808 5138174069 :39/:84）。
- 对其他冻结影子：退场后的删除执行票须列出 `analysis/` 中的消费者并逐个记录归宿（#808 ④「判死只能写在旧的消费者上」），不得以「依赖退场后不可运行是预期」代替。

——确认或否决。

**B-Q18 正本落点**：默认值 **docs/adr/ 新 ADR，编号落地时按 main 顺序分配**；CONTEXT.md 只做 Q9 的词义订正。（出处：ADR 0039 同系先例；domain-modeling SKILL.md:64「It is a glossary and nothing else」）——确认或否决。

**B-Q19 nautilus 旧决策链与 Python 执行/风控资产的收编去向**：默认值如下（出处：ADR 0028:72；ADR 0039:214/:232/:234/:277/:285；#1385 :15/:17）——确认或否决。
- `lmt_executor`/`maker_optimizer` 的纯转换与撤单策略函数 → X 内 adapter，在 X 内受控调用；
- `risk_controls` 中能写成纯谓词的部分 → E 放行判据，阈值取值 `AwaitingPolicy`；
- `broker_config` 不收编为凭据源，凭据只经 launcher 注入 X；
- nautilus `order_adapter` 的纯转换 → X 内 NT adapter；
- `theta_strategy`/`theta_pi_strategy`/`backtest_engine` 的决策与装配部分不收编。

**A 类（已裁，直接沿用）**：
- Q1 模块级清单载体：ADR 0039:190/:298/:306-307。
- Q4 收编唯一形式 = 替换式：ADR 0027:62/:64；ADR 0039:186/:300；先例 `rust/src/economic_session/book.rs:4`。
- Q6 核验清单交 #1413：ADR 0039:309；#1413:13。
- Q17 失败处理：ADR 0027:32；ADR 0028:46；ADR 0030:44；ADR 0039:318。

## 裁定

本条依 #1323 Notes N-2 由主控代裁（B 类默认值）；分级 T 部分依 N-1。本票只做模块级终态清单、替代入口、owner 落点、退场条件、时间表与核验清单的技术取舍，依据全部来自 ADR 0027/0028/0030/0039、#1340 已采纳合同与 #1323 Destination ③。本票不改任何入口级终态，不裁名分轴，不碰资金、场所、账户、权限，也不替 #1096/#1304 定退役取舍。

正本落点：docs/adr/0048-legacy-engine-module-level-dispositions.md

ADR 0048 编号写死：origin/main `66c409e7e3` 最大 0046，预留表 adr-reservations.json 最大 46，同批 #1531 取 0047、本票取 0048 并已登记预留。文件首三行按 ADR 0039:1-3 先例写正本头（`> 正本：本文件是旧引擎与简化臂模块级终态的正本。`）。

### 一、模块级终态清单（main `66c409e7e3`）

**通用规则**
- 每行只取 C11.4 三态之一。
- 「替代入口」按功能取 B-Q5 的七值域。
- 「退场条件」是这条缝消失的机械判据（AGENTS.md:89），满足它**不等于**授权删除。删除另由执行票逐依赖验收（X6，ADR 0039:188），并逐个记录消费者归宿，`analysis/` 也要记（B-Q16）。
- 所有冻结影子行自本 ADR 起，不得接在 S/E/B/X 的任何 if-else 上（ADR 0039:300），输出须带 provenance（ADR 0039:298）。
- 表中「`#1385:19✓`」一类写法表示：`gh issue view 1385 --json body --jq .body | grep -c '^- \[x\] <该行条目首词>'` 输出 `1`。

**A. `recursive_t`（OE11）——冻结影子，不收窄为删除**

| 模块/符号 | 文件:行 | 终态 | 替代入口 | 失去的写权 | 保留用途 | 退场条件 | 删除执行 |
|---|---|---|---|---|---|---|---|
| `pub mod recursive_t`（15 项：`DEPRECATED.md`、`center.rs`、`divergence.rs`、`ffi.rs`、`mod.rs`、`operator.rs`、`prove_guards.rs`、`rec_driver.rs`、`rec_engine.rs`、`rec_stream.rs`、`stream.rs`、`t_engine.rs`、`t_engine_run.rs`、`trend.rs`、`types.rs`） | `rust/src/lib.rs:66-67`（`#[cfg(not(test))]`） | 冻结影子 | 结构事实→S；目标敞口与账→E/Bᵢ；流式驱动→`production_session` | 不持任何域 `.writer.lock`/`writer_epoch`、不持凭据、不产交易意图（ADR 0039:292-297） | 历史对照；24 个 `t_fugue_*.json` 的复现依赖 | `gh issue view 1096 --json state --jq .state` = `CLOSED` ∧ `#1385:19✓`（OE11）∧ `#1385:22✓`（#1096 全查） | 不由本票授权。归 #1096 路径；前置为 #808 ③ D1-D3 由 #1096 逐条举证，D2 须人显式表态，本票不代答 |
| `run_recursive_t` | `recursive_t/ffi.rs:76`；导出 `lib.rs:2826-2827` | 冻结影子 | 同上 | 同上 | 批量复现 | 同第一行 | 同第一行 |
| `PyTFugueStream`（`TFugueStream`） | `ffi.rs:221`；导出 `lib.rs:2832-2833` | 冻结影子 | `production_session` | 同上 | `backtest_t_fugue.py` 的流式壳 | 同第一行 | 同第一行 |
| `PyRecStream`（`RecTStream`） | `ffi.rs:296`；导出 `lib.rs:2835-2836` | 冻结影子 | `production_session` | 同上 | 对照 | 同第一行 | 同第一行 |
| 嵌套深度 `divergence.rs:72-81` | `.chanlun/definitions/beichi.md:486` | 冻结（教义层已裁，移交 #859） | S | — | — | 承 beichi.md:486，本票不动 | — |
| `trading_system/backtest_t_fugue.py` + `data_cache/t_fugue_*.json`（24 个） | `:1` FROZEN(#951) | 冻结影子（研究回测） | `production_session` simulation | 不读凭据、不开域库（ADR 0039:235） | 复现 | 同第一行 | 同第一行，同批处置，不单独删 |

当前消费者（#808 D2 的分母事实，供 #1096 核对）：`git grep -ln "RecTStream\|TFugueStream\|run_recursive_t\|PyRecStream" HEAD -- '*.py'` 命中 10 件，即 `analysis/capture_ratio_matrix.py`、`t3_exit_trigger_diag.py`、`t_vs_v3_comparison.py`、`tc_churn_{diagnostic,discriminant,exhaust,reason,safety_margin,stoploss_probe}.py`，加上 `trading_system/backtest_t_fugue.py`。

**B. π 线（`theta_v0`）**

| 模块/符号 | 文件:行 | 终态 | 替代入口 / 收编去向 | 失去的写权 | 退场条件 |
|---|---|---|---|---|---|
| `parser::ParseLayerIncr`、`classifier::*` | `rust/src/bin/s_structure_session.rs:27-28` 已接通 | 收编 → S（ADR 0039 裁定二） | S 是唯一计算者 | — | 无（正本） |
| `pi_theta_fill_loop`/`_voice`/`_overlay` | `theta_v0/backtest/fill.rs:750/:795/:4777`（`pub(super)`） | 冻结影子 | 账与 fill 纯函数只以替换式进 E/Bᵢ（先例 `economic_session/book.rs:4` 调 `ledger_step_components`）；不搬状态 owner | 只被 `backtest_bin` 门内 runner 调用；可见性不扩 | `#1388:17✓`（OE32）∧ #1413 CLOSED |
| `run_theta_v0_pi`/`_chi`/`_chi_shrink`/`_overlay` | `theta_v0/backtest/runner.rs:262/:277/:297/:549` | 冻结影子 | `production_session` simulation | 同上 | 同上行 |
| `theta_backtest` bin 及其余 20 个 `backtest_bin` 门内 bin | `rust/Cargo.toml:85-89` | 冻结影子 | `production_session` simulation | `backtest_bin` 不进 `production_session` feature | `#1390:17✓`（OE35）∧ #1413 CLOSED |
| 42 个无 required-features 的 bin、`lav_seal_samples` | ADR 0039 裁定二 | 冻结影子 | 无（非生产职责） | 不在生产二进制集合 | `#1390:17✓`（OE35）∧ #1413 CLOSED |
| `ThetaPiStream::push_bar` + `PyThetaStream` | `theta_v0/stream.rs:117`；`theta_v0/ffi.rs:59`；导出 `lib.rs:2838` | 冻结影子 | `production_session`（#1304 外壳接入由 #1550 裁） | 生产进程不加载 cdylib | `#1385:16✓`（OE08）∧ #1413 CLOSED ∧ #1550 CLOSED；删除归 #1304 路径（#1323:194） |
| `trading_system/strategy/theta_rec_strategy.py`、`trading_system/backtest/theta_backtest_py.py`、`tests/test_theta_pyo3_bridge.py` | `ThetaStream` 的 Python 调用面 | 冻结影子 | 同上 | 生产宿主无凭据 | 同上行 |
| `theta_strategy.rs:136`、`theta_pi_strategy.rs:161` `submit_order`；`backtest_engine.rs:141` `run_theta_backtest`；`order_adapter.rs` | 门控 `rust/src/theta_v0/nautilus/mod.rs:69-83` | 冻结影子 | 决策与装配部分不收编；`order_adapter` 纯转换函数 → X 内 NT adapter | `nautilus` 不进 `production_session` feature | `#1385:17✓`（OE09）∧ #1413 CLOSED |

**C. organic / spiral / fugue_v3——冻结影子，退场按依赖链串写**

| 模块/符号 | 文件:行 | 终态 | 收编去向 | 退场条件 |
|---|---|---|---|---|
| `PyFugueV3Stream`、`run_fugue_v3` | `fugue_v3/ffi.rs:264/:342`；导出 `lib.rs:2824-2825` | 冻结影子 | 结构事实类纯函数只进 S；账与重状态纯函数只以替换式进 E/Bᵢ | A 表退场条件已满足 ∧ `rg -l "^use crate::fugue_v3" rust/src --glob '!rust/src/fugue_v3/**'` 输出为空 ∧ `#1385:18✓`（OE10）∧ #1413 CLOSED |
| `PySpiralStream`、`run_spiral` | `spiral/ffi.rs:283/:359`；导出 `lib.rs:2821-2822` | 冻结影子 | 同上 | fugue_v3 行已满足 ∧ `rg -l "^use crate::spiral" rust/src --glob '!rust/src/spiral/**'` 输出为空 ∧ `#1385:20✓`（OE12）∧ #1413 CLOSED |
| `run_organic`、`run_positional`、`run_recursive`；导出 `PyOrganicTape`/`run_organic_rust`/`run_recursive_rust`/`run_positional_rust`/`PyUnnStream` | `trading/runner.rs:313`、`positional.rs:1066`、`recursive_position.rs:140`；`lib.rs:1610/:1956/:2121/:2187/:2610`；导出 `lib.rs:2815-2819` | 冻结影子 | 可收编资产（ADR 0027:62）：结构事实只进 S，账与重状态纯函数只进 E/Bᵢ，不搬 owner | spiral 行已满足 ∧ `#1386:15✓`/`:16✓`/`:17✓`（OE15-17）∧ #1413 CLOSED |
| `trading_system/backtest_fugue_v3.py`、`backtest_unn.py`、`backtest_unn_stream.py`、`backtest_spiral_stream.py`、`compare_spiral_unn.py`、`analyze_fugue_v3_losses.py`、`scripts/batch_backtest.py`、`scripts/av_recursive_run.py` | ADR 0039:235 | 冻结影子（研究回测） | `production_session` simulation | 各随所调导出的那一行退场 |

**D. Python `trading_system/`**

| 模块/符号 | 文件:行 | 终态 | 替代入口 / 收编去向 | 退场条件 / 执行 |
|---|---|---|---|---|
| `trading_system/live/runner.py` | `:61` print 骨架、`:69` `return 1` | **删除**（ADR 0039:230 已定） | `production_session` | 执行票 **#1385**（OE13），一个提交完成：删文件；`trading_system/README.md:18/:67/:100` 改指 `production_session`；`trading_system/live/__init__.py:1` docstring 改为「历史：实盘骨架已删（ADR 0048，#1532）」。逐依赖验收：执行前重跑 B-Q3 的 rg，退出码须为 1 |
| `trading_system/backtest/runner.py` | ADR 0039:229 | 冻结影子 | `production_session` simulation | `#1385:21✓`（OE13）∧ #1413 CLOSED |
| `strategy/chanlun_strategy.py` + `execution/lmt_executor.py:70` + `execution/maker_optimizer.py:89` | ADR 0039:232 | 冻结影子 | LMT-only 下单转换与 maker 撤单的**纯函数**（无 strategy 句柄，不调用 `submit_order`/`cancel_order`）→ X 内 adapter；策略经济判断不收编 | `#1385:15✓`（OE07）∧ #1413 CLOSED |
| `config/broker_config.py:79` | 读 `HYPERLIQUID_PRIVATE_KEY` | 冻结影子 | **不收编为凭据源**：凭据只经 launcher 从 profile `credential_scope` 注入 X（ADR 0039:277） | `#1385:15✓` ∧ `#1385:21✓` ∧ #1413 CLOSED |
| `execution/risk_controls.py:1` PreTradeRiskGate | ADR 0028:72 | 冻结影子 | 能写成纯谓词（输入显式、无状态）的部分 → E 放行判据；阈值取值 = `AwaitingPolicy`（ADR 0041:21），本票不裁 | `#1385:15✓` ∧ #1413 CLOSED |
| `live/databento_live_runner.py` | ADR 0039:231 | 冻结影子 | S 输入侧 feeder | `#1385:21✓` ∧ #1413 CLOSED |
| `live/hl_verify_sdk.py`、`live/hl_verify_nautilus.py`、`analysis/buy_28316_warrant.py` | ADR 0039:251-253 | 冻结影子（只在非生产实验环境使用，C11.3） | X adapter（#1523） | `#1390:15✓`（OE14）∧ #1383 CLOSED ∧ #1413 CLOSED |

**E. 查询壳内第二套判定——冻结影子（承 #1286 不删代码）**

| 模块/符号 | 文件:行 | 终态 | 替代入口 | 退场条件 |
|---|---|---|---|---|
| `src/newchan/orchestrator/recursive.py`；查询壳 import 闭包内 10 个引用者（`gateway.py:34-35` 直接 import；`gateway.py:57-58` 经 `orchestrator/timeframes.py`、`replay.py:13-15` 间接；`live_engine.py`、`live_pool.py`；`server.py` 延迟 import 的 `ab_bridge_newchan.py`、`nested_pipeline.py` 及其下 `a_level_fsm_adapter.py`、`a_nested_divergence.py`；`scripts/realtime_server.py:25/:67`） | ADR 0039:242/:244 | 冻结影子 | 查询壳改读 S 发布，由 #1384（OE01/03/04）收编为只读 adapter | `#1384` OE03/OE04/OE05/OE06 四条全勾（B-Q12 第一条命令输出 `4`）∧ #1413 CLOSED ∧ B-Q12 第二条 `rg -l` 命令（10 个引用者加种子 server.py）输出为空 |
| `PyRecursiveOrchestrator`（`lib.rs:952`）、`PyBiEngine`（`:145`）、`PyOnlineMacdState`（`:832`）；导出 `lib.rs:2839-2841` | ADR 0039:223 | 冻结影子 | S（唯一 Classification） | `#1384:20✓`（OE06）∧ #1413 CLOSED ∧ `rg -l "newchan_rust\.(RecursiveOrchestrator\|BiEngine\|OnlineMacdState)" src scripts trading_system tests --glob '*.py'` 输出为空 |

**F. 非入口（登记于附录，不进分母）**

- **C-28 四个文件**：`rust/src/trading/consolidation_ablation.rs:25`、`sublevel_confirmation_ablation.rs:28`、`trade_behavior.rs:31`（`FLOOR_LADDER = 2`），`gate_state_dump.rs:66`（`L2_LADDER = 4`）、`:253`（`PREREG7`）。它们都是 `#[cfg(test)]` 模块（`trading/mod.rs:66-67/:71-72/:89-90/:93-94`），没有 `[[bin]]`，改称「trading/ 消融守卫测试套件」。按 ADR 0039:174 不是生产入口，处置承 AUDIT.md:189 的 T 结论：留用，值不改，`PREREG7` 原样保留；各套件写输出时须写出所用的 `FLOOR_LADDER`/`L2_LADDER` 值和 `PREREG7` 表；不参与准入（AGENTS.md:55）。
- **`analysis/*.py`**：推论层，本票不改不删（ADR 0012:65），处置见 B-Q16。
- **交易外常驻服务**（ADR 0039:245 OE41/42/45）：承 ADR 0039，不纳入分母。

### 二、收编的唯一形式：替换式

收编只允许「纯函数被 S/E/Bᵢ/X 替换式调用」，要求如下：
- 调用方是唯一 owner 进程内的代码；
- 旧入口不再保留对该函数的生产调用；
- 不搬任何状态 owner；
- 生产路径上不留 `if legacy {...} else {...}`（ADR 0027:64、ADR 0039:300、AGENTS.md:58）；
- 收编不改变所属入口的终态，也不给入口带回 (a)(b)(c)（ADR 0039:190）。

### 三、删除执行

本票唯一授权执行的删除是 `trading_system/live/runner.py`，执行票 #1385。#1385 在执行中若发现任何尾责事实（旧账户、未完成 T/voice、外部尾债，OE11 :192 同类），删除即告失败并停止，事实交 #1402，不改用其他终态继续。其余所有行本票**不授权删除**：
- `recursive_t` 与 `backtest_t_fugue.py` 归 #1096 路径；
- `ThetaPiStream`/`PyThetaStream` 归 #1304 路径；
- 其他冻结影子满足退场条件后，删除须另开执行票逐依赖验收（X6），并逐个记录消费者归宿（B-Q16）。

### 四、名分锚点与注释订正（ADR 0039:299「一并提交」，随本 ADR 落地 PR 提交）

每处改一行、带票号，不锚 ADR 行号，其余文字不动（行号为 main `66c409e7e3` 实测；落地时以 `rg -n "CLI 生产入口" rust/Cargo.toml rust/src/theta_v0/mod.rs` 定位为准）：

1. `rust/Cargo.toml:68` → `# 历史名「CLI 生产入口」（theta_backtest 为 π 组件证据 CLI、冻结影子，非生产 launcher；ADR 0039 裁定二、ADR 0048，#1532）：解 theta_v0::backtest 的 #[cfg(test)] 门控为公开 API，链接 run_theta_v0_pi/load_by_symbol。`
2. `rust/Cargo.toml:85` → `# ── theta_backtest：π 组件证据 CLI（历史名「CLI 生产入口」；冻结影子，非生产 launcher；ADR 0048，#1532） ──`
3. `rust/src/theta_v0/mod.rs:118` → `/// 历史名「CLI 生产入口」解门控（theta_backtest 为冻结影子，ADR 0048，#1532）：let theta_backtest bin 链接 run_theta_v0_pi/load_by_symbol）。`
4. `docs/agents/generation-constitution.md:35` → `- **引擎层**：**π 是唯一现役引擎**（代码谱系名分，相对 v1/dual；不是生产 owner 裁定——π 结构核收编为 S 的唯一计算者，π 回路经济部分为冻结影子，见 ADR 0039 裁定二与 ADR 0048，#1532）。锚定方式 = 家族根函数名（`pi_theta_fill_loop_overlay`），不锚行号（行号随版本漂移）。v1/dual 已按 #499 删除。`
5. `CONTEXT.md:364` 中的「**最小生产路径 CLI** 指 `theta_backtest` 用仓内数据加载、π 引擎与内部 fill 串成的可运行闭环，它证明 CLI 可驱动引擎，但不等于交易场所集成」改为「**π 组件证据 CLI**（历史名「最小生产路径 CLI」）指 `theta_backtest` 用仓内数据加载、π 引擎与内部 fill 串成的可运行闭环，只产组件证据，不是生产入口，也不等于交易场所集成」。其余两义不动。
6. `rust/src/recursive_t/DEPRECATED.md` 在 `:3` 之后加一行：`**生产终态**：冻结影子（ADR 0039 裁定二、ADR 0048，#1532）；名分轴与删除取舍归 #1096。`「已退役」字样属名分轴，本票不改（ADR 0039:299）。
7. `rust/src/lib.rs` 在 `:66` `#[cfg(not(test))]` 之前加一行：`// 生产终态：冻结影子（ADR 0048，#1532）；删除取舍归 #1096。``:29`「已判现役」属名分轴，不改文字，由这一指路行覆盖。

### 五、「默认启动不触发旧核」逐条核验清单（交 #1413）

默认启动 = `production_session`（ADR 0039:309）。#1413 在同一受测 commit/profile 下逐条核（#1413:13「源码形状检查不能替代真实调用阻断」）：

1. **构建层**（判据）：ADR 0039 验收 B11 的 `jq -e` 命令退出码为 0，即 `production_session` feature 不含 `backtest_bin`、`nautilus`；生产二进制集合恰为 ADR 0039:295 所列五个 bin。
2. **加载层**（判据）：`production_session` 及其拉起的 S/E/Bᵢ/X 子进程，运行期进程映射里没有 `newchan_rust` cdylib。以运行期映射为证。
3. **调用层**（分诊线索，不是判据）：`rg -n "recursive_t::|fugue_v3::|spiral::|run_organic|pi_theta_fill_loop|ThetaPiStream" rust/src/bin/production_session.rs rust/src/bin/x_execution_session.rs rust/src/economic_session` 零命中。
4. **行为层**（判据）：在 `simulation` profile 下跑一轮「默认启动 → 纯观察 → 停止/重启」（#1413:11），S/E/Bᵢ/X 域库的写者身份与 TB-09 指纹记录里不出现一·A-E 任何冻结影子行的 core id；影子输出全部带 provenance，不进 TStage 触闸计数（ADR 0039:298）。
5. **一·F 套件**：验收 B5 的命令输出为空。

第 3 条和第 4 条查的是不同的面（形状线索 vs 运行行为），不是同一判断的两档。

### 六、时间表（顺序闸门，不写日期）

- **T0**：本 ADR 及第四节订正合入 main（验收 A 段）。
- **T1**：#1385 执行三·`live/runner.py` 删除（B1/B2）。
- **T2**：#1523 交付 `production_session`/`x_execution_session`（ADR 0039 B10/B11），#1383 交付两条测试（B12/B13）。
- **T3**：#1413 按第五节逐条核（B3）。
- **T4**：#1535 定迁移，#1368 执行切换（B4）；默认启用须明确批准（#1413:14）。
- **T5**：各冻结影子按一·各表的退场条件，经各自路径处置（`recursive_t` → #1096；`ThetaPiStream` → #1304；其余 → 另开执行票逐依赖验收）。

### 七、失败处理

任一闸门或删除步骤失败，都按失败处理：不回退旧 writer、不恢复任何旧导出、不改用其他终态继续（ADR 0027:32、ADR 0028:46、ADR 0030:44、ADR 0039:318）。本 ADR 不新增同一判断的宽严两档。

## 验收清单

**A. 本裁定落地（关本票条件；命令在仓库根目录执行）**

- A1 文件存在：`find docs/adr -name '*-legacy-engine-module-level-dispositions.md' | wc -l | tr -d ' '` 输出 `1`。
- A2 可执行命令：`rg -c '^> 正本：本文件是' "$(find docs/adr -name '*-legacy-engine-module-level-dispositions.md')"` 输出 `1`。
- A3 可执行命令，输出为空：
  ```
  f=$(find docs/adr -name '*-legacy-engine-module-level-dispositions.md'); for k in recursive_t run_recursive_t PyTFugueStream PyRecStream backtest_t_fugue.py pi_theta_fill_loop run_theta_v0_pi theta_backtest ThetaPiStream PyThetaStream theta_rec_strategy.py theta_pi_strategy.rs backtest_engine.rs order_adapter.rs PyFugueV3Stream run_fugue_v3 PySpiralStream run_spiral run_organic run_positional PyUnnStream PyOrganicTape 'trading_system/live/runner.py' 'trading_system/backtest/runner.py' chanlun_strategy.py lmt_executor.py maker_optimizer.py broker_config.py risk_controls.py databento_live_runner.py hl_verify_sdk.py hl_verify_nautilus.py 'orchestrator/recursive.py' PyRecursiveOrchestrator PyBiEngine PyOnlineMacdState realtime_server.py FLOOR_LADDER L2_LADDER PREREG7 '#808' '#1096' '#1304' '#1385' '#1413' '#1550' '#1654' AwaitingPolicy; do rg -q -F -- "$k" "$f" || echo "MISSING $k"; done
  ```
- A4 可执行命令（CONTEXT.md 词义订正）：`rg -c "π 组件证据 CLI" CONTEXT.md` 输出 `1`，且 `! rg -q -F "**最小生产路径 CLI** 指" CONTEXT.md` 退出码 `0`。
- A5 可执行命令（注释订正），输出为空：`rg -n "CLI 生产入口" rust/Cargo.toml rust/src/theta_v0/mod.rs | rg -v "历史名"`。
- A6 可执行命令：`rg -c -F "v1/dual 已按 #499 删除" docs/agents/generation-constitution.md` 输出 `1`。
- A7 可执行命令：`rg -q "#1532" rust/src/recursive_t/DEPRECATED.md && rg -B2 "^pub mod recursive_t;" rust/src/lib.rs | rg -q "#1532"` 退出码 `0`。
- A8 可执行命令（写权闭包未因本票变动），输出为空：ADR 0039:336 的 `diff` 命令原样执行。
- A9 可执行命令（本票不删任何文件）：`test -e trading_system/live/runner.py && test -d rust/src/recursive_t && test -e trading_system/backtest_t_fugue.py` 退出码 `0`。
- A10 票状态：`gh api repos/xy7365527-lang/NewChanlun/issues/1385/dependencies/blocked_by --jq 'map(.number)|contains([1532])'` 输出 `true`。
- A11 票状态：`gh api repos/xy7365527-lang/NewChanlun/issues/1413/dependencies/blocked_by --jq 'map(.number)|contains([1532,1523,1383])'` 输出 `true`。

**B. 下游票验收（判据二值；不作关本票条件）**

- B1 文件不存在（#1385）：`test ! -e trading_system/live/runner.py` 退出码 `0`。
- B2 可执行命令（#1385），输出为空：`rg -n "live/runner" trading_system`。
- B3 票状态：`gh issue view 1413 --json state --jq .state` 输出 `CLOSED`。
- B4 票状态：`gh issue view 1368 --json state --jq .state` 输出 `CLOSED`。
- B5 可执行命令（#1413 第五节第 5 条），输出为空：
  ```
  for m in consolidation_ablation sublevel_confirmation_ablation trade_behavior gate_state_dump; do rg -U -q "#\[cfg\(test\)\]\n(?:\s*//.*\n)*mod $m;" rust/src/trading/mod.rs || echo "NOT_TEST $m"; done
  ```
- B6 票状态（`recursive_t` 退场条件，供 #1096 消费）：`gh issue view 1096 --json state --jq .state` 输出 `CLOSED`；`gh issue view 1385 --json body --jq .body | grep -c '^- \[x\] OE11 '` 输出 `1`；`gh issue view 1385 --json body --jq .body | grep -c '^- \[x\] #1096 rec_t_strategy'` 输出 `1`。

## 受影响票与 blocking 边变化

**受影响票**

- #1323：Decisions so far 加一行（FU-12 模块级终态落 ADR 0048）。
- #1340：C11.4 条目级的「替代入口/退场条件」具名到 ADR 0048；C11.6 对 #1096 的要求不变。
- #1530（CLOSED）：ADR 0039 裁定五 FU-12 栏由本 ADR 承接。ADR 0039:299/:307 的「`Cargo.toml:54`」在本 ADR 订正为 `:68`/`:85`，并补上 `theta_v0/mod.rs:118`；:214/:296 的 `nautilus/mod.rs` 写全为 `rust/src/theta_v0/nautilus/mod.rs`。ADR 0039 本文不改（ADR 0012:65 口径）。
- #1385（TB-10-B）：承接三·删除执行；一·各表退场条件引用它的 OE07-OE13 与 :22。**新增被本票阻塞**。
- #1413（TB-10-I）：承接五·核验清单。**新增被 #1523、#1383 阻塞**。
- #1523、#1383（TB-09-A）：没有新义务，各新增一条出边到 #1413。
- #1384（TB-10-A）、#1386（TB-10-C）、#1388（TB-10-E）、#1390（TB-10-G）：一·各表退场条件读它们的 OE 勾选状态；无新边。
- #1402（TB-10-H）：接收三·删除中发现的尾责事实；无新边。
- #1535（FU-13）、#1365（TB-09）、#1368（TB-10）、#1369（TB-11）、#1551（FU-20）：现存 #1532 出边保留。
- #1096：保留退役还是收编的选择权。本 ADR 提供一·A 表的消费者现状和机械退场条件，并写明 #808 ③ D1-D3 由 #1096 承接（D2 须人显式表态）。在 #1096 留评论指针，不新增边。
- #1304、#1550（FU-19）：`ThetaPiStream` 行以 #1550 CLOSED 为退场前置，删除归 #1304 路径；不新增边。
- #1654：对齐一·A 表，只在终态内改注释和名分；删 `Stroke` 档以 A 表删除前置为前提。在 #1654 留评论指针，不新增边（它被 #1627 阻塞）。
- #808（CLOSED）、#951（CLOSED）：判死条件与冻结口径由本 ADR 引用承接，无动作。
- #1586 / AUDIT.md C-28：由一·F 处置。
- #1544（CLOSED，裁定票）：无动作。

**现存原生边（2026-10-21 实查）**

- #1532 blocked_by = 1530, 1544；blocking = 1365, 1368, 1369, 1413, 1535, 1551。
- #1385 blocked_by = 1360, 1363, 1365, 1366, 1367；blocking = 1402。
- #1413 blocked_by = 1351, 1360, 1363, 1365, 1366, 1367, 1402, 1532；blocking = 空。
- #1523 blocked_by = 1362, 1363, 1365, 1366；blocking = 1383, 1524, 1525, 1675（2026-10-22 实查）。
- #1383 blocked_by = 1351, 1362, 1523, 1530；blocking = 1401。
- #1402 blocking = 1413。
- #1096 blocked_by = 1332；blocking = 空。
- #1654 blocked_by = 1627；blocking = 空。

**处置**

- **新增 #1532 → #1385**：#1385 执行本 ADR 第三节的删除，须以本 ADR 为准。不成环：#1385 唯一出边是 #1402，#1402 唯一出边是 #1413，#1413 无出边。
- **新增 #1523 → #1413**：第五节第 1、2、4 条要实跑 `production_session`（#1413:11），该二进制由 #1523 交付（ADR 0039 B10/B11）；`rust/src/bin/production_session.rs` 当前不存在。不成环：#1413 无出边。
- **新增 #1383 → #1413**：第五节第 4 条依赖 TB-09 指纹和第二写者拒绝测试（ADR 0039 B12/B13，归 #1383）；`rust/tests/production_session_entry.rs` 当前不存在。不成环：#1413 无出边。
- **保留**：#1532 → #1365、#1368、#1369、#1413、#1535、#1551 六条现存出边；#1530 → #1532、#1544 → #1532 两条入边（两票均已 CLOSED）。
- **不新增**：
  - #1532 → #1096、#1304、#1550、#1654：退场条件只读这些票的状态，不阻塞也不被阻塞，加边等于替其义务定序。
  - #1532 → #1384、#1386、#1388、#1390：它们执行的 OE 条款由 ADR 0039/#1340 持有，不消费本 ADR 的裁定。
- **删除**：无。
- **research/probe 票**：不开。考古没有 C 类缺口；一·A 表的尾责调查（OE11 :192）归 #1385:19/:22 与 #1402 现有义务。

## 授权引用

本条依 #1323 Notes N-2 由主控代裁（#1323 评论 6310866929：P 类问表中有先例的 B 类默认值确认由主控按推荐值裁定，经独审后落地；本票不涉真钱、风险偏好、已批方向或 spec）。另依 N-1 预授权判定。用户 2026-10-18 原话（#1323 评论 6261281111，xy7365527-lang 2026-10-19 记录；#1323 票面 :41 Notes N-1）：「其实很多技术的东西 完全不需要我来做 我完全信任你的判断」。分级评论 6284962135、6311051879 判 T；主控重写评论 6310613611 裁定 `recursive_t` 走路径 (a)，并要求按 T 类重跑。本稿据此起草，没有夹带 P 类取舍：不替 #1096/#1304 定退役，不删 `recursive_t`，不代答 #808 D2，不裁资金、场所、账户、权限或阈值。
