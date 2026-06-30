# codex 严格修复方案：acceptance[4] extract O(n) ceiling（2026-06-30）

异质源 codex CLI（gpt-5.5,xhigh,125247 tokens）。**codex 实读代码库**（引用真实行号）。原始：/tmp/codex_ans_q3.txt

## 总结论
严格最小修复**不是再优化 TreeKey**，而是把热路径从'每 bar 重新证明整棵树没变'改成'classifier 发出可观察变更代次(gen)/delta，extract cache 只按 delta 更新'。`TreeKey::of` 和任何 hit-path `build_*_index` 必须退出 per-bar 热路径。

## 严格修复（按优先级，最小→彻底）

### 1. 接通 tower_gen（最优先·最小改动·可能直接打掉 exp≈2.0 主项）
**关键发现**：interp.rs:494 **已存在** `coverage_elements_and_gamma_with_tower_cached_gen(..., tower_gen)` 版本，但 runner.rs:571 **还没传 Some(tower_gen)**——这是已实装但未接线的修复。接通后 `TreeKey::of` 全树递归彻底移出热路径。
严格签名：`F: FnMut(usize) -> (Classification, Vec<Rc<Vec<LeveledMove>>>, u64)`，classify_at 返回 tower_gen，两处 extract 调用传 Some(tower_gen)。

### 2. 若诊断证实 hit 仍 rebuild 三索引 → 索引并入 TreeCache，hit 只 Rc::clone
TreeCache{ gen, tree:Rc, by_level:Rc, by_interval:Rc, by_parent:Rc, by_id:Rc }。build_*_index 只在 gen miss/显式 full reset 出现。ElementView::with_base_indices 注入 base，candidate/restore overlay 单独小索引不扫 base。

### 3. delta 接口
enum ElementDelta{ Add / RemoveFromSnapshot(只改可见性不删registry) / UpdatePayload(仅indexed key变才remove+insert) / Reparent(只移parent bucket,ElementId不变) }。apply_delta=O(|delta|)。

### 4. ElementId 脱钩
ElementId 必须只由'元素自身构造 witness'决定,不含 TreeKey/根路径/Vec index/parent path。
- 当前 repo (level,ordinal) 是**可接受最小形式**（ordinal来自同级确定性扫描:全量enumerate/增量prefix_count+i,见 recursive_tower.rs:55）。
- 更严格 witness: ElementId = H(level, stable_birth_span, child_id_window, center_witness, tie_break)。
- **parent_id 是关系不是身份**：根变只允许改 parent_id/snapshot index,不允许改 id。

### miss/根变·精确必须改集合
Added=Id(T_new)-Id(T_old) / Removed=Id(T_old)-Id(T_new)(只移出snapshot可见集) / Changed={payload_without_vec_index differs} / Reparented={parent_id变}。
新最高级出现=新增new root+把它直接覆盖的旧root改为children;旧root的descendants**不改ID不重建索引**。
persistent DAG: ExtractNode{id,payload,parent,first_child,next_sibling,prev_sibling} + ExtractDag{nodes:HashMap,roots:SmallVec,index}。根分裂=Add(new_root)+Reparent(child_1..k),非重写子树。IndexState::apply(fragment)=O(|fragment|)。

## 诚实的不可消除项（bit-exact 下严格下界）
1. **若 API 每 bar 必须返回完整 Vec<CoverageElement> 且 parent:Option<usize> 逐元素写成当前 preorder index → Θ(|E|) 输出下界,bit-exact 不可消除。** 唯一解=热路径改 DAG/ElementView,完整 Vec 仅 debug/export materialize。
2. 若 extract_elements 最高级-only 语义导致 Removed=Θ(n) → present-only 索引删除也 Θ(n) 下界。解=全局 DAG+lazy visibility,不按节点逐个清 present。**这是表示层改变,非 workaround。**
3. **root miss 若稀疏=偶发 Θ(n),不决定摊还指数;若数据能构造频繁 Θ(n) visible-set 翻转,那是真实下界,不能诚实标成 O(1)。**

## 落地顺序建议（Lead 解读）
步骤1(接通tower_gen)是最小且可能直接见效的——先做诊断确认 hit 路径是否还在跑 TreeKey::of/build_*_index,若是,接通 gen 即可能让 exp 大降。步骤2-4 是若步骤1不够时的递进。步骤'不可消除项'是诚实边界,达到即止,不伪装。

## 认识论：L0(方案/定义层,codex实读代码)。落地后须 L2 实测 exp + bit-exact 验证。
