# m3 win9 断言①违例：炸点 bar 逐笔账勘察（issue #226 勘察阶段）

- 日期：2026-07-24
- 勘察票：#226「修复：m3 win9 断言①违例（fold/S7 物理域，HEAD 既存，Core{1} 残余 518.0948228547287）」
- 勘察基准：main HEAD `125c645e16`（#220 路④落地后）
- 纪律：只读勘察；临时 `eprintln` 探针（coverage.rs 断言①挂载点，不改任何决策/记账）用后撤销还原；
  构建口径 `RUSTFLAGS="-C debug-assertions=on" cargo test --release`（门在纯 release 被剥离，同 #220 口径）
- 复跑：m3 单跑两轮（701.18s / 第二轮同点复现），win7=543、win8=541 typed 与 #220 基线逐位一致后，
  win9 同点同额复现：`Core{1}` 残余 **518.0948228547287**（与票面逐位一致）

## 0. 白话总结（10 行）

1. 断言①是物理侧硬门：一类卖点平完某级别后，该级 Core 物理持仓（sep_legs）必须为零（S7「级别内全平是必须」，#209 终态）。
2. win9（2024-02-17→2024-08-16）bar=17032：一类卖（sell1，level 1）把持仓长腿 (1,35) 关掉了——关得很干净，fold/账面两侧都正常。
3. 但同一根 bar 上还有一个**更低级别（level 0）的开仓候选**，它的父 carrier 恰好就是这条刚被关掉的 (1,35)。
4. 现行代码为了让这个候选通过 AncOK 持仓准入（§13：父容器必须在场），会把父链从持久注册表「restore」回活动集——而 registry 里被关的腿**从不作废**（生产路径从不 invalidate），于是 (1,35) **当 bar 复活**，重新拿到 518.09 的 sizing，重新变成真实物理持仓。
5. 持仓腿的 restore（held 路）有子树清仓 𝒟_x^† 兜底——复活的被关父会命中种子被再清掉（#183 已锁）；但 **open 候选父注入（ℬ_x 段）不经 𝒟_x^†**——这是 #183 在案、#179 裁决保留的不对称面（当时为保「一类/二类当场反手」留的口子，注释里写着「留 fog」）。
6. 复活的 (1,35) 正是被关的那条腿自己——残余 = 触发腿自身 q_units，518.0948228547287 一分不差。
7. 判据链（判别式证明）：残余腿 work idx=176 在**树前缀**（非候选段）；它在 held 循环里因已关被跳过，不可能从 A_t 段回来（回来必被 𝒟_x^† 种子命中）；唯一能把它以树前缀 idx 推进 ℬ_x 段的路径 = open 父注入 restore 的 id_idx 复用。QED。
8. 教义定性：S3「父声部终结时后代被 AncOK 剪除」——父本 bar 已被裁决终结，子候选不应借父复活入场；这与 #183 给 A_t 段定的「restore 不得复活被关父」是同一条教义，ℬ_x 段漏了执行。
9. 修复形状（最小）：open 父注入 restore 遇「当 bar 已被裁决终结」（∈ buckets.close 种子）的祖先即中断，不复活；候选因父链断裂被统一 AncOK 正常剪除。held 路不动（𝒟_x^† 已兜住，轨迹 bit-exact）；反手候选自身元素不经 restore、不受该滤——ID-3「允许当场反手」不动。
10. 门口径（#209 批次边界）本身无违教义——残余是真实物理持仓（参与 p̃→真实下单），不是账面幻觉；门该炸。修的是物理域的复活泄漏，不是门。

## 1. 炸点事实（逐笔账转储，win9 bar=17032，lvl=1）

