# codex CLI 双议题裁决（2026-06-30）

异质源：OpenAI Codex CLI（gpt-5.5, reasoning=xhigh）。service_tier="fast"覆盖无效priority。
完整原始输出：/tmp/codex_ans_q1.txt（议题一,57902 tokens）+ /tmp/codex_ans_q2.txt（议题二,77970 tokens）

## 议题一：acceptance[4] extract侧O(n)ceiling理论可达性

**verdict：O(1)摊还可达（非不可增量）。当前是表示层下界,非Chanlun树数学下界。**

- 可达条件：高层新结构几何稀疏(✓,26 miss@16K≈2·log2(n)) + 低层元素身份稳定 + 索引持久化共享(非root-relative全量物化)。
- 真正下界来源=表示：若TreeKey/索引键把'根相对路径'当身份,根一变所有旧节点key/ancestor/parent坐标全变,物化必须写Θ(n),miss时Ω(n)不可避免。**但这是当前表示的下界,不是树本身的下界。**
- 关键反诘：26次miss符合O(log n)量级,但只有miss次数O(log n)**不足以**推出总成本O(n)。实测x_exp≈2.09不太可能只由'26次几何full rebuild'解释。须检查三件事：
  1. miss是否在大n端密集(而非按层级几何分布)
  2. build_*_index是否在**非miss路径也隐式扫全树**(独立于miss的常态O(tree))
  3. miss时是否因root-relative key使旧元素语义全部'改名'→D_t=Θ(n)

**最契合范式**：persistent DAG/rope/finger tree + cached summaries + versioned indexes。类比B-tree根分裂——根变但叶子不重写,新root引用旧稳定子树。**非link-cut tree。**

**落地结构（6步）**：
1. 元素身份脱离TreeKey：ElementId由稳定区间+级别+构造witness决定
2. TreeKey只表示版本/root,不参与旧元素身份
3. 树改持久化DAG：新高层节点持有旧top nodes引用
4. 三索引改可组合片段：每subtree缓存自己的index fragment,新root只合并子摘要
5. parent/ancestor不批量重写,用versioned parent overlay或interval containment查询
6. 未确认尾部保留dirty suffix,稳定前缀冻结后不再重抽取

**验收建议**：记录每次miss的(bar,tree_size,old_height,new_height,changed_stable_ids,touched_index_entries)。比较 S(N)=Σtree_size_at_miss vs R(N)=Σchanged_stable_ids_at_miss。S大R小⟹表示层全量重建可修；R=Θ(S)⟹语义本身改全树,须先改'稳定前缀'定义才能承诺O(变动)。

## 议题二：650 goal事件reader/writer契约分裂三立场

**verdict：B最自洽；A2可成立但本质=B+恢复审计事件,非对A的综合；A不自洽。最终建议=B+A2的审计部分。**

- **问题1（reader读writer不产生的事件）**：不一定是bug。合法='读历史宽容,写新严格'(老版本曾合法写/upcaster兼容/文档化legacy/核心闭合不依赖它/仍走统一schema验证)。**非法边界**=reader把writer永不产生+schema未承认+靠裸append的事件当**当前状态机关键语义输入**。GOAL_RESUME若重锚base_head=reader拥有了schema外写入协议=契约破裂。
- **问题2（base_head语义）**：应是**不可变历史锚**(GOAL_SET时刻=goal在某代码世界被定义的事实)。base_head≠current_head预警有价值=降级信号(验收语义可能被代码漂移污染/旧EVIDENCE未覆盖当前HEAD)。每次resume重锚=把'定义时刻'与'继续工作时刻'混成一字段+drift warning永久失效。若确需移基线→显式高权限GOAL_REBASE(reason+old_base+new_base+author+影响范围),非热启动自动。
- **A不自洽**：把session恢复升级成goal语义变更+抹掉过时预警。
- **B最干净**：删reducer RESUME重锚,承认base_head过时是正确预警,reader只消费schema承认的事件。
- **A2可接受非严格扬弃**：GOAL_RESUME补进schema但纯审计(goal_id+note+ts+author,不影响base_head/acceptance/closure)→核心语义上A2站在B这边。
- **验收闭合**：EVIDENCE=材料,CHECK_PASS=裁决,reducer只认CHECK_PASS没错,但protocol必须提供evidence→pass授权提升路径。前置缺口准确：先给EVIDENCE稳定evidence_id,再让CHECK_PASS引用(acceptance_id+evidence_ids+verifier+head+verdict)。goal不闭合是**协议缺失**,非reducer应私自猜测。

## 认识论等级
两裁决均L0（概念/定义层,异质源codex独立产出）。议题一的可达性判断需议题一落地后L2实测验证(S(N)vs R(N))。
