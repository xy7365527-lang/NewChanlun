# #817 N-6 考据报告：两条生产 `N^δ` 严格度不同

**基线 commit**：`5cc6dcd22a`（`docs(#817): 区间套正本升 v0.2（追加 N-2 节，受影响代码清单并到 38 条）`）
**日期**：2026-08-05
**性质**：**考据报告，不做裁定。** 只查事实、摆各方承重论据、给形态清单与代价。裁定由人做。
**授权**：只读 + 写本报告。**本轮零 `.rs`／`.py`／`.lean` 改动。**

> ⚠️ **开工即撞第十次**：worktree 初始 HEAD = `19b4015927`（祖传线，`rust/src/theta_v0/`、`formal/` 皆不存在）。已 `git fetch` + `git reset --hard origin/main-rewritten` 归正到 `5cc6dcd22a`，本报告全部读数在归正后的树上取得。

**前置（已读，不重复其工作）**：`.chanlun/definitions/qujiantao.md`（v0.2，729 行）、`.chanlun/review-results/issue817-n1-nesting-coordinate-system-20260804.md`、`issue817-n2-descent-stop-level-20260804.md`、`issue817-n3-nesting-operator-doctrine-20260804.md`、`.chanlun/definitions/zhongshu.md`、`.chanlun/review-results/issue809-doctrine-triage-20260730.md`、`AGENTS.md` §收敛通则／§总缝规则。

---

## 0. 摘要（先给结论，后面逐条举证）

### 0.1 ★★ 票面前提逐条核实：一半对、一半错

| # | 票面说法 | 实测 |
|---|---|---|
| **A** | 「`n_delta` **宽**、`chi_bool` **严**」 | **方向对，但理由不是票面写的那个。** 与 #815 不同，本组不是前提反了——沿规范遗忘映射（链 → 证书：丢 `lvl`、取 `chosen()`）**确实是严格包含 `A(chi_bool) ⊊ A(n_delta)`**（§2）。但「严」这四条条件里**没有一条**在生产上被求值（§4）⟹ 严格度差是**类型层的**，不是**行为层的** |
| **B** | 「`n_delta` 用于回测准入门」 | **成立但要加两条限定**：(1) `econ_positive.rs:398` 那个准入门 **生产不可达**（唯一消费者是 `#[ignore]` 集成测试）；(2) 真在生产 π 上的那条（`admission.rs:300`／`:1017` ← `fill.rs:4941`）**由 `THETA_NEST_CERT_GATE` 门控，默认关**（§4.2、§5） |
| **C** | 「`chi_bool` 用于环 2」 | **成立，且它是两条里唯一无条件常开的。** 但它在生产上恒等于**一个 bit**（`conf_plus`／`conf_minus`），四条「严」条件全部恒真或死码（§4.1） |
| **D** | 「两条通则都判不了」 | **一半错。** 「不按级别分叉」⟹ **总缝规则真的判不了**（其判别式的前件是「两级」，本组没有级别轴）——这是**真前件**。「不接在同一 if-else」⟹ **收敛通则并没有因此判不了**：`AGENTS.md:58` 原文是「不是（同一 if-else）⟹ **按上面三条限定词逐条对**」，只是**快捷判别式**失效，通则本体照走。这条是**转述**，不是真前件（§3） |
| **E** | N-2 已明记「臂 C『更严』那部分在生产没跑到」 | **核实成立，且比 N-2 写的更彻底**：不只 `sub_b` + 级别严格递减没跑到，**`chi_bool` 的五条区分性条件（`lvl == ev` / `chosen().is_some()` / `lvl` 严格递减 / `sub_b` / 链反向守卫）没有一条在生产上被求值**。保证它的是 `interp.rs:580/:581/:582/:586` 四行（§4.1）。N-2 引的行号逐字复核**全部成立**（`:580`／`:581`／`:582`／`:586`） |

### 0.2 一句话结论（供裁定参考，不是裁定）

**两条判据在类型层确有严格包含关系，但在可达输入域上从未被应用于同一个输入，且「严」的那一档四条条件全部死码 ⟹ 按 `.chanlun/definitions/zhongshu.md:69` 的可达性条款，这是伪分歧。** 另有一条更强的独立论据：两条**接在 AND 上串联**（`chi_bool` 在 Γ 装配层置 `nest_confirmed` 位，`n_delta` 在 π 准入层判链），**没有任何 if-else 让宽档接管严档的失败** ⟹ 收敛通则的核心禁止条（兜底）本来就不成立（§3.2）。

### 0.3 三个数字

- `chi_bool` 生产链长恒 **1**（`interp.rs:580` 单元素数组字面量），全仓 `NestLevel` **构造点 7 处**，6 处在 `strategy/nest.rs` 自己的 `#[cfg(test)]` 内（`:220`／`:228`／`:236`／`:259`／`:278`／`:298`），生产唯一 1 处 = `interp.rs:580`。（**订正 N-2 的「10 处，9 处在 cfg(test)」**：那个计数把 `strategy/nest.rs:90` 的 `pub struct NestLevel {` 与 `:97` 的 `impl NestLevel {` 也数进了 `grep "NestLevel {"`。结论方向不变。）
- `n_delta` 生产 rungs 长度：门开时 **95.36% 为 0**（`econ_positive.rs:363-365` 自陈 BTC 300K 实测），真跨级 4.64%，max 深度 1。
- `NestRung` 结构体字段数 **4**（`confirm_src`／`child_interval`／`interval`／`cand`），**没有 `lvl` 字段**（`classifier/nest.rs:158-169`）⟹ `n_delta` 结构上**不可能**检查级别。这是全组严格度差的根。

---

## 1. 两条判据逐字核实 + 条件对照表

### 1.1 `chi_bool` 完整函数体（`rust/src/theta_v0/strategy/nest.rs:116-146`，逐字）

```rust
/// N^δ 良基递归证书 χ^δ ∈ {0,1}（Bool 全定义，对照 Lean `chiBool`/`nestCertB`）。
///
/// 输入：执行级 `ev`、级别链 `chain`（ℓ₀ 在表头，级别严格递减）。按结果包 §6 递归：
/// - **空链** ⟹ `false`（无级别 = 无候选 = 0）。
/// - **末级 ℓ** 且 `ℓ.lvl == ev` ⟹ 终端 `Confirm^δ` = `ℓ.confirm_ok`（且本级有候选）。
/// - **链头 ℓ + 次级 + 余链**（`ℓ.lvl > ev`）⟹ `Candidate^δ ∧ [次级 ⊆ 本级] ∧ 次级严格低一级 ∧ 递归`。
/// - **级别不匹配 / 链反向** ⟹ `false`（链不良构 = 无定位 = 0）。
///
/// **全定义**（对任意输入有确定 Bool 值，**绝不未定义**）：无候选 / 链不良构 / 区间套不成立
/// ⟹ `false` = 0。对照结果包「χ^δ ∈ {0,1}」「若不存在候选，则值为 0，绝不能是"未定义"」。
pub fn chi_bool(ev: u32, chain: &[NestLevel]) -> bool {
    match chain {
        [] => false,
        [last] => last.lvl == ev && last.chosen().is_some() && last.confirm_ok,
        [head, next, rest @ ..] => {
            if head.lvl == ev {
                // 已到执行级却还有下级链 ⟹ 链不良构（执行级应是末级）⟹ 0
                false
            } else if head.lvl > ev {
                // 操作级/中间级：候选成立 ∧ 次级严格低一级 ∧ 次级套本级 ∧ 递归
                let mut tail = Vec::with_capacity(rest.len() + 1);
                tail.push(next.clone());
                tail.extend_from_slice(rest);
                head.candidate_ok && next.lvl < head.lvl && sub_b(next, head) && chi_bool(ev, &tail)
            } else {
                // head.lvl < ev：链反向（违反 o_v > … > e_v）⟹ 0
                false
            }
        }
    }
}
```

配套两个被调谓词（同文件）：

```rust
// :109-114
fn sub_b(sub: &NestLevel, par: &NestLevel) -> bool {
    match (sub.chosen(), par.chosen()) {
        (Some(js), Some(jp)) => jp.start_time <= js.start_time && js.end_time <= jp.end_time,
        _ => false,
    }
}
// :99-101
pub fn chosen(&self) -> Option<Interval> { select_theta(&self.cands) }
```

**溯源标签**：文件头 `:3-4` 写「契约锚：`formal/Origin/IntervalNestCertificate.lean`（L2-B 补全工位，cov-strategy G3 / cov-classification F2 缺口）+ gpt 结果包 FULL_USER_FORMULA_SOURCE.md §6（line 312-369）」。`:19-22` 自标认识论 **L0**。**本文件 `reference:` 标签数 = 0**（用「契约锚」措辞，不用 `reference:`）。

### 1.2 `n_delta` 完整函数体（`rust/src/theta_v0/classifier/nest.rs:306-361`，逐字，含全部注释）

