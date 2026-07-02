# codex 裁决⑤：七条证据补全重裁（task #42）

**日期**：2026-07-02
**工位**：ws-codex-resubmit（codex-challenger）
**效力来源**：编排者「要裁决的全问 codex」明令，team-lead 委派 task #42
**前置**：裁决②（codex-ritual-grammar-theorems-20260702.md）中 662/660/659/615/667 五条 UNDECIDABLE + 642/665-Yi-addendum 两条驳回，共同根因=传给 codex 的证据摘要缺量化细节（该文件"关键分歧"第5条自证）。本次逐条读谱系原文全文+引用数据档案，把完整证据段落重新提交 codex decide。**逐条独立裁决，未打包**。
**排除**：647-pibsp 不在本批（需真异质审计，另案）。

## 判决总表

| 候选 | 裁决②原判 | 重裁判决 | 完整记录 |
|------|-----------|---------|---------|
| 662 正交性证伪 | UNDECIDABLE | **成立**（限定代码层/操作定义） | codex-decide-20260702-162400-b756.md |
| 660 L3降级inconclusive | UNDECIDABLE | **成立** | codex-decide-20260702-162450-2817.md |
| 659 跨引擎范畴错误 | UNDECIDABLE | **成立** | codex-decide-20260702-170539-1d7a.md |
| 615 Layer1⊊Layer2 | UNDECIDABLE | **部分成立**（反例合格但对象须收紧，见下） | codex-decide-20260702-170759-3c52.md |
| 667 G=∅→inconclusive | UNDECIDABLE | **成立** | codex-decide-20260702-171007-02d0.md |
| 642 定义冲突降级工程缺口 | 驳回 | **成立**（限定"机制级已解决"） | codex-decide-20260702-171308-bfa6.md |
| 665-Yi 市场基准恒等式 | 驳回 | **成立** | codex-decide-20260702-171526-9dbb.md |

**统计**：6 成立（其中 662/642 带限定语）+ 1 部分成立（615 需收紧表述后闭合）。无一条维持 UNDECIDABLE/驳回——七条的原判负确均系证据摘要缺量化细节，非候选内容有错（裁决②"关键分歧"第5条的预判被证实）。

## 逐条详情

### 662：LCB门控 vs 功效损失 正交性证伪 → 成立

- **本次补全证据**：AcceptSet(LCB)⊆AcceptSet(裸μ) 的逐点代数不等式证明（LCB=mean−z_α·std/√n≤mean 恒成立→集合包含形式证明）+ `lcb_vs_naive_l2.rs:57` 单调性诊断全类别数值验证。论证形式=确定性代数证明，非统计相关性。
- **codex 判决**：成立，限定为代码层/操作定义下的证伪。DIM-1/DIM-2 都从 Δ=AcceptSet(裸μ)\AcceptSet(LCB) 派生，是同一收紧操作的两个投影，构成完全函数依赖。**明确拒绝**"必须用协方差/条件独立才能判非正交"——这不是随机变量相关性问题，是门控集合的确定性包含关系，统计证据非必要条件（即裁决②原判要求的证据类型被 codex 自己撤回，代数证明已足够）。
- **边界条件**（codex 给出）：①DIM-2 功效损失改由独立功效模型定义（外部采样成本/补样策略）则翻转；②LCB 与裸μ用不同阈值/样本全集/候选 universe 使包含关系不成立则翻转；③z<0 等非法实现值。

### 660：L3 falsification 降级 inconclusive → 成立

- **本次补全证据**：n=5 单侧符号检验精确二项功效计算——拒绝域仅{X=5}，Power(p_true)=p_true^5，60-90%胜率假设下检出力仅 7.8%-59.0%，80%检出力需 p_true≥0.956；实测 3/5 (p=0.5)。独立结构证据：16K 短窗高级别 μ 估计退化+样本饥饿（count=1 类占比高），P0 修复后裁定不变。
- **codex 判决**：成立。应记录为 `INCONCLUSIVE_LOW_POWER`。"p≥0.05 不直接映射为 falsification；只有在预设效应下功效足够时，非显著结果才可升级为证伪证据。"拒绝维持 falsification（把"未显著"误当"无效应"）。
- **边界条件**：扩样至充分功效后仍 LCB≤0 则可转真负证据。

