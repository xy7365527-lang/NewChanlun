# 出场=反向区间套证书消费实装卡 + MFE 百分比止盈撤回（nest-exit-implementation-card）

**日期**：2026-07-19 ｜ **工位性质**：撤回（已执行）+ 设计（不实装）工位
**编排者最高口径**：出场必须走区间套背驰——买点买卖点卖，所有买卖点必须经区间套背驰确认。
止盈不是新规则、不是百分比阈值——**止盈 = 反向买卖点**（区间套确认版）。进出场同一条线。
closePred 不加百分比止盈项（拍阈值违反 v3/090）。区间套把滞后压到因果下限但不承诺零滞后——
浮盈回吐一部分是「不预测」的必然代价；修的是「等错层级」的滞后，不是 causal floor 本身。
验收按可修复分量 vs 内禀分量分别落账。
**纪律**：v3 硬禁令（不引入概率/统计推断作决策基础、不用回测验证策略、不假设 EMH）；
090（禁简化实装、禁补丁方案、声明=能力；照实否定合格）；无 git mutation；主仓零写。

---

## §1 MFE 百分比止盈撤回（已执行，验证完毕）

### 1.1 撤回对象与理由

W3 曾在 `rust/src/theta_v0/strategy/exit.rs` 实装「第五析取 MFE 回撤止盈」：
`MfeState` / `mfe_init` / `mfe_advance` / `mfe_take_profit_hit` / `MFE_TP_RETRACE_NUM/DEN`（1/2）/
`exit_decision_for_nested_mfe` + 三个 `mfe_*` 单测。该实装为 **armed-but-inert 未接线**
（runner.rs/nautilus 两个调用方从未改调，生产路径逐字节不变）。

撤回理由（编排者裁定）：回撤比例 X=1/2 是**拍出来的百分比阈值**——无论冠以「算子声明式
政策参数」名义，其触发条件是「浮盈峰值回撤 ≥ X·MFE」这一价格幅度判据，不经过任何区间套
背驰确认；止盈项作为第五析取并入 closePred，正是编排者明文禁止的形态（closePred 不加
百分比止盈项，拍阈值违反 v3/090）。故**整体撤回**，不留降级形态。

### 1.2 撤回 diff 摘要（干净，无残留引用）

| 项 | 撤回前 | 撤回后 |
|---|---|---|
| `exit.rs` 相对 HEAD 的 diff | +372/−2（F1 + MFE） | **+179/−2（仅 F1 + 撤回登记注记）** |
| MFE 符号（`MfeState`/`mfe_*`/`MFE_TP_*`/`exit_decision_for_nested_mfe`/`take_profit`） | 定义 + 调用 + 测试 | 全仓 grep 零命中（仅 exit.rs:33-37 撤回登记注记提及名字作谱系记录） |
| `exit_decision_impl` 签名 | 8 参（含 `take_profit: bool`） | 7 参（恢复四析取 + F1 锚原语义），exit.rs:142 |
| `mfe_*` 单测 | 3 个 | 0 个 |
| F1 实装（`short_diff_anchor_hit` exit.rs:280 + `short_diff_anchor_hit_semantics`/`f1_anchor_replaces_reverse_for_shortdiff` 两测） | 在 | **原样保留，逐字节不动** |

谱系登记：模块头 exit.rs:33-37 留有「MFE 百分比止盈撤回登记（090 谱系）」注记——记录曾
存在、因何撤回、替代方案在哪（本卡），非功能残留。

### 1.3 验证结果

- `cargo test --release --lib`：**1749 passed; 0 failed**（128 ignored）。基线 1752 = 撤回后
  1749 + 删除的 3 个 mfe_* 测试，差值逐笔对应，**零变红**。
- `cargo check --lib`：零 error（warning 数与撤回前同量级，无新增）。
- 生产路径 bit-exact：MFE 从未接线，撤回不触碰任何通电路径；F1 路径、exec.rs `close_pred`
  Lean 锚四析取、mod.rs close 桶路径全部逐字节不动。

---

## §2 出场=反向区间套证书消费实装卡（设计，不实装）

### 2.1  doctrinal/裁定锚

- 买卖点定义同源：一类点只由趋势背驰构成（027:66）；盘背与二、三类及类一关系（027:18）；
  三买/三卖的中枢离开语义（024:44）。
- 关③ `pan-terminal-endorsement-ruling-20260718.md`：P1（Trend 族终端背书 = 破 B 一类点，
  二/三类非法，:68-73）；P2（Pan 盘背族同向一/二/三类全合法，:75-85）。
- 区间套证书契约：`formal/Origin/IntervalNestCertificate.lean` + gpt 结果包 §6（line 312-369），
  Rust parity 实装 `rust/src/theta_v0/strategy/nest.rs:122`（`chi_bool`，全定义，绝不未定义）。