```
A1FIRE bar=17032 lvl=1 residual=518.0948228547287
TRIG  leg (1,35) Long Ambient parent_id=None op_parent=None
      cand class=1 dir=Short lvl=1 bits={sell1} gidx=1 nest=true
SURV  (1,35) Long role_v=Ambient q=518.0948228547287 parent_id=None
      origin=tree-prefix widx=Some(176) in_prev=true in_closed=true
      reg=Some((dir=Short, level=1, invalidated=false, snapshot_present=true, structural_parent=None))
PREV  (1,35) Long Ambient boundary=true closed=true
CLOSED(1,35) Long Ambient exit=CloseRoot | trig class=1 dir=Short bits={sell1} gidx=1
SEP-ALL (1,35) Long Ambient q=518.0948228547287 parent_id=None
NEXT  (1,35) Long parent_id=None op_parent=None boundary=true
```

逐笔对照：

| 事实 | 读法 |
|---|---|
| (1,35) 在 prev_active（Long/Ambient/边界根） | 本 bar 之前的合法持仓长腿 |
| sell1（lvl 1，已确认）一类触发 ⟹ (1,35) CloseRoot 关闭 | fold/channel 关闭本身**正常**（#202 外部化 closes，S2 CloseRoot） |
| 同 bar (1,35) 又出现在 next_active，q=518.09，origin=tree-prefix(widx=176) | **复活**——且复活源是树前缀元素复用（restore 的 id_idx 命中路径），不是候选段拷贝 |
| in_prev=true ∧ in_closed=true ∧ 在 next_active | 残余腿 = 触发腿自身：被关 → 当 bar 复活 |
| SEP-ALL 仅此一条 lvl 1 腿 | 残余 518.09 = 该腿 q_units 本身，无第二条腿 |
| registry[(1,35)].dir=Short | 候选 upsert 覆盖序「candidate 胜」（persistent.rs merge 断言2 在案行为）：sell1 触发候选附着 carrier (1,35)、eps=Short，把 registry 条目 dir 覆盖成 Short。与本次复活无因果（复活走的是 id_idx 树前缀复用，元素 eps=Long 取自树）——但记录在案 |

## 2. 机制定性（判别式证明）

**断言**：(1,35) 经 **open 候选父注入 restore**（coverage.rs open 循环的 (I-1) 父注入段）复活入 ℬ_x 段，绕过了 𝒟_x^† 子树清仓。

**证明**（排除法，每步有代码锚）：

- P1：残余腿 widx=176 < candidate_start ⟹ 来自树前缀元素复用。候选自身入 raw 只 push 候选段 idx（candidate_start+gamma_index）⟹ 不是候选拷贝。
- P2：(1,35) ∈ buckets.close（in_closed=true）⟹ held 循环 `closed.contains` 跳过，不可能经 (A_t∖𝒟_x) 对位回 raw。
- P3：held 路 restore（Stale LiveDetached 持仓腿的 op_parent 链恢复）注入的元素落在 raw **A_t 段**（a_t_end 之前）⟹ 必过 𝒟_x^†；(1,35) 是种子本身，命中即清（#183 `t4_subtree_close_unifies_production_active_set_step` 锁「restore 不得复活被关父」）。它活着 ⟹ 不是 held 路。
- P4：open 路 restore（open 候选父注入）注入的元素落在 raw **ℬ_x 段**（a_t_end 之后）⟹ 不经 𝒟_x^†（#183 在案不对称，coverage.rs「★不对称记录」注释：ℬ_x 段 open 父注入 restore 不经 𝒟_x^†，#179 裁决保留面，留 fog）。
- P5：restore 内部 `id_idx.get(&pid)` 命中树前缀（registry snapshot_present=true 与之一致）⟹ 复用 idx 176，元素 eps=Long（树方向）⟹ 复活为 Core{1} 多腿，strategy_target_legs 给 sizing 518.09 ⟹ sep_legs Core{1} 残余 = 518.0948228547287。
- ∴ 唯一路径：某 open 候选（depth>0 子声部，parent 链经 (1,35)）触发 open 父注入 restore，把**当 bar 刚被一类卖裁决终结的父 carrier** 从树前缀复活进 ℬ_x 段；统一 AncOK 因父在场放行了子候选。QED。

