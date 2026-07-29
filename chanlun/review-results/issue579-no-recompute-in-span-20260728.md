# #579 研究：`no_recompute_in_span` 5 只——是否并「C 候选集变化」入重算触发口径

> 角色：research（只读）；本 session 未改仓内任何文件，只写本报告。
> 票据：#579（#527/#559 遗留②）；根因票 #523；分水岭票 #527（commit `7b4547b623`）+ 修复轮 #559（commit `77d3554332`）
> 数据源：`/tmp/wt527fix-post-100k.dump`（#527/#559 修复轮实测产物，corresponds to 当前 HEAD `04c360a410` 之下的
> `77d3554332`；`nest_lifecycle.rs`/`p123_fast_replay.rs` 在本 session 观察窗内未被并发改动，dump 与源码口径一致）。
> 复现方法见 §5。全部分析基于既有 dump 与源码静态阅读，**未跑新探针二进制**（理由见 §3）。

## 0. 结论（先给答案，理由见下）

**不建议并入「C 候选集变化」作为独立触发条件——它不是一个真实存在的正交信号，而是
`forest_epoch ∨ frontier` 的严格子集（代码自身在 `p123_fast_replay.rs:1042-1045` 声明的
purity 不变量决定了这一点：`locate`/`extreme` 是 confirmed 侧与 frontier 值的纯函数，
「候选集变化」不可能在两者都不变时发生）。5 只逐条现场证实：即使把触发条件放到最宽
（每 bar 强制重算），这 5 只在其名义活跃期 `[c_start, completed_at]` 内依然拿不到窗——
阻塞点不是触发频率，是两种更深的结构原因（§1 模式 A/B），其中模式 B（4/5）的唯一解法是
「从已确认段回放重建 C」，那正是 #523 根因 / R43 裁定明令禁止的架构。**

建议：5 只维持现状（无原因码升级为可修复窗口），但**归因标签应从「重算未触发」订正为
「候选 C 在 parser 实时视图中从未作为独立 pending 对象存在过」**——本质上与既有
`structure_not_locatable`（29 只）同源，只是发生机制更极端（不是「扫描了但不满足结构判据」，
而是「根本没有被扫描的可能」）。折中方案（在诊断面新增标签、不改变触发口径与产窗行为）
在 §4 给出。

## 1. 5 只逐条钉现场

对每只，用 `/tmp/wt527fix-post-100k.dump` 的 `L1_LIVE_RECOMPUTE`（每次重算的 frontier 快照）、
`L1_LIVE_MISS`/`L1_LIVE_HIT`（逐 run 诊断行，按 `b_center_start` 过滤）、`COMPLETION_SIGNAL`/`REV`
三类行独立重建现场。

### 1.1 as_of=1693　span=(1334,1352)　seg_a=(628,718)　b_center=737　side=Long

```
最近前置重算：as_of=1320 frontier=(1205,1225,Up)                    ← 不同段，早于 c_start
唯一匹配 frontier 的重算：as_of=1368 frontier=(1334,1352,Down)      ← 恰好命中该 C 的坐标，但比
                                                                        completed_at 晚 16 bar
该次重算诊断行数：0（b_center_start=737 在全量 dump 的 L1_LIVE_MISS/HIT 中零命中）
```

**为何两次触发事件都没命中**：前一次（1320）frontier 是另一段，与本身份无关；后一次（1368）
frontier 坐标虽精确匹配 `seg_c_full=(1334,1352)`，但 `recompute_lifecycle_window_stems` 的产窗循环
`for level in 1..2.min(tower.len())`（`p123_fast_replay.rs:1610`）在 `tower.len() < 2` 时是**空区间**——
此时 L1（`tower[1]`）尚未被塔构造出任何窗，循环体一次都不执行，**连一条诊断行都不会写**。
全量 dump 中第一条非 `no_active_frontier` 的 `L1_LIVE_MISS` 出现在 `as_of=1693`
（`b_center_start=1352`——注意这是紧接在本身份 C 段之后的**下一个**中枢，不是本身份自己），
证实 L1 分类器在 bar≈1693 前对这段早期历史尚未产出任何可分类窗口。

本身份的 `Observed`/`FirstProvable`/`StructureCompleted`/`Confirmed` 四条 REV 全部压缩在
`as_of=1693` 同一 bar——provider 是在 L1 塔终于"追上"这段早期历史后，一次性把它补记为
完成对象，而不是先给活窗再完成。