### 659：filter-spec 跨引擎范畴错误 → 成立

- **本次补全证据**：两引擎范畴结构对照表（t_backtest_8x3=`backtest.rs` `apply_bsp` 纯多头无方向门控 vs 547=`rec_engine.rs:81-100` `LongEntry::CrossLevel` 跨级别方向耦合）+ backtest.rs/backtest_run.rs 对 rec_engine/cascade 零匹配 + 627 号风险1形式条件（有效域=引擎线X范畴结构的结论不可应用于不含该前提的引擎线Y）+ 坐标漂移证据（`cascade_reverse_to_core` 函数名源码不存在）。
- **codex 判决**：成立。codex 自主导航源码复核了 `backtest.rs:252`（买点空仓建多/卖点只 clear/trim，TradeDir::Long）与 `rec_engine.rs:2323`（"无更高活跃级别逆向段"判据），确认 547 病理前提（跨级别方向耦合/方向门控）在 t_backtest_8x3 不存在，切断前置依赖边。建议（技术选型层）：filter/spec 依赖带 EngineLine/ValidityDomain 类型标注。
- **注**：codex 的复核是独立源码导航（非仅复述我方证据），判决含行级引用。

### 615：Layer1⊊Layer2 → 部分成立（需收紧表述）

- **本次补全证据**：条件1（Layer1 可嵌入 Layer2 同一载体 BSPLabelSet）+ 条件2 反例 `BSPLabels.lean:105 twoB_threeB_can_coincide`（vReversalEndpoint 同时携带 2B/3B，bit-vector 可表达而互斥 sum-type 不可表达）。
- **codex 判决**：**反例合格但对象须收紧**。精确形式：
  - `ExclusiveSumLayer1 ⊊ Layer2`：**成立**，twoB_threeB_can_coincide 是有效 witness（互斥 sum-type 把共现状态编码成不可能事件）。
  - 原始 603 版 `ConstructorExhaustiveLayer1` vs Layer2：该定理只证明 Layer2 需要非互斥表示；若 603 Layer1 的正式定义只是"所有 move 落入某构造子无遗漏"且与 Layer2 共享 BSPLabelSet 载体，则 Layer1 本身不必然排斥 {2B,3B}——严格性闭合还需**另立定理**：展示一个满足构造子穷尽但违反 Layer2 complete/realized/Eval/δ 语义条件的分类器。
  - 方向订正：若按"满足标准的实现集合"写，应是 `Layer2 ⊊ Layer1`（Layer2 更强→实现集合更小）。615 的"Layer1⊊Layer2"指的是"标准强度序"（Layer1 的要求集是 Layer2 要求集的真子集），两种写法须显式区分，否则断言不稳。
- **下游行动**（定理类，供 genealogist/形式化工位消费）：①615 表述收紧——反例命题改述为"互斥单标签分类器严格过弱"；②补一个 ConstructorExhaustive-but-not-complete 的投影分类器定理以闭合原始严格性断言。
- **边界条件**：若后续形式化证明 2B/3B 真实语义必须互斥（vReversalEndpoint 是建模错误）则反例失效；若 Layer1 重新定义为"非互斥标签集构造子穷尽"则反例不再区分两层。

### 667：L1/L2 G=∅ → inconclusive → 成立

- **本次补全证据**：LCB(μ)>0 ⟺ n_eff>(1.645·CV)² 充要条件 + CV=10-20（codex-lq-audit-20260701 自给）→门槛 271-1083 + L1/L2 OOS n=38-188 远低于门槛的对照表 + "LCB<0 ⊬ μ≤0"推导（判据结构性无检出力的必然产物）。
- **codex 判决**：成立。G=∅ 只能说明"当前样本和门槛下没有候选通过"，不能推出"候选真实不存在"。技术选型：三条件法输出不建模为 bool/空集合，用带原因的三态判定（Confirmed/Refuted/Inconclusive{Underpowered}）。拒绝把 G=∅ 判 Refuted（混淆"未检出"与"不存在"）、拒绝判 Confirmed（功效不足只反驳强证伪，不给存在正证据）、拒绝临时降 LCB 门槛救活候选（改变三条件法语义）。
- **边界条件**：CV_oos 实测显著低于经验值使门槛降至 n 以下，或扩样后充分功效下仍 LCB≤0，则翻转。