**触发候选身份**（第二轮探针实证，§4）：`OPEN-CAND lvl=0 class=3 dir=Long role=FollowParent bits={buy3} gidx=0 src=16943`——level-0 **三类买点顺父开多**候选（父 (1,35) Long 持仓期间 ℓ0 买点开级联多腿，S6 顺父面）；同 bar level-1 一类卖点（顶背驰）终结父结构的同一 fold 内，该候选规则3 开仓、其父注入 restore 把刚被终结的父复活，父子双双准入（NEXT：(0,146) Long parent=(1,35) 与 (1,35) Long 同在场）。同 bar 另有 (2,4) Short / (2,5) Long 两条 level-2 边界根持仓（更高级别结构，与本违例无关，跨级别独立）。

**教义锚**：

- S3（ADR 0001）：父声部终结 ⟹ 后代经 AncOK 剪除（子树清仓）——父本 bar 已终结，子候选借父复活入场 = 终结不终。
- #183 归一时给 A_t 段的同一教义实装（restore 复活的被关父命中种子同清）；ℬ_x 段的不对称是 #179 裁决**保留面**（保留动机 = 反手候选与种子同 id 时按 id 误清会禁绝一类/二类反手，#200 基线 typed=2 实证）——但保留面的字面是「ℬ_x 不经 𝒟_x^†」，其**副作用**（restore 复活被关父 + 子候选借尸入场）并非反手保护所必需：反手候选是**候选自身元素**（候选段拷贝，可同 id 反向重开），与 restore 注入的**祖先元素**是两物。滤 restore 不滤候选 ⟹ 反手不动、复活封死。
- ID-3「允许当场反手」：反手 = 同 carrier 反向**候选**重开（ℬ_x 候选段），本修复不触碰候选推送（#216 三规则原样）⟹ 保留。

**与门口径的关系（090）**：断言①门（#209 批次边界：一类批末该级 Core 物理清零）口径本身**不违教义**——残余腿是真实物理持仓（进 next_active、拿 sizing、计入 p̃→真实净额下单），不是账面/口径幻觉；门炸得对。修复落在物理域（restore 复活泄漏），门口径零改。

**与 #220 的关系**：#220 修的是 opened 外化双重配对（账面域幽灵）；本违例是物理域 restore 复活——两 bug 同根于「ℬ_x 段 restore 扩集」这一保留面，但机制、证据链、修复点各自独立。#220 结案评论的归因实验（HEAD+断言②降级探针同额复现）已证本违例 HEAD 既存、与 #220 零因果。

## 3. 修复形状（勘察建议，按根因最小改动）

`restore_ancestor_chain_from_registry` 增「关闭种子」参数：walk 中遇 pid ∈ buckets.close（当 bar 已被裁决终结）即中断（计数 `restore_break_closed_seed`），不复活、不上溯。

- held 路调用点传空切片：A_t 段既有 𝒟_x^† 兜底（语义/轨迹 bit-exact 不变）。
- open 路调用点传 buckets.close：种子不再复活；触发候选因父链断裂被统一 AncOK 剪除（S3 兑现）。
- 候选推送零改 ⟹ 反手（ID-3）保留；种子已在 raw（经反手候选同 id 重开）时 restore 首检 already_in_raw 先命中收敛，不误伤。
- 探针封闭性：`restore_calls = restore_complete + restore_break_already_in_raw + restore_break_registry_lost + restore_break_closed_seed`（runner.rs 697 L2 自检同步四项化）。

## 4. 第二轮探针（触发候选补证，701.31s 同点同额复现）

```
A1FIRE bar=17032 lvl=1 residual=518.0948228547287（与第一轮逐位一致）
OPEN-CAND lvl=0 class=3 dir=Long role=FollowParent bits={buy3} gidx=0 src=16943
NEXT (2,4) Short boundary=true
NEXT (2,5) Long  boundary=true
NEXT (0,146) Long parent_id=Some((1,35)) op_parent=Some((1,35)) boundary=false   ← 借尸准入的子腿
NEXT (1,35) Long parent_id=None boundary=true                                    ← 复活的被关父（残余本体）
```