```rust
    /// ★诚实边界（cert F-01）：`n_delta` 只能复验证书**自身字段**（逐级 `cand` 取值 ∧ 相邻 ⊆ ∧
    /// 基例 `Conf^δ_e`）——装配层前置（方向逐级来源、级别连续性、rung 与
    /// `CandDeltaEvent` 的身份对应）证书无字段可复验，`n_delta=true` **不**蕴含它们成立；它们由
    /// [`assemble_certificate`] 三门 DFS 在生产时保证（数据载体旁路见 [`Self::from_parts`]）。
    ///
    /// 区间套证书 `N^δ_{ℓ↓e}(x) ∈ {0,1}`：按 ℓ 从高到低逐级校验。
    ///
    /// ## 结果包（六要素）
    /// - **结论**：返回 0/1（bool）值的级别递归谓词。基例 ℓ=e（`rungs` 空）取方向化确认
    ///   `Conf^δ_e(x)`=[`BspBits::confirm_side`]；递归步合取 `Cand^δ_ℓ(x)` 与本级闭包含边。
    ///   严格装配边取 `I(A_child)⊆D_parent`；旧数据载体边保持 `J_child⊆J_parent`。
    /// - **定义依据**：spec `2026-06-28-recursive-complete-classification-bsp-pdf-extract.md`
    ///   P5 §6 分段方框（line 277-285）+ ∃! 方框（line 295）+ 结果包（line 308-312）。`side` 选 δ；
    ///   `terminal` 满足基例 `Conf^δ_e=⋁ B_{i,e}`/`⋁ S_{i,e}`；`rungs[k].cand` 提供 `Cand^δ`；
    ///   相邻 `interval` 满足 ⊆。
    /// - **边界条件**：(1) ⊆ 方向是**子⊆父** `J^δ_{ℓ-1}⊆J^δ_ℓ`（[`is_sub`]：child.lo≥parent.lo
    ///   ∧ child.hi≤parent.hi，闭口径；lo=`start_time`/hi=`end_time`），方向反则证书失效。
    ///   (2) 任一级 `Cand^δ_ℓ=false`、或任一相邻 ⊆ 不成立、或基例 `Conf^δ_e=false` ⟹ 整体翻转为 0。
    ///   (3) 方向 δ 翻转（Long↔Short）则基例改判 `conf_plus`↔`conf_minus`，结论可翻转。
    ///   (4) 前置 e≤ℓ 由结构保证（rungs 非负长度），e>ℓ 不可表达（spec：e>ℓ 递归未定义）。
    /// - **下游推论**：N^δ 是 Γ 候选集→确认信号提升（环2→环3）的方向化确认层；返回 bool ⟹ ∃!
    ///   ∈{0,1}（2 值确定函数，无需 Finset）；级别每步严格下降（rungs 缩短）⟹ 有限终止。
    ///   Lean 端对应 (ℓ-e):Nat 结构递归。
    /// - **谱系引用**：第三类边界谱系（MEMORY: theta-v0-type3-boundary）——基例只消费
    ///   [`BspBits::confirm_side`]（委托 `conf_plus`/`conf_minus` → buy3/sell3 bit），B3 边界已在
    ///   `bsp::endpoint_to_bsp`（rust `>=`）结算，本层**不重判边界**。`Cand^δ_ℓ` 定义式缺失见
    ///   spec 疑点2（[需人工确认]）。
    /// - **影响声明**：在 nest.rs 新增方向化区间套证书；不改 `Chi`/`confirm`/`is_sub`（契约锚保留），
    ///   不动 classifier/mod.rs。L0 操作语义结构（非 L2 alpha）。
    pub fn n_delta(&self) -> bool {
        Self::n_delta_rec(self.side, &self.terminal, &self.base_interval, &self.rungs)
    }

    /// `N^δ` 的级别递归核（mirror Lean (ℓ-e):Nat 结构递归）。`rungs` 从高(ℓ)到低(e+1)。
    fn n_delta_rec(
        side: Side,
        terminal: &BspBits,
        base: &NestInterval,
        rungs: &[NestRung],
    ) -> bool {
        match rungs.split_first() {
            // 基例 ℓ=e：N^δ_{e↓e} = Conf^δ_e。
            None => terminal.confirm_side(side),
            // 递归步 ℓ>e：Cand^δ_ℓ ∧ [J^δ_{ℓ-1} ⊆ J^δ_ℓ] ∧ N^δ_{ℓ-1↓e}。
            Some((top, rest)) => {
                // 子级 J^δ_{ℓ-1}：下一梯级区间；rest 空 ⟹ 子级是执行级 J^δ_e。
                let child = top
                    .child_interval
                    .as_ref()
                    .unwrap_or_else(|| rest.first().map_or(base, |r| &r.interval));
                top.cand
                    && is_sub(child, &top.interval)
                    && Self::n_delta_rec(side, terminal, base, rest)
            }
        }
    }
```

配套谓词：

```rust
// classifier/nest.rs:71-73
pub fn is_sub(inner: &NestInterval, outer: &NestInterval) -> bool {
    inner.start_time >= outer.start_time && inner.end_time <= outer.end_time
}
// types.rs:274-279
pub fn confirm_side(&self, side: Side) -> bool {
    match side { Side::Long => self.conf_plus(), Side::Short => self.conf_minus() }
}
// types.rs:260-262 / :264 起
pub fn conf_plus(&self) -> bool { self.buy1 || self.buy2 || self.buy3 }   // Conf^- 镜像
```

**溯源标签**：文件头 `:1-14` 写「契约重锚（legacy Strict/Nest → `Origin.SubLevelDescent`）」并逐条列六个 ↔（含 `Sub`、`selKey`、`Confirm`、`N^δ_{ℓ↓e}`）；`:328` 的正文自陈「Lean 端对应 (ℓ-e):Nat 结构递归」。本文件 `reference:` 标签 **11 处**（`:53`、`:59`、`:68`、`:75`、`:98`、`:107` 等，全部挂在**底层谓词**上，`n_delta` 本身**没有** `reference:` 标签，只有正文一句「Lean 端对应」）。`:12`、`:155-157`、`:167` 三处 `[需人工确认]`（`Cand^δ_ℓ` 无定义式，spec 疑点 2 ⟹ 归 **N-5**）。`:248`、`:160` 两处 `[新缠论]`（`base_confirm_src`／`confirm_src` 独立确认时点，#37 P0）。

### 1.3 ★ 判定条件逐条对照表（「严格度不同」落到具体条件）

**先说清口径**：两条的输入类型不同（`&[NestLevel]` vs `NestCertificate`），要对照必须先给一个映射。**规范遗忘映射 Φ**（链 → 证书）：丢弃每级 `lvl`、把每级 `cands` 用 `Sel_Θ` 折成 `chosen()`、末级折成 `terminal`+`base_interval`、其余各级折成 `rungs`（高→低）、`candidate_ok ↦ cand`、`confirm_ok ↦ confirm_side(side)`。**Φ 不是单射**（`lvl` 被丢掉）——这一点是后面 §2 全部结论的技术支点。

| # | 条件 | `chi_bool`（`strategy/nest.rs`） | `n_delta`（`classifier/nest.rs`） | 是否同名同式 |
|---|---|---|---|---|
| 1 | **空输入** | `:128` `[] => false` | **无对应物**——`NestCertificate` 结构上必带 `terminal` + `base_interval`，「无级别」不可表达（`:254` 自陈「空 ⟹ ℓ=e（纯基例）」） | **只有 chi_bool 有**（对 n_delta 是类型层排除） |
| 2 | **终端确认** | `:129` `last.confirm_ok`（调用方给的裸 bool，**方向无关**） | `:348` `terminal.confirm_side(side)`（按 δ 选 `conf_plus`／`conf_minus`） | **两边都有，底层同源**：`interp.rs:575-578` 正是用 `bits.conf_plus()`／`conf_minus()` 填 `confirm_ok`，与 `confirm_side` 委托的**是同一个析取** `B1∨B2∨B3`。差别是**方向的落点**：n_delta 内置 δ，chi_bool 把 δ 推给调用方。**不是严格度差，是 locus 差** |
| 3 | **终端有候选** | `:129` `last.chosen().is_some()`（`cands` 空 ⟹ false） | **无对应物**——`base_interval` 是单个 `NestInterval`，恒存在，不可为空 | **只有 chi_bool 有** ⟹ 严格度差 ① |
| 4 | **终端级别 == 执行级** | `:129` `last.lvl == ev` | **无对应物**——`NestRung` 无 `lvl` 字段（`:158-169`）；`n_delta` 签名里根本没有 `e` 或 `ℓ` | **只有 chi_bool 有** ⟹ 严格度差 ② |
| 5 | **本级候选谓词** | `:139` `head.candidate_ok` | `:356` `top.cand` | **两边都有，逐字同义**（同一个 `Cand^δ_ℓ` 0/1 取值，两边都不给定义式 ⟹ N-5） |
| 6 | **区间套 ⊆** | `:139` `sub_b(next, head)` ⟹ `:111` `jp.start_time <= js.start_time && js.end_time <= jp.end_time` | `:357` `is_sub(child, &top.interval)` ⟹ `:72` `inner.start_time >= outer.start_time && inner.end_time <= outer.end_time` | **两边都有，不等式逐位等价**（`a<=b` ⟺ `b>=a`，同一闭口径）。**两处差别**：(a) `sub_b` 先要两侧 `chosen()` 都 `Some`（`:112` `_ => false`），`is_sub` 直接吃两个必存在的区间；(b) `is_sub` 的 child 有 `child_interval` 覆盖分支（`:352-355`，严格装配边 `I(A_child)⊆D_parent`），`sub_b` 无此概念 |
| 7 | **次级严格低一级** | `:139` `next.lvl < head.lvl` | **无对应物**（同 #4，无 `lvl` 字段） | **只有 chi_bool 有** ⟹ 严格度差 ③ |
| 8 | **链头已到执行级却还有下级** | `:131-133` `if head.lvl == ev { false }` | **无对应物** | **只有 chi_bool 有** ⟹ 严格度差 ④ |
| 9 | **链反向（head.lvl < ev）** | `:140-143` `else { false }` | **无对应物** | **只有 chi_bool 有** ⟹ 严格度差 ⑤ |
| 10 | **级别连续性**（相邻级差恰 1） | **两边都没有**。`chi_bool` 只要 `<`（`:139`），不要 `= head.lvl - 1`；`n_delta` 完全不看级别 | — | **两边都缺**（Lean 侧有，见 §6）⟹ 这是**两条共同的缺口**，不是分歧 |
| 11 | **退化区间守卫** | 无（`sub_b` 用裸不等式） | 无（`is_sub` 用裸不等式，`:71` 无守卫） | **两边都缺**。第三份实现 `cand_event/key.rs:192 interval_is_sub` **有**守卫（`!interval_is_degenerate(child) && !interval_is_degenerate(parent) && ...`）⟹ 这是 N-1 移交本组的那条（`qujiantao.md:216`），见 §2.4 |

**⟹ 「严格度不同」的确切落点：条件 #3／#4／#7／#8／#9 五条，全部只有 `chi_bool` 有，全部关于「级别号」与「候选存在性」。** 而 `n_delta` 没有任何一条 `chi_bool` 缺的条件（#2 是 locus 差、#6(b) 的 `child_interval` 覆盖是**更精确的边选取**而非**额外约束**——它替换而不追加合取项）。

---

## 2. 接受集关系（逐条件推，不照抄票面）

### 2.1 沿规范遗忘映射 Φ：**严格包含 `A(chi_bool) ⊊ A(n_delta)`**

设 `c ∈ &[NestLevel]`、`ev`，令 `cert = Φ(ev, c)`（把 `confirm_ok` 映到 `confirm_side(side)`，即取 §1.3 #2 的同源桥）。

