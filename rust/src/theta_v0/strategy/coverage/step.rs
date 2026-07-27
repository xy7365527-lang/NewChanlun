use super::*;

/// [`coverage_step_from_buckets`] 的净额兼容出口（22 处旧调用点保持二元返回，bit-exact）。
/// 委托 [`coverage_step_from_buckets_sep`] 丢弃第三分量 `sep_legs`（M5 声部执行层暴露，纯只读，
/// 不进决策路径）——单源无平行状态机。
pub(crate) fn coverage_step_from_buckets(
    work: ElementView,
    prev_active: &[ActiveLeg],
    buckets: &Buckets,
    base_units: f64,
    config: &VoiceConfig,
    risk: Option<&RiskConfig>,
    registry: &super::super::persistent::PersistentRegistry,
) -> (Vec<ActiveLeg>, f64) {
    let (next_active, p_tilde, _sep) = coverage_step_from_buckets_sep(
        work, prev_active, buckets, base_units, config, risk, registry,
    );
    (next_active, p_tilde)
}

/// ★M5 声部执行层 sep 出口（多空对冲.pdf p16 关卡10）：与 [`coverage_step_from_buckets`] **同一
/// 决策路径**（三桶→对位→AncOK→净额，单源），额外暴露逐声部目标头寸 `sep_legs: Vec<SepLeg>`
/// （`P^sep_t=Σ_{v∈A_{t+1}}q_vσ_v e_v` 的分量，携 carrier ElementId + role(v) + parent(v)）。
///
/// `sep_legs` 是本函数**已算** `legs`（post G7 gross cap）+ `next_idx` 对位元素 ElementId 的只读
/// 重打包——**不新计算**，决策路径逐字节不变（`net_target_units(&legs)==p_tilde` 恒等）。runner
/// 的 [`OverlayState`](super::super::overlay_state) hedge-mode 簿据此建持久逐声部 P^sep 账本 + ΔN 订单。
pub(crate) fn coverage_step_from_buckets_sep(
    mut work: ElementView,
    prev_active: &[ActiveLeg],
    buckets: &Buckets,
    base_units: f64,
    config: &VoiceConfig,
    risk: Option<&RiskConfig>,
    registry: &super::super::persistent::PersistentRegistry,
) -> (Vec<ActiveLeg>, f64, Vec<SepLeg>) {
    // ★热点② O(n²) 消除：work = ElementView{base=持久树前缀借用零拷贝, overlay=本 bar candidate 段}。
    // 旧 `elements.to_vec()` + 上游 `tree.clone()` 每 bar O(tree)×n=O(n²) 双双消除（base 借 Rc 树，
    // candidate 在 overlay）。restore 路径继续 push overlay 尾（绝大多数 bar 不触发）。
    // candidate_start = base 段长（candidate 从此起，在 overlay）。
    let candidate_start = work.base_len();
    let mut raw: Vec<usize> = Vec::new();
    // ★票#315/#350（#284 评审 MED-1 + #347 评审 LOW-3）：待统一修补 parent/attached_dir 的元素 idx
    // 累加器——held 腿占位（LivePresent/LiveDetached，push 时 parent_id 已知但父未必已在场）与
    // restore 链恢复元素（`restore_ancestor_chain_from_registry` 本 bar 可能被多次调用，票#350起
    // 不再各自函数内立即修补）共用同一累加器，两个物化循环（本循环 + 下方 open 候选父链恢复，
    // 含循环内触发的全部 restore 调用）结束、AncOK 判定前统一 fixup（见函数尾部）。替代立即式
    // （push/物化当轮即修补）——立即式对「父在本轮更晚才物化进 raw（无论经哪条路径）」的场景查
    // 不到父，误留 None/None（#315 held 腿占位坐实；#350 坐实 restore 链同类时序孔）。
    let mut pending_parent_fixup: Vec<usize> = Vec::new();

    // ponytail: H6 单次建 tree 前缀 ElementId→idx 索引——tree 前缀在持仓腿对位期间不变
    //（Stale 追加在 candidate_start 之后，不污染 tree 前缀）⟹ 建一次、多次查 O(1)。
    // codex Q4：按 ElementId 结构映射查表（spec §13），替代旧 (level,ρ,eps)/(level,λ,eps) 值比较。
    // tree_prefix(tree_end) **借用 base**（candidate_start ≤ base.len()）⟹ build_tree_id_index 不 to_vec。
    let tree_end = candidate_start.min(work.len());
    // ★工位 4d 热点②：缓存命中复用 `Rc<id_idx>`（`Rc::clone` O(1)，§16 tree 前缀不变 ⟹ ElementId→idx
    // 不变；独立持有 ⟹ 不借 work，下游 restore push 可变借用 work 不冲突）；缺失（测试/无缓存路径）⟹
    // fallback 现建 O(tree)。tree_end==candidate_start==base_len ⟹ 缓存覆盖范围 == tree_prefix(tree_end)
    // == 完整 base，bit-exact 一致。
    let id_idx: Rc<std::collections::HashMap<ElementId, usize>> = match &work.base_id_idx {
        Some(rc) => Rc::clone(rc),
        None => Rc::new(build_tree_id_index(work.tree_prefix(tree_end))),
    };
    // ★工位 4f：overlay 段（candidate ++ restore push）id→idx，restore 复用查 O(1)（消 work.iter().position
    // O(work)/层=O(n²)）。初始 = candidate 段（base_len..work.len()，首次出现序）；restore push 时累积。
    // bit-exact == 旧 work.iter().position：查找 id_idx(base) 优先 → overlay_seen，与 position「base 段在前」
    // 序一致；or_insert 存首次 idx，与 position 首个匹配一致。
    let mut overlay_seen: std::collections::HashMap<ElementId, usize> =
        std::collections::HashMap::new();
    for i in candidate_start..work.len() {
        if let Some(e) = work.get(i) {
            overlay_seen.entry(e.id).or_insert(i);
        }
    }

    // (A_t ∖ 𝒟_x)：持仓腿（除 close 认领）按 ElementId 对位回当前因果树元素；不在树 ⟹ 按
    // is_boundary_root 决定 root/prune（发现 A 修复：Stale 不伪造 parent:None）。
    let closed = close_indices(prev_active, &buckets.close);
    for (i, leg) in prev_active.iter().enumerate() {
        if closed.contains(&i) {
            continue; // 𝒟_x：本腿关闭，不入 A^raw
        }
        // tree_end 固定 + Stale 追加在 overlay（tree_end 之后）⟹ base 前缀内容不变；每轮重借（NLL）。
        let m = held_leg_tree_index_indexed(work.tree_prefix(tree_end), leg, &id_idx);
        match m {
            // Exact（ID 命中=同一走势，父延伸也同 ID）：对位回当前树元素 idx（携真父链）⟹
            // 其子声部腿的 AncOK 祖先齐全。
            HeldLegMatch::Exact(idx) => {
                if !raw.contains(&idx) {
                    raw.push(idx);
                }
            }
            // Stale（snapshot 找不到）：★persistent overlay 修复（anc.pdf §8/§10）。
            // Stale(L) ⟺ pid(e)∉Pj or explicit invalidation（非 pid(e)∉Ej）。
            // e∉Ej 只是 snapshot_present(e)=0，不是 persistent_alive(e)=0（§8）。
            // 检查 persistent registry：LiveDetached 保留（op_parent 持久，I4+I5），Invalidated 才 prune。
            HeldLegMatch::Stale => {
                ancok_probe_bump(|p| p.stale_arm += 1);
                let held_state = registry.held_state(leg);
                match held_state {
                    super::super::persistent::HeldLegState::LivePresent => {
                        ancok_probe_bump(|p| p.state_live_present += 1);
                        // 理论不可达（Exact 未命中但 registry LivePresent = snapshot 不一致）；
                        // 按持久身份保留（I1），op_parent 驱动 AncOK。
                        let idx = work.len();
                        work.push(CoverageElement {
                            lambda: leg.lambda,
                            rho: leg.source_index,
                            eps: leg.dir,
                            level: leg.level,
                            parent: None,
                            attached_dir: None,
                            id: leg.id,
                            parent_id: leg.op_parent,
                        });
                        // ★票#315（#284 评审 MED-1，订正票#267 立即式）：占位角色输入修补改收集 idx，
                        // 循环后统一 fixup（见函数尾部 AncOK 判定前，helper doc 详述时序孔成因）。
                        pending_parent_fixup.push(idx);
                        raw.push(idx);
                    }
                    super::super::persistent::HeldLegState::LiveDetached => {
                        ancok_probe_bump(|p| p.state_live_detached += 1);
                        // ★anc.pdf §10 核心修复：LiveDetached 不 prune，不伪造 root。
                        // parent 仍是 op_parent(L)（§15），只是当前 snapshot 没展示。
                        // op_parent 在 persistent registry 中 live（I4）→ AncOK 通过（I5）。
                        // ★I5 + §11 归纳：从 registry 递归恢复整条操作祖先链（op_parent 及其祖先），
                        // 全部加入 work/raw，使 ancestor_close_by_id 通过（§11：每条未关闭腿的操作父
                        // live ⟹ 所有 depth<d 腿通过 persistent AncOK）。
                        if let Some(op_pid) = leg.op_parent {
                            restore_ancestor_chain_from_registry(
                                &mut work, &mut raw, registry, op_pid, &id_idx, &mut overlay_seen,
                                &mut pending_parent_fixup,
                            );
                        }
                        let idx = work.len();
                        work.push(CoverageElement {
                            lambda: leg.lambda,
                            rho: leg.source_index,
                            eps: leg.dir,
                            level: leg.level,
                            parent: None,
                            attached_dir: None,
                            id: leg.id,
                            parent_id: leg.op_parent,
                        });
                        // ★票#315（#284 评审 MED-1，订正票#267 立即式）：占位角色输入修补改收集 idx，
                        // 循环后统一 fixup（同上；restore 已把 op_parent 祖先链物化入 work/raw，统一
                        // fixup 时点可解析，见函数尾部 + helper doc）。
                        pending_parent_fixup.push(idx);
                        raw.push(idx);
                    }
                    super::super::persistent::HeldLegState::Closed | super::super::persistent::HeldLegState::Invalidated => {
                        // 显式关闭/作废 → prune（§9 rule 5：只有 close/risk close/invalidation 才退出 live）。
                        if leg.is_boundary_root {
                            ancok_probe_bump(|p| p.closed_inval_boundary_kept += 1);
                            // 真边界根 ∂：作根保留（parent_id=None 合法，AncOK 不剔）。
                            let idx = work.len();
                            work.push(CoverageElement {
                                lambda: leg.lambda,
                                rho: leg.source_index,
                                eps: leg.dir,
                                level: leg.level,
                                parent: None,
                                attached_dir: None,
                                id: leg.id,
                                parent_id: None,
                            });
                            raw.push(idx);
                        } else {
                            ancok_probe_bump(|p| p.closed_inval_pruned += 1);
                            // 非边界根且 invalidated/closed → prune（不入 raw，§9 rule 5）。
                        }
                    }
                }
            }
        }
    }

    // ∪ ℬ_x：开启候选 → 其 638 附着因果树元素索引（candidate_start + gamma_index）。
    //
    // ★工位 H 修复（级别容器.pdf §13）：开仓激活的位置节点身份 = carrier 容器 hostOf(g)（其 ElementId
    // 在候选元素 id 上携带，见 interp.rs `coverage_elements_and_gamma_with_tower`），**不是**买卖点叶子。
    // 故候选自身入 raw 即等价于「激活 carrier 上的位置节点」（PDF §14 简化 position instance）。子声部
    // 的 parent_id（= par_C(carrier)）由 ancestor_close_by_id 检查是否在 raw（= 持仓父位置节点在 A_t）。
    //
    // ★删除旧 G host 注入（级别容器.pdf §6/§7 + §13 否定）：旧 G 把 `(c.level, c.source_index)` 命中的
    // **同级 host 叶子**注入 raw（叶子作持仓 = a_carrier 仍 0），是 PDF §6/§7 反证的"开叶子"错形式，
    // 且对 depth>0 子无效（注入的是叶子自身非父 carrier）。位置节点身份改为 carrier id 后，候选 id=
    // carrier id 入 raw 作位置节点。
    //
    // ★(I-1) 修正（codex 行级坐实）：候选自身入 raw 只激活 **本级** carrier 位置节点；depth>0 **子声部**
    // 候选的准入还需其**父** carrier（= par_C(carrier)，候选 parent_id）在 raw（§13 AncOK 子声部不漂浮
    // 在不存在的父上）。父 carrier 几乎从不与子同 bar 共现/持仓（sd_parent_held=0），仅 registry
    // LiveDetached 存活——故 open 候选自身入 raw **不足以**让父在场。下方补 open 候选父链注入（no-patch：
    // 缺失逻辑补全，非 AncOK 加特例）。
    for c in &buckets.open {
        let idx = candidate_start + c.gamma_index;
        if idx < work.len() && !raw.contains(&idx) {
            raw.push(idx);
        }
        // ★(I-1) open 候选父注入（codex 异质审查行级坐实，642/644）：
        //
        // depth>0 子声部候选（ShortDiff/FollowParent）的真 Compose 父 carrier（= par_C(carrier)，候选
        // 元素 parent_id 携带）几乎从不与子同 bar 作 open 候选，也几乎从不是 prev_active 持仓腿
        // （L2 诊断：sd_parent_held=0），但**在 persistent registry 中 LiveDetached 存活**
        // （sd_parent_registry_alive≈sd_total）。Stale 持仓腿路径（上方 LiveDetached 分支）已用
        // [`restore_ancestor_chain_from_registry`] 把 op_parent 祖先链注入 raw，但 open 候选路径
        // **从不触发**该恢复 ⟹ 父 carrier 不在 raw ⟹ ancestor_close_by_id 判子声部祖先不齐 ⟹ AncOK
        // 全剪 depth>0 子腿（accepted_cert_carrier=0）。
        //
        // 修复：把 Stale 路径的祖先链恢复机制**扩展到 open 路径**——对每个 open 候选的父 carrier
        // （work[idx].parent_id），若 registry live 且不在 raw，从 registry 递归恢复整条结构祖先链
        // （§11 归纳：每条子声部腿的操作父 live ⟹ depth<d 祖先全在 raw ⟹ AncOK 通过）。这让持仓父
        // carrier 的位置节点进入 A_t（§8 父声部 carrier 跨 bar 持有，§9 祖先闭合兑现），depth>0 子腿
        // 准入。**非** AncOK 加特例放行——父链真实注入后由原 ancestor_close_by_id 正常判定（no-patch）。
        if idx < work.len() {
            if let Some(parent_pid) = work[idx].parent_id {
                let parent_in_raw =
                    raw.iter().any(|&r| work.get(r).map(|e| e.id == parent_pid).unwrap_or(false));
                if !parent_in_raw && registry.registry_live(&parent_pid) {
                    restore_ancestor_chain_from_registry(
                        &mut work, &mut raw, registry, parent_pid, &id_idx, &mut overlay_seen,
                        &mut pending_parent_fixup,
                    );
                }
            }
        }
    }

    // ★票#315/#350（#284 评审 MED-1 + #347 评审 LOW-3）：held 腿占位 + restore 链恢复元素统一
    // fixup——两个物化循环（上方 prev_active held 腿 + open 候选父链恢复，含循环内触发的**全部**
    // `restore_ancestor_chain_from_registry` 调用，该函数本 bar 可能被多次调用）均已结束、AncOK
    // 判定前，对本轮收集的 `pending_parent_fixup` 统一重建 parent/attached_dir。此时 id_idx/
    // overlay_seen/raw 三张查表均为**本 bar 终态**——立即式（push/物化当轮即修补）的时序孔（父在
    // 本轮更晚才物化进 raw，无论经 held 腿占位路径、restore 路径、或 open 候选/`Closed|Invalidated`
    // 边界根直接 push 路径，此刻都查不到）不再存在：父只要本轮曾被任一路径物化（不论早于/晚于该
    // 待修补元素被处理），统一 fixup 都能经三级解析命中。父确实从未物化（真断链
    // `restore_break_registry_lost` 或该 bar 内父从未被任何路径 push）时，保持 None/None 不伪造，
    // 随后被下方 AncOK 剪除，命中计入 `placeholder_parent_unresolved` probe。
    // ★#347 MED-1：收集本 bar 未解析的元素 idx（`rebuild_placeholder_parent_attached` 返回
    // false），随后与 `next_idx`（AncOK 存活集）交叉核对——不在 `next_idx` 即被剪除，命中计入
    // `placeholder_pruned_by_ancok` probe（使函数头声明"可与 AncOK 剪除计数交叉核对"可执行）。
    let unresolved_pending_fixup =
        resolve_pending_parent_fixups(&mut work, &pending_parent_fixup, &id_idx, &overlay_seen, &raw);

    // 步2：A_{t+1}=AncOK(A^raw)——剔除真 Compose 父容器不在 raw 的孤儿子腿（§13 持仓准入：未持父则剔除）。
    // ★codex Q4：按 parent_id 结构映射闭包（spec §13 `p:C_ℓ→C_{ℓ+1}`），非 per-bar 索引链。
    let next_idx = ancestor_close_by_id(&work, &raw);

    if !unresolved_pending_fixup.is_empty() {
        let next_idx_set: std::collections::HashSet<usize> = next_idx.iter().copied().collect();
        let pruned = unresolved_pending_fixup.iter().filter(|&&idx| !next_idx_set.contains(&idx)).count();
        if pruned > 0 {
            ancok_probe_bump(|p| p.placeholder_pruned_by_ancok += pruned as u64);
        }
    }

    // p̃=Σ Leg(g)（depth 权重沿真父链 + 方向净额聚合，ShortDiff 空腿部分对冲父多腿）。
    let mut legs = strategy_target_legs(&work, &next_idx, base_units, config);
    // f3 反事实（config.disable_shortdiff）：剔 ShortDiff 腿的净头寸贡献，测多重赋格对冲增量。default false→bit-exact。
    if config.disable_shortdiff {
        legs.retain(|l| l.role.v != Vertical::ShortDiff);
    }
    // ★G7 毛头寸约束（#122 终裁 + decide 5b46）：净额折叠**之前**施加（净标量已丢失毛敞口信息，
    // 事后诊断门被拒）。enforce_gross_cap=false（default）/risk=None ⟹ 不激活（frozen bit-exact）。
    // gross_zeroed = 被整体零化（c_r=0/Ḡ=0）的腿 e_idx（幽灵腿防护消费，见下）。
    let mut gross_zeroed: Vec<usize> = Vec::new();
    if let Some(r) = risk {
        if r.enforce_gross_cap {
            gross_zeroed = apply_gross_cap(&work, &mut legs, base_units, r);
        }
    }
    let p_tilde = net_target_units(&legs);

    // A_{t+1} 回 ActiveLeg（638 身份，喂下一 bar interpret 闭环 + 跨 bar 对位）。
    // ★幽灵腿防护（裁定4，#124 force_flat 同款模式禁止复制）：被毛约束**整体零化**（c_r=0/Ḡ=0）
    // 的**开仓**腿不入 next_active——目标为 0 的开仓从未建仓，照常延续 = 从未建仓的幽灵腿跨 bar
    // 传播（腿账本与订单效果必须一致）。部分缩放（c∈(0,1)）真实开仓（缩小目标流入 p̃→订单），
    // 账本只携身份不携 units ⟹ 一致，保留。held 腿零化时保留身份：其减仓/平仓经净订单真实兑现，
    // 账本身份的关闭归 typed close/𝒟_x 路径（#124 范围，本路径不越界删 held）。
    // gross_zeroed 恒空（default 不激活/未触发/仅部分缩放）⟹ 走原路径，逐位不变。
    let next_active: Vec<ActiveLeg> = if gross_zeroed.is_empty() {
        next_idx.iter().map(|&i| element_as_leg(&work[i])).collect()
    } else {
        let open_idx: std::collections::HashSet<usize> =
            buckets.open.iter().map(|c| candidate_start + c.gamma_index).collect();
        next_idx
            .iter()
            .filter(|&&i| !(open_idx.contains(&i) && gross_zeroed.contains(&i)))
            .map(|&i| element_as_leg(&work[i]))
            .collect()
    };
    // ★(I-1) 双计守卫（codex 异质审查）：next_active 每 ElementId 必唯一——同 carrier 不得在 raw 中以
    // 两个 idx（树前缀 + registry 追加）出现，否则 strategy_target_legs 双计 ⟹ p̃ 伪证。
    // restore_ancestor_chain_from_registry 已复用现有 idx 保证唯一；此 assert 锁不变量防回归。
    debug_assert!(
        {
            let mut ids: Vec<_> = next_active.iter().map(|l| l.id).collect();
            ids.sort_by_key(|id| (id.level, id.ordinal));
            ids.windows(2).all(|w| w[0] != w[1])
        },
        "next_active 含重复 ElementId ⟹ strategy_target_legs 双计 p̃（restore 未复用现有 idx）"
    );
    // ★M5 sep 暴露（多空对冲.pdf p16）：把已算 `legs`（post G7 cap）按 work-index e_idx 对位到
    // carrier ElementId + role(v) + parent(v)，打包 SepLeg。**只读重打包，不新计算**——
    // `net_target_units(&legs)==p_tilde` 恒等 ⟹ `Σ σ_v·q_units == Net(P^sep)` 与净额路径一致。
    // 幽灵腿（gross_zeroed 开仓腿）units 已被零化 ⟹ 对 P^sep 贡献 0，与 next_active 剔除一致。
    let sep_legs: Vec<SepLeg> = legs
        .iter()
        .filter_map(|leg| {
            work.get(leg.e_idx).map(|e| SepLeg {
                id: e.id,
                side: leg.side,
                q_units: leg.units,
                role_v: leg.role.v,
                parent_id: e.parent_id,
            })
        })
        .collect();
    (next_active, p_tilde, sep_legs)
}