- 形态定名：**「同 bar 一类终结父 × 次级顺父候选挂尸」**——一类卖点（lvl 1，顶背驰）终结父 carrier 的
  同一 fold 内，三类买点（lvl 0）顺父级联候选规则3 开仓，(I-1) open 父注入 restore 把种子父从
  树前缀复活（id_idx 复用 idx 176），子腿借复活父链 AncOK 准入。
- 子腿 (0,146) 是 Core{0}（FollowParent×Long）——残余只算 lvl 1（父），但子腿的准入本身是同一
  教义违例（S3：父终结 ⟹ 后代不得立）。修复后父子同清。
- 对照：(2,4)/(2,5) 两条 level-2 持仓照常存活（S7 跨级别独立——低级别终结不强制高级别），
  佐证修复面只需精确到「种子子树」，不伤更高级别。

## 5. 未验证/留疑（090）

- 修复后轨迹重排，win9 余程/win10 是否暴露下游新违例（旧轨迹被本炸点阻断，无从知晓）——修复票复跑自明。**已自明：暴露第二炸点（Core{2}，见 §6 追加）——门口径教义冲突面，按纪律停手待裁**。
- registry 条目 dir 被候选 upsert 覆盖（candidate 胜序）对「restore 从 registry 取元素」路径的方向保真影响：本炸点复活走 id_idx 树前缀复用（方向取树=Long），未消费 registry dir；若他窗形态走 registry 推送路径，复活腿方向 = 候选覆盖后的方向——另案留疑，非本票根因。
- m6 各窗是否藏同形态（open 父注入 × 当 bar 被关父）：m6 现绿说明同形态未在一类批末对齐出现（或未出现）；修复后 m6 复跑核对轨迹守恒。**已核：m6 修复后全绿（508s，三窗 resid≈0，断言①②③违例=0）**。
- BTC train 16000 窗是否含本形态（open 候选父链 × 当 bar 被关种子）：决定三锁是否翻动——以修复后双口径复跑为准。

---

## 6. 追加（2026-07-24）：修复后第二炸点——Core{2} 残余 14.72 与断言①门口径的教义冲突（停手待裁）

### 6.0 经过

#226 修复（§3 形状）落地后 m3 复跑（debug-assertions release，823.32s）：win7=535、win8=539 逐窗绿（typed 计数随轨迹重排：543→535、541→539，分区守恒逐窗成立——挂尸候选不再入场的预期效果），win9 **越过原炸点 bar=17032**（断言①原违例清零），推进至 **bar=222794** 炸**新的断言①违例**：`Core{2} 残余 14.723373768219792`（不同级别、不同形态）。第三轮只读探针（用后已撤销还原）取转储如下。

### 6.1 第二炸点逐笔账（win9 bar=222794，lvl=2）

```
A1FIRE2 bar=222794 lvl=2 residual=14.723373768219792
TRIG   leg (2,100) Long Ambient parent=(3,22) | cand class=1 sell1 lvl=2 nest=true
SURV   (2,98) side=Short role_v=FollowParent q=14.723373768219792 parent=(3,22)
       origin=tree-prefix(widx=2054) in_prev=true in_closed=false
       prev_entry_v=Some(ShortDiff)   ← runner 冻结入场身份 = 短差
       reg=(dir=Long, invalidated=false, snapshot_present=true)   ← dir 被候选覆盖，同 §1 在案行为
PREV   (2,98)  Short entry_v=ShortDiff parent=(3,22) closed=false   ← 幸存腿本体
PREV   (2,100) Long Ambient parent=(3,22) closed=true    ┐
PREV   (2,101) Long Ambient boundary=true closed=true    ├ 三条多腿全关（CloseRoot×sell1）
PREV   (2,104) Long Ambient parent=(3,23) closed=true    ┘
NEXT   (3,22) Short boundary=true（存活的空父）+ (2,98) Short 在内共 13 条
```

### 6.2 机制定性（与第一炸点不同类）

