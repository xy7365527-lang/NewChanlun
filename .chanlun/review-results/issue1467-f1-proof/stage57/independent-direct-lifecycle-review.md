# Stage57 直接订单生命周期独立比较审查

2026-10-05，研究 #1467，归图 #1465。名分为工作草稿。本工位为新上下文比较审查，读取冻结作者包和已经隔离完成的实际声明读回，没有作者会话。采用本树 math-research-workflow、rigorous-open-math-research 及 verification loop；未修改作者包、历史、Progress、formal 或生产。证据目录为 `/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage57/direct-review/`。

**结论：A57 在明示全程有效双边报价域内的普通数学适配、D54ᴰ 的关系性质及 L56 迁移成立；两项有限控制和实际 Lean 声明与作者限定的意图相符。未发现要求打回这项有限结论的 finding。** 本轮新增可采用结果是来源保持的生命周期适配，以及直接订单原始历史上的非空 Closed⁰/P⁰ 见证。A57、整个 D54ᴰ 扫描及 B56 没有进入本轮 Lean 根；原义 P、同向 Zn/DD/GG、固定 F₂ 孩子和父生命周期均未获得认证。

## 冻结输入与验证

作者 manifest SHA-256 为 `86e066a7998c820c97c083b5d90a7178804e4e0426726b75c5892e55d721dfcf`；作者稿 SHA 为 `68284067dd0cf3c9361c16931afe29efa22cf5b1438b2581da2fac717169e627`。实际读回 SHA 为 `b505b68a959188684876bf5c977bb2e28f66387bcf34f91da00236c386f09570`，读回回执 SHA 为 `64d8e31e7470be04704ecfa12da8342f8ff1050f89f8efcd621e7d6fc01b50e9`。入口、manifest 中全部 sourceBindings/outputs、作者回执及独立读回所列回执共 46 项逐字节校验通过，运行后再次校验未变，详见 `input-integrity.json`。

本轮亲自执行两项检查。其一，把作者 checker、controls、Stage54/53 依赖按原相对目录原样复制到仓外后运行。Node v22.23.1，exit=0，生成 results 的 SHA 与冻结 `results-v1.json` 完全一致。其二，另写独立复算器，以 ID Map 和插入次序重建账本，检验原 Step 的前插/单位删除关系，再按手列 17 份前缀表对照来源、几何、时钟；没有调用作者的 replay、分组或生命周期函数。主控制的 D54 结论另按五条已确认腿直接计算。记录见 `independent-controls.json`。这两种检查都是有限证据，没有扩大枚举、下载、训练或市场采样。

复现命令：

```bash
node /Users/silencehan/Documents/Codex/research-evidence/issue1467/stage57/direct-review/run-review.mjs
```

## 实际声明和作者意图的比较

[DirectLifecycle.lean](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/stage57/direct-evidence/DirectLifecycle.lean:11) 的 `primitive_strict` 是一般条件声明，要求 Through、Reads、长度等式、construct 成功、实际输出成员及该成员 geometry=some。结论只涉及源跨度与严格有向端差。它不要求块已确认，不证明所有块都存在几何，也没有包含生命周期扫描。

同文件 `witness_root` 的实际类型为 `WITNESS_TARGET`，展开是 Main 的 `Fin 9` 与 NoneControl 的 `Fin 8` 全部前缀的 Through、Reads、长度和 `construct.map(view)` 字段等式。`view` 比较跨度、firstKnown、knownAt、pressure、原 Event 列表及可选几何的全部五字段。Event/Book 本身不含 orderId。作者 JSON 的 ID 检验是单位订单来源的保守补充，随后向原列表价格模型投影；它没有被冒称为原 Lean Book 字段。

独立读回工位已从实际源码重编译 16 个本地研究模块，Lean 命令全部 exit=0，另行打印声明通过。根传递公理为 `[propext, Quot.sound]`；辅助引理为 `[propext, Classical.choice, Quot.sound]`，辅助不在根依赖中。本工位核对回执及源字节后复用这份独立编译证据，没有再重编译全部模块。读回包装器 exit=1 的原因是未指定预期合同而启用 strict-exit；其状态是 `declaration_closed_uncompared`，不等于 Lean 编译失败。本比较补上意图范围核对，不把旧机器回执改写为机器 exact-root 比较通过。

formal/Origin 仍使用哈希绑定的预编译依赖。没有建立这些完整源文件与 olean 的全量重新构建对应；实际打印支持所用 Segment/Direction 字段。载入 SourceAxioms 模块不代表根使用其全部假设。作者在第100行明确区分普通 A57、辅助引理和有限根，范围与实际声明一致。

## A57 的数学核验

域为任意有限原事件历史及指定簿路径 `B₀,…,Bₙ`，每事件满足原 Step，Reads 真实，`|qs|=n+1`，每个原状态切点 `0≤k≤n` 都记录 some QuoteOf。全程有效报价是额外明示的前件；construct 的长度门本身不保证它。价格为 Int，非空单位列表允许重复价格与一般正数量；cancel/execute 在原模型中共用删除关系，不因此获得真实场所协议语义。