**机制标签：模式 A——L1 塔尚未构造完成（bootstrap 不可用）**。这与"触发条件是否够宽"无关：
即便每 bar 强制重算，`tower[1]` 在 bar<1693 附近本就不存在可分类的窗，扫描循环体仍是空的。

### 1.2 as_of=33862　span=(33670,33799)　seg_a=(33111,33270)　b_center=33274　side=Long

```
最近前置重算：as_of=33849 frontier=(33591,33828,Up) l0_segments=253   ← 同一个自 33591 起持续
                                                                          延展的 Up pending 段，
                                                                          从未在 33670 处转向
触发重算：    as_of=33862 frontier=(33799,33828,Up) l0_segments=255   ← l0_segments 一步跳 253→255
                                                                          （+2，非逐段确认）
```

`b_center_start=33274` 在 `L1_LIVE_MISS`/`HIT` 中的**第一条记录**就在 `as_of=33862`
（`reason=structure_not_locatable`），即这个 B 中枢直到本身份完成的同一 bar 才第一次被扫描到——
且此时 frontier 已经是 `(33799,...)`（本身份的**下一段**），不是 `(33670,33799)`。此后该中枢的
命中（`as_of=34029/34063/34572`，`reason=window`）产的窗 `c_start=33950`，**始终不是**本身份的
`33670`。

**为何两次触发事件都没命中**：33849 的 frontier 仍是自 33591 起连续延展的 Up 段（未曾在 33670
分叉出一个独立的 Down 段）；33862 触发时 `l0_segments` 一步跳 2（`253→255`），说明 parser 在这一
bar **回溯性地把一条长 Up 段重划成了两段**（`(33591,33670)` 之类的前段 + `(33670,33799)` 的目标段 +
新 pending `(33799,...)`）——目标段 `(33670,33799)` 是这次重划的**产物**，从未作为独立 pending
frontier 单独存在过，故它不可能在任何一次「重算」里被单独观察到。

**机制标签：模式 B——回溯性多段重划（parser tail cascade redivision）**。

### 1.3 as_of=64500　span=(64374,64459)　seg_a=(63651,63789)　b_center=63877　side=Long

```
最近前置重算：as_of=64274 frontier=(64224,64239,Up) l0_segments=493
触发重算：    as_of=64500 frontier=(64462,64493,Up) l0_segments=495   ← +2 一步跳
```

`b_center_start=63877` 早在 `as_of=64274`（**先于**本身份 c_start=64374）就已经被扫描并
`reason=structure_not_locatable`——说明该中枢分类本身不晚，但从 64274 到 64500 长达 226 bar
**没有任何一次重算**（frontier 值全程不变，代表这 226 bar 里没有新的 pending 极值也没有 confirmed
侧变化），本身份的整段生命都落在这个"寂静期"内，直到 64500 一次性 `+2` 段跳变把它连同下一段
一起回溯确认。

**机制标签：模式 B（同 1.2）**，且叠加"该中枢早已可分类但那段时间恰好全局静默"的巧合。

### 1.4 as_of=86879　span=(86812,86828)　seg_a=(86365,86470)　b_center=85727　side=Long

```
最近前置重算：as_of=86518 frontier=(86483,86517,Up) l0_segments=671  （该次重算 stems_after=1，
                                                                        产出的是别的候选的窗）
触发重算：    as_of=86879 frontier=(86836,86878,Up) l0_segments=673   ← +2 一步跳
```

同一 bar（`as_of=86879`）**同时**出现两条 `COMPLETION_SIGNAL`（本身份 Long `seg_c=(86812,86828)`
+ 另一只 Short `seg_c=(86483,86799)`）——印证这是一次较大幅度的回溯重划，一次性确认了不止一段。
`b_center_start=85727` 在 86189/86293/86511/86879/86938 反复 `structure_not_locatable`，直到
86949 才首次 `reason=window`，命中 `c_start=86836`（同样不是本身份的 86812）。

**机制标签：模式 B（同 1.2）**。

### 1.5 as_of=94693　span=(94447,94510)　seg_a=(93934,93956)　b_center=93994　side=Long

```
最近前置重算：as_of=94370 frontier=(94272,94359,Up) l0_segments=747
中间重算：    as_of=94636 frontier=(94590,94635,Down) l0_segments=748 （+1，先确认一段）
              as_of=94678 frontier=(94590,94672,Down) l0_segments=748 （同段延展，stems_after=1
                                                                        ——产出的是别的候选的窗）
触发重算：    as_of=94693 frontier=(94590,94672,Down) l0_segments=750 ← +2 再跳一次
```