- 幸存腿 (2,98) 是**持仓腿**（非 restore、非当 bar 新开）：Short 方向，父 (3,22) 是**存活的 level-3 空头**边界根。
- fold 行为**教义正确**：sell1（lvl 2）反向谓词只关多腿（reverse_signal(Long, sell-bits)），三条 Ambient Long 全关（S2「该级该方向全部清仓」字面成立）；(2,98) 是 Short——S2「该方向」不含空腿，S7 空仓账「平空=一类买点」、S6「短差平=ℓ 级买点」——**没有任何教义条文要求 level-2 一类卖点平掉这条空腿**。其入场身份（runner 冻结 entry_v）= ShortDiff（S6：父级多头时 ℓ 级卖点开短差空头——入场时父为多；声部 S1 一经建立不可变）。
- 炸门原因在**门的身份来源**：断言①按 sep_legs 的**当 bar sizing 层重算角色**（operation_role：当前父已翻空 ⟹ 顺父=FollowParent）调 `identity_of(FollowParent, Short)=Core{2}` 把这条短差/空仓腿计入「必须清零的本仓」。
- 教义账（#197 裁定层，断言②所读）按**冻结 entry_v** 记账：该腿入**短差账**而非 Core{2}——**同一系统内，教义账说它不是本仓，门说它是不仓**。同 bar 断言② balance(Core{2})=0 正常通过（账面无 Core{2} 残余）——两门对同一腿的归属判定互相矛盾。
- 冲突点不在 fold（关闭正确）、不在 restore（本腿非注入）、而在**门口径**：#209 终态门把「当 bar 重算角色 FollowParent×Short」纳入一类卖批末清零集——这与 S7 本仓账定义（「开多首开/加仓记本仓账」——空腿非开多）、S6（短差独立账、买点平）、S1（声部身份入场冻结、不可变）三方张力；且该口径在**一类买批**对称过度（买批只平空，却会要求多腿 Core{L}=0）。

### 6.3 纪律结论（按 #226 票面「门口径违教义则停手回报」）

- 第一炸点（本票根因）：修复正确且已验证——复活父是真实物理 Core{1} 多仓，教义必须清零，门炸得对；修复后该点清零。
- 第二炸点：**门口径本身的教义冲突面**——幸存腿按教义账/关闭谓词/声部不可变三方均系合法存活，门按漂移的当 bar 角色把它算作待清本仓。修门=改 #209 用户裁 A 的批次边界口径（须裁决：门的身份源应否对持仓声部取冻结 entry_v、对 restore/新开元素取当 bar 角色；及 FollowParent×Short 在一类卖批的归属）；修 fold/physical 侧去关这条腿则直接违 S2/S6/S7（一类卖点平空/平短差）。
- **故停手**：断言①门零触碰；第二炸点留待裁决（建议另立票：门口径身份源裁定，候选形态 = 门对持仓声部改读冻结 entry_v——账面/物理两口径归一——连同 FollowParent×Short 归 Core 的 #185 裁定在一类卖批语义下复审）。
- 修复后 win9 bar 222794 之后、win10 的断言①②状态：**未知**（被第二炸点阻断）。win9 至 bar 222794 为止断言②无违例（②先于①评估则早已 panic）。

### 6.4 090 备查

- (2,98) 的入场考古：prev_entry_v=ShortDiff（runner TW ctx 冻结值）vs 当 bar 角色 FollowParent（父 (3,22) 现为 Short）⟹ 入场时父方向为多、现翻空（或角色重算沿 sibling 漂移）——父方向翻转而 ElementId 稳定的机制未深挖（不影响 §6.2 定性：无论顺父空还是短差，一类卖点均无权平它）。
- 第二炸点在第一炸点修复前轨迹上是否同点既存：不可知（旧轨迹阻断于 bar 17032）；其机制（门对合法空腿计残余）与轨迹无关——任何推进到该类「sell 批 × 顺父空/短差幸存腿」形态的轨迹都会撞上。
- 第三轮探针已撤销还原（备份 /tmp/coverage.rs.bak-a1-226-fix）；第一、二轮探针备份 /tmp/coverage.rs.bak-a1-226。