/// 环5+环6 端到端（`Classification` + per-bar 因果塔 → Γ → 三桶 → `A_{t+1}=AncOK[...∪RegistryRestore]` → `p̃`），
/// element-coverage 生产入口（**σ_p 来源 + §13 AncOK 持仓准入双机制就位**）。
///
/// 串接：
/// 1. **真父子组合元素** `(elements, candidate_start)`=[`interp::coverage_elements_with_tower`]
///    （tree ++ 638 附着候选；候选携真 `parent`=Compose 父、`attached_dir`=σ_p=**父容器方向**，639）。
/// 2. **环5**：`gamma`=[`interp::assemble_gamma_with_tower`]（角色 V 由真父派生：FollowParent/ShortDiff/
///    Ambient）+ [`interp::interpret`] ℛ_Θ 唯一化三桶（含反向关闭 𝒟_x，喂 `prev_active`）。
/// 3. **环6**：[`coverage_step_from_buckets`] 活动集递归 + §13 AncOK 持仓准入（未持父则剔除 ShortDiff）。
///
/// **★两机制正交（639）**：① **σ_p 来源**（§7.2，`assemble_gamma_with_tower` 经 `attach_bsp_to_tree`
/// 从 per-bar 因果塔查父容器方向，**与持仓无关**）；② **持仓准入**（§13 AncOK，`coverage_step_from_buckets`
/// 经 `prev_active` 对位真树元素判父容器腿是否持有）。`prev_active` **进** interpret（𝒟_x 反向关闭）
/// **与** AncOK 准入（父容器腿在场判据），**不进** σ_p 计算（错口径"活动父腿"已删，639）。
///
/// **★执行层因果塔**：调用方（runner）须喂 **per-bar 前缀因果塔**（`classify_with_tower(l0[0..=t])`，
/// 只用 ≤t 数据 → 因果）；全窗塔非因果，执行层禁用（639）。有向父容器 ⟹ V 真出 FollowParent/ShortDiff；
/// 父=胚元∂（缺塔/host 是根）⟹ σ_p=0 ⟹ Ambient。
///
/// **不变量契约**：[`interp::coverage_elements_with_tower`] 与 [`interp::assemble_gamma_with_tower`]
/// 在同一 `(classification, tower)` 上**确定重建同一树**（同 `levels×bsp` 序），故候选 `gamma_index`
/// 与 `elements` 候选段偏移 1:1 对齐（`elements[candidate_start + gamma_index]` 即该候选元素）。
///
/// **边界条件**：`tower.len()<2`（仅 L0，无 Compose 父）⟹ 所有候选 σ_p=0 ⟹ 全 Ambient 根 ⟹ AncOK
/// 恒等准入（与扁平 [`interp::assemble_gamma`] 一致，tower-export-i 边界）。空 `levels`/空 bsp ⟹ 空 Γ。
/// **认识论 L1**（管线正确性，非 L2 alpha）。
pub fn coverage_step_classification(
    classification: &Classification,
    tower: &[Rc<Vec<LeveledMove>>],
    prev_active: &[ActiveLeg],
    base_units: f64,
    config: &VoiceConfig,
    risk: Option<&RiskConfig>,
    registry: &super::super::persistent::PersistentRegistry,
) -> (Vec<ActiveLeg>, f64) {
    // H2 优化：单次建树产 (elements, candidate_start) + gamma——消除旧版每 bar 双调
    // extract_elements(tower) 的冗余（coverage_elements_with_tower + assemble_gamma_with_tower
    // 各建树一次，第二次纯重复）。bit-exact：同 (classification, tower) 同一建树输出。
    let (tree, candidates, gamma) =
        interp::coverage_elements_and_gamma_with_tower(classification, tower);
    let work = ElementView::from_parts(&tree, candidates);
    coverage_step_prebuilt(work, &gamma, prev_active, base_units, config, risk, registry)
}