`b_center_start=93994` 早在 94283（先于 c_start=94447）已被扫描并 `structure_not_locatable`，
94636/94678 两次 `reason=window` 命中的是 `c_start=94590`（同样不是本身份的 94447）。本身份
是这次分两步（`+1` 再 `+2`）的回溯重划里被跳过的那一段——它的完成事件最终判定
`Invalidated{NeverConstituted}`（力度未达阈值，`area_a=9.76e8` vs `area_c=1.72e9`），
即便拿到活窗，最终归宿也不会是 Confirmed。

**机制标签：模式 B（同 1.2）**。

### 1.6 五只汇总

| as_of | span (c_start,completed_at) | b_center | 模式 | 前置重算 frontier | 触发重算 l0_segments 跳变 | 该中枢首次被扫描时机 |
|---:|---|---:|---|---|---:|---|
| 1693 | (1334,1352) | 737 | A（塔未构造） | (1205,1225,Up) | +1（1368，但 tower.len()<2 空循环） | 从未（全 dump 零命中） |
| 33862 | (33670,33799) | 33274 | B（回溯重划） | (33591,33828,Up) | +2 | 与完成同 bar，frontier 已过 |
| 64500 | (64374,64459) | 63877 | B（回溯重划） | (64224,64239,Up) | +2 | 完成前 226 bar 即已知，寂静期空转 |
| 86879 | (86812,86828) | 85727 | B（回溯重划，双完成同 bar） | (86483,86517,Up) | +2 | 完成前已多次扫描均 miss |
| 94693 | (94447,94510) | 93994 | B（回溯重划，分两步） | (94272,94359,Up) | +1→+2 | 完成前已多次扫描均 miss |

全局背景数：2269 次重算里，`l0_segments` 单步跳变 ≥2（即"回溯多段确认"事件）仅 **61 次
（2.7%）**——4/5 本身份的完成事件都精确落在这 61 次事件之一上，不是巧合分布。

## 2. 反事实量化

**若把「C 候选集变化」并入触发条件，5 只无一能拿到窗——净增量 = 0。**

论证不依赖跑新探针（§3 说明为何这一步是可以纯代码推导的 L0 结论），而是直接引用production
code 自己的 purity 声明：

> `p123_fast_replay.rs:1042-1045`：「活窗结构分量在「confirmed 侧变（forest_epoch）∨ active C
> frontier 变」时重算……frontier 是纯值 ⟹ 值不变 ⟹ 定位输出不变（locate/extreme 都是其纯函数），
> 跳过重算是等价优化而非行为改动。」

`recompute_lifecycle_window_stems`（`p123_fast_replay.rs:1592-1608`）里"候选集"（`centers`/
`kinds`）的全部输入是 `tower[level]`（`let windows = &tower[level]`），而 `tower[level]` 的任何
字节变化都由 `forest_epoch` 捕获（`classifier/mod.rs:2235-2241` 注释：「E1-E3 折叠（forest_dirty，
含 E1 L0 塔重建 / E2 upper extend / E2a frontier pop / E3 cascade clear）循环后统一 bump」）。
故「候选集变化」在定义上是 `forest_epoch` 变化的**真子集**——它不能比 `forest_epoch ∨ frontier`
更早或更多地触发任何一次重算。

对 5 只逐条应验证：

- **1693（模式 A）**：候选从未被扫描的原因是 `tower.len()<2`（L1 塔本身不存在），与"该不该
  重算"完全无关——重算再多次，`for level in 1..2.min(tower.len())` 在 `tower.len()<2` 时仍是空区间。
- **33862/64500/86879/94693（模式 B）**：目标 C 段从未作为**独立** frontier 值存在过——它是
  回溯重划的产物，在它"存在"的那个回合，parser 的实时 tail 展示的是一个**更长、尚未拆分**的段
  （如 1.2 中 `(33591,33828,Up)` 覆盖了 33670）。「候选集变化」若要捕获这些身份，必须让
  `recompute_lifecycle_window_stems` 把**已确认段**也当作候选 C 来重新扫描——这正是 #523
  根因论定、R43 裁定与 #527 分水岭明令禁止的架构（"C 只能是行进中段，禁 confirmed segments
  回放重建"，`nest_lifecycle.rs:1327-1333`）。不是"触发口径设计问题"，是**教义边界问题**。

**对 156/46/46 归因结构与反超/寿命统计的影响：零。** 5 只无论并不并入新触发条件，都不会产生
活窗，因此：
- earlier-live 156 不变；
- L1 缺口 46（= 8+21+6+0+2+4+5）内部只是**标签**层面从 `no_recompute_in_span` 并回
  `structure_not_locatable` 一类（本质同源：结构上此刻不可定位），总数仍是 46；
