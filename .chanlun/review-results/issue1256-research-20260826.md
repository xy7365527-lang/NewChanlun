# #1256 research：#991 I-3 / 力度分量 / 研究线消费面考据

> 范围：只 research，不裁定。
> 写入时间：2026-08-26。
> 取证边界：`gh issue view 991/874/1256 --json ...` 本轮均返回 `error connecting to api.github.com`，故未直接读取 GitHub issue 正文/评论；以下 #991/#874 相关结论只基于本地 commit、源码注释、正本文本与已落盘报告。

## 0. 方法与引文纪律

- 缠论原文只取 `docs/chanlun/text/blog/` 正本路径。
- 原文引文逐字核对行号；同时检查：
  - `↑正文` 界标：`029-第29课.md` 的正文界在 `:90`，本报告引用 `:52` 位于界前；
  - 行首注：引用行未以「（注：」「(娇注：」等开头；
  - 行内注：引用句内未嵌「（注：」「(娇注：」等非作者注。`029:50` 有行内注，本报告不以该行为正文依据。
- `046-第46课.md` 本文件未见 `↑正文` 界标；本报告引用 `:14`、`:16`、`:28` 均在作者行 `:12` 之后、每日解盘 `:36` 之前的文章开头段，故界别记为**正文·推定**，不冒充有界标确证；引用行行首无注、句内无注。`046:26` 有行内「(娇：反向）」注，本报告不以该处为核心引文。
- 代码/定义文件引用均区分：原文、正本层裁定、代码现状、研究/对照/冻结线名分。

## 1. 029:52「背驰后的级别与力度」逐字核验

### 1.1 原文逐字

`docs/chanlun/text/blog/029-第29课.md:52`：

> 以上三种情况，就完全分类了某级别背驰后的级别与力度，也就是某级别的第一类买点后将发生怎么样的情况，而第一类卖点的情况是一样的，只是方向相反。注意，这里说的是最精确的情况，由于第一种情况很少发生且和第二种情况有所类似，所以粗糙地说，也可以说背驰以后就意味着盘整和反趋势。那么，怎么分别这几种情况，关键就是看反弹中第1个前趋势最后一个中枢级别的次级别走势（例如前面的下跌是5分钟级别，就看1分钟级别的第1次反弹），是否重新回抽最后一个中枢里，如果不能，那第一种情况的可能就很大了，而且也证明反弹的力度值得怀疑，当然这种判别不是绝对的，但有效性很大。

核验：

- 正本路径：`docs/chanlun/text/blog/029-第29课.md`。
- 正文界：`:90` 为 `---------↑正文---------`，`:52` 在界前。
- 行首注：无。
- 行内注：无。

### 1.2 这句里的「力度」是什么位置

同课上文列出三种情况：

- `029:22`：一、该趋势最后一个中枢的级别扩展；
- `029:24`：二、该级别更大级别的盘整；
- `029:26`：三、该级别以上级别的反趋势；
- `029:30` 对第一种说「背弛后最弱的反弹」；
- `029:38` 对第二、三种说「这二种情况就是发生转折的两种情况，原理是一样的，只是相应的力度有区别」。

据此，`029:52` 的「完全分类」分类对象是「某级别背驰后」将发生的级别/走势演化与反弹力度强弱：最后中枢扩展、更大级别盘整、上级别反趋势。它给出的区分判据是「反弹中第 1 个前趋势最后一个中枢级别的次级别走势」是否回抽最后一个中枢。

研究结论：`029:52` 不是 MACD 面积 / 振幅 / ForceL 的分量清单，也不是把这些数值量列入「完全分类轴」；它是背驰后结果形态与反弹力度的分类句。

## 2. #991 I-3 撤 `leg_strength` / `leg_macd_force` 教义地位的本地依据

### 2.1 本轮未读取在线 #991/#874

本轮三次查询均失败：

- `gh issue view 991 --json number,title,body,comments` → `error connecting to api.github.com`
- `gh issue view 874 --json number,title,body,comments` → 同上
- `gh issue view 1256 --json number,title,body,comments` → 同上

所以下面只说「本地落盘证据显示」，不声称已直接读取 #991/#874 issue 正文。

### 2.2 #991 commit 落盘信息

`git log --grep` 找到 commit：

- `9cac8faa69 (ticket-991-retirement) chore(force): I-3 #991——退役清理：force_conformance 删除（样板留档）+ recursive_t 力度定义撤教义地位`

该 commit message 明写：

- `recursive_t/divergence.rs 文件头退役标注：leg_strength/leg_macd_force 教义地位撤销（046:16 引证被 #874 推翻），「冻结中」措辞撤销（G2 裁定四）；函数保留供 47 探针复现历史读数（#772 判例），禁进生产判定路径`

当前文件也保留同一标注，`rust/src/recursive_t/divergence.rs:48-52`：

```rust
//! ★#991 I-3 退役标注（G2 #978 裁定二/四，2026-08-16）：本文件的力度定义（`leg_strength`/
//! `leg_macd_force`）**教义地位已撤**——「中枢个数律」引证被 #874 推翻（046:16 是一天走势的
//! 看盘分类），「冻结中」措辞同步撤销（生产判定路径=theta_v0/NT 实盘链，本模块不在辖，#799 不禁）。
//! 函数保留供 47 个研究探针复现历史读数（#772 判例：探针是可复用测量装置）；**不得进任何
//! 生产判定路径**；新读数一律用教义判据（ForceL，#873 经 #989 反查）。
```

### 2.3 #874 推翻「中枢个数律」引证的文本细节

被本地 #991 标注点名的原文是 `046:16`。逐字：

`docs/chanlun/text/blog/046-第46课.md:16`：

> 1、只有一个中枢；2、两个中枢。3、没有中枢，其力度依次趋强。

但它的上下文限定在 `046:14`：

> 本来要继续写上节所遗留的第一个中枢形成后走势类型的分类问题，但发现太多人，连每天如何看盘都搞不清楚，这事情可能更迫切，所以先说一下。当然，如果是按某级别的严格操作，每天具体怎么走是关系不大的，走势不会因为交易是按天来的就有什么本质的不同。但针对每天的走势进行一些分类，至少是一个好的辅助。一天的交易是4小时，等于有8个30分钟K线组成的一个系统。把3个相邻30分钟K线的重叠部分当成一个每天走势上的一个中枢，那么，一般来说显然，任何一天的走势，无非只有三类：

同段后文 `046:28` 又说第三类是：

> 三、没有中枢（最强的单边走势）这是最强的单边走势，8根K线，没有相临3根是有重叠部分的，一旦出现这种情况，就是典型的强烈走势，一旦出现这种走势，该日K线都是具有重要意义的。

因此，本地证据链里的「#874 推翻」细节是：

1. `046:16` 的「1/2/0 个中枢，力度依次趋强」是在「每天如何看盘」的辅助分类里说的；
2. 其构造前提是一天 4 小时 = 8 根 30 分钟 K 线，并把 3 根相邻 30 分钟 K 线重叠部分当成每日走势上的中枢；
3. 这不是一般走势段、任意级别 leg、或 recursive_t 单元链的通用「中枢个数律」；
4. 即使严格留在该日内分类域，原文的顺序也是「1 个→2 个→0 个」依次趋强，不是「中枢个数」的单调标量函数，更没有给出对段内 `inner_zhongshu_count` 求和的聚合规则；
5. `046:28` 同行还警告「无中枢」的强烈日内走势并不保证趋势延续，在大级别日 K 线中可能是骗线；
6. 用它支撑 `leg_strength = Σ inner_zhongshu_count` 的教义地位，属于把每日辅助分类外推成一般力度定义；本地 #991 commit 与当前文件头将此称为「046:16 引证被 #874 推翻」。

相邻旧引证也不能补足该公式：`065:94`【作者后续帖，非课内正文】只说线段之下可以用「类似背驰的力度比较方法」谈类背驰，并未把力度定义为振幅或中枢计数。

### 2.4 被撤的是教义地位，不是函数物理删除

当前 `rust/src/recursive_t/divergence.rs` 仍有实现：

- `:75-87`：`leg_strength`，先求 `inner_zhongshu_count` 之和；若全段无内部中枢，则退化为几何振幅 `hi - lo`。
- `:89-99`：`leg_macd_force`，上涨段取 `area_pos` 之和，下跌段取 `area_neg` 之和。
- `:108-112`：`macd_area_diverges` 用 `leg_macd_force` 比较 A/C 段面积。

但同文件头 `:51-52` 明确这些函数只保留供研究探针复现历史读数，不得进入生产判定路径；新读数一律用 ForceL。

旁证：#951 实施报告 `chanlun/review-results/issue951-impl-20260814.md:129-133` 已把 recursive_t 收束为「无生产消费者、只服务历史复现的冻结件」；`docs/agents/generation-constitution.md:35` 明写「π 是唯一现役引擎」。同时 `rust/src/recursive_t/stream.rs:30-31` 说明 recursive_t 的「现役」只指仍有非测试调用者，非 π 生产路径。

## 3. 力度分量在「完全分类轴清单」中的位置

### 3.1 本票所问的「完全分类轴」：029:52 背驰后结果轴

本票的语境是原文完全分类轴清单，不是 `formal/Tlayers/Operational.lean` 另一个「级别间关系三轴」的同名分类。与本票直接相关的原文轴只有一条：

| 原文轴 | 轴的值域 | 原文锚 | 与力度数值分量的关系 |
|---|---|---|---|
| 某级别背驰后的级别与力度 | ①最后中枢级别扩展；②该级别更大级别盘整；③该级别以上级别反趋势 | `029:22/24/26/52`【正文】 | 这是背驰已成立之后的**结果/演化轴**；MACD 面积、振幅、ForceL 是上游力度测量或 Weak/背驰确认判据，不是这三个结果类的并列成员 |

已落盘的原文盘点 `.chanlun/review-results/issue1218-doctrine-complete-classification-20260824.md:48`、`:81` 也把 029:52 登记为「某级别背驰后的级别与力度」三型轴；该报告是二级盘点证据，本报告的主证仍是上表所列 blog 原文。

### 3.2 形式化侧对 MACD/振幅/ForceL 的层级边界

`formal/Tlayers/Signal.lean:149-154` 给 T11 的范围：本模块只补「力量维衰减结构骨架」，并明写不声明 MACD 面积 / DIF peak / 价格强弱等 L2 命题。

`formal/Origin/ForceInterface.lean:39-44` 对 MACD 面积计算标注为 L2，不在本文件实装；`:68-84` 把 `measure` 与 `strength` 分开：

- `measure : α → Force` 是抽象 MACD 代理函数，具体计算由外部 L2 引擎提供；
- `strength : α → Rat` 是真力度，承载已裁定的力度定义。

同文件 `:295-321` 把真实 MACD 引擎是否满足 mono/faithful 标成 L2 待验证义务，`:428-440` 再次说明 MACD L2 引擎仍缺，真力度实例落在 `Origin.ForceVelocity` 的 `strength := impulse` 方向。

研究结论：形式化边界也支持上述分层：MACD 面积是代理/测量接口的 L2 候选，价格振幅/价格强弱不在 T11 的 L0 声明内，ForceL/`strength` 是真力度接口；三者都不是 029:52 那三个背驰后结果类。

### 3.3 当前正本层：ForceL 的名分

`.chanlun/definitions/beichi.md:336-342` 的 #873 裁定给出：

> **定义：`L(段) = 该段的速度净增量 = v(段末) − v(段初)`；速度在结构给定的最小单元（笔）上量，即「这一笔涨了多少 ÷ 这一笔用了几根 K 线」。**

同处 `:342` 标注名分：

> **名分：`[新缠论:推论]`**

同文件 `:374-383` 把推理链写成：原文起点是 MACD 同色柱面积代理；中间经 DEA/DIF/速度近似；终点推出「力度 = 速度的净增量 = 冲量」。这说明 ForceL 是**教义正本中以 `[新缠论:推论]` 标注的力度定义**，不是缠师原文直接给出的 MACD 公式，也不是 029:52 的「三种情况」分类轴。

当前代码落点：

- `rust/src/theta_v0/parser/segment.rs:835-840`：`segment_force_l` 实现 `L(段) = v(末笔) − v(首笔)`；
- `rust/src/theta_v0/classifier/cand_predicate.rs:28-31`：条件 4 默认 `ForceL` 教义判据 `L(C)<L(B)`，`MacdArea` 降为显式对照档；
- `cand_predicate.rs:84-92`：生产默认 `DivergenceGauge::default()` 为 ForceL，MacdArea 为显式对照档，且无笔数据时 ForceL 无源不判、不降级；
- `cand_predicate.rs:362-381`：实现上同时计算 `macd_c_lt_a` 与 `l_c_lt_b`，最后统一进 `confirm_divergence_l`。

### 3.4 MACD 面积的位置

缠师原文中 MACD 面积可作力度代理；正本层也保留这一点。`.chanlun/definitions/beichi.md:374-383` 的推理链第 1 步列为原文起点：MACD 同色柱面积是力度代理。

当前 theta_v0 代码：

- `rust/src/theta_v0/classifier/divergence.rs:231-238`：`segment_macd_area` 文档锚到 `060:44` 的同色柱面积口径；
- `:251-258`：`is_divergence(prev_area, curr_area)` 判定后段面积严格小于前段面积；
- `cand_predicate.rs:28-31` 与 `:277-280`：MacdArea 在 div_cand 当前口径中是显式对照档，不是默认教义档。

因此，MACD 面积的位置是「旧缠论原文支持的力度代理 / 度量层代理 / 显式对照档」，不是 029:52 的完全分类轴，也不是 #873 后的默认真力度定义。

### 3.5 振幅与多 proxy 力度族的位置

当前 `rust/src/theta_v0/classifier/divergence.rs:381-392` 将 DIF、价格振幅、速度等称为「力度原语层」与 selector/judge 层的多 proxy，并明确不下沉 buy1 定义层；`:412-425` 的 `ForceFeatures` 字段包括：

- `macd_area`
- `dif_peak`
- `price_amplitude`
- `price_speed`

同文件 `:442-455` 的 `ForceStateA5` 描述 A/C 段在 proxy 族上的支配态，但也承认完整 `𝒜_ℓ` 尚缺递归次级别力度 `SubMovePower`；`:475-497` 当前实际比较的是 4 proxy，并在注释中说明 TV 第 5 维已处决。

`.chanlun/definitions/beichi.md:477-487` 对旧实现族也有处置表：

- MACD 段面积：度量层默认代理、`[旧缠论]`；
- 振幅×时长：退场，理由是原文零根据且踩收敛通则兜底；
- 嵌套深度：冻结，解冻前不许出现在任何生产判定路径上；
- 四档 `DivergenceGauge{MacdArea, ThetaDom, Conjunction, ThetaLex}`：研究对照臂，不产生交易决策。

研究结论：价格振幅是多 proxy 力度原语/研究或 selector 特征的一维；不是缠师原文 029:52 的分类轴，也不是当前默认教义力度判据。

### 3.6 力度与候选身份的关系

`.chanlun/definitions/qujiantao.md:760-763` 裁定：

> **教义条款：`Cand^δ_ℓ` ＝ 结构宽候选 ＝ `dir(s) = −δ ∧ Comparable ∧ Extreme`。力度（MACD 面积衰减）**不进候选身份**，它是候选之后的独立门。**

这给出了一个关键定位：即使在当前生产候选链里，力度也不属于候选身份本身，而是候选之后的独立 Weak/确认门。它也不是「完全分类轴」。

## 4. 消费力度族判据在「研究线」的名分证据

### 4.1 代码现状：消费仍在

`rust/src/recursive_t/divergence.rs` 中：

- `:181-241`：`judge_trend_divergence` 仍计算 `structural = trend_structural_filter(...)` 与 `macd = macd_area_diverges(...)`，再由 `combine_modes` 组合；
- `:287-330`：`trend_diverging_segment` 在趋势分支同样消费 `trend_structural_filter` 与 `macd_area_diverges`；在盘整分支直接比较 `leg_strength(leave_leg) < leg_strength(enter_leg)` 并比较 MACD 面积；
- `:394-430`：`judge_consolidation_divergence` 同样消费 `leg_strength` 与 `macd_area_diverges`；
- `:559`：`nest_chain_complete` 存在；
- `:603`：`d_top` 存在，`:641` 调 `nest_chain_complete`。

`rust/src/recursive_t/rec_stream.rs:326-339` 与 `:372-386` 直接调用 `crate::recursive_t::divergence::trend_diverging_segment(...)`，形成 recursive_t 研究/旧线上的消费面。

### 4.2 名分：不是 π 生产路径，但仍有研究/复现/非测试调用者

同一文件头存在两层看似冲突、实则分域的标注：

1. `rust/src/recursive_t/divergence.rs:48-52`：`leg_strength`/`leg_macd_force` 教义地位已撤，保留供 47 个研究探针复现历史读数，不得进生产判定路径；新读数用 ForceL。
2. `rust/src/recursive_t/divergence.rs:54-59`：本文件的 `trend_diverging_segment`/`d_top`/`CONSOL_DOWN_DIAG` 在 `rec_stream.rs` 路径被直接调用，是 recursive_t/(b) T 引擎编译期硬依赖，名分标注为「现役」。

结合 `rust/src/recursive_t/stream.rs:30-31`：

> 本支的"现役"仅指"仍有非测试调用者"，非"π 生产路径"。

再结合 `docs/agents/generation-constitution.md:35`：

> **π 是唯一现役引擎**。

可得研究性描述：消费力度族的 `judge_trend_divergence` 等判据仍在 recursive_t 旧线/研究线/复现线可达；但这不等于它们拥有 π 生产判定路径名分，也不恢复 `leg_strength`/`leg_macd_force` 的教义地位。

### 4.3 已落盘报告对这个中间态的表述

`.chanlun/review-results/issue1226-frontier-scan-synthesis-20260825.md:25` 记录：

> **recursive_t 力度族消费面**（B 观察3）：#991 I-3 已撤教义地位，但 judge_trend_divergence/judge_consolidation_divergence/trend_diverging_segment/nest_chain_complete/d_top 仍消费 leg_strength/leg_macd_force（研究线 rec_stream 生产路径调用）——「撤了力度、没撤消费它的判据」中间态。

该报告是二级证据；本报告以上述源码行作为主证据。这里的「生产路径调用」须按 recursive_t 旧线自身语境理解，不可上升为 π 生产路径。

## 5. 汇总：三问的 research 口径

1. **#991 I-3 的依据**：本地 commit `9cac8faa69` 与当前 `rust/src/recursive_t/divergence.rs:48-52` 明确撤销 `leg_strength`/`leg_macd_force` 教义地位；撤销理由是 #874 推翻了「046:16 可作一般中枢个数律」的引证。原文核验显示 `046:16` 只在每日走势辅助分类、8 根 30 分钟 K 线系统、3 根相邻 K 线重叠成日内中枢的上下文中成立。
2. **MACD 面积 / 振幅 / ForceL 在完全分类轴清单的位置**：与本票直接相关的原文轴是 029:52 「某级别背驰后的级别与力度」三型结果轴。MACD 面积、价格振幅、ForceL 都不是该轴的结果类；它们处在上游力度测量/Weak 确认层。MACD 面积是原文支持的力度代理与对照/度量层读数；振幅是多 proxy 特征或已退场旧口径；ForceL 是教义正本中按 `[新缠论:推论]` 标注的真力度定义，也是当前 div_cand 默认 Weak 判据。
3. **消费力度族在研究线的名分**：`judge_trend_divergence`、`judge_consolidation_divergence`、`trend_diverging_segment`、`nest_chain_complete`、`d_top` 仍在 recursive_t 路径消费 `leg_strength`/`leg_macd_force` 族读数；但本地 #991 标注禁止其进入生产判定路径，新读数用 ForceL。该状态可描述为研究/复现线中间态：「撤了力度、没撤消费它的判据」。

## 6. 未完成/不可声称项

- 未直接读取 GitHub #991/#874/#1256 的在线 issue body/comments；若后续裁定需要票面原话，必须在 GitHub API 可用时重查。
- 本报告不裁定是否删除 recursive_t 消费面、不裁定 ForceL 与 MACD 面积的最终优先级；只记录现有证据。
- 本报告未修改任何代码或定义文件。