- E2E-L 一等谱系 + 五钟：`doc-divergence-endtoend-prototype-20260718.md` §6.1（:194 起）——
  事件五钟 `observed_at/first_provable_at/structure_end_at/confirmed_at/invalidated_at`，
  谱系两钟 `opened_at/closed_at`；完成态锚 `chanlun/plans/mainline-merged-roadmap-20260717.md:75`
  （E2E-N5：五钟全由同一前缀产生）、:73（E2E-N3：E2E-L 成为 Classification 原生输出）。

### 2.2 现状行号锚（代码直读）

| 机制 | 锚 | 现状 |
|---|---|---|
| 退出谓词（策略层） | `exit.rs:129-243`（`exit_decision_for_nested`（:129）→ `exit_decision_impl`（:142）） | 四析取 `¬ParentValid ∨ χ^{σ_p} ∨ Stop ∨ RiskClose` + F1 段终点锚（depth>0） |
| 反向项的消费对象 | `exit.rs:161-164` → `exec.rs:255-261`（`reverse_signal`） | **裸 BspBits**（持多遇 sell1\|sell2\|sell3 任一即触发）——无区间套确认 |
| close 桶路径 | `interp.rs:1194-1209`（`interpret` fold 规则2） | 同级活动腿遇反向 bits 即入 𝒟_x，同样只读裸 bits |
| 退出类型映射（结构决定） | `interp.rs:250-258`（`reverse_exit_type`） | ShortDiff→CloseShortDiff；trigger_class=3→ReduceCore；1/2→CloseRoot |
| 区间套证书机器 | `nest.rs:122-145`（`chi_bool`） | N^δ 良基递归，结构形式完整 parity |
| 证书喂入现状 | `interp.rs:378-391`（`nest_confirm`） | **单级末端 Conf 基例**（候选自身级别 e=ℓ，无跨级塔链；:375-377 诚实标注 Classification 不导出塔） |
| 候选证书字段 | `interp.rs:285/:304-315`（`assemble_gamma` 的 `nest_confirmed`） | **贴标**——fold 规则1-4（interp.rs:1188-1224）从不读它，零拒绝率 |
| E2E-N3 证书链 | roadmap:73 现状列 | `NestCertificate` 为塔外装配物（sidecar，91 张：A=46/B=45，`nest-cert-bsp-recon-20260717.md`）；econ 门逐级 `is_sub∧cand∧递归`（econ_positive.rs:890-1009）是塔外近似 |
| 五钟 | `runner.rs:1645-1656`（`LedgerOpen`） | 只有 `entry_bar` 一钟；`first_provable_at` 需跨 bar 记忆（e2e-fix-roadmap Q3，:443-445） |

### 2.3 缺席照实登记：trig-bsp-class-mapping-recon 不存在（090）

任务口径要求「基于 trig-bsp-class-mapping-recon 的映射结论定退出反向点类别」。经全仓检索
（`chanlun/`、`.chanlun/`、`rust/`、`docs/`，文件名与内容双向 grep），**该文档不存在**
（疑似他工位未落盘或名称近似）。照实否定合格，不伪造引用。本卡类别映射改用以下**实存**
单源锚，若该 recon 后续落盘，§2.4 类别表须与其逐行对账：

1. `reverse_exit_type`（interp.rs:250-258，G4 #134 单源判据，生产统计层已接线）；
2. 关③ P1/P2 合法性约束（Trend 仅一类 / Pan 一、二、三类全合法）；
3. `bsp-type-usage-audit-20260719.md` §1（518 笔类型消费画像：一类 0.97% 构成性稀有，
   方向隔离 518/518 逐笔成立）——画像事实，不作绩效论证（v3）。

### 2.4 设计：反向区间套证书消费谓词

**核心命题**：退出 = 消费一张**方向相反**的区间套证书，与进场同一台机器（`chi_bool`）、
同一谓词（N^δ χ^δ=1）、同一谱系对象（E2E-L `LineageKey` 可复验），仅 δ 方向镜像——
χ⁺ 链确认开多 / χ⁻ 链确认平多；χ⁻ 链确认开空 / χ⁺ 链确认平空。closePred **不加新析取项**，
是把既有反向析取项 χ^{σ_p} 的**消费对象**从裸 BspBits 升格为 typed nest 证书。

**谓词定义**（持仓声部 v，当前 bar t）：