- ForceOvertake 35 条链、非闪现寿命 192 只（1/79.5/423 bar）、`IdentityVanished` 两类拆分
  （25/12）**均不受影响**——这 5 只从未产生过活窗身份，不进入寿命/反超统计的输入集合。

## 3. 成本侧

**未新建 /tmp 探针二进制**——原因：§2 的论证是从代码自身声明的 purity 不变量（一条已在生产
代码里写明、且有专门测试覆盖的等价性断言）严格推出的 L0 结论，补一个"跑一遍看看频次差"的
探针**不会产生新信息**，只会重新验证一个已经被源码结构性保证为真的命题（同义反复，
参照 `formalization-validity-domain` 规则对 L0→L1 信息增量为零的判据）。若把「候选集变化」
按票面字面意思实现为"扫描已确认段作为候选 C"，那不是"频率"意义上的成本增加，而是**架构类别
变化**——退回 #523 根因方案（"C 取自已完成 lower legs"），此时成本讨论的度量单位也会从
"重算次数" 变成 "是否重新引入首见即完成 bug"，与 P-H3 复用率无关。

仍给出触发口径本身的现有成本基线（供后续若讨论"扩大触发范围"的其它提案时参照）：

| 指标（BTC 100k，来自现有 dump，无需重算） | 值 |
|---|---:|
| 总 bar 数 | 100,000 |
| `L1_LIVE_RECOMPUTE` 总次数 | 2,269（2.269%） |
| 其中 `l0_segments` 单步跳变 ≥2（回溯多段确认） | 61（2.7% of 重算，0.061% of bar） |
| 其中 `l0_segments` 单步跳变 =1（普通逐段确认） | 677 |
| 其中 `l0_segments` 跳变 =0（纯 frontier 值变，无新确认段） | 1,529 |
| P-H3（`refresh_lifecycle_cache` 侧，独立缓存，非本触发） | provider_requests=2099 / reevals=87 / reuses=2012 = 95.855% |

（P-H3 一栏与本票触发口径是两套不同的缓存——P-H3 是 `evaluate_run` 的 per-(level,run) dirty
判据，本票讨论的 `forest_epoch ∨ frontier` 是 `recompute_lifecycle_window_stems` 的独立触发；
两者不应混算成本，本表分列列出以防误读。）

## 4. 推荐（不替裁）

**推荐：不并入「C 候选集变化」作为新触发条件。**

- 模式 A（1/5）与触发频率无关，扩大触发口径无法修复；真正缺口是 L1 塔 bootstrap 早期的
  分类盲区，若要处理需在 §7 遗留 4（"L1 46 只缺口可修性"）项下单独评估是否值得为塔构造早期
  阶段单独补一条冷启动路径——本票不替裁是否值得做。
- 模式 B（4/5）的唯一解法是把已确认段重新纳入候选 C 扫描范围，这与 R43/#523/#527 裁定的
  "C 只能来自行进中段" 教义边界直接冲突，**不是本票 Scope 内可以"并"的口径调整**，
  是需要单独教义评审的架构问题。

**折中方案（如果需要，仅供参考，不预设裁定）**：在诊断面（`L1LiveDiagRow`/dump）新增一个
可审计标签，把当前 `no_recompute_in_span` 的 5 只按本报告 §1 的机制标签重新打成
`bootstrap_unavailable`（模式 A）与 `retroactive_redivision_skipped`（模式 B），不改变触发口径、
不改变产窗行为、不改变 46 只总数——只是让归因表的可读性与本票研究结论对齐（当前
`no_recompute_in_span` 这个名字暗示"是触发口径的问题"，而 §1/§2 证明其实不是）。是否值得
为 5 只单独开一个诊断标签，本票不替裁。

## 5. 结果包六要素

1. **结论**：见 §0。5 只逐条现场（§1）+ 反事实量化（§2：净增量=0）+ 成本侧（§3：无需新增
   探针，代码 purity 声明已给出 L0 证明）+ 推荐（§4：不并，理由是教义边界而非频率）。
2. **定义依据**：
   - 触发口径本体：`p123_fast_replay.rs:1042-1078`（`forest_epoch ∨ frontier` 判据与 purity 声明）；
   - 候选集来源：`recompute_lifecycle_window_stems`（`p123_fast_replay.rs:1592-1666`，`centers`/
     `kinds` 全部派生自 `tower[level]`）；
   - `forest_epoch` 覆盖域：`classifier/mod.rs:2235-2241`（E1-E3 折叠，含塔任意层级字节变更）；
   - C 只能来自行进中段的教义边界：`nest_lifecycle.rs:1327-1333`（#527 分水岭登记）、
     R43 裁定（`chanlun/escalate/r43-lifecycle-ruling-20260721.md:31-39`）、#523 根因判词。
