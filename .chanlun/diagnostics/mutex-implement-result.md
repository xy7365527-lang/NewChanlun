# 工位 #27 [mutex-implement] 结果包：rust 角色分类器 + AncOK + Eat 验证器

## 1. 结论

**现状勘察（不重写已有）**：
- **MR1 角色分类器**：四分类已实装于 `rust/src/theta_v0/complete/state.rs`（`RootDirection` Long/Short/Flat + `voice_direction(v)=σ_r·(-1)^{d(v)}` 子声部方向递归，FULL §8 line 466）。复用，未重写。
- **MR2 AncOK 不提前剔除**：已实装于 `rust/src/theta_v0/complete/mod.rs:103` `VoiceForest::ancestor_closed()`（FULL §9 line 517：子激活蕴含父激活）+ `exact_unit_match()`（§9 line 525）。复用，未重写。
- **Eat 单笔判定**：已实装于 `rust/src/theta_v0/classifier/voice_eat.rs::eat()`（C06 逐字，10 测试）。复用。

**补缺（本工位新增，voice_eat.rs）**：
- `ContainmentReport { total_elements, uncovered_elements }`
- `verify_containment(tower: &[&[LeveledMove]]) -> ContainmentReport`：∀e∈E 遍历塔每级每个上级走势的 `sub_moves`，检查 WF-Contain（`parent.start ≤ sub.start ∧ sub.end ≤ parent.end`）。
- 2 新测试（13/13 全过）：真实塔 uncovered=0（情况 A 确认）+ 依附 host 反例被抓（验证器非死代码）。

**uncovered_elements = 0**（情况 A：par = host^struct 构成关系确认）。

## 2. 定义依据

- **C06 Eat(v,b)** ⟺ ∀t∈I_b, a_{v,t}=1 ∧ σ_v=ε_b ∧ q_{v,t}>0（spec complete-classification §3）。Eat 的数据层必要条件 = 元素 par 的 WF-Contain 包含。
- **WF-Contain**（#24 + on2.pdf 情况 A）：par(e)=a ⟹ [λ_e,ρ_e)⊆[λ_a,ρ_a)。
- **WF-Par**：par 只指向构成父（父区间=子并集）。代码依据 `LeveledMove::compose`（recursive_tower.rs:134-135）：`start_index=subs.first().start_index`、`end_index=subs.last().end_index` ⟹ 父区间 = 子元素并集 [λ_{e1},ρ_{em})，且 `descend(parent)==subs` 不变量 ⟹ `sub_moves` 是真构成父，非最近容器。

## 3. 认识论等级标注（formalization-validity-domain 231号）

**关键诚实声明（不冒充 L2）**：

| 验证对象 | 等级 | 理由 |
|---------|------|------|
| `verify_containment` 判定逻辑正确 | **L1** | 验证器逻辑 bit-exact 对齐 WF-Contain 定义；反例测试确认能否证 |
| 真实塔 uncovered=0 | **L0+L1** | WF-Contain 由 `compose_level` 构造结构保证（父区间=子并集），**与喂什么 bar 数据无关**——任何经 `compose_level` 建的塔恒成立 |
| gap-B 成立（情况 A） | **L0** | par=host^struct 是代码结构的逻辑必然（codex 异质审确认），非经验数据特征 |

**为何不是 L2**：`verify_containment` 检查的是 par 的**构成性结构**（代码不变量），不是市场数据特征。在真实 BTC bar 上跑也只会 uncovered=0，因为生产塔唯一构造路径是 `compose_level`，其父区间定义恒满足包含。真正的 L2 否证只在「par 改为 host^op 依附」时出现——那是**定义层变更**，已由反例测试 `verify_containment_catches_dependency_host_counterexample` 覆盖（手造 a=[0,5),e=[2,8) ⟹ uncovered=1）。

**不伪造「真实数据 L2 验证」**：no-patch-mentality + formalization-validity-domain 禁止把 L0/L1 包装成 L2。如实标注：uncovered=0 是**结构必然的确认性结果**，信息增量低；否定性路径（uncovered>0）只能由依附 host 触发，已用反例测试覆盖。

## 4. 边界条件（结论翻转点）

- 若 `compose_level` 改为以「最近容器/右端点命中容器」（host^op 依附）确定 par，而非子并集 ⟹ WF-Contain 可被违反 ⟹ uncovered>0 ⟹ gap-B 反例真实存在 ⟹ Eat 失败。codex 指出裸调 `compose(subs=[[0,5),[2,8),[5,6)])` 可构造反例，但 `compose_level` 的连续窗口 [i,i+1,i+2] 不变量排除此输入。
- 若 task#14 K_i carrier forest 的 par 用 host^op 而非 host^struct ⟹ 同样反例。**本工位确认当前 carrier（LeveledMove）的 par = host^struct 构成关系**。

## 5. 异质审（codex，约束4，2026-06-30）

codex exec 审「LeveledMove::compose 的 par 是 host^struct 还是 host^op」三问结论：
1. par = WF-Par 构成父（父区间=三段子走势外包跨度 [subs[0].start, subs[2].end)），非额外寻找的 host。
2. 有效塔不变量下每个 sub ⊆ parent（连续窗口按 K 序递增）；裸调非法序列可构造反例但非合法 `compose_level` carrier。
3. **par = host^struct（descend(parent)==subs 构成关系），gap-B 在此 carrier 下成立**；a=[0,5),e=[2,8) 是改 par 为依附 host 后的反例，非真实 compose 父子反例。

## 6. 下游推论

- gap-B（祖先生命期包含）在当前 LeveledMove carrier 下成立 ⟹ #24 Lean 证明（AncestorLifespan.lean 闭端点包含）的前提（par=构成关系）在 rust 实装上得到结构确认。
- ∀e∈E Eat(e) 的数据层必要条件（par 包含）满足；充分条件还需声部活动流 `a_{v,t}/q_{v,t}`（运行时，由策略层提供）——`eat()` 单笔判定已就绪。
- AncOK（`ancestor_closed()`）不提前剔除子声部 ⟹ MR2 满足。

## 影响声明

- 改动文件：`rust/src/theta_v0/classifier/voice_eat.rs`（+`ContainmentReport`+`verify_containment`+2 测试）。
- 未改：complete/state.rs、complete/mod.rs（MR1/MR2 复用未重写）；无 bit-exact 路径触碰。
- 测试：`cargo test --lib voice_eat` 13/13 通过。
