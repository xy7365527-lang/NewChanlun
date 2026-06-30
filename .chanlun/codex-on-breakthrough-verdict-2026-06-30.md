# codex O(n) 突破最终裁决：O(n) 是错误目标（2026-06-30）

异质源 codex CLI(gpt-5.5,xhigh,134153 tokens,**实读源码** interp.rs:554/892/710 coverage.rs:1782/1411)。编排者'把O(n)解决,问codex裁决'。原始/tmp/codex_ans_on_breakthrough.txt

## 最终裁决：O(n) 是错误目标。4前提逐个不可推翻。

真实下界:完整物化per-bar bucket Ω(Σ B(t))=Ω(n^2.26);最强等价增量/压缩fold Ω(B(n))=Ω(n^1.26)。engine在bit-exact+忠实缠论下不能比Ω(n^1.26)更好。

### 逐前提裁决
**① bsp渐近O(n):不可推翻**。事实B(n)≈n^1.26,源码candidate/BSP段来自classification.levels[*].bsp全量(interp.rs:554),bsp∝n^1.26每bar全量候选构造爆炸。**除非另有结构定理证明BSP总数线性**(←唯一理论突破口)。
**② 放弃每bar完整fold:语义不可放弃,算法循环可替换**。核心语义每bar须满足(D_t,B_t,K_t)=R_Θ(Γ_t,A_t)+A_{t+1}=AncOK[(A_t∖D_t)∪B_t]。prev_active明确进interpret再进活动集递归(coverage.rs:1782/1411)。'只处理新增BSP忽略旧confirmed在当前A_t新角色'=改语义。可增量算同一R_Θ,不可放弃逐bar extensional等价。
**③ A_t局部性:不可作突破口**。bucket规则非按parent_leg,而是按候选level/dir/bits+当前active同级腿状态(interp.rs:892)。单腿变化只影响同level方向流,但非'只影响依附该腿BSP'=同级slot全局谓词。要输出完整record序列仍须表示同级大量未变候选。
**④ BSP压缩摘要:实现优化,不破Ω(n^1.26)**。可per-level/dir候选流+slot状态+close/open前沿+record惰性区间。避免每bar扫所有BSP且展开后bit-exact。但API/测试要求每bar产Vec<Candidate>完整open/record/close则展开本身Ω(B(t))(interp.rs:710 candidate拼接/merge快照O(bsp)/bar)。

### 正确目标(替代strict O(n))
output-sensitive: O(n + B(n) + frontier_delta)。当前B(n)≈n^1.26下不是strict O(n)。

## 唯一理论突破口(连新goal)
codex①的例外:'除非另有结构定理证明BSP总数线性'。若PDF互斥分类'吃到每个元素'+缠论级别递归能证bsp实际O(n)(非n^1.26),下界破。这是新goal g-mutex-eat-every-element(#23推导链复核)可能触及的——但须L0结构定理,非工程优化。当前实测n^1.26是经验事实,推翻它需要证明高级别中枢数量不随历史超线性累积。

## 对acceptance[4](旧goal)的终局
acceptance[4]'exp≈1.0'=错误设定(假设bit-exact下不可达复杂度)。6工位(#11/#12/#13/#18/#20/#21)已诚实把exp 2.00→1.89。正确终局:acceptance[4]重设为output-sensitive Ω(n^1.26)达标,或接受1.89为有效域边界。**不开第七工位硬追O(n)**(=no-workaround:硬追证明不可达的目标)。

## 认识论 L0(复杂度下界证明,codex实读源码)。否定性结果(231号):否证O(n)可达性=否证acceptance[4]目标。