3. **边界条件（结论在何时翻转）**：
   - 若未来 `forest_epoch` 的 bump 判据改为"不覆盖"塔某一层级的变更（当前文档承诺覆盖全部
     E1-E3），则 §2 的子集论证失效，需重新核验"候选集变化"是否仍是 `forest_epoch` 的子集；
   - 若模式 B 的 4 只中出现"目标 C 段确实短暂作为独立 frontier 存在过，但被某个未覆盖的 bug
     漏记"的反例（本报告未发现此类证据——4 只的前置重算 frontier 值均确认覆盖了目标 C 的整个
     生命窗），则诊断应升级为「触发遗漏」而非「架构边界」，推荐随之翻转为「应并」；
   - 若编排者裁定放开"确认段可作候选 C 回放"这一教义边界（即推翻 R43/#523/#527 的现有裁定），
     则本票的"不并"结论随裁定失效，届时讨论的已不是触发口径而是候选来源本体。
4. **下游推论**：
   - `no_recompute_in_span` 这个诊断标签名称具有误导性（暗示"触发不够"），下游若据此提出
     "调宽触发口径"的实装票，会直接撞上 §2 的子集论证——本报告可作为该类提案的预先否证；
   - 5 只在 §4 折中方案下的重新打标不影响 156/46/46 顶层归因，只影响 46 只内部子类命名；
   - #559 遗留 2（`located_other_c`/`located_other_center` 6 只）与本票模式 B 共享同一根因族
     （B 中枢分类与 C 段确认的时序错配），但 6 只与本票 5 只不重叠（前者是"扫描到了但选错/
     漏选候选"，本票是"从未被扫描"）——两票应分别推进，不要合并成一个修复。
5. **谱系引用**：#523（PanLive provider 接缝缺口，根因）；R43（pan 行进中对象合法存在，
   同时是"C 不可从已完成段重建"的教义边界）；#527（`provide_l1_active_pan_live_windows`
   分水岭裁定）；#559（影子评审 C1，本票 5 只的原始钉因来源）；`formalization-validity-domain`
   （本报告 §2/§3 的核心论证是 L0 纯代码推导，§1 的现场重建是 L2 真实数据单窗读数，两者未混淆）；
   `no-patch-mentality`（拒绝"调宽触发口径"这种表面能解决但实际是无效功的补丁式提案）。
6. **影响声明**：本报告为只读产出，**未修改仓内任何文件**，只新增本文件。未改
   `p123_fast_replay.rs`/`nest_lifecycle.rs`、未改 issue/map 状态、未关票、未改 roster、
   未跑新的 cargo build/test（全部分析基于既有 dump 与源码静态阅读）。

## 6. 复现方法

本报告的全部数据来自已存在的产物文件，复现只需重新解析，无需重新编译/重放：

```bash
# dump 来源（#527/#559 修复轮已产出，本票直接复用）：
#   /tmp/wt527fix-post-100k.dump（24.7MB，125,176 行）

# 5 只现场重建（本票方法，示例：定位任一身份的重算历史）：
grep "b_center_start=<B_CENTER> " /tmp/wt527fix-post-100k.dump | grep "L1_LIVE_MISS\|L1_LIVE_HIT"
grep "seg_a=(<A0>, <A1>) seg_c_full=(<C0>, <C1>) b_center_start=<B_CENTER> " /tmp/wt527fix-post-100k.dump | grep "^REV"

# 全局 l0_segments 跳变分布（§3 表）：
grep -c "^L1_LIVE_RECOMPUTE" /tmp/wt527fix-post-100k.dump   # 2269

# 若 dump 文件被清理，重新生成（#527/#559 报告 §8/§9.7 命令，本票未执行，仅记录复现路径）：
cd rust && CARGO_TARGET_DIR=/tmp/kimi-nest-target-579 cargo build --release --bin p123_fast_replay
P116_MAX_BARS=100000 P116_CKPT=0 \
  P421_LIFECYCLE_DUMP=/tmp/issue579-repro-100k.dump \
  ./target/release/p123_fast_replay ../analysis/data_cache/btc_1m_full.json \
  > /tmp/issue579-repro-100k.stdout 2> /tmp/issue579-repro-100k.stderr
```
