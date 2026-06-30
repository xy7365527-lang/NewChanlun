# codex 最终判定：引擎能否 O(n)？——不能，bit-exact 下 O(n) 不可达（2026-06-30）

异质源 codex CLI(gpt-5.5,xhigh,49435 tokens,实读profile数据)。编排者问"引擎能不能变成o(n)"。原始/tmp/codex_ans_engine_on.txt

## 裁决：bit-exact 约束 + bsp~n^1.26 下，engine 不能真正变 total O(n)。

### 严格论证
设 n=K线数, B(t)=第t bar累计confirmed BSP数, 实测 B(t)=Θ(t^β) β≈1.26, A_t=当前活动腿集, fold结果 bucket(b,A_t)∈{close,open,record}。

**1. BSP超线性已破坏strict O(n)**：bit-exact须真实生成/持有BSP ⟹ T(n)≥Ω(B(n))=Ω(n^1.26)>O(n)。正确目标最多output-sensitive O(n+B(n)+delta传播)。

**2. fold每bar消费完整BSP集 ⟹ 下界Ω(n^2.26)**：每bar C(t)≥Ω(B(t))=Ω(t^1.26),T(n)≥Σt^1.26=Ω(n^2.26)。per-bar摊还=Ω(n^1.26)**非常数,数学上不可达O(1)**。实测exp 1.89只是benchmark区间未进残余主导区,不能反证下界,渐近向2.26靠。

**3. delta fold理论有路但当前语义不给O(ΔA_t)**：需证 Σ|Affected_t|=O(n)。但fold规则'slot空→open'/'方向反向→close'是**全局活动状态谓词**——一次A_t变化可能影响所有历史同向/反向BSP ⟹ |Affected_t|=Θ(B(t))。反向索引只能更快找到全部受影响者,不能消掉产出close/open事件的成本(红线:同一BSP的桶不能丢,不能压缩)。最优也只到 O(|ΔA_t|log B+|Affected_t|),非天然O(|ΔA_t|)。

### 诚实下界分层
```
当前bit-exact完整fold语义: total下界Ω(n^2.26), per-bar摊还Ω(n^1.26), O(n)不可达
若证delta fold且受影响BSP可控: total下界≥Ω(B(n))=Ω(n^1.26), 仍非strict O(n)
claim O(n)须推翻≥1前提: ①BSP渐近非n^1.26且可一次生成 ②bit-exact不要求每bar完整重判历史BSP ③A_t影响严格局部Σ|Affected_t|=O(n) ④BSP/事件可用证明等价的压缩摘要替代逐项物化
```

## 对 acceptance[4] 的含义（关键）
acceptance[4] 的'全引擎per-bar exp≈1.0'目标**本身是错误设定**——它假设了bit-exact下不可达的复杂度。不是6个工位没做到(#11/#12/#13/#18/#20/#21已把exp 2.00→1.89),是目标设定时未意识到 bsp超线性(n^1.26) + fold全局A_t依赖 的联合下界。这是L0否定性结果(231号):缩小了acceptance[4]的有效域=暴露目标设定的范畴错误。

## 可选路径(供编排者裁决)
1. **接受下界,重设acceptance[4]**：目标从'exp≈1.0'改为'output-sensitive最优 O(n+B(n))'或'达到delta-fold下界Ω(n^1.26)'。诚实承认strict O(n)不可达。
2. **推翻前提②(放弃每bar完整fold bit-exact)**：若下游只需聚合摘要不需逐项close/open物化,可压缩。但这改变语义,可能触及spec定义(定义冲突→escalate)。
3. **推翻前提①(改BSP表示)**：若bsp可压缩表示不逐项物化。high risk重构。
4. **接受现状(exp 1.89)作为有效域边界**：不再投入,acceptance[4]标'已优化至1.89,strict O(n)经codex严格证明bit-exact下不可达(下界Ω(n^1.26))'。

## 认识论 L0(复杂度下界证明,codex实读profile)。这是否定性结果=否证acceptance[4]目标可达性。