以下推导直接检查了 [MovingQuoteSpec/Proof](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/MovingQuoteSpec.lean:40)、[DirectOutputSpec](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/DirectOutputSpec.lean:8) 和 [实际支撑声明及证明](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/DirectOutputSourcePathProof.lean:62)，不由两控制归纳成全称结果。

1. 原 group 以非空最大同压力块唯一分割原事件。locate 给连续的 `(aᵢ,bᵢ]` 和首反号确认 `κᵢ=bᵢ+1`，最后块保持活动。包含 None 的全部原输出才覆盖全部输入；已确认前缀以外的源留在尾部。
2. 正事件只能增加买侧或删除卖侧，故 bid/ask 均不降；负事件反向。这由 Step 的集合包含及 BestBid/BestAsk 得出，不需要预先假定 geometry 成功。块内传递后，正块 `q_a.bid<q_a.ask≤q_b.ask`，负块 `q_b.bid≤q_a.bid<q_a.ask`。结合窗口长度至少2和所有报价 ready，原 readout 的全部检查成立，故域内每块有几何。这填足了使用后续 some 条件定理的前件，没有循环借用 some。
3. 相邻原块共用同一个源切点、簿状态及 Quote，压力翻转，所以其选边终值等于下块起值，严格方向交替。块内 bid/ask 弱单调给出实际所有 L1 spread 支撑的最小包含区间：正块为 `[q_a.bid,q_b.ask]`，负块为 `[q_b.bid,q_a.ask]`，上下界均由实际端点报价取得。这与 actual_exact_root 的实际路径对象相符；不是深档、触价或成交价包络。
4. 取边界骨架 `u₀,v₀,v₁,…` 只作有限代数表示。严格交替使每条边成为一个最大单调边，完整连续腿片的边界包络等于全部实际 L1 支撑包络。D54 的三范围 seed、闭触、L/R 角色、外缘及游标操作因此保持。事件序号必须经各腿 `(aᵢ,bᵢ]` 映回，不能把骨架边序号当原事件号。实际 κ 由原证书携带，不从仅含已确认端点的骨架重新推断。
5. 最小 seed 和逐邻判断均单值，游标严格增加；触核前 `H={s,…,s+2}`，触核后当前 `H={s,…,j}`，故成功时恒有 `M={s,…,j−1}`。seed 保留且 memberEnd=L.start<L.end，MemberConflict 在此域不可达。whole 包括游标起点到 L 的所有完整源腿，R 留右侧，因而输出加尾部不重不漏。追加同一实际源历史时，已确认块及其全部源证书固定，较早失败窗和扫描判断也固定，得到已发稳定。此处不比较另行重排的另一条 Book 列表路径。

L56 也能直接在这些区间上证明。seed 三范围含 C。首次候选即成功时，L 好角色使其跨 C 边界。首次触核时，由 seed 末端、相邻方向及 R 触 C 推出中间 L 触 C；后续新腿曾作为触 C 的 R，最终 L 也触 C。因此 s 到 j 的每个范围均触 C。对其中任意正宽三交 J，分别有 `J.low≤C.high` 和 `J.high≥C.low`，所以闭交非空。起点在 s 之前的三窗已被最早 seed 搜索拒绝，包括跨入 seed 的窗口。whole 由完整腿构成，证据 R 在外。这给出精确的闭相交结论，不给 J=C、正宽相交或同一原义实例。

Stage37 的 [PressureDCSpec](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/PressureDCSpec.lean:59) 与 kernel 实际声明已经表达全双边域的选边路径、方向恢复、分组/时钟投影、leg 公式以及整数 δ=1 同价取末更新。其文档仍是作者＋机器、`not_reviewed`。本轮识别这项既有结果，未重新签署其整个根，也不把既有报价单调性包装为全部新发现。A57 的新增部分是把带原来源和确认的腿交给 D54/L56，并提供非空原订单见证；它没有证明所有 R_D 等同所有 R_W，且无需以 Stage37 整包通过为前提。

## 两份控制与时钟

独立重建得到主流九个 Quote：`100/102,101/102,100/102,101/102,100/102,1/102,1/99,2/99,1/99`。八个 Step 都合法，E6 加 ask99 前的最佳 bid 已为1，不交叉。六条原 Flow 如下。

| Flow | 拥有事件 | 端值 | 完整 L1 范围 | knownAt |
|---|---|---|---|---|
| F0 | E1 | 100→102 | [100,102] | 2 |
| F1 | E2 | 102→100 | [100,102] | 3 |
| F2 | E3 | 100→102 | [100,102] | 4 |
| F3=L | E4–E6 | 102→1 | [1,102] | 7 |
| F4=R | E7 | 1→99 | [1,99] | 8 |
| F5 | E8 | 99→1 | [1,99] | none |

首 seed 为 F0/F1/F2，C=[100,102]，seedEnd=3、seedKnownAt=4。R.high=99<100，L 的下降角色满足 `102≥100>1`，R向上。`M=H={0,1,2}`，没有触核迭代，releasedFromDevelopment 为空；本例没有覆盖曾入 H 后释放的数值分支。