**(⊆) 方向**：`chi_bool(ev, c) = true ⟹ n_delta(cert) = true`。逐条件核：
- 链长 1：`chi_bool` 给 `lvl==ev ∧ chosen().is_some() ∧ confirm_ok`；`cert` 的 rungs 空 ⟹ `n_delta = confirm_side(side) = confirm_ok = true`。✓
- 链长 k>1：`chi_bool` 的每一步给出 `candidate_ok ∧ (lvl 严格降) ∧ sub_b ∧ 递归`；`n_delta` 的对应步只要 `cand ∧ is_sub ∧ 递归`。`candidate_ok = cand` 同值（#5）；`sub_b(next, head) = true ⟹` 两侧 `chosen()` 都 `Some` 且不等式成立 ⟹ `is_sub(Φ(next).chosen, Φ(head).chosen) = true`（#6 不等式逐位等价）；递归同构下降。✓
⟹ `A(chi_bool) ⊆ A(n_delta)`。

**(⊊) 严格**：给三个手算反例（都落在差集里，`n_delta=true ∧ chi_bool=false`）：

**反例 W1（末级级别不等于执行级 —— 条件 #4）**
```
ev = 0
chain = [ A{lvl=5, cands=[I(end=25,start=5,idx=1)], candidate_ok=true, confirm_ok=false},
          B{lvl=3, cands=[I(end=20,start=10,idx=5)], candidate_ok=true, confirm_ok=true} ]
```
- `chi_bool`：head=A，`A.lvl=5 > ev=0` ⟹ `candidate_ok(T) ∧ 3<5(T) ∧ sub_b(B,A)`：`5<=10 ∧ 20<=25` = T ⟹ 递归 `chi_bool(0,[B])` = `[last]` 分支 = `3 == 0` ⟹ **false**。整体 **false**。
- `Φ(chain)` = `NestCertificate{ side=Long, terminal={buy1:true,..}, base_interval=I(20,10,5), rungs=[NestRung{interval:I(25,5,1), cand:true, child_interval:None}] }`
- `n_delta`：rungs 非空 ⟹ `child = rest.first().map_or(base, ..)` = `base` = I(20,10,5)；`top.cand(T) ∧ is_sub(I(20,10,5), I(25,5,1))`：`10>=5 ∧ 20<=25` = T ∧ `n_delta_rec(rungs=[])` = `confirm_side(Long)` = `buy1∨buy2∨buy3` = **true** ⟹ 整体 **true**。
- **关键**：把 `B.lvl` 从 3 改成 0，`chi_bool` 翻成 true，而 **`Φ` 的像一个字节都不变**（`NestRung` 无 `lvl`）⟹ `n_delta` 两种输入同为 true。**这就是不可注入性的具体现场。**

**反例 W2（级别未严格递减 —— 条件 #7）**
```
ev = 0
chain = [ A{lvl=2, cands=[I(25,5,1)], cand_ok=T, conf_ok=F},
          B{lvl=2, cands=[I(22,8,3)], cand_ok=T, conf_ok=F},
          C{lvl=0, cands=[I(20,10,5)], cand_ok=T, conf_ok=T} ]
```
- `chi_bool`：head=A，`next.lvl=2 < head.lvl=2` = **false** ⟹ **false**（这就是 `strategy/nest.rs:296-305` 那条现成单测 `chi_bool_non_decreasing_levels_zero` 锁的行为）。
- `n_delta`：rungs=[I(25,5,1),I(22,8,3)]，`is_sub(I(22,8,3), I(25,5,1))`：`8>=5 ∧ 22<=25` T；`is_sub(I(20,10,5), I(22,8,3))`：`10>=8 ∧ 20<=22` T；基例 T ⟹ **true**。

**反例 W3（末级无候选 —— 条件 #3）**
```
ev = 0；chain = [ D{lvl=0, cands=[], cand_ok=T, conf_ok=T} ]
```
- `chi_bool`：`0==0 ∧ [].chosen().is_some()` = `select_theta(&[]) = None` ⟹ **false**（现成单测 `chi_bool_singleton_no_cand_zero`，`:277-285`）。
- `n_delta`：`base_interval` 无法为「空」；`Φ` 只能给它填一个区间（或整个证书构造不出来）⟹ **该输入在 `n_delta` 侧不可表达**。这一条严格说是**类型层排除**，不是接受集差 —— 照实登记。

### 2.2 若不给 Φ 那个桥：**互不包含**

`NestLevel.confirm_ok` 是**自由 bool**，与 `BspBits` 无类型层联系。取 `chain = [{lvl=0, cands=[I(1,1,0)], cand_ok=T, confirm_ok=true}]`、`ev=0`，而对应证书 `terminal = BspBits::default()`（全零 ⟹ `conf_plus=false`）：`chi_bool = true`，`n_delta = false`。⟹ **反向差集非空。**

**⟹ 接受集关系的诚实表述（两句，缺一不可）**：
1. **沿规范遗忘映射（`confirm_ok ≡ confirm_side(side)`，即生产上真实成立的那个桥，`interp.rs:575-578` 逐字保证）：严格包含 `A(chi_bool) ⊊ A(n_delta)`，`chi_bool` 严。**
2. **把 `confirm_ok` 当自由参数（类型允许）：互不包含。**

票面的「宽/严」在第 1 种口径下**成立**。这是本批第一次票面前提在方向上没错的组（#815 是反的）—— 但**理由与票面写的不同**：严格度差不来自「用途不同」，来自 **`NestRung` 没有 `lvl` 字段**这一个结构事实。

### 2.3 ★ 差集上界：**Φ 的像里，两条唯一可能分歧的维度就是级别号 + 候选存在性**

条件 #5／#6 逐位等价、#2 同源、#1／#3 是类型层、#4／#7／#8／#9 全是级别号谓词。⟹ **在 `Φ` 下，`A(n_delta) \ A(chi_bool)` = {级别号不良构 ∨ 末级无候选} 的那些输入。** 这个刻画是完全的（无遗漏条件）。

### 2.4 附：⊆ 判据的第三份实现（N-1 移交项，`qujiantao.md:216`）

同一个「区间包含」判定，全仓三份：

| # | 载体 | 表达式 | 退化守卫 | 生产可达 |
|---|---|---|---|---|
| 1 | `classifier/nest.rs:71-73` `is_sub` | `inner.start>=outer.start && inner.end<=outer.end` | **无** | 是（门开时） |
| 2 | `strategy/nest.rs:109-114` `sub_b` | 同式 + 两侧 `chosen()` 须 `Some` | **无** | **否**（死在 `[head,next,..]` 分支里，§4.1） |
| 3 | `cand_event/key.rs:191-197` `interval_is_sub` | `!degenerate(child) && !degenerate(parent) && parent.0<=child.0 && child.1<=parent.1` | **有**（`:186-188` `interval.0 > interval.1`） | **否**——消费者只有 `bin/p129_44ke_strict_skip.rs:292`（bin，非 π）与 `classifier/cand_sub.rs:39/:233`（N-1 已判生产不可达） |

⟹ **退化守卫这一档严格度差同样落在生产不可达的那一侧。** 本组照实登记，不替 N-4 作答（`qujiantao.md:216` 把它归 N-4／N-6 两处）。

---

## 3. 两条通则为什么判不了 —— 逐字引用 + 真前件 / 转述之分

### 3.1 正本表述（`AGENTS.md`，唯一正本入口，#517 裁定）