```text
ExitNestCert(v, t) ⟺ ∃ 证书链 c = [ℓ₀, …, ℓ_k]（ℓ₀ > … > ℓ_k = e_v）：
    side(c) = flip(side(v))                       // 反向：持多消费卖侧链，持空消费买侧链
  ∧ chi_bool(e_v, c) = true                        // 同一台机器（nest.rs:122），含区间套 ⊆ 逐边见证
  ∧ first_provable_at(deepest(c)) ≤ t              // 最深处证书确认即退出（E2E-L 五钟首证钟，
                                                   // roadmap:75——全由同一前缀产生，非事后回填）
  ∧ lineage(c) 是 E2E-L Closed 谱系对象             // LineageKey/EdgeKey 闭区间见证可复验（§6.1）
```

**伪码（前 → 后）**：

```rust
// 前（现状，exit.rs:161-164，裸 bits 消费）：
let reverse_raw = groups.get(i)
    .map(|ds| ds.iter().any(|d| reverse_signal(hv.side, &d.bsp)))
    .unwrap_or(false);

// 后（设计，未实装）：反向项消费对象升格为 typed nest 证书——
let reverse_cert = groups.get(i)
    .map(|ds| ds.iter().any(|d| {
        reverse_signal(hv.side, &d.bsp)               // 方向镜像不变（exec.rs:255-261）
            && nest_cert_confirmed(d, i)              // chi_bool(e_v, chain(d)) == true
                                                    // ∧ first_provable_at(deepest) ≤ i
    }))
    .unwrap_or(false);
// X' = ¬ParentValid ∨ reverse_cert（depth=0）∨ F1 段终点锚（depth>0）∨ Stop ∨ RiskClose
// ——四析取结构不动，无第五项，无百分比。
```

**与进场同谓词论证**：进场侧 Γ 候选的 `nest_confirmed`（interp.rs:285）= `chi_bool` 对候选
自身链的读出；本设计出场侧 `nest_cert_confirmed` = 同一 `chi_bool` 对**反向**链的读出。
机器同一（nest.rs:122 单实现）、谓词同一（N^δ 良基递归：Candidate∧⊆∧递归，末级 Conf）、
谱系同一（E2E-L 谱系对象，进场链与出场链都是 `LineageKey` 实例，仅 root event 的 side 相反）。
「买点买卖点卖」自此是**同一形式化对象的两个方向实例**，不是两条平行规则。

**反向点类别（结构决定，映射单源 = `reverse_exit_type` interp.rs:250-258）**：

| 触发反向证书最深点的类别 | 被关腿 entry_v | 退出类型 | 语义 |
|---|---|---|---|
| 一类（趋势背驰，027:66） | 非 ShortDiff | `CloseRoot` | 根反转，全平 |
| 二类（一类的次级确认） | 非 ShortDiff | `CloseRoot` | 同属根反转语义（interp.rs:245-247 读法） |
| 三类（中枢离开确认） | 非 ShortDiff | `ReduceCore` | 核心仓减仓，非清仓 |
| 任意类 | ShortDiff | `CloseShortDiff` | P7：子声部关闭语义压过触发类判定 |

合法性过滤（关③）：Trend 族证书的终端背书点只认一类（P1）；Pan 族一/二/三类皆可（P2）。
类别不是参数、不是选择——由最深点自身的结构谓词（`third_class_proof?`/`extreme_proof`，
E2E-L §6.1 事件载荷）决定。

**滞后论（验收分账口径）**：现状裸 bits 反向项的滞后含**可修复分量**——错层级触发（本级别
未背驰时被次级别裸 bits 抢先/拖延关闭，即「等错层级」）；升格为区间套证书后，触发点 =
最深处证书的 `first_provable_at`，滞后压到**因果下限**（证书可证即触发，不预测未来 bar）。
**内禀分量**（因果下限 ≠ 零滞后：确认需要次级别走势走出来，此间浮盈回吐是「不预测」的
必然代价）不验收为缺陷。两分量分别落账：可修复分量以「同数据下触发点从裸 bits 时点移到
first_provable 时点」的机制差验收；内禀分量照实登记，不设归零目标。

### 2.5 bit-exact 影响与验证协议

**本卡只撤回+设计，不实装**——本节约束的是后续实装工位：

1. **未接线零变化**：实装必须以 wrapper/新函数落地（先例 = 关⑤ `exit_decision_for_nested` 对
   `exit_decision_for` 的委托形态，exit.rs:110-141），默认路径逐字节不变 + 回归锁测试
   （先例 = C4 根回归锁，`cascade_root_only_regression`）。
2. **exec.rs `close_pred` Lean 锚四析取不动**（bit-exact 保留）；证书消费发生在策略层
   （exit.rs / interp.rs），同 F1 的实装层位。