### 642：定义冲突→工程缺口降级 → 成立（限定"机制级已解决"）

- **本次补全证据**：①具体工程缺口定位（coverage.rs:1253 旧 host 注入 key=(c.level,c.source_index) 同级 vs AncOK 要求 c.level+1 父）；②**修复已落地**（本工位核实：当前 coverage.rs:1655-1701 已改为候选 carrier id 入 raw + parent_id 检查 + registry-live 时 restore_ancestor_chain_from_registry——642 原文写作时标注"未解决/待 Lead 派工位"，此后已实装）；③**3 个 H2 回归测试实际执行全通过**（engine_bootstrap_container_bsp_admits_depth_child_from_empty / engine_bootstrap_does_not_admit_orphan_shortdiff_without_container_bsp / cross_bar_held_container_admits_depth_child_next_bar，后者覆盖跨 bar 持仓父链真实生产路径；`cargo test --release` 3 passed 0 failed）。
- **codex 判决**：成立。"旧问题不是两套定义冲突，而是实现没有忠实承载既有定义"——testing-override.md 判据下为实现错误（定理类）。codex 独立复核了当前 coverage.rs:1655 的修复代码。
- **限定语**（codex 明确给出，与 231 铁律一致）："已解决"只覆盖 H2/AncOK 准入机制（结构性回归测试通过）；**不等价于真实市场数据下 ΔSharpe 已非零或盈利成立**——后者需 L2 重测，不预设结果。
- **诚实声明**：裁决②要求的"修复后通过 H2 证据"在 642 原文（2026-06-29）确实不存在（当时修复未落地）——本次重裁的新证据来自此后落地的修复代码+本工位实际执行测试，属"新数据补齐"而非"原文证据重新转述"。这是七条中唯一一条靠新数据（而非原文既有证据）翻案的。

### 665-Yi-addendum：市场基准恒等式 → 成立

- **本次补全证据**：B_market 精确定义=close[exit]−close[entry]（同期 BTC 自身实际涨跌），明确排除 codex 原判列举的四种替代口径（指数/永续合约/滞后归一化/条件化预测）+ 数值验证 corr(raw,B_market)=1.0、mean(raw−B_market)=−3.6e−14（12626 信号台账×4.6M bar 实测）+ 三口径对比表（Y_scalar perm_p=0.0005 假阳性反证精确基准定义的必要性）+ 663 恒等式推导链。
- **codex 判决**：成立。"这不是统计相关性，而是同一物理量的两条计算路径……真正支撑恒等式的是定义闭合，corr=1.0 是实现一致性的证据。"残差恒为 −ce（零 alpha 纯成本）在单标的口径下成立。拒绝把 B_market 当普通外部 benchmark 继续跑 alpha 检验；拒绝 Y_scalar 的 h·ḡ 平均漂移作"真同期市场基准"（保留 beta 泄漏）。
- **边界条件**（原文自带，codex 认可）：多标的 L3 引入独立市场因子则 raw≠B_market 判据3重新可执行；因果预测 beta 口径测的是不同假设须重新定义。

## 结果包（简化版——纯技术性产出）

1. **结论**：七条重裁 6 成立 + 1 部分成立（615 反例合格、断言对象须收紧+补一定理闭合）。裁决②的预判（"判负系摘要缺细节非内容有错"）被证实 7/7。642 特殊：靠修复落地+测试通过的**新数据**翻案，非原文证据转述。
2. **边界条件**：各条翻转条件已在逐条详情中列出（源自 codex 判决原文）。整体边界：本批裁决效力=codex-cli 单模型异质裁决，若后续真异质审计（OpenAI 配额恢复后）对同一命题给出矛盾判决，以矛盾上浮处理。
3. **影响声明**：不改任何代码/定义文件。下游消费者：genealogist（662/660/659/667/642/665-Yi 六条可依本裁决推进结算流程；615 需先收紧表述+补定理）；Lead（642 的 ΔSharpe L2 重测仍是待做行动类工位；deltasharpe memory 修正标注可写入）。七次完整交互已由 CLI 自动持久化至 .chanlun/review-results/codex-decide-20260702-{162400-b756,162450-2817,170539-1d7a,170759-3c52,171007-02d0,171308-bfa6,171526-9dbb}.md。