whole 发生边界为6，拥有 E1–E6；M 只拥有 E1–E3，L 拥有 E4–E6 且不作最终成员。memberOuter=developmentOuter=[100,102]，wholeOuter=[1,102]。R 的 E7 与封 R 的 E8 不属于该 whole；尾部拥有 E7、E8，实际 L1 范围 [1,99]。whole 内仅两个完整三腿窗，其交均为 [100,102]。

已确认 Flow 数为 `0,0,1,2,3,3,3,4,5`。n≤3 是 NoCore，n=4…7 是 AwaitPair，E8 首次发布。E6 的 L 尚未封，E7 仅封 L 而 R 活动，均不能提前发布。原始事件时钟不能误作物理市场接收/发布时钟。

直接接口消费已有 κ；若只发已确认端点再调用 batchLegs，E8 只有 `100,102,100,102,1,99`，其五条边中仅前四条有后续反向边，R 尚未二次封定。须等原 F5 被后续正事件封定才会交付该反向端点，所以存在额外延迟；未来也可能一直没有反号。本轮没有为此新增第三条事件控制。包含活动 Flow 当前端值的骨架可用于同前缀区间计算，但不是逐原事件只追加的冻结标量串。骨架确认位置 k 应映为原 Flow k−1 的 firstKnown，不能映到可能继续延长的 end。

None 控制在状态3买侧空，确实不存在 QuoteOf。最终 geometry.isSome 为 `[true,false,false,true,true]`，knownAt 为 `[2,4,6,7,none]`。两缺几何块共拥有 E2–E5，前一 some 在状态1结束、后一 some 在状态5开始。过滤后即丢四事件并伪造源相邻，偶然共端价也不能修复。原 construct 保留五块七事件，已确认与有几何是不同条件。作者 checker 在全双边域外停止生命周期计算，不提供跨 None 恢复协议；不把它的全尾返回解释成跨域追加稳定定理。

计数为两控制、15原事件、17个含初态前缀、0市场样本。不能把不同前缀当独立样本，不能把有限 JS 对照升级为一般程序精化。

## 原义 P、B56、方向和固定 F₂

本轮从正本 `docs/chanlun/text/blog/` 重新读取相关行，核对 Stage56 摘录所列9份源文件哈希及69条摘录逐字一致，见 `source-check.json`。以下采用正文或明确作者答复，并核对正文界、行首及行内注：017:40/44/52；018:24/26/40/44/64；020:52/54/58；031:382/386 等号后的作者回答；038:18/22/24/26及300作者回答；061:44；084:52/54/56。018:24 行内娇注、018:46–48 编注不进入前提；027:854/856 不进入依赖。这里只认证指定转载可见归属，没有另作历史评论身份认证。

这些来源允许非递归初始构造，故不能要求所有 primitive 先递归完成，也不能因它不是旧成交序列而否定 A57。与此同时，017:44/018:26 的 P 仍以完成走势和一原义核为前提；018:40 允许满足三段条件后继续延伸。A57 的稳定退役是所选数值关系的规则，L56 是区间不分离，均未给出实例 K 到原义核的解释映射，也未给出 whole 在 end(L) 的原义结束依据。018:64 的离开/回抽包含次级走势角色，061:44 的第四/第五分工只说明不能复用第三 seed 作第四，不能把任意带 pressure 的原 Flow 自动认证为这些角色。B56 仍为待证的相容性命题，尚无本轮证明，也没有被本轮反例否定。

方向同样未补齐。031:386 的作者回答区分盘整与趋势方向；pressure、离开 side 和 whole 端点净变化不是原义 P 的概念方向证明。020:52 的 Zn 限定为与中枢形成方向一致的次级走势，全部最终 M 的 hull 不自动等于 DD/GG。主控制数值恰同也不能建立普遍对象同一。

因此一份 Closed⁰/P⁰ 不能充作固定 F₂ 的合格孩子，更不能由原三 seed 代替三个完成 whole。全三真实范围、严格正宽、真实连接、方向资格及父生命周期义务全部保留。当前可交付的是非空初始数值生命周期与来源证书。未来可能存在适用的非递归相容模型，本审查未全局否定它；也不因作者已写“未证”而省略上述逻辑检查。

## 交付范围

可采用：A57 明示域内的来源保持适配和普通关系证明；L56 的精确闭触迁移；两固定控制的原 Step/Reads/construct 有限根及 JS 数值结果；E8 确认时钟和 None 禁止拼接的边界。未发现阻断这些结论的 finding。

仍未证明：A57/D54ᴰ 全扫描的 Lean 形式化、一般 JS 实现精化、任意含 None 历史的恢复与稳定、真实订单协议、原义 P/B56、Zn/DD/GG、固定 F₂ 及生产/收益结论。Stage37 全包语义审查保持原状态。本报告没有把这些剩余义务当作已证结果，也没有把未证自动判为反例。