**收敛通则**（`AGENTS.md:46-58`，[#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 裁定 2026-07-30）：

> **同一个判断不得存在宽严两档实现，更不得让宽松的那档接管严格那档的失败。**
>
> 「严格版判不出来 ⟹ 换宽松版再判一次 ⟹ 放行」不是合法的通道分派——它把**判据失败**当成了**换判据重判**的触发器。收敛一个概念时，凡发现该概念底下有两套判据且其中一套在给另一套兜底的，**先判违规、再谈收敛到哪一套**，两件事分开做。
>
> **限定词（防误伤正当的分档）**：本条管的是**同一个判断的两个宽严档位**，不管以下三类正当分化——
> 1. **不同判断**：判的对象本就不同（中枢构造 vs 背驰确认），各自独立成立，互不兜底；
> 2. **诊断模式 vs 生产模式**：诊断口径可更宽（多打日志、多记候选），但**不得参与准入**；
> 3. **已声明的独立对照臂**：见 ADR-0005 —— 对照臂不产生交易决策，不在「生产判定路径」管辖内（#799 裁定六）。
>
> **判别一句话**：看两套判据**是不是接在同一个 if-else 上**。是 ⟹ 系统自己声明了二者可互换 ⟹ 违规；不是 ⟹ 按上面三条限定词逐条对。

**总缝规则**（`AGENTS.md:64-84`，[#804](https://github.com/xy7365527-lang/NewChanlun/issues/804) 裁定 2026-07-30）：

> **同一个判定，每一级必须跑同一个判定；级别的差异只准以传进去的值出现，不准是另一套判据。**
>
> #### ① 规则落在语义，不落在代码形状
> **判别式**：把两级的输入换成同一份数据喂进去，**两边输出必须逐位相同**——不同就是两个判断，即违规。
> （…）**grep 降级为分诊线索，不是判据**：`level ==` 命中不等于违规，零命中也不等于合规，它只提示「这里值得看一眼」。

**可达性条款**（`.chanlun/definitions/zhongshu.md:69`，已入中枢正本）：

> **总缝规则与收敛通则的判别式，在「可达输入域」上评估。落在生产路径产生不出来的输入上的差异，判为伪分歧；主张伪分歧的一方必须举出可达性论证（点名到保证它的那行代码），举不出就按真分歧办。**

### 3.2 票面理由一：「不接在同一 if-else」—— **是转述，不是真前件**

**事实核实**：两条确实不接在任何 if-else 上。逐跳核过（§4）：`chi_bool` 的返回值经 `Candidate.nest_confirmed` 字段落到 `interp.rs:1541`（`if !c.nest_confirmed { record; continue }`）与 `:193`／`:245`；`n_delta` 的返回值落到 `admission.rs:300`／`:1017` 再到 `fill.rs:4941` 的 `.filter(|c| ... admit)`。两处在 π 上是**串联的两道 AND**，不存在「chi_bool 判不出来 ⟹ 交给 n_delta 再判一次 ⟹ 放行」的结构。

**但票面把这个事实当成「通则判不了」是错的**：`AGENTS.md:58` 原文明写「不是 ⟹ **按上面三条限定词逐条对**」。⟹ if-else 只是**快捷判别式**，不成立时通则本体照走三条限定词。逐条对：

| 限定词 | 本组是否命中 | 依据 |
|---|---|---|
| ① **不同判断** | **部分命中，且这是本组最强的一支辩护** | 两者判的对象**不同**：`chi_bool` 在生产上判的是「执行级终端有没有买卖点确认」（一个 bit，`conf_plus`／`conf_minus`）；`n_delta` 在门开时判的是「从塔顶到执行级的跨级 rung 链是否逐级闭合」。**两者是合取的两个因子，不是同一判断的两个档位。**⚠️ **反方登记**：两处 docstring 都自称在实装同一个符号 `N^δ`（`strategy/nest.rs:15`「[`chi_bool`]：N^δ 良基递归证书 χ^δ」；`classifier/nest.rs:215`「方向化区间套证书 `N^δ_{ℓ↓e}`」）⟹ **系统自己声明二者是同一判断**。这一条不清不楚，须裁 |
| ② **诊断 vs 生产** | **不命中**（对本对而言）。但对 `n_delta` 的**第三个**消费点命中：`backtest/opsem_dump.rs:224` 的 `assemble_certificates_terminal` sidecar 由 `OPSEM_DUMP_DIR` 门控、`:15` 自陈「不参与决策」⟹ 正当分化 | `opsem_dump.rs:419` 「未设 ⟹ 全部方法 no-op」 |
| ③ **独立对照臂** | **部分命中**：`admission.rs:1139` 的 L2 旧臂（`nest_gate_admit` → `build_gate_certificate`）**恒双读落账供 cross 对照**，#75 起「typed 真链为唯一 nest 判定源」（`admission.rs:255-258`）⟹ 旧臂已是声明过的对照臂 | `admission.rs:1089-1092` |

⟹ **收敛通则不是判不了，是「限定词①要人裁」。** 这是本报告对票面最实质的一处订正。

### 3.3 票面理由二：「不按级别分叉」—— **是真前件**

总缝规则的判别式（`AGENTS.md:70`）原文要求「把**两级**的输入换成同一份数据喂进去」。本组的两条判据**不是按级别分叉的**：分叉轴是**层**（`strategy/`环 2 vs `backtest/`π 准入）与**用途**，不是 `level`。两条都不含任何 `if level == …`（实测 `chi_bool` 完全不 match level 常量；`n_delta` 连 level 都读不到）。

⟹ **判别式的前件（存在「两级」可对拍）不成立 ⟹ 总缝规则对本组无从适用。这条理由是真前件，票面说得对。**

**附带核实**：总缝规则 §① 三条理由里第 2 条（`AGENTS.md:72`）明写「实测四个病灶只有一个是形状病」并把 grep 降级为线索（`AGENTS.md:78`）⟹ 本报告没有用 grep 结果当判据，全部依据是打开每一行读到的语义。

### 3.4 两票在案来源核对

- [#804](https://github.com/xy7365527-lang/NewChanlun/issues/804)（总缝规则）在 `docs/adr/0011-operation-decomposition-layer.md:68` 有一处应用先例：「判定没有按级别分叉，是按角色分叉，而角色是数据（这个中枢被谁消费），不是分支」⟹ **按「谁消费」分叉不算违规**，这条先例**直接可援引到本组**（本组正是按「谁消费」分：环 2 消费 vs π 门消费）。
- [#809](https://github.com/xy7365527-lang/NewChanlun/issues/809) 的分诊表（`issue809-doctrine-triage-20260730.md:54-59`）列了 V1–V6 六条违规，**本组的两条 `N^δ` 不在其中**——V1 是 `div_cand`／`sublevel_diverges`（已由 #814 D-2 裁完，正本落 `beichi.md:303`），V2 是 `Cand^δ` 的 per-rung 恒真档（归 N-5）。⟹ **「两条 N^δ 严格度不同」这个题面不是 #809 分诊出来的违规，是本票 #817 自己新提的。** 溯源上它最早出现在 `qujiantao.md:600` 的 N-6 题面。

---

## 4. 可达性判定（本组最承重的一件）

### 4.1 `chi_bool` 的生产链：**可达、承重，但四条区分性条件全部恒真或死码**

**逐跳点名到行（全部打开确认）**：

```
入口一：rust/src/theta_v0/strategy/interp.rs:459
        let nest_ok = nest_confirm(lvl, point.source_index, &point.bits, dir);   // assemble_gamma
入口二：rust/src/theta_v0/strategy/interp.rs:1256
        nest_confirmed: nest_confirm(lvl, point.source_index, &point.bits, dir), // ..._with_tower_cached_gen
   ↓   （fill.rs:4852-4858 调 coverage_elements_and_gamma_with_tower_cached_gen ⟹ 入口二在 π 主循环上）
     rust/src/theta_v0/strategy/interp.rs:569-587  fn nest_confirm
        :575-578  let confirm_ok = match dir { Long => bits.conf_plus(), Short => bits.conf_minus(), Flat => return false };
        :580      let chain = [NestLevel {
        :581          lvl: level,
        :582          cands: vec![Interval::new(source_index as u64, source_index as u64, 0)],
        :583          candidate_ok: true,
        :584          confirm_ok,
        :585      }];
        :586      nest::chi_bool(level, &chain)
   ↓
     rust/src/theta_v0/strategy/nest.rs:126  pub fn chi_bool(ev: u32, chain: &[NestLevel]) -> bool
```

**返回值的生产消费点（三处，全部无 env 门）**：
- `interp.rs:1541` `if !c.nest_confirmed { buckets.record.push(*c); continue; }` —— **规则 2 证书门**，`:1539-1540` 注释自陈「未确认者不能落规则3（§13 ℬ_x 会真实开腿）」⟹ **它真的挡开腿**。
- `interp.rs:193` `if !cand.nest_confirmed || cand.dir == VoiceSide::Flat || cand.bsp_class == u8::MAX { return None; }`（`parent_certificate_projection`，母子 campaign trigger token）。
- `interp.rs:245` `child.nest_confirmed && …`（`trigger_projection_sound`）。

**⟹ `chi_bool` 无条件常开、真参与准入。** 但把上面的链字面量代进函数体，逐条件求值：

| `chi_bool` 条件 | 在生产输入上的取值 | 保证它的那一行 |
|---|---|---|
| `[] => false` | **不可达**（链恒长 1） | `interp.rs:580` 单元素数组字面量 |
| `last.lvl == ev` | **恒真** | `interp.rs:581` `lvl: level` 与 `:586` `chi_bool(level, …)` 是**同一个变量 `level`** |
| `last.chosen().is_some()` | **恒真** | `interp.rs:582` `cands: vec![Interval::new(...)]` —— 恒好一个元素，`select_theta` 必 `Some` |
| `last.confirm_ok` | **= `conf_plus`／`conf_minus`** | `interp.rs:575-578` |
| `[head, next, rest @ ..]` 整臂 | **死码** | 同 `:580` |
| ⟹ `head.lvl == ev` / `head.lvl > ev` / `candidate_ok` / `next.lvl < head.lvl` / `sub_b` / 递归 | **全部从未被求值** | 同 `:580` |

**⟹ `chi_bool` 在生产上恒等于：`dir == Flat ? false : (Long ? conf_plus : conf_minus)`。** 一个 bit。它的「严」——五条区分性条件（§1.3 #3/#4/#7/#8/#9）——**没有一条被求值过**。

**核实 N-2 的移交项（票面 E）**：N-2 写的是「臂 C『更严』的那部分（`sub_b` + 级别严格递减）在生产根本没跑到」。**成立，且范围应扩大**：不是两条，是**五条**；且 `[] => false` 这条也不可达。N-2 与 `qujiantao.md:574` 第 28 条引的四行 **`:580`（`let chain = [NestLevel {`）／`:581`（`lvl: level`）／`:582`（`cands: vec![Interval::new(...)]`）／`:586`（`nest::chi_bool(level, &chain)`）逐字复核全部成立**，无需订正。

**全仓 `NestLevel` 构造点复核**：`strategy/nest.rs` 的 `#[cfg(test)]` 内 **6 处**（`:220`／`:228`／`:236`／`:259`／`:278`／`:298`）+ `interp.rs:580` 生产唯一 1 处 = **共 7 处**。（N-2 记的「10 处，9 处在 cfg(test)」多算了 `strategy/nest.rs:90` 的结构体定义与 `:97` 的 `impl` 头——`grep "NestLevel {"` 会命中这两行。结论方向不变。）

### 4.2 `n_delta` 的生产链：**两条路径，一条不可达、一条 env 门控默认关**

**路径 A（票面说的「回测准入门」）**：
```
rust/src/theta_v0/backtest/econ_positive.rs:398
        Some(GateCertificate::Nest(cert)) => cert.n_delta(),
   ↑  所在函数 = fn collect_signals（econ_positive.rs:270）
   ↑  唯一调用者 = pub fn decompose_capturable_spread（econ_positive.rs:229）
   ↑  decompose_capturable_spread 全仓消费者（grep --include=*.rs 全树）：
        rust/tests/econ_oddeven_diagnosis.rs:23（use）／:55（调用）
        —— 该文件 :3  #![cfg(feature = "backtest_bin")]
                  :39 #[ignore = "L2 奇偶交替持有窗诊断；需 BTC 全量；--release --ignored"]
```
**⟹ 路径 A 生产不可达，且在 CI 里也不执行**（`.github/workflows/ci.yml:187` 跑 `cargo test --all-targets --features backtest_bin`，但 `#[ignore]` 默认不跑）。这确认并加强了 N-2 的读数。另一处同源门函数 `build_multilevel_nest_cert`（`econ_positive.rs:783-796`）的调用者只有 `econ_positive.rs` 自己的 `#[cfg(test)] mod tests`（`:2377`／`:2470`／`:2713` 等）。

**路径 B（真在生产 π 上）**：
```
rust/src/theta_v0/backtest/fill.rs:4473
        let nest_gate_hist: Option<Vec<f64>> = if nest_cert_gate_enabled() && !bars.is_empty() { … }
rust/src/theta_v0/backtest/fill.rs:4489
        let mut nest_chain_gate: Option<NestChainGate> = if nest_gate_hist.is_some() { … }
rust/src/theta_v0/backtest/fill.rs:4908
        let step_gamma_trade = match &nest_gate_hist { Some(hist) => { … }
rust/src/theta_v0/backtest/fill.rs:4941
        gate.admit(&tower_i, c, hist, i, &classification_i)
   ↓
rust/src/theta_v0/backtest/admission.rs:1017   pass |= cert.certificate().n_delta();   ← typed 真链（#75 起唯一判定源）
rust/src/theta_v0/backtest/admission.rs:1139   nest_gate_admit(tower, c, hist, confirm_index, classification);  ← L2 旧臂双读落账
rust/src/theta_v0/backtest/admission.rs:300    let pass = cert.n_delta();              ← 旧臂内
   ↑ typed 链的证书来源：
rust/src/theta_v0/backtest/admission.rs:853    classifier::nest_index::build_nest_certificate_index(…)
rust/src/theta_v0/classifier/nest_index.rs:262 pub fn build_nest_certificate_index(
rust/src/theta_v0/classifier/nest_index.rs:295-297
        let cert = (exec..=top_max).rev().find_map(|top| {
            assemble_typed_certificate(events_by_level, base, top, caliber, &terminal_of) });
```
**⟹ 路径 B 可达，且 rungs 可以非空（真跨级链）**——`assemble_typed_certificate` 从 `events_by_level` 逐级装配，不是单元素字面量。**但整条路径由 `nest_gate_hist` 是否 `Some` 决定，即 `nest_cert_gate_enabled()`（见 §5）。**

**⚠️ 对 N-1／N-2 在案结论的一处订正**：N-1 判「`cand_sub.rs` 生产不可达」、N-2 判「`classifier/nest.rs:346-348` 的唯一入口 `decompose_capturable_spread` 全部消费者在 `cfg(test)` 内」。**后者只覆盖路径 A。** `classifier/nest.rs:346-348`（`n_delta_rec` 的 `match` + 基例）在**路径 B** 上是可达的（门开时），入口是 `assemble_typed_certificate` ／`assemble_certificate`，不是 `decompose_capturable_spread`。⟹ **「`n_delta` 唯一入口是 `decompose_capturable_spread`」这句话不成立，须订正。** 这不影响 N-2 的教义结论（它的结论是关于「停在哪一级从未被求值」，那条仍成立于 `chi_bool` 侧），但影响 N-6 的可达性论证形状。

### 4.3 ★ 核心问题：可达差集有没有？

**答：没有可达的差集，但成因不是「两条都不可达」，而是「两条从未被应用于同一个输入」。** 分三层说清：

**层一：不存在共同输入。** `chi_bool` 吃的是 `interp.rs:580` 现搓的单元素链（其 `Interval` 用 `source_index` 同时当 `end_time` 和 `start_time`，`idx=0` —— 一个退化点区间，不是真定位区间）；`n_delta` 吃的是 `nest_index` 从 `events_by_level` + 塔装配出来的 `TypedNestCertificate`。两者的输入**由不同代码在不同层各自构造，从不互换**。⟹ §2 那个 `Φ` 是**报告为了做对照而引入的分析工具，生产里没有任何一行实现它**。

**层二：「严」的那五条条件全部死码。** §4.1 表已点名到行：保证它们死的是 `interp.rs:580` 一行（单元素数组字面量）。⟹ 即便硬要在同一输入上比，`chi_bool` 也**没有能力**表现出比 `n_delta` 更严。

**层三：两者在 π 上是串联 AND，不是二选一。** 门开（`THETA_NEST_CERT_GATE=1`，如 armR 复跑）时，同一个候选先在 Γ 装配层拿 `nest_confirmed`（`chi_bool` ≡ conf bit），再进 `gate.admit`（`n_delta` 真链）。**合取的两个因子，谁也不给谁兜底。** 门关（默认）时，只有 `chi_bool` 那一个 bit 在跑，`n_delta` 完全不参与决策。

**⟹ 按 `zhongshu.md:69` 可达性条款，本组符合「落在生产路径产生不出来的输入上的差异」⟹ 伪分歧的构成要件成立。** 保证它的每一行（履行「点名到保证它的那行代码」的举证义务）：

| # | 保证什么 | 文件:行 | 逐字 |
|---|---|---|---|
| G1 | `chi_bool` 链恒长 1 ⟹ 五条严条件全死 | `rust/src/theta_v0/strategy/interp.rs:580` | `let chain = [NestLevel {` |
| G2 | `last.lvl == ev` 恒真 | `.../interp.rs:581` + `:586` | `lvl: level,` ／ `nest::chi_bool(level, &chain)` |
| G3 | `chosen().is_some()` 恒真 | `.../interp.rs:582` | `cands: vec![Interval::new(source_index as u64, source_index as u64, 0)],` |
| G4 | `candidate_ok` 从不被读（死在 `[head,next,..]` 臂） | `.../interp.rs:583` + `strategy/nest.rs:130` | `candidate_ok: true,` ／ `[head, next, rest @ ..] => {` |
| G5 | `n_delta` 路径 A 不可达 | `rust/tests/econ_oddeven_diagnosis.rs:3` + `:39` | `#![cfg(feature = "backtest_bin")]` ／ `#[ignore = "L2 奇偶交替持有窗诊断；需 BTC 全量；--release --ignored"]` |
| G6 | `n_delta` 路径 B 默认不评估 | `rust/src/theta_v0/backtest/admission.rs:69-71` | `std::env::var(crate::theta_v0::env_registry::THETA_NEST_CERT_GATE).ok().as_deref() == Some("1")` |
| G7 | 门关 ⟹ 整条 π nest 门跳过、逐字节不变 | `rust/src/theta_v0/backtest/fill.rs:4473` + `:4489` + `:4908` | `if nest_cert_gate_enabled() && !bars.is_empty()` ／ `if nest_gate_hist.is_some()` ／ `match &nest_gate_hist {` |
| G8 | `NestRung` 无 `lvl` 字段 ⟹ `n_delta` 结构上不可能判级别 | `rust/src/theta_v0/classifier/nest.rs:158-169` | `pub struct NestRung { confirm_src, child_interval, interval, cand }` |

**⚠️ 反方登记（不裁）**：门开时 `n_delta` **是真在跑真链**的（armR 复跑即是，`scripts/check_armR_trades_digest.py:17`）。所以本组的「伪分歧」**不能表述成「两条都是死码」**——`chi_bool` 是活的（只是塌成一个 bit），`n_delta` 在 armR 口径下也是活的。**准确表述是：两条严格度不可比，因为它们没有共同的可达输入域，且严的那一档的严格性在生产上不可观测。**

---

## 5. 「回测准入门」与「环 2」各是什么、现在活不活

### 5.1 环 2（`chi_bool`）

**是什么**：`strategy/interp.rs:37` 逐字「**区间套证书 [`nest::chi_bool`]**（环2，spec §6 N^δ）：[`assemble_gamma`] 的 `nest_confirmed`」；`:68` 「`nest_confirmed`：区间套确认 N^δ（环2，证书基例 Conf^δ，[`nest::chi_bool`]）」。⟹ 环 2 = Γ 候选集装配层给每个候选贴「证书成立位」。

**活不活**：**无条件常开。** 无 env、无 config、无 feature gate。逐跳已在 §4.1 给出。CI 也跑（`ci.yml:184` `cargo test --all-targets`，`interp.rs:2080`／`:2111` 两条断言在内）。

### 5.2 回测准入门（`n_delta`）

**是什么**：`econ_positive.rs:371` 逐字「二通道准入门（小转大 landing）：区间套 Nest→n_delta；小转大 Xzd→gate_pass」；`admission.rs:53` 「`THETA_NEST_CERT_GATE=1` ⟹ ①π 开仓准入门」。

**活不活**：**默认关。** 门函数逐字（`admission.rs:62-72`）：

```rust
pub(super) fn nest_cert_gate_enabled() -> bool {
    #[cfg(test)]
    {
        if let Some(v) = NEST_CERT_GATE_OVERRIDE.with(|c| c.get()) { return v; }
    }
    std::env::var(crate::theta_v0::env_registry::THETA_NEST_CERT_GATE)
        .ok()
        .as_deref()
        == Some("1")
}
```

**⚠️ 已在案陷阱的对照核实（票面点名要防的那个）**：`recursive_t/rec_engine.rs:305-312` 用 `std::env::var(..).is_ok()`（**设了就开，哪怕空串**）。**theta_v0 侧不是这个形状**——上面是 `.as_deref() == Some("1")`，**空串／`"0"`／`"true"` 全部判关**。⟹ **本轮没有踩同款陷阱**，「默认关」在代码内是真的。同时核实：**查的不是某个 `off()` 辅助函数**，查的是 `fill.rs:4473` 那条唯一的默认构造路径（`nest_gate_hist` 的初始化），它直接调 `nest_cert_gate_enabled()`。

**注册表交叉核实**：`rust/src/theta_v0/env_registry.rs:211-217` 逐字 —— `key: THETA_NEST_CERT_GATE` / `semantic: "π 开仓准入区间套证书门——T3 起严格链判定为唯一 nest 判定源"` / `kind: GateKind::Behavior` / `default_arm: "未设/非\"1\" ⟹ π 门整体跳过（全路径 bit-exact 不变）"` / `layer: "backtest"`。**与实现一致。**

### 5.3 `.github/workflows/` 逐条通读（N-2 留下的未测项）

三个 workflow 文件：`ci.yml`（219 行）、`codeql.yml`、`devskim.yml`。

| 查项 | 结果 |
|---|---|
| 有没有设 `THETA_NEST_CERT_GATE` 或任何 `THETA_*` | **零。**（`grep -rn "THETA_" .github/` 全树零命中） |
| 有没有 `env:` 块设 nest 相关变量 | **无。**`ci.yml` 全文 `nest` 只出现 1 次：`:166` 注释「`rust/tests/nest_isolation_guard.rs` 的『违规即测试红』此前只编译不运行」 |
| 跑不跑 `cargo test` | **跑。**`:183-184` `cargo test --all-targets`；`:186-187` `cargo test --all-targets --features backtest_bin`（#810 起）⟹ 总缝规则要求的「机械锁不能只编译不执行」这一前置**已经闭合**（`AGENTS.md:80` 记的「已定为下游实施链第一条动作」已完成，那处注记可更新——不在本报告改动范围） |
| `#[ignore]` 的测试跑不跑 | **不跑**（无 `-- --ignored`）⟹ `econ_oddeven_diagnosis.rs` 全部 `#[ignore]`，路径 A 在 CI 里也不执行 |
| 其他 job | `:37` 一条 grep gate（`P101_TAG\|P101_MULTI\|cert_bsp_tag` 在 `rust/src/theta_v0/` 零命中）、`:101` pytest `-m "not slow"`、`:154-158` 两步 `cargo check`、`:161-163` `cargo fmt --check`、`:214` `lake build`、`:219` `check_fixture_drift.py` |

**⟹ 「默认关」在 CI 层也成立。** 但 `scripts/` 层有覆写（在案反例，本轮复核成立）：`scripts/check_armR_trades_digest.py:17` 逐字 `M8_WIN_FILTER=$tag VOICE_EXEC=1 THETA_NEST_CERT_GATE=1 \`；`:355` `"_note": "臂R（VOICE_EXEC=1 THETA_NEST_CERT_GATE=1，fee_schedule=None/enforce_level_cap=false）"`。⟹ **凡引用 armR 系读数，`n_delta` 路径 B 都是开着的**（这是 §4.3 反方登记的实证）。

---

## 6. 两条与 Lean 三套骨架的对应（逐条比，不信注释）

### 6.1 三套骨架逐字

**① `formal/Origin/NestingCertificate.lean:185-190`（Nat 级别差）**
```lean
def nestCert (δ : Dir) (f : Nat → LevelData) (e : Nat) : Nat → Bool
  | 0 => Conf δ (f e)
  | d + 1 =>
      candOf δ (f (e + d + 1))
        && subB (jOf δ (f (e + d))) (jOf δ (f (e + d + 1)))
        && nestCert δ f e d
```
（`:199-200` `def N (δ) (f) (ℓ e) : Bool := nestCert δ f e (ℓ - e)`，`:194-196` 明写 `ℓ < e` 时 Nat 截断退化为基例，「不臆造 ℓ<e 的语义」。）

**② `formal/Origin/IntervalNestCertificate.lean:416-431`（列表链）**
```lean
def nestCertB (ev : Nat) : List NestLevel → Bool
  | [] => false
  | [ℓ] =>
      decide (ℓ.lvl = ev) && (ℓ.chosen.isSome) && ℓ.confirmOK
  | ℓ :: ℓ' :: rest =>
      if ℓ.lvl = ev then false
      else if ℓ.lvl > ev then
        ℓ.candidateOK && decide (ℓ'.lvl < ℓ.lvl) && subB ℓ' ℓ
          && nestCertB ev (ℓ' :: rest)
      else false
```
（`:440` `def chiBool (ev) (chain) : Bool := nestCertB ev chain`。）

**③ `formal/Strict/Nest.lean:218-222`（第三套，Prop）**
```lean
def NestCertificate : List LevelNode → Prop
  | [] => True
  | [ℓ] => ℓ.Candidate
  | ℓ₀ :: ℓ₁ :: rest =>
      ℓ₀.Candidate ∧ Sub ℓ₁.chosen ℓ₀.chosen ∧ NestCertificate (ℓ₁ :: rest)
```

### 6.2 锚指向哪套 + 真对齐核实

| Rust | 声明的锚（逐字） | 真对齐？ |
|---|---|---|
| `chi_bool`（`strategy/nest.rs:15`／文件头 `:3`） | 「对照 Lean `chiBool`/`nestCertB`」；文件头「契约锚：`formal/Origin/IntervalNestCertificate.lean`」 | **✅ 真对齐，逐分支同构、合取项顺序一致、连注释文字都对得上。** 五个分支一一对应；`[head,next,rest]` 臂的合取顺序 Rust `candidate_ok → next.lvl<head.lvl → sub_b → 递归` = Lean `candidateOK → decide(ℓ'.lvl<ℓ.lvl) → subB → nestCertB`。**唯一实现差**：Rust `:136-138` 为满足 slice 模式手工重建 `tail` Vec，Lean 直接 `ℓ' :: rest`——语义等价，无行为差 |
| `n_delta`（`classifier/nest.rs:328`） | 正文一句「Lean 端对应 (ℓ-e):Nat 结构递归」（**不是** `reference:` 标签，是散文） | **⚠️ 半对齐，两处实质不齐**：<br>(a) **递归载体不同**：Lean 递归于 `d : Nat`（级别差），Rust 递归于 `rungs : Vec<NestRung>`（列表）。二者只有在「rungs 长度 = ℓ−e 且第 k 个 rung 就是级 ℓ−k」时才同构，而**这个前提证书里无字段可复验**（`:306-309` 自陈的「诚实边界」）；<br>(b) **级别连续性 Lean 有、Rust 无**：Lean 的 `subB (jOf δ (f (e+d))) (jOf δ (f (e+d+1)))` 从**级别索引函数 `f`** 取两个区间，`e+d` 与 `e+d+1` 相邻**由构造保证**；Rust 的 `child` 取自 `top.child_interval` 或 `rest.first()` 或 `base`（`:352-355`），**没有任何东西保证它是「级 ℓ−1」的区间**。⟹ **Lean 白拿的不变量，Rust 丢了。** `:307-308` 明写「级别连续性…证书无字段可复验，`n_delta=true` **不**蕴含它们成立」——**注释是诚实的，但这意味着锚只在「装配层前置成立」的条件下才对齐，而那是 `assemble_certificate` 三门 DFS 的职责，不在证书里** |
| 两条与第三套 ③ | **都不锚它，且是明确弃用**：`classifier/nest.rs:3` 文件头逐字「**契约重锚（legacy Strict/Nest → `Origin.SubLevelDescent`）**」 | **✅ 弃用关系成立。** ③ 是三套里**最松**的：空链为 `True`（②是 `false`、Nat 套无此分支）、单级只要 `Candidate`（无 `ev` 比较、无 `confirm`）、无级别严格递减检查。⟹ **`chi_bool` 与 ③ 不同构（③ 更松），`n_delta` 与 ③ 也不同构（③ 没有基例 Conf）** |

### 6.3 ★ 一条对 N-4 有用的副产品（登记，不裁）

**三套 Lean 骨架自己就构成「同一判定的三档严格度」**：① Nat 差（级别连续性白拿，「到执行级」是算术必然）／② 列表链（`ev` 相等是可失败的运行时检查）／③ Prop 链（无 `ev`、无 `confirm`、空链为真）。`NestingCertificate.lean:426` 自标 `[需人工确认]`（①②无等价证明，N-2 已记）。⟹ **Rust 侧的两档严格度，在 Lean 侧有对应的两档，且 Lean 侧同样没有等价证明。** 收敛 Rust 那两条时，Lean 那两套要不要跟着收，是**同一个决定的两半**（本报告不裁；与 N-4 强耦合）。

**fixture 漂移**：三套骨架都**不在** `ParityFixtureExport.lean`／`CenterConstruct.lean` 的 import 闭包内（N-2 已核，本轮不重跑）⟹ 改它们不须重落 fixture。本报告零 `formal/` 改动 ⟹ 不触发 `CLAUDE.md` 的漂移检查节拍。

---

## 7. 原文考据：110 课对「同一判定在不同用途上可以宽严不同」怎么说

### 7.1 ★★ 直接命中：**说了，而且明确许可 —— 但许可的条件是「不产生原则性问题」**

**`035:18`【正文】**（`docs/chanlun/text/blog/035-第35课.md:18`，↑正文标记在 `:38` ⟹ 正文层）逐字：

> 上面就用最低级别的中枢把走势在最低级别上进行了完全分类（…）然后就可以严格定义各级别的中枢与走势类型而不涉及任何循环定义的问题。**但如果按严格定义操作，必须从最低级别开始逐步确认其级别，太麻烦也没多大意义，所以才有了后面1、5、15、30、60分钟，日、周、月、季、年的级别分类。在这种情况下，就可以不大严格地说，三个连续1分钟走势类型的重叠构成5分钟的中枢，三个连续5分钟走势类型的重叠构成15或30分钟的中枢等话。在实际操作上，这种不大严格的说法不会产生任何原则性的问题，而且很方便，所以就用了，对此，必须再次明确。**

**这是原文对本组题面的直接回答**：同一个判定（中枢／级别）确实有**两档严格度**，分档轴就是**用途**（理论定义 vs 实际操作），且缠师**明确批准**了宽档。

**但许可自带三个限定，缺一不可**：
1. **理由是「太麻烦也没多大意义」（成本），不是「严档判不出来」** —— 宽档不是严档失败后的兜底，是严档在实际操作上不经济时的经济替代。这与 `035:30`【正文】的成本门（N-2 已裁）是同一条理由。
2. **必须论证输出等价** —— 「不会产生任何原则性的问题」。这是**举证责任**：宽档要立住，得说清它在什么范围内与严档同结果。
3. **必须显式声明** —— 「所以就用了，对此，必须再次明确」。缠师亲自把这条宽档写成明文，不是默默用。

**`049:30`【正文】**（↑正文在 `:78`）第二处命中，形态更纯：

> 2、当下之前已出现该中枢第三类卖点（正出现也包括在这种情况下，**按最严格的定义，这最精确的卖点，是瞬间完成的，而具有操作意义的第三类卖点，其实是一个包含该最精确卖点的足够小区间**）。

⟹ **同一判定（第三类卖点）在理论口径是一个点、在操作口径是一个区间。** 「宽严两档按用途分」的教科书式原文。

**`072:68`【正文】**（↑正文在 `:78`）第三处，给出**退场条款**的原文形态：

> 注意，有时候课程是由浅入深，**前面不严格的，后面引进新概念后，就可以严格定义了**。例如，最开始时，说上涨、盘整，都是用高、低点之类的东西，因为当时没说中枢，所以不可能严格定义，后来说了中枢，就可以给出严格定义。

⟹ 这一档宽严差**有退场条款**：新概念到位 ⟹ 宽档退场。对照 `016:410-412`【答疑】（↑正文在 `:80`，故是答疑层）：「有关趋势与盘整的最严格定义没法给出（…）**以后会说到的**。所以现在各位先用这个通用的，但**不完全严格的定义**（…）该定义**唯一不精确的地方**就是这个顺势平台」——宽档 + 到期日 + **精确指出差在哪一条**（顺势平台）。

**`065:94`【答疑】**（↑正文在 `:56`，故答疑层）给出**反面判例**：

> 根据最严格的定义（…）一般来说，在线段之下讨论背驰概念是没意义的，但可以根据类似背驰的力度比较方法来讨论线段之下**类背驰**的现象，**但这和背驰是两回事情**。

⟹ 缠师在这里**拒绝**把宽档叫成同一个判定，而是另起名字 + 明确宣告「两回事」。**这正是收敛通则限定词①「不同判断」的原文根据。**

其余同族（不逐条展开）：`033:18`【正文】「最严格的意义（…）但这样会相当累，也没这个必要」；`053:22`【正文】「有这最精细最严格的方法（…）但这样太累，而且毫无必要。**理论是用来用的，只要不违反理论的基础与绝对性，当然要选择更简单的用法**」；`060:44`【正文】「这是按最严格的，并没有太大操作意义的分析」；`061:150`【答疑】（↑正文在 `:56`）「关键是看你用多大的精确度」+ 4022.69/4022.42 的三档取整实例。

### 7.2 ★ 与 `035:30`【正文】的关系：**同一件事的两种形态，还是两件事？**（这个判断承重）

票面点名要判这一条。逐字比：

- **`035:30`【正文】**（N-2 已裁为成本门）：「而究竟按什么级别来分析、操作，**和你的资金等具体条件相关**（…）那些太小的级别，不足以让交易成本、交易误差等相对买卖点间波幅足够小」⟹ **同一个判据（买卖点），按条件（资金／成本）取不同的参数（操作级别）。** 判据本身**一份**。
- **`035:18`／`049:30`／`072:68`**：**同一个判定有两个不同的实现（定义式），按用途选。** 判据本身**两份**。

**⟹ 是两件事，不是同一件事的两种形态。** 分界一句话：**「换参数」 vs 「换判据」。** `035:30` 换的是参数（级别号），判据（中枢／背驰／买卖点定义）一个字没变；`035:18` 换的是判据（严格递归确认 vs 「三个1分钟重叠 = 5分钟中枢」），二者是**两个不同的定义式**。

**这一判断的下游后果（承重点）**：
- 若判「两件事」（本报告的读法）：则 N-2 的「判据 + 参数」形态**不能自动移植到 N-6**。`chi_bool` 与 `n_delta` 的差**不是参数差**（没有任何一个参数能把 `NestRung` 变出 `lvl` 字段），是**判据差** ⟹ §8 的「参数化同一条」那一支**在本组不可行**（除非先改数据类型，即 §8 支 C 的真实代价）。
- 若判「同一件事」：则可援 `035:30` 直接批准两条并存 ⟹ §8 支 B 得原文背书。
- **⚠️ 但注意**：即便判「两件事」，`035:18`／`049:30`【正文】仍然**独立地**批准了「同一判定按用途分宽严两档」——只是许可条件是 §7.1 那三条（成本理由 + 输出等价论证 + 显式声明）。**本组现状三条全部不满足**：(a) 没有成本理由（两条并存是历史沿革，不是成本权衡）；(b) 没有输出等价论证（`NestingCertificate.lean:426` 自标 `[需人工确认]`，两套无等价证明）；(c) 有部分声明（两处 docstring 各自声明锚，但**没有任何一处声明「本仓有两条 N^δ 且它们的关系是 X」**）。

### 7.3 ↑正文 标记核实（逐条打开确认）

| 引文 | 该课 ↑正文 行号 | 引文行号 | 层 |
|---|---|---|---|
| `016:410`／`:412` | `016-第16课.md:80` | 410／412 > 80 | **【答疑】** |
| `033:18` | `033-第33课.md:52` | 18 < 52 | **【正文】** ✓（与已知界一致） |
| `035:16`／`:18`／`:30` | `035-第35课.md:38` | 16／18／30 < 38 | **【正文】** ✓（与已知界 `035:38` 一致） |
| `049:30`／`:50` | `049-第49课.md:78` | 30／50 < 78 | **【正文】** |
| `053:22` | `053-第53课.md:36` | 22 < 36 | **【正文】** |
| `060:44` | `060-第60课.md:60` | 44 < 60 | **【正文】** ✓（与已知界 `060:60` 一致） |
| `061:150` | `061-第61课.md:56` | 150 > 56 | **【答疑】** ✓（与已知界 `061:56` 一致） |
| `065:52` | `065-第65课.md:56` | 52 < 56 | **【正文】** |
| `065:94` | 同上 | 94 > 56 | **【答疑】** |
| `072:68` | `072-第72课.md:78` | 68 < 78 | **【正文】** |

已知界 `017:98`、`027:78`、`084:68` 本组未引，未复核。

---

## 8. 形态清单（候选，每支给代价；不裁）

**共同前提**：无论走哪支，`Cand^δ` 无定义式（N-5）与装配方向／退化守卫（N-4）都不动 ⟹ 本组任何形态都不能单独让区间套「在生产上真起作用」（`qujiantao.md:519-526` 的耦合硬约束成立）。

### 支 A：**收敛成一条**（哪条留、哪条死、退场条款）

**A1 留 `chi_bool`（列表链）、`n_delta` 退场**
- 改哪些代码：`classifier/nest.rs` 删 `n_delta`／`n_delta_rec`／`NestCertificate` 判定面（构造面 `assemble_typed_certificate` 保留但改返回列表链）；`admission.rs:300`／`:1017`／`nest_index.rs:295-297` 改调 `chi_bool`；`NestRung` 须**加 `lvl` 字段**（否则喂不出 `NestLevel`）；`interp.rs:580` 的单元素链要么保留（则环 2 行为不变）要么换真链。
- 翻掉哪些在案读数：**所有 armR 口径读数**（`THETA_NEST_CERT_GATE=1` 下的 trades digest、`NEST_GATE_STATS` 分账 `nest_pass`／`nest_n_delta_false`／`xzd_pass`／`xzd_gate_fail`、`effective_nest_depth` 分布、`econ_positive.rs:363-365` 那条 95.36%/4.64% 读数）。门关口径读数不变（`chi_bool` 那一个 bit 不动）。
- 名分能标到哪级：判据形状 `[旧缠论]`（`035:18` 明写「三个连续1分钟走势类型的重叠构成5分钟的中枢」＝ 级别链逐级相套的原文形态）；「末级必须恰为 ev」`[新缠论:推论]`（原文只说买卖点在次级别以下某一级别，不说必须恰好在指定级 —— `035:34`【正文】「不一定就是次级别的」）。
- 耦合：**与 N-4 冲突**（`n_delta` 的 `child_interval` 严格装配边 `I(A_child)⊆D_parent` 是 N-4 的对象，删了 `n_delta` 等于替 N-4 作答）。

**A2 留 `n_delta`（Nat 差/rungs）、`chi_bool` 退场**
- 改哪些代码：`strategy/nest.rs` 整个文件（306 行）退场；`interp.rs:569-587` `nest_confirm` 改直接读 `bits.confirm_side(side)`（**因为它现在就等于这个**）⟹ **改动后行为逐字节不变**（§4.1 已证）。
- 翻掉哪些在案读数：**零**（`chi_bool` 生产值 ≡ conf bit，替换后 bit-exact）。
- 名分：`[工程]`（这是删死码，不是教义变更）。
- 代价最小的一支，但**把「区间套在环 2 层的名分」整个删掉** —— `interp.rs:37`／`:68` 那两句「环2 区间套证书」将失去载体，环 2 变成裸 `Conf^δ`。**这是名分损失，不是行为损失。**
- 耦合：与 N-2 冲突（N-2 的说法② 载体就是 `chi_bool:129`，`qujiantao.md:579` 第 33 条明写「判据本身不改」）。

### 支 B：**两条并存但各自声明有效域**（按用途分，须给分界判据）

- 分界判据（须写进两处 docstring + 正本）：**`chi_bool` = 环 2 单级证书成立位（有效域：链长恒 1，只判终端 `Conf^δ`，级别谓词不适用）；`n_delta` = π 准入跨级链判定（有效域：`THETA_NEST_CERT_GATE=1`，rungs 由塔装配）。** 两者串联为 AND，互不兜底。
- 改哪些代码：**零 `.rs` 改动**，只加注释 + 正本一节。可选加一条**测试锁**（总缝规则 §① 要求的机械锁形态）：`assert!(chi_bool(l, &[单元素链]) == bits.confirm_side(side))`，把「环 2 恒等于一个 bit」这件事锁住，防止哪天有人偷偷改链长而无人察觉。
- 翻掉哪些在案读数：零。
- 名分：分界判据本身 `[工程]`（层次纪律，非缠论）；「按用途可宽严不同」可标 `[旧缠论]`（`035:18`／`049:30`【正文】），**但须同时落 §7.1 那三条许可条件**，其中「输出等价论证」现在拿不出（`NestingCertificate.lean:426` `[需人工确认]`）⟹ 名分只能到 `[旧缠论]` 的**形态**，不到**本仓这两条具体实现的合法性**。
- 耦合：把 N-2 的实施侧继续挡着（`qujiantao.md:523`「要它生效，先得让生产产出长度 > 1 的链」）。

### 支 C：**判为参数化同一条**（严格度 ＝ 参数，呼应 N-2 的「判据 + 参数」形态）

- 要做的事：给 `NestRung` **加 `lvl` 字段**，把 `n_delta` 的级别谓词做成可关的参数（`strictness: Lenient | Strict`），两条收成一个函数两个参数值。
- 改哪些代码：`classifier/nest.rs:158-169`（结构体 + 两个构造器 `new`／`assembled`）、`econ_positive.rs:995-1075`（`build_nest_certificate` 每个 rung 要填 `lvl`）、`nest_index.rs`、`assemble_typed_certificate`、`opsem_dump.rs:224`、7 个 `bin/`（`p92`／`p123`／`p107`／`p124`／`p116`／`p124_merge`／`strict_nest_check`）—— **粗估 8 个文件、数十个构造点**。
- 翻掉哪些在案读数：**armR 全部读数 + 7 个 bin 的全部重放读数**（结构体加字段会改 `PartialEq`／序列化，`opsem_dump` sidecar 格式变）。
- 名分：**`[工程]`，不能标 `[旧缠论]`**。理由见 §7.2：`035:30` 的「换参数」讲的是**级别号**这个参数，不是**严格度**这个参数。把「严格度」做成参数在原文里**没有对应物** —— 原文的两档是两个定义式（`035:18`），不是一个定义式的两个参数值。**能推的不许标「选择」，推不出的不许标「旧缠论」** ⟹ 此支只能 `[工程]`。
- **⚠️ 这一支有一个隐性代价**：`strictness` 参数一落地，就正好长成收敛通则要消灭的形状（`zhongshu.md:77` 已有先例：「否掉的乙案（Lean 保留 `≤` 当必要条件、另立严格谓词）：**正好长成收敛通则 #799 要消灭的宽严两档形态**」）。除非同时论证两个参数值**不接在 if-else 上、不互相兜底**。

### 支 D：**判伪分歧，不动代码只补文档**

- 依据：§4.3 的三层论证 + `zhongshu.md:69` 可达性条款 + 本图 Notes N-1 预授权。
- 改哪些代码：**零。**
- 补什么文档：`qujiantao.md` 加 N-6 节，写明 (a) 沿规范遗忘映射是严格包含 `A(chi_bool) ⊊ A(n_delta)`、(b) 差集在生产上不可达（点名 G1–G8 八行）、(c) 分界判据（同支 B）、(d) 退场条款 —— **「本判定在 `interp.rs:580` 改成非单元素链、或 `NestRung` 加上 `lvl` 字段的那一刻失效，须重开本组」**（总缝规则 §② 要求举证书必带退场条件，`AGENTS.md:84` 起「举证门：例外可以有，但必须自带退场条件」）。
- 名分：伪分歧判定本身 `[工程]`（可达性论证）；不占总缝规则例外配额（**因为总缝规则对本组不适用，§3.3**）⟹ 与 `zhongshu.md:93/:560` 的 E1 撤销先例同形（那条也是判伪分歧、配额回到「用 0 剩 3」）。
- 耦合：**不解 N-2 的实施侧阻塞**（`qujiantao.md:526`「『N-2 裁完就生效』是错觉」照旧）。
- **⚠️ 代价（照实）**：伪分歧判定**不修**「两条都不检查级别连续性」这个**共同缺口**（§1.3 #10）——那是 Lean ① 白拿而 Rust 两条都丢的不变量，归 N-4。判伪分歧会让人以为区间套判定这一块清完了，其实 #10 还开着。

### 支 E（本报告新增）：**先补测试锁，把判定推迟到锁红为止**

- 做法：只加 §支 B 说的那条 bit-exact 锁 + 一条「`chi_bool` 的 `[head,next,..]` 臂在生产上不可达」的覆盖率断言（或 `#[cfg(debug_assertions)] unreachable!` 桩）。锁绿 ⟹ 支 D 的可达性论证有了机械载体（不再是一次性人工核对）；锁哪天红了 ⟹ 自动重开本组。
- 代价：一次性写两条测试（约 30 行）；CI 已跑 `cargo test`（`ci.yml:184`）⟹ 载体有效，不是「只编译不执行」。
- 名分：`[工程]`。
- **这一支与支 D 可叠加**，是支 D 的退场条款的机械化。

---

## 9. 反例自评：最可能错在哪、什么证据能推翻

| # | 最可能错的地方 | 推翻它需要什么证据 |
|---|---|---|
| **1** | **§2 的「严格包含」依赖我自己引入的映射 Φ，而生产里没有任何代码实现 Φ。** 如果裁定者认为「没有实现的映射不算依据」，则唯一诚实答案是 §2.2 的**互不包含**，本报告 §2.1 整节作废 | 指出 `NestLevel` 与 `NestCertificate` 之间存在（或应当存在）一个**代码里真有的**转换 —— 我 grep 了 `From<NestLevel>`／`impl.*NestCertificate` 未见；若有人找到，§2 须重写 |
| **2** | **§4.3 的「伪分歧」结论建立在「chi_bool 生产链恒长 1」上，而我只核了 `interp.rs:580` 一处构造点。** 若存在第二个生产构造点（比如某个 `bin/` 或 `nautilus/` 适配层自己搓 `NestLevel`），结论翻 | `grep -rn "NestLevel {" rust/ src/` 出现第 8 个构造点、且在生产路径上。本轮读数：7 个构造点（6 test + 1 生产） —— 但**我没有查 `rust/src/nautilus/` 与 python 侧**（`NestLevel` 未导出到 pyo3，但没逐字确认）⟹ **这是本报告最大的未测项** |
| **3** | **§4.2 说路径 B 门开时 `n_delta` 吃真跨级链，但我没有实测门开时 rungs 的实际长度分布。** `econ_positive.rs:363-365` 那条 95.36% 读数是**路径 A（旧臂）**的，typed 真链（路径 B）的深度分布我没有读数 | 跑一次 `THETA_NEST_CERT_GATE=1` + `NEST_GATE_CHAIN` 落账，读 typed 链的 rung 长度直方图。若 typed 链也 95%+ 长度 0，则 `n_delta` 在生产上**同样**塌成一个 bit ⟹ **本组的结论会从「不可比」升级为「两条在生产上恒等」，形态清单该塌向支 A2** |
| **4** | **§7.2 判「换参数 vs 换判据是两件事」，这是我的推断，不是原文明说。** 若裁定者读成一件事，§8 支 C 的名分从 `[工程]` 升到 `[旧缠论]`，代价评估全变 | 找到原文一处把「级别参数」与「定义式严格度」当同一件事讲的【正文】。我搜了「不完全严格／最严格／严格的定义」全 110 课（25 处命中），没找到；`035:18` 与 `035:30` 是同一课相邻两段，**缠师用了两套不同的理由**（`:18` 是「太麻烦」、`:30` 是「资金/成本」）—— 这是我判两件事的主要依据，但它是**语用推断**，不是明文 |
| **5** | **§6.2 说 `n_delta` 的 Lean 锚「半对齐」，可能过苛。** `classifier/nest.rs:306-309` 已经诚实写了「级别连续性证书无字段可复验，由 `assemble_certificate` 三门 DFS 保证」—— 若三门 DFS 真的保证了，则锚在**装配后的证书集合**上是对齐的 | 打开 `assemble_certificate` 逐门核实「级别连续性」真的是三门之一（我**没有逐行核这一条**，只读了 `n_delta` 侧的自陈）⟹ **第二个未测项**。若成立，§6.2 (b) 应降为「锚对齐但对齐的是装配层不变式，不是证书自身」 |
| **6** | **§3.2 判「收敛通则限定词① 部分命中，须人裁」可能是回避。** 若裁定者认为两处 docstring 都自称 `N^δ` 就足以定「同一判断」，则限定词① 不成立、三条限定词全不命中 ⟹ 收敛通则**判违规**，形态清单该塌向支 A | 这是纯裁定题，不是事实题。本报告已把两侧证据摆齐（自称同名 vs 实际判不同对象），**不替人裁** |
| **7** | **N-1 裁定三的适用边界**：`classifier/nest.rs` 的 `NestInterval` 链父子关系**未验**（已毕业成 [探针 #908](https://github.com/xy7365527-lang/NewChanlun/issues/908)）。本报告 §2 的 W1/W2 反例用到了 `is_sub` 在该链上的取值 | 若 #908 查出该链的父子关系与我假设的不同（比如实际数据上 `child_interval` 恒 `None` 或恒等于 `base`），W1/W2 的「手算样例」仍成立（它们是**类型层可构造的输入**，不声称在真实数据上出现），但 §4.3 的「差集不可达」会多一条独立理由。**本报告不替 #908 作答，照实登记** |

---

## 10. 受影响代码清单增补（供 `qujiantao.md` §5 合并，不在本报告落盘）

| # | 文件:行 | 结论 | 归票 |
|---|---|---|---|
| N6-1 | `rust/src/theta_v0/classifier/nest.rs:158-169`（`NestRung` 四字段，**无 `lvl`**） | **登记，不改**：全组严格度差的结构根。任何「收敛成一条」或「参数化」形态第一步都是改这里 | **N-6** |
| N6-2 | `rust/src/theta_v0/strategy/interp.rs:580`（`let chain = [NestLevel {`） | **登记，不改**：保证 `chi_bool` 五条严条件全死码。`qujiantao.md:574` 第 28 条四行逐字复核成立，无需订正 | **N-6**（本票 28 条同一行） |
| N6-3 | `rust/src/theta_v0/backtest/admission.rs:69-71`（`.as_deref() == Some("1")`） | **登记，不改**：`n_delta` 路径 B 默认关的唯一保证；**与 `rec_engine.rs:305-312` 的 `is_ok()` 陷阱不同形**，本处严格 | **N-6** |
| N6-4 | `rust/tests/econ_oddeven_diagnosis.rs:3` + `:39` | **登记**：`n_delta` 路径 A（`econ_positive.rs:398`）生产 + CI 双不可达的保证 | **N-6**（订正 N-2「唯一入口 `decompose_capturable_spread`」的表述） |
| N6-5 | `rust/src/theta_v0/classifier/cand_event/key.rs:186-197`（`interval_is_degenerate`／`interval_is_sub`） | **登记**：⊆ 判据第三份实现，带退化守卫，生产不可达（消费者 = `bin/p129:292` + `cand_sub.rs`） | **N-4**／N-6（N-1 移交，`qujiantao.md:216`） |
| N6-6 | `rust/src/theta_v0/backtest/opsem_dump.rs:224`（`assemble_certificates_terminal` sidecar） | **登记，不改**：`OPSEM_DUMP_DIR` 门控 + `:15` 自陈不参与决策 ⟹ 命中收敛通则限定词②（诊断模式），正当分化 | N-6（登记） |
| N6-7 | `AGENTS.md:80`（总缝规则「本条落地的前置是先让 CI 跑 `cargo test`（已定为下游实施链第一条动作）」） | **注记已过期**：`.github/workflows/ci.yml:183-187` 自 #810 起已跑 `cargo test --all-targets`（含 `--features backtest_bin`）⟹ 该前置已闭合，注记须更新（不在本报告改动范围） | 教义正本维护 |

---

**报告完。本轮零 `.rs`／`.py`／`.lean` 改动；零 `formal/` 触碰 ⟹ 不触发 fixture 漂移节拍。**
