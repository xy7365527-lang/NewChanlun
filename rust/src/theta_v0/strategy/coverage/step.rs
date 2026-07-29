use super::*;

/// [`coverage_step_from_buckets`] 的净额兼容出口（22 处旧调用点保持二元返回，bit-exact）。
/// 委托 [`coverage_step_from_buckets_sep`] 丢弃第三分量 `sep_legs`（M5 声部执行层暴露，纯只读，
/// 不进决策路径）与第四分量 `next_active_idx`（#220 opened 配对键透出，仅 `pi_theta_step_traced`
/// 主路径消费）——单源无平行状态机。
pub(super) fn coverage_step_from_buckets(
    work: ElementView,
    prev_active: &[ActiveLeg],
    buckets: &Buckets,
    base_units: f64,
    config: &VoiceConfig,
    risk: Option<&RiskConfig>,
    registry: &super::super::persistent::PersistentRegistry,
) -> (Vec<ActiveLeg>, f64) {
    let (next_active, p_tilde, _sep, _idx) = coverage_step_from_buckets_sep(
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
///
/// 第 4 返回分量 `next_active_idx`（★#220 路④）：与 `next_active` 逐位对位的 work idx——
/// opened 外化的配对键（候选自身元素 idx ∈ next_active_idx ⟺ 信号真实准入；按 id 配对会把
/// 同 id 的 restore 在场腿误配给被 #216 规则①/③ 让位/湮灭的候选，一腿双登记 ⟹ 账面孤儿，
/// 勘察报告 assertion2-restore-balance-scope-20260724「炸点实证」）。gross 零化剔除在 idx 层
/// 完成（与 next_active 同一 filter），纯透出、决策路径不变。
///
/// ★#247 契约指针（**本 doc 不含 C1/C2/C3 正文**，别在这里找）：第三来源 `ℛ_x`、C1 环路硬门、
/// C3 三条声明见 [`restore_ancestor_chain_from_registry`] doc；C2 见 [`held_stale_reregister_idx`] doc。
pub(super) fn coverage_step_from_buckets_sep(
    work: ElementView,
    prev_active: &[ActiveLeg],
    buckets: &Buckets,
    base_units: f64,
    config: &VoiceConfig,
    risk: Option<&RiskConfig>,
    registry: &super::super::persistent::PersistentRegistry,
) -> (Vec<ActiveLeg>, f64, Vec<SepLeg>, Vec<usize>) {
    coverage_step_from_buckets_sep_with_risk_seeds(
        work, prev_active, buckets, &[], base_units, config, risk, registry,
    )
}

/// 父 campaign 入场坐标 → 当前结构树 carrier（#572 / D1 身份重建）。
///
/// 判据结构与 strategy 层既有 `carrier_of_entry` 一致（先同级严格右端点命中；若 carrier
/// 入场后发生 ρ 延伸，则只接受唯一的左开右闭 span `λ < signal_index <= ρ`），**但并非同
/// 口径**——`carrier_of_entry` 的严格右端点阶段用 `find` 取首个命中，不检测该阶段的多重
/// 命中；本函数在严格右端点阶段同样以 filter+双 next 检测歧义，两阶段都 fail-closed。
/// risk-close 场景比 D1 附着一致判据更严：宁可漏补 seed（父仍照常 RiskExit，只是后代不
/// 被子树连坐清除，残留裸腿——见 [`super::ancok::AncokProbe::risk_seed_carrier_ambiguous`]
/// 计数），也绝不按方向或区间宽度猜测，以免把无关同向腿误注入 risk-close 子树。
fn risk_seed_carrier(tree: &[CoverageElement], seed: &ActiveLeg) -> Option<ActiveLeg> {
    let mut exact = tree
        .iter()
        .filter(|e| e.level == seed.level && e.rho == seed.source_index);
    match (exact.next(), exact.next()) {
        (Some(e), None) => return Some(element_as_leg(e)),
        (Some(_), Some(_)) => {
            ancok_probe_bump(|p| p.risk_seed_carrier_ambiguous += 1);
            return None;
        }
        (None, _) => {}
    }

    let mut spans = tree.iter().filter(|e| {
        e.level == seed.level
            && e.lambda < seed.source_index
            && seed.source_index <= e.rho
    });
    match (spans.next(), spans.next()) {
        (Some(e), None) => Some(element_as_leg(e)),
        (Some(_), Some(_)) => {
            ancok_probe_bump(|p| p.risk_seed_carrier_ambiguous += 1);
            None
        }
        (None, _) => None,
    }
}

/// #572：在既有三桶活动集推进中额外注入逐腿 risk-close seeds。
///
/// seeds 只扩展现役 `𝒟_x` 子树闭包；KΘ 净额夹紧仍由调用方的 `KThetaRiskGate`
/// 独立承担仓位投影。本函数不新增退出管线，父/后代仍由同一个
/// `step_active_set_with_subtree_close` 裁决。
#[allow(clippy::too_many_arguments)]
pub(super) fn coverage_step_from_buckets_sep_with_risk_seeds(
    mut work: ElementView,
    prev_active: &[ActiveLeg],
    buckets: &Buckets,
    risk_close_seeds: &[ActiveLeg],
    base_units: f64,
    config: &VoiceConfig,
    risk: Option<&RiskConfig>,
    registry: &super::super::persistent::PersistentRegistry,
) -> (Vec<ActiveLeg>, f64, Vec<SepLeg>, Vec<usize>) {
    // ★热点② O(n²) 消除：work = ElementView{base=持久树前缀借用零拷贝, overlay=本 bar candidate 段}。
    // 旧 `elements.to_vec()` + 上游 `tree.clone()` 每 bar O(tree)×n=O(n²) 双双消除（base 借 Rc 树，
    // candidate 在 overlay）。restore 路径继续 push overlay 尾（绝大多数 bar 不触发）。
    // candidate_start = base 段长（candidate 从此起，在 overlay）。
    let candidate_start = work.base_len();
    let mut raw: Vec<usize> = Vec::new();

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
    // ★#216：候选段终点（= 进入 held 循环前的 work.len()）。此后 work 只被 restore/held push 增长，
    // 故 overlay_seen 命中 idx ≥ overlay_cand_end ⟺ 本 bar restore push（可复用）；< 之 ⟺ 候选段
    // 拷贝（held 重注册**不得**复用——候选 eps 是信号方向、lambda==rho 点元素、parent_id 非本腿
    // op_parent，复用 = 持仓身份被候选属性覆盖，code-review Spec 轴 (c)1）。
    let overlay_cand_end = work.len();

    // (A_t ∖ 𝒟_x)：持仓腿（除 close 认领）按 ElementId 对位回当前因果树元素；不在树 ⟹ 按
    // is_boundary_root 决定 root/prune（发现 A 修复：Stale 不伪造 parent:None）。
    let mut direct_close_seeds = buckets.close.clone();
    for seed in risk_close_seeds {
        if !direct_close_seeds.iter().any(|leg| leg.id == seed.id) {
            direct_close_seeds.push(*seed);
        }
        // campaign 腿的 id 是候选元素 id；其后代的持久 `op_parent/parent_id` 指向承载该
        // 入场点的结构 carrier id。逐腿 stop 只命中有 LedgerOpen 的 campaign，因此按 D1
        // 从当前结构树确定性重建 carrier；身份有歧义则不补 seed（失败关闭，不误杀同向腿）。
        if let Some(carrier) = risk_seed_carrier(work.tree_prefix(tree_end), seed) {
            if !direct_close_seeds.iter().any(|leg| leg.id == carrier.id) {
                direct_close_seeds.push(carrier);
            }
        }
    }
    let closed = close_indices(prev_active, &direct_close_seeds);
    // ★#269 翻向种子（事件化，#261 终裁）：当 bar 载体翻向**结构事件**激活的持仓腿 = 旧世代
    // 终结（#227 裁决蓝图两步形①）。事件谓词 = registry 首见方向 ≠ 当前树元素方向（树段
    // upsert 方向冲突；σ/ε 出生对立非事件，不再产种子——#264 误杀面收口）。
    // 下方 step 调用并入关闭种子，经现成 𝒟_x^† 子树清仓连清其后代（**只作用 A_t 段**——
    // ℬ_x 段反手/新世代候选不连坐，#183 分段/ID-3 反手保护同构）。翻向腿自身与其连清
    // 后代不在 close 桶、不在 next_active ⟹ 自动落 StepTrace.silent_drops 入轨（runner
    // 既有消费链：typed ledger via_structural_prune + 账户镜像 StructuralPrune——close
    // 事件入轨不新造外化轨）。
    let mut flipped_seeds: Vec<ActiveLeg> = Vec::new();
    for (i, leg) in prev_active.iter().enumerate() {
        if closed.contains(&i) {
            continue; // 𝒟_x：本腿关闭，不入 A^raw
        }
        // tree_end 固定 + Stale 追加在 overlay（tree_end 之后）⟹ base 前缀内容不变；每轮重借（NLL）。
        let m = held_leg_tree_index_indexed(work.tree_prefix(tree_end), leg, &id_idx, registry);
        match m {
            // Exact（ID 命中=同一走势，父延伸也同 ID）：对位回当前树元素 idx（携真父链）⟹
            // 其子声部腿的 AncOK 祖先齐全。
            HeldLegMatch::Exact(idx) => {
                if !raw.contains(&idx) {
                    raw.push(idx);
                }
            }
            // ★#269 Flipped（载体翻向**事件**激活 ⟹ 翻向 = 声部终结）：旧世代腿**不保留**
            // ——不当 Exact 对位、不重注册、不 restore（事件在场时方向盲 ID 对位复活路径仍
            // 废除；I2：同一持久元素方向不变，「父翻向」只能解析为「父反手 = 父关闭 + 新父开」）。
            // 进翻向种子（子树连清）；新世代元素（树里的新方向同 id）若被 ℬ_x 候选挂为父，
            // 经 open 父注入 restore 的 id_idx 树复用在场（新世代声部重登记路径，蓝图两步形②）。
            HeldLegMatch::Flipped => {
                ancok_probe_bump(|p| p.held_flip_terminated += 1);
                flipped_seeds.push(*leg);
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
                        // 原注「理论不可达（Exact 未命中但 registry LivePresent = snapshot 不一致）」
                        // 在 overlay 候选段场景**可达**：id 仅现于候选段（snapshot_present=true 经候选
                        // upsert）而 base 树前缀无（id_idx 只覆盖 base ⟹ Exact miss）——#216 评审锁定
                        // 测试 held_leg_id_hits_candidate_copy_keeps_held_identity 实证。
                        // 按持久身份保留（I1），op_parent 驱动 AncOK。
                        // ★#216：重注册复用 restore push 现有 idx（见 [`held_stale_reregister_idx`]）。
                        let idx =
                            held_stale_reregister_idx(&mut work, &id_idx, &mut overlay_seen, overlay_cand_end, leg);
                        if !raw.contains(&idx) {
                            raw.push(idx);
                        }
                    }
                    super::super::persistent::HeldLegState::LiveDetached => {
                        ancok_probe_bump(|p| p.state_live_detached += 1);
                        // ★anc.pdf §10 核心修复：LiveDetached 不 prune，不伪造 root。
                        // parent 仍是 op_parent(L)（§15），只是当前 snapshot 没展示。
                        // op_parent 在 persistent registry 中 live（I4）→ AncOK 通过（I5）。
                        // ★I5 + §11 归纳：从 registry 递归恢复整条操作祖先链（op_parent 及其祖先），
                        // 全部加入 work/raw，使生产 AncOK（#183 归一后 = exit::step_active_set_with_subtree_close）
                        // 通过（§11：每条未关闭腿的操作父 live ⟹ 所有 depth<d 腿通过 persistent AncOK）。
                        if let Some(op_pid) = leg.op_parent {
                            // ★#226：held 路传空种子表——A_t 段由 𝒟_x^† 种子命中兜底（restore
                            // 复活的被关父同清，`t4_subtree_close_unifies_production_active_set_step`
                            // 锁定），语义/轨迹 bit-exact 不变。
                            restore_ancestor_chain_from_registry(
                                &mut work, &mut raw, registry, op_pid, &id_idx, &mut overlay_seen, &[],
                            );
                        }
                        // ★#216：重注册复用 restore push 现有 idx（见 [`held_stale_reregister_idx`]）。
                        let idx =
                            held_stale_reregister_idx(&mut work, &id_idx, &mut overlay_seen, overlay_cand_end, leg);
                        if !raw.contains(&idx) {
                            raw.push(idx);
                        }
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

    // ★#183 T4 归一分段点：raw[0..a_t_end] = **A_t 段**（held 循环产出：持仓腿对位/Stale 重注册/
    // LiveDetached restore/边界根保留），raw[a_t_end..] = **ℬ_x 段**（open 循环产出：开启候选 +
    // open 父注入 restore）。★#247 缺口一：两段各自内嵌**第三来源** `ℛ_x`（registry 恢复祖先，
    // 见函数 doc「转移的第三来源」）⟹ 实装转移 = `AncOK[(A_t∖𝒟_x^†)∪ℬ_x∪ℛ_x]`。
    // spec §13 字面 `A_{t+1}=AncOK[(A_t∖𝒟_x^†)∪ℬ_x]`——子树清仓 𝒟_x^† 的定义域
    // 是 A_t（持仓腿），ℬ_x 新开腿不经 𝒟_x^†（同 bar 同 carrier 先平后开的反手候选与种子同 id，
    // 误清会禁绝一类/二类反手——#200 基线 typed=2 实证，ID-3「允许当场反手」行为口径）。
    // ★#233/#269：翻向种子（held 循环 Flipped 分派产出）同只作用 A_t 段——ℬ_x 段的新世代候选
    // （父翻向同 bar 挂该父的开仓）不连坐，新世代父元素经 open 父注入 restore id_idx 树复用
    // 在场（蓝图两步形②新世代重登记）。
    let a_t_end = raw.len();

    // ∪ ℬ_x：开启候选 → 其 638 附着因果树元素索引（candidate_start + gamma_index）。
    //
    // ★工位 H 修复（级别容器.pdf §13）：开仓激活的位置节点身份 = carrier 容器 hostOf(g)（其 ElementId
    // 在候选元素 id 上携带，见 interp.rs `coverage_elements_and_gamma_with_tower`），**不是**买卖点叶子。
    // 故候选自身入 raw 即等价于「激活 carrier 上的位置节点」（PDF §14 简化 position instance）。子声部
    // 的 parent_id（= par_C(carrier)）由生产 AncOK（#183 归一后 = exit::step_active_set_with_subtree_close）
    // 检查是否在 raw（= 持仓父位置节点在 A_t）。
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
    // ★#216：本循环 push 的候选 id→(idx,eps)（同 bar 多候选同 carrier 判重/湮灭用，见下）。
    let mut open_pushed: std::collections::HashMap<ElementId, (usize, VoiceSide)> =
        std::collections::HashMap::new();
    for c in &buckets.open {
        let idx = candidate_start + c.gamma_index;
        // ★#216 restore 缺口根因修复（m6 p3fold 诊断 dump 实证，2026-07-24）：同 carrier ElementId
        // 可被多个 open 候选（同 bar 双信号 attach 同 carrier，候选段各携一份点元素拷贝）重复
        // 激活——由按 idx 判重改为按 **id** 判重，分三种碰撞形态（spec §13 元素集语义：同一
        // ElementId 在 A_t 唯一；否则 next_active 同 id 两槽 ⟹ strategy_target_legs 双计 p̃）：
        //   ① id 已在 raw（held/restore 任一前序路径）⟹ 持仓身份优先，跳过候选拷贝（持仓腿生命
        //      周期归 close/risk 路径，不由 open 净额静默对冲）；
        //   ② id 为本循环先序**同向**候选 ⟹ 首现序优先（与 build_tree_id_index/overlay_seen 的
        //      or_insert 首现序约定一致），跳过重复拷贝（同向双计 = 本守卫防的 p̃ 伪证）；
        //   ③ id 为本循环先序**反向**候选 ⟹ 成对湮灭：剔除先序拷贝且本候选不入（同 bar 反向双
        //      信号净敞口 0 = 幽灵腿防护裁定4 同款——净零目标从未建仓，不开仓、不占活动集槽位；
        //      与修复前净额路径 p̃ 贡献 ±q−q=0 一致，轨迹不翻）。
        //   多重集（同 id 候选 >2）：按 open 桶序**逐对**处理（同向判重留首、反向湮灭成对）——
        //      序敏感但确定性；生产轨迹实证仅两候选形态（m6 p3fold dump），更多重集未见（诚实声明）。
        if idx < work.len() {
            let (cid, cdir) = (work[idx].id, work[idx].eps);
            if let Some(&(pidx, pdir)) = open_pushed.get(&cid) {
                if pdir != cdir {
                    raw.retain(|&r| r != pidx);
                    open_pushed.remove(&cid);
                }
            } else {
                let id_in_raw =
                    raw.iter().any(|&r| work.get(r).map(|e| e.id == cid).unwrap_or(false));
                if !id_in_raw {
                    raw.push(idx);
                    open_pushed.insert(cid, (idx, cdir));
                }
            }
        }
        // ★(I-1) open 候选父注入（codex 异质审查行级坐实，642/644）：
        //
        // depth>0 子声部候选（ReverseOpen/FollowParent）的真 Compose 父 carrier（= par_C(carrier)，候选
        // 元素 parent_id 携带）几乎从不与子同 bar 作 open 候选，也几乎从不是 prev_active 持仓腿
        // （L2 诊断：sd_parent_held=0），但**在 persistent registry 中 LiveDetached 存活**
        // （sd_parent_registry_alive≈sd_total）。Stale 持仓腿路径（上方 LiveDetached 分支）已用
        // [`restore_ancestor_chain_from_registry`] 把 op_parent 祖先链注入 raw，但 open 候选路径
        // **从不触发**该恢复 ⟹ 父 carrier 不在 raw ⟹ 生产 AncOK（#183 归一后 =
        // exit::step_active_set_with_subtree_close）判子声部祖先不齐 ⟹ AncOK
        // 全剪 depth>0 子腿（accepted_cert_carrier=0）。
        //
        // 修复：把 Stale 路径的祖先链恢复机制**扩展到 open 路径**——对每个 open 候选的父 carrier
        // （work[idx].parent_id），若 registry live 且不在 raw，从 registry 递归恢复整条结构祖先链
        // （§11 归纳：每条子声部腿的操作父 live ⟹ depth<d 祖先全在 raw ⟹ AncOK 通过）。这让持仓父
        // carrier 的位置节点进入 A_t（§8 父声部 carrier 跨 bar 持有，§9 祖先闭合兑现），depth>0 子腿
        // 准入。**非** AncOK 加特例放行——父链真实注入后由统一 AncOK 判据正常判定（no-patch）。
        // ★#226：open 父注入传 `buckets.close` 种子表——ℬ_x 段不经 𝒟_x^†（下方「不对称记录」），
        // 当 bar 已被裁决终结的祖先不得经本路径复活（S3 子树清仓；m3 win9 断言①炸点根因：
        // 一类终结父 (1,35) 同 bar 经本路径复活成 Core{1} 残余 518.09）。种子不复活 ⟹ 候选父链
        // 断裂 ⟹ 统一 AncOK 正常剪除；反手候选自身元素不经 restore，ID-3 反手不受影响。
        if idx < work.len() {
            if let Some(parent_pid) = work[idx].parent_id {
                let parent_in_raw =
                    raw.iter().any(|&r| work.get(r).map(|e| e.id == parent_pid).unwrap_or(false));
                if !parent_in_raw && registry.registry_live(&parent_pid) {
                    restore_ancestor_chain_from_registry(
                        &mut work,
                        &mut raw,
                        registry,
                        parent_pid,
                        &id_idx,
                        &mut overlay_seen,
                        &direct_close_seeds,
                    );
                }
            }
        }
    }

    // 步2：A_{t+1}=AncOK(A^raw)——剔除真 Compose 父容器不在 raw 的孤儿子腿（§13 持仓准入：未持父则剔除）。
    // ★#183 T4 归一（#179 裁决：结构对应是硬要求，子树清仓接线进生产 π loop）：
    // 活动集一步更新 A_{t+1}=AncOK[(A^raw ∖ 𝒟_x^†)] 归一到镜像函数
    // [`super::super::exit::step_active_set_with_subtree_close`]——
    //   ① subtree_close 把关闭种子 𝒟_x 扩为子树闭包 𝒟_x^†（父终结 ⟹ 全部后代同刻清除，
    //      短差腿无豁免；restore 复活的被关父腿同样命中种子——归一前 restore 可复活被关父、
    //      子借父链 AncOK 准入，「父关则子关」被 restore 扩集击穿，见证测试
    //      `t4_subtree_close_unifies_production_active_set_step`）；
    //   ② 声部层 AncOK 闭合（判据与旧 `ancestor_close_by_id` 逐点等价：raw 内 ElementId 唯一
    //      （#216 三处注册路径闭合）+ `is_boundary_root ⟺ parent_id.is_none()`（element_as_leg
    //      同源构造）+ 链断裂（父 id 不在集）即剪，两实现同判）。
    // restore 注入的扩集语义**保留**（#179 裁决）：held/open 路径的 restore 注入照常（open 路
    // 遇当 bar 种子中断除外——下方「不对称记录」#226 收口），仅最终成员资格由镜像函数统一
    // 裁决——存活腿的祖先链恢复行为不变（守护测试
    // `t4_unify_preserves_restore_expansion_for_surviving_legs`）。
    // 分段（spec §13 公式形态）：A_t 段（held 循环产出）经 𝒟_x^† 子树清仓；ℬ_x 段（open 循环
    // 产出：候选 + open 父注入）作 opened 并入、不经 𝒟_x^†——反手候选与种子同 id 时按 id 误清
    // 会禁绝先平后开（#200 基线 typed=2 实证）。
    // ★不对称记录（code-review Spec 轴 (c)1 在案）**#226 收口更新**：ℬ_x 段的 open 父注入 restore
    // 原不经 𝒟_x^†——父本 bar 被裁决终结且同 bar 候选以其为 carrier 时，父可经 open 注入复活、
    // 候选 AncOK 准入；该面与 A_t 段「restore 不得复活被关父」不对称，曾作 #179 裁决保留面留
    // fog。**#226 实证其教义代价**（m3 win9 bar=17032：一类终结父 (1,35) 同 bar 经 open 父注入
    // 复活成 Core{1} 物理残余 518.0948228547287，断言①炸点）后**收口**：open 父注入 restore 遇
    // 当 bar 关闭种子即中断（`restore_break_closed_seed`），复活泄漏封死。保留面收窄为「反手
    // 候选**自身元素**与种子同 id 重开不经 𝒟_x^†」（ID-3「允许当场反手」，#200 基线）——过滤
    // 只作用 restore 祖先注入，候选推送零改（守护测试
    // `open_reverse_candidate_on_closed_carrier_unaffected`）。
    let a_t_legs: Vec<ActiveLeg> = raw[..a_t_end]
        .iter()
        .map(|&i| element_as_leg(&work[i]))
        .collect();
    let b_x_legs: Vec<ActiveLeg> = raw[a_t_end..]
        .iter()
        .map(|&i| element_as_leg(&work[i]))
        .collect();
    // ★#233/#269：翻向种子并入关闭种子（只作用 A_t 段——ℬ_x 段反手/新世代候选不连坐，见上方
    // flipped_seeds 注释）。翻向父不在 a_t_legs（旧世代不保留）时，其后代经 exit.rs
    // `subtree_close` 的「父不在 A 数济判据」连坐连清。
    let close_and_flip_seeds: Vec<ActiveLeg> = direct_close_seeds
        .iter()
        .chain(flipped_seeds.iter())
        .copied()
        .collect();
    let next_legs =
        super::super::exit::step_active_set_with_subtree_close(&a_t_legs, &close_and_flip_seeds, &b_x_legs);
    // ★#183 生产路径不变量见证（#148 验收2「镜像层测试升级为生产路径测试」）：每步活动集
    // 推进后 ∀v∈A, Anc(v)⊆A 恒成立（step L0 构造内蕴）。debug 构建逐 bar 核验——**debug
    // profile 下**全测试库的每一次生产推进都是本不变量的见证实例（release 构建编译消除，
    // 不作 release 见证声称）。
    debug_assert!(
        super::super::exit::anc_subset_of_active(&next_legs),
        "AncOK 活动集不变量破裂：∀v∈A, Anc(v)⊆A（#148 验收2，#183 生产路径逐 bar 见证）"
    );
    // ★#183 映射前提的 fail-closed 见证（code-review Standards 轴 judgement call 2 采纳）：
    // raw_id_idx 由 raw 直建，重复 id 会静默取后者（存量腿属性错配 opened 段索引）——
    // 「raw 内 ElementId 唯一」（#216 三处注册路径闭合）自此逐 bar 硬门核验，不依赖隐性前提。
    debug_assert!(
        {
            let mut ids: Vec<_> = raw.iter().map(|&i| work[i].id).collect();
            ids.sort_by_key(|id| (id.level, id.ordinal));
            ids.windows(2).all(|w| w[0] != w[1])
        },
        "raw 内 ElementId 唯一性破裂（#216 注册路径闭合失效）⟹ raw_id_idx 静默错配（#183）"
    );
    // 映射回 work 索引供 strategy_target_legs 消费。**id→idx 从 raw 直建**（raw 内 ElementId
    // 唯一，#216 三处注册路径闭合的不变量）：不走 id_idx/overlay_seen——那是「全 work 首个同 id
    // 元素」的首现序查找语义（restore 复用判定用），held 重注册在候选段后 push 自身元素时首现
    // 槽位是候选拷贝，按它映射会把持仓身份错指成候选属性（`held_leg_id_hits_candidate_copy_
    // keeps_held_identity` 实证）。next_legs ⊆ (A_t∖𝒟_x^†)∪ℬ_x 保 raw 序 ⟹ next_idx 序与旧
    // ancestor_close_by_id 输出一致。
    let raw_id_idx: std::collections::HashMap<ElementId, usize> =
        raw.iter().map(|&i| (work[i].id, i)).collect();
    let next_idx: Vec<usize> = next_legs
        .iter()
        .map(|l| {
            *raw_id_idx
                .get(&l.id)
                .expect("step_active_set_with_subtree_close 产出 ⊆ raw（raw 内 id 唯一，#216）")
        })
        .collect();

    // p̃=Σ Leg(g)（depth 权重沿真父链 + 方向净额聚合，ReverseOpen 空腿部分对冲父多腿）。
    let mut legs = strategy_target_legs(&work, &next_idx, base_units, config);
    // f3 反事实（config.disable_reverse_open）：剔 ReverseOpen 腿的净头寸贡献，测多重赋格对冲增量。default false→bit-exact。
    if config.disable_reverse_open {
        legs.retain(|l| l.role.v != Vertical::ReverseOpen);
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
    // ★#220 路④：先定 `next_active_idx`（gross 零化剔除在 idx 层做，与旧 filter 同判据）再 map
    // 成腿——idx 对位透出为第 4 返回分量，供 opened 外化按「候选自身元素真实准入」配对（id 配对
    // 会把同 id restore 在场腿误配给让位候选，一腿双登记）。重构本身 bit-exact。
    let next_active_idx: Vec<usize> = if gross_zeroed.is_empty() {
        next_idx.clone()
    } else {
        let open_idx: std::collections::HashSet<usize> =
            buckets.open.iter().map(|c| candidate_start + c.gamma_index).collect();
        next_idx
            .iter()
            .copied()
            .filter(|&i| !(open_idx.contains(&i) && gross_zeroed.contains(&i)))
            .collect()
    };
    let next_active: Vec<ActiveLeg> =
        next_active_idx.iter().map(|&i| element_as_leg(&work[i])).collect();
    // ★(I-1) 双计守卫（codex 异质审查）：next_active 每 ElementId 必唯一——同 carrier 不得在 raw 中以
    // 两个 idx（树前缀 + registry 追加）出现，否则 strategy_target_legs 双计 ⟹ p̃ 伪证。
    // 唯一性由三处注册路径闭合保证：restore_ancestor_chain_from_registry 复用现有 idx、held Stale
    // 重注册复用 overlay 现有 idx、open 候选按 id 判重（同向首现）/反向成对湮灭（#216）。
    // ★#446：`debug_assert!` 单独把关在 release 编译消除 ⟹ 违规静默——改为 release/debug 都计数
    // （`duplicate_active_id_violations`），debug 额外 fail-fast，供真实跑批逐窗硬断言。
    let mut active_id_seen: std::collections::HashMap<ElementId, usize> =
        std::collections::HashMap::new();
    let duplicate_active_id = next_active_idx.iter().find_map(|&idx| {
        let id = work[idx].id;
        active_id_seen.insert(id, idx).map(|prior_idx| (id, prior_idx, idx))
    });
    if let Some((id, first_idx, second_idx)) = duplicate_active_id {
        ancok_probe_bump(|p| p.duplicate_active_id_violations += 1);
        debug_assert!(
            false,
            "next_active 含重复 ElementId {id:?}（idx {first_idx} 与 idx {second_idx}）⟹ \
             strategy_target_legs 双计 p̃（活动集注册路径未按 id 判重，#216/#446）"
        );
    }
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
    (next_active, p_tilde, sep_legs, next_active_idx)
}

/// 环5+环6 端到端（`Classification` + per-bar 因果塔 → Γ → 三桶 → `A_{t+1}=AncOK[...]` → `p̃`），
/// element-coverage 生产入口（**σ_p 来源 + §13 AncOK 持仓准入双机制就位**）。
///
/// 串接：
/// 1. **真父子组合元素** `(elements, candidate_start)`=[`interp::coverage_elements_with_tower`]
///    （tree ++ 638 附着候选；候选携真 `parent`=Compose 父、`attached_dir`=σ_p=**父容器方向**，639）。
/// 2. **环5**：`gamma`=[`interp::assemble_gamma_with_tower`]（角色 V 由真父派生：FollowParent/ReverseOpen/
///    Ambient）+ [`interp::interpret`] ℛ_Θ 唯一化三桶（含反向关闭 𝒟_x，喂 `prev_active`）。
/// 3. **环6**：[`coverage_step_from_buckets`] 活动集递归 + §13 AncOK 持仓准入（未持父则剔除 ReverseOpen）。
///
/// **★两机制正交（639）**：① **σ_p 来源**（§7.2，`assemble_gamma_with_tower` 经 `attach_bsp_to_tree`
/// 从 per-bar 因果塔查父容器方向，**与持仓无关**）；② **持仓准入**（§13 AncOK，`coverage_step_from_buckets`
/// 经 `prev_active` 对位真树元素判父容器腿是否持有）。`prev_active` **进** interpret（𝒟_x 反向关闭）
/// **与** AncOK 准入（父容器腿在场判据），**不进** σ_p 计算（错口径"活动父腿"已删，639）。
///
/// **★执行层因果塔**：调用方（runner）须喂 **per-bar 前缀因果塔**（`classify_with_tower(l0[0..=t])`，
/// 只用 ≤t 数据 → 因果）；全窗塔非因果，执行层禁用（639）。有向父容器 ⟹ V 真出 FollowParent/ReverseOpen；
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
    // 环6：A_{t+1}=AncOK[(A_t∖𝒟_x)∪ℬ_x] + p̃（§13 持仓准入：ReverseOpen 未持父则剔除，639(c)）
    //      + G7 毛头寸约束（legs 折叠前，risk.enforce_gross_cap 门控）。
    coverage_step_from_buckets(work, prev_active, &buckets, base_units, config, risk, registry)
}

// ════════════════════════════════════════════════════════════════════════════
//  §9 环7：目标头寸 p̃_{t+1} → 全定义策略 π_Θ → 唯一订单 O_{t+1}
//        （spec §15 P12 line 717-756：𝒦_Θ / J_x / LexArgmin / Schedule_Θ；
//         定理 spec §16 P13 line 795：∀x ∃! O_{t+1}=π_Θ(x)，七链 **环7** rust 兑现）
//
//  π_Θ(x) = Schedule_Θ[ LexArgmin_{p∈𝒦_Θ(x)} J_x(p) − p_t ]   （spec line 756 方框）
// ════════════════════════════════════════════════════════════════════════════


#[cfg(test)]
#[path = "step_tests.rs"]
mod tests;