3. **全绿纪律**：`cargo test --release --lib` 零变红（当前基线 1749 passed）。
4. **真门拒绝率实证**：接线后须 dump 统计反向候选中 `nest_confirmed=false` 被拒比例——
   拒绝率 = 0 即贴标（090 声明=能力，不得称「证书门」）。注意现状单级基例的拒绝率天然
   近零（`nest_confirm` 对方向候选几乎恒真，见 §3.2），真拒绝率依赖 E2E-N3 真链喂入。
5. **E2E 挂钩**：五钟持久化（E2E-N5/S5）、Closed 谱系原生输出（E2E-N3/S4）、消费事务
   （E2E-N7/S8）按 roadmap:79 依赖序先行；本设计是 N7 消费点的出场臂。
6. **机制验收非 pnl 验收**：出场时点 ≤ 段终点/证书确认点的机制一致性（v3：不以回测
   盈亏验证策略）。

---

## §3 进场准入升格设计：裸 BSP 候选 → typed nest 证书当门

### 3.1 现状（贴标，零拒绝率）

`interpret` fold（interp.rs:1188-1224）规则1-4 的分流依据只有 `dir`/`bsp_class`/slot 占用——
`c.nest_confirmed`（interp.rs:85）从不进入任何规则，是随候选落盘的证书**字段**（trades.jsonl
的 `nest_confirmed` 列即此贴标）。进场准入实际消费的是**裸 BSP 候选**（`trig.bsp_class` 方向
bits + min_class，interp.rs:283-284）。

### 3.2 单级基例的诚实限度（v0 当门拒绝率 ≈ 0）

`nest_confirm`（interp.rs:378-391）是单级末端 Conf 基例：`confirm_ok = conf_plus/conf_minus`
与 `candidate_dir`（interp.rs:333-349）的方向读出**同源**——凡由六 bit 消歧出方向的候选，
基例恒真；仅 struct_break_dir 护栏1 回退候选（零 bit + 破中枢方向，interp.rs:339-347）
会得 false。故 v0 把基例当门，拒绝率天然近零——**照实声明：基例当门不是真门**，真拒绝率
只能来自 E2E-N3 跨级真链（`Candidate^δ ∧ 次级严格低一级 ∧ 次级套本级 ∧ 递归`，nest.rs:130-138
的 ℓ>e 分支——区间套 ⊆ 与层级关系会构造性拒掉单级读不出的伪候选）。

### 3.3 升格设计（两步，均不实装）

**v0（基例当门，过渡形态）**：fold 规则3 开启臂前加拒绝臂——

```rust
// 规则3'（设计）：typed 证书门——基例不成立 ⟹ 𝒦_x（记录不执行）
if !c.nest_confirmed { buckets.record.push(*c); continue; }
```

效果：struct_break_dir 回退候选（无 conf 确认）被构造性拒绝，贴标变门。拒绝率须 dump 实证
（预期低但 > 0）。签名/默认路径不动 + 回归锁（同 §2.5-1 纪律）。

**v1（E2E-N3 真链当门，目标形态）**：候选的证书字段从 `nest_confirmed: bool` 升格为 typed
证书对象——`chain: Vec<NestLevel>`（真嵌套塔导出，`coverage_elements_with_tower`
interp.rs:424 的塔已就位；缺的是 Classification → 候选链的原生喂入，interp.rs:375-377 的
coverage-engine-needs-tower-export-bridge 缺口）+ `first_provable_at` 五钟（E2E-N5）。门谓词
= `chi_bool(e_v, &chain)` 真链读出。此时拒绝率来自跨级区间套的构造性失败（层级倒挂、
⊆ 不成立、末级 Conf 缺席），是真当门用。依赖序：E2E-N2（跨级边见证）→ N3（Closed 谱系）
→ N5（首证钟）→ 本门（N7 消费点的进场臂）。

**与出场的对称性**：v1 落地后，进场门与出场门消费**同一 typed 证书对象**——进场消费
`side(c) = 开仓方向` 的链，出场消费 `side(c) = flip` 的链；进出场同一条线（编排者口径）
在类型层闭合。

---

## §4 授权边界与未决项

1. 本卡 = 撤回（已执行）+ 设计（不实装）。§2/§3 任何实装须另开工位授权。
2. trig-bsp-class-mapping-recon 缺席已照实登记（§2.3）；若落盘须与 §2.4 类别表对账。
3. F1 段终点锚（depth>0 ShortDiff 腿）与证书门的关系：本设计只升格 depth=0 根的通用反向
   项；F1 锚是否也要求区间套证书化（对冲腿退出同样「买点买卖点卖」），留作裁定项——
   现行 F1 锚是结构代理（中枢对向沿首次触及），未走证书链，倾向答案为「是，同一口径」，
   但需编排者裁定后实装。
4. runner.rs / nautilus 调用方、mod.rs close 桶路径（interp.rs:1194-1209 规则2 的证书化）
   均在本工位授权外，登记为待接线项。