/// **工位 K 性能：环5+环6 用预建 `(elements, candidate_start, gamma)`**（消除 runner per-bar
/// 双调 `coverage_elements_and_gamma_with_tower`——一次 `pi_theta_step` + 一次 merge 的 elements，
/// 现共享同一预建产物）。bit-exact == [`coverage_step_classification`]：同 elements/gamma 同 interpret
/// 同 AncOK。**不改 AncOK 准入逻辑**（[`coverage_step_from_buckets`] 原样），仅消除重复建树。
pub(crate) fn coverage_step_prebuilt(
    work: ElementView,
    gamma: &[Candidate],
    prev_active: &[ActiveLeg],
    base_units: f64,
    config: &VoiceConfig,
    risk: Option<&RiskConfig>,
    registry: &super::super::persistent::PersistentRegistry,
) -> (Vec<ActiveLeg>, f64) {
    // 环5：解释器三桶（𝒟_x 反向关闭喂 prev_active）。
    let buckets = interp::interpret(gamma, prev_active);
    // 环6：A_{t+1}=AncOK[(A_t∖𝒟_x)∪ℬ_x∪RegistryRestore] + p̃（§13 持仓准入：ShortDiff 未持父则剔除，639(c)；
    //      RegistryRestore=registry 持久祖先物化第三来源，票#247）
    //      + G7 毛头寸约束（legs 折叠前，risk.enforce_gross_cap 门控）。
    coverage_step_from_buckets(work, prev_active, &buckets, base_units, config, risk, registry)
}

#[cfg(test)]
#[path = "step_tests.rs"]
mod tests;
