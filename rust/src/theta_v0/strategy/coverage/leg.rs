use super::*;

// ════════════════════════════════════════════════════════════════════════════
//  §5 LegTarget 每活动元素一腿（M17/M28，方向 ε_e + 单位 s_e 按 role/depth 权重）
// ════════════════════════════════════════════════════════════════════════════

/// ★M5 声部执行层暴露的**逐声部目标头寸**（多空对冲.pdf p16 关卡10 `P^sep_t=Σ_{v∈A_t}q_vσ_v e_v`）。
///
/// [`LegTarget`] 用 work-index `e_idx`（本 bar 树内偏移，跨 bar 不稳定）作声部键，无法建**持久**
/// 逐声部账本（P^sep 需跨 bar 追踪 entry_v/exit_v/pnl_v）。`SepLeg` 把腿的**跨 bar 稳定身份**
/// [`ElementId`]（carrier v）+ 方向 σ_v + 目标单位 q_v + 角色 role(v) + 父 parent(v) 一并暴露，
/// 供 runner 的 [`OverlayState`](crate::theta_v0::strategy::overlay_state) hedge-mode 簿消费。
///
/// ★646号范畴（命名区分）：`q_units` 是 sizing 层**资本加权连续目标敞口**（`base_units×w_depth×w_dir`，
/// 含 dir_weight；post gross-cap，f64）——与 §9 voice 层整数手数 `q_v` 是不同投影空间的量。数据源 =
/// `LegTarget.units`（本文件 :2321 透传，`LegTarget.units=base_units×w_depth×w_dir` 已含 w_dir，见
/// :1411 注释）。overlay 簿取整为整数 q_v（手数）时在 [`OverlayState`] 内做（lot 对齐），本结构如实
/// 透传 sizing 连续量，不臆造整数。
///
/// ★认识论 L1：本结构是 [`coverage_step_from_buckets`] 已算 `legs`+`next_active` 的**只读暴露**
/// （同一 `next_idx` 元素，携 ElementId），非新计算——决策路径 bit-exact 不变。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SepLeg {
    /// carrier v 的跨 bar 稳定身份（= [`ActiveLeg::id`]，持久声部键）。
    pub id: ElementId,
    /// σ_v：声部方向（Long/Short；Flat 不入活动集）。
    pub side: VoiceSide,
    /// q_v：sizing 层目标单位（`base_units×w_depth`，f64；overlay 簿取整为手数）。
    pub q_units: f64,
    /// role(v) 垂直轴（`ReverseOpen`=反向子声部对冲腿，PDF §8 overlay 头寸 H_t 载体）。
    pub role_v: Vertical,
    /// parent(v)：真 Compose 父容器身份（None=边界胚元∂根声部）。
    pub parent_id: Option<ElementId>,
}

/// 单元素的目标头寸腿 `LegTarget(e)`（M17/M28 `TargetLeg` 的 rust 镜像）。
///
/// 每个活动元素一腿（对齐 `SeparateStrategyTarget.legTarget`）：
/// - `e_idx`：腿归属的元素索引（声部坐标 ν(e) 的 rust 表示）。
/// - `side`：腿方向 = ε_e（M17 `σ_{ν(e)}=ε_e`：多腿 Long / 空腿 Short）。
/// - `units`：目标单位数 s_e（按 role/depth 权重 `w_depth` × 基准，M28 深度权重）。
/// - `role`：元素角色 R(g)=(H,V,δ)（`role.v==ReverseOpen` 标记反向子声部腿，净额执行时与父对冲）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LegTarget {
    /// ν(e)：腿归属的元素索引。
    pub e_idx: usize,
    /// ε_e：腿方向（Long=多腿 / Short=空腿）。
    pub side: VoiceSide,
    /// s_e：目标单位数（role/depth/σ_higher 权重）。
    ///
    /// ★646号 rider（命名区分，不改语义）：`LegTarget.units` 是 **sizing 层资本加权目标敞口**
    /// （`base_units × w_depth × w_dir`，f64 连续量），**不是** §9 voice 层的单位计数 `q_v`（手数，
    /// 整数）——二者是不同投影空间的量（646号裁决 CONFIRM：范畴错误，depth_weight 不等权应保留）。
    pub units: f64,
    /// R(g)=(H,V,δ)：元素 24 类角色（`role.v==Vertical::ReverseOpen`=反向子声部腿）。
    pub role: OperationRole,
}

/// 生成单元素的目标头寸腿 `LegTarget(e)`（M17/M28，方向 ε_e + role/depth 权重单位数）。
///
/// 单位数 `s_e` = `base_units × w_depth(depth) × w_dir(ℓ,δ,σ_higher,role)`（v1，prereg-rev4 §1.2）。
/// `w_depth`（M28 深度资金权重，对齐 `voice::depth_weight`：w=[0.60,0.30,0.10]）消费 SizeΘ 第 4 参数
/// N^depth；`w_dir`（见 [`dir_weight`]）消费第 5 参数 σ_higher（父级方向，经 role 隐式解码）。`depth` =
/// 元素在嵌套树的深度（根=0，沿 parent 链长度）——**真嵌套深度**（铁律：来自 parent 链，非级别差）。
/// 腿方向直接取 `e.eps`（M17 `σ_{ν(e)}=ε_e` 定义性满足）。default config `ThetaDirPreset::Neutral`
/// 下 `w_dir≡1.0` ⟹ bit-exact == v0（prereg §6.2）。
///
/// ★ReverseOpen 反向子声部腿（M11/M28，hedge-subvoice 提案 P1-P4）：短差元素的腿方向 = ε_e = δ_g =
/// −σ_{p(g)}（父级方向翻转，`operation_role` 已判 `role.v==Vertical::ReverseOpen`，spec §9）——父声部
/// 不动，本腿作独立反向子声部（净额执行 [`net_target_units`] 时部分对冲父仓）。本函数只产腿，对冲
/// 在净额合并实现。
/// w_dir：q_Θ v1 σ_higher 分级符号权重（prereg-rev4 §4.1 冻结形式）。
///
/// σ_higher = σ_{α(e)}（父级方向）从 [`OperationRole`] 隐式解码（prereg §3.3，不需新数据源）：
/// - `FollowParent`：δ_e = σ_higher ⟹ σ_higher = `dir_sign(delta)`（+δ_e）
/// - `ReverseOpen`：δ_e = −σ_higher ⟹ σ_higher = −`dir_sign(delta)`（−δ_e；CASE 1 先返回，理论不到 CASE 3）
/// - `Ambient`：σ_higher = 0（父容器无方向，去根化）
///
/// 三 CASE（优先序，prereg §4.1）：
/// 1. **ReverseOpen 豁免**（§7.5 `s_g=s_α` 同股数要求，定理 1）⟹ `1.0`（反父方向 = GPT AgainstParent 商映射，
///    含同级别+次级别反父整体豁免——G 轴不进 dir_weight）
/// 2. **根级/无上级方向**（σ_higher=0，Ambient）⟹ `1.0`（无 σ_higher 可消费）
/// 3. **分级查表** otherwise ⟹ `Θ_dir[ℓ][sign(δ_e · σ_higher)]`
///
/// `sign(δ·σ_higher)`：+1=顺上级（FollowParent 同向子腿），−1=逆上级。CASE 3 仅 `FollowParent` 可达
/// ⟹ sign 恒 +1。**sign=−1 槽经 V 路径不可达**（GPT 命名冲突裁决 §六：eta_adv 冻结依赖 V::SameReverse
/// 作 canonical state 的前提失效——V 回三分类后 ReverseOpen 整体豁免，η_adv 经 V 路径不可达（Follow
/// sign=−1 槽无消费方），但 η_same 经 V 路径**可达**（Adversary FollowParent sign=+1 槽）；
/// G 轴细化不进 dir_weight，Follow/Adversary 的 alpha 解释须 q_Θ^full 消费 G 后重跑）。
///
/// ★bit-exact：[`ThetaDirPreset::Neutral`] 下所有 CASE 返 1.0 ⟹ w_dir≡1 ⟹ `leg_target` 输出
/// == v0（prereg §6.2 唯一 bit-exact 保留情形，g2 实装自检基线）。
pub fn dir_weight(role: &OperationRole, depth: u32, config: &VoiceConfig) -> f64 {
    // CASE 1: ReverseOpen 豁免（买卖点.pdf §7.5 s_g=s_α，prereg §5.1 定理 1）——优先于一切预注册套。
    // ReverseOpen = δ_g=−σ_p（商映射 AgainstParent，含同级别+次级别反父），整体豁免——G 轴不进 dir_weight。
    if role.v == Vertical::ReverseOpen {
        return 1.0;
    }
    // σ_higher 从 role.v + δ_e 解码（prereg §3.3）。ReverseOpen 已被 CASE 1 返回，此处仅 Ambient/FollowParent 可达。
    let sigma_higher: i8 = match role.v {
        Vertical::FollowParent => dir_sign(role.delta),
        Vertical::Ambient => 0,
        Vertical::ReverseOpen => -dir_sign(role.delta), // 理论不可达（CASE 1 已返），列全保穷尽
    };
    // CASE 2: 无上级方向可消费（σ_higher ∈ {None,0}，RootDir/Ambient）。
    if sigma_higher == 0 {
        return 1.0;
    }
    // CASE 3: 分级符号查表 Θ_dir[ℓ][sign(δ_e · σ_higher)]。
    let sign = dir_sign(role.delta) * sigma_higher; // ∈ {+1,-1}
    theta_dir_slot(&config.theta_dir, depth, sign)
}

/// w_grade：G 轴（grade_rel）sizing 权重（prereg-wg 推荐 (c) 因子化，GPT §九 `w_{ℓ,σ_higher,role,G,...}`）。
///
/// G 轴进 sizing 权重 w 的下标，**不进 μ 桶键**（prereg-wg §1，696 同构：轴进 A 层 ≠ 进 B 层）。
/// 消费 [`OperationRole::grade`]（[`GradeRel`] {SameLevel, SubLevel}），返回 `config.w_grade[grade_index]`。
///
/// ★bit-exact：`w_grade=[1.0,1.0]`（[`VoiceConfig`] default）⟹ 恒 1.0 ⟹ [`leg_target`] 输出 == v0。
///
/// C4 实质担忧的形式化：同级别反父（SameLevel AgainstParent）vs 次级别反父（SubLevel ReverseOpen）
/// 在 sizing 区分对待——两者 V 轴同归 ReverseOpen（[`dir_weight`] CASE 1 整体豁免），G 轴独立区分。
/// 与 `theta_dir`（V 轴 σ_higher）正交：G 轴独立消费，不绑 Follow/Adversary preset。
pub fn w_grade(role: &OperationRole, config: &VoiceConfig) -> f64 {
    let idx = match role.grade {
        GradeRel::SameLevel => 0,
        GradeRel::SubLevel => 1,
    };
    config.w_grade[idx]
}

/// 三套预注册的 (ℓ, sign) 槽读出（prereg §4.3）。
///
/// `same`=sign=+1（顺上级）槽，`opp`=sign=−1（逆上级）槽。越界 level 视 1.0（与 `depth_weight`
/// 越界返 0 的语义不同：w_dir 越界不应意外零化腿单位，保权更安全）。
pub(crate) fn theta_dir_slot(preset: &ThetaDirPreset, depth: u32, sign: i8) -> f64 {
    let eta = |v: &[f64]| v.get(depth as usize).copied().unwrap_or(1.0);
    let slot = |same: f64, opp: f64| if sign > 0 { same } else { opp };
    match preset {
        ThetaDirPreset::Neutral => 1.0,
        // 顺上级保权 1.0，逆上级降权 η_adv。
        ThetaDirPreset::Follow { eta_adv } => slot(1.0, eta(eta_adv)),
        // 逆上级保权 1.0，顺上级降权 η_same。
        ThetaDirPreset::Adversary { eta_same } => slot(eta(eta_same), 1.0),
    }
}

pub fn leg_target(
    elements: &[CoverageElement],
    e_idx: usize,
    base_units: f64,
    config: &VoiceConfig,
) -> LegTarget {
    let e = &elements[e_idx];
    // element_depth 现接 ElementView（双段）；非 indexed 简单版包 base-only view（overlay 空，零拷贝）。
    let depth = element_depth(&ElementView::new(elements), e_idx);
    let role = operation_role(elements, e_idx);
    // v1（prereg-rev4）+ w_grade（prereg-wg (c)）：s_e = base_units × w_depth × w_dir × w_grade[grade]。
    let w = depth_weight(depth, config) * dir_weight(&role, depth, config) * w_grade(&role, config);
    LegTarget {
        e_idx,
        side: e.eps,
        units: base_units * w,
        role,
    }
}

/// ★工位 4d：双段索引的 leg_target 变体——base 兄弟（缓存 tree-only）+ overlay 兄弟（本 bar candidate/
/// restore 段），热循环 strategy_target_legs 单次建 overlay 索引、多次查。
/// bit-exact == [`leg_target`]（role 经 [`operation_role_two_segment`] 双段 partition_point 同逻辑）。
pub(crate) fn leg_target_two_segment(
    elements: &ElementView,
    e_idx: usize,
    base_units: f64,
    config: &VoiceConfig,
    base_sibling: &std::collections::HashMap<(Option<usize>, u32), Vec<usize>>,
    overlay_sibling: &std::collections::HashMap<(Option<usize>, u32), Vec<usize>>,
) -> LegTarget {
    let e = &elements[e_idx];
    let depth = element_depth(elements, e_idx);
    let role = operation_role_two_segment(elements, e_idx, base_sibling, overlay_sibling);
    // v1（prereg-rev4）+ w_grade（prereg-wg (c)）：与 [`leg_target`] 同步——w_depth × w_dir × w_grade，保 bit-exact == leg_target。
    let w = depth_weight(depth, config) * dir_weight(&role, depth, config) * w_grade(&role, config);
    LegTarget {
        e_idx,
        side: e.eps,
        units: base_units * w,
        role,
    }
}

/// 元素的真嵌套深度（沿 parent 链长度，根=0；铁律：真父子，非级别差）。
/// parent（usize 索引）指向 base 段 carrier（< candidate_start ≤ base.len）；★#247 起 restore 恢复
/// 元素的 parent 亦可指向 overlay 段（同一 walk 恢复的更高祖先）。
///
/// ## ★#247 C1（影子评审阻断级）：环路 fuel 硬门
///
/// **订正旧表述**：原注写「链仍无环（registry `structural_parent_id` 严格上溯，walk 遇重复即止）」
/// —— 这是**错的**。「walk 遇重复即止」（[`restore_ancestor_chain_from_registry`] 的
/// `already_in_raw` / `overlay_seen` 复用分支）证明的是 **walk 自身终止**，**不**证明回填后的
/// `parent` 图无环。反例（registry 中 A.parent=B、B.parent=A）：walk(A) push A(0)、push B(1)、
/// 再遇 A 已在 raw ⟹ 正常 break；回填 A.parent=Some(1)、B.parent=Some(0) ⟹ **环成立**，本函数
/// 裸 `while let` 无限循环 ⟹ 生产（回测/实盘）挂死。
///
/// 无环性**不能白拿**：`persistent.rs` 三处 `structural_parent_id` 赋值均为直接写入、其中一处是
/// **跨 bar 可变更新**（同一持久元素的父可从 `parent_a` 改成 `parent_b`），写入侧无「父 level >
/// 子 level」校验、无环检测。
///
/// 硬门：上溯步数 `> elements.len()` ⟹ parent 图**必含环**（`elements` 上的简单路径最长
/// `len−1` 条边，故 `len` 步内不成环的链必已终止）⟹ 计入探针
/// [`AncokProbe::element_depth_fuel_exhausted`] 后 **显式 panic**。
/// **不静默钳制**（违规显式失败纪律）：钳制会把环化 parent 图伪装成一个合法深度，静默改
/// `w_depth` ⟹ 下单权重被脏数据污染且无任何告警面。debug/release 同门（`debug_assert` 在
/// release 被编译消除 ⟹ 恰好在生产侧失守）。
pub(crate) fn element_depth(elements: &ElementView, e_idx: usize) -> u32 {
    let fuel = elements.len();
    let mut depth = 0u32;
    let mut cur = elements.get(e_idx).and_then(|e| e.parent);
    while let Some(p) = cur {
        depth += 1;
        if depth as usize > fuel {
            ancok_probe_bump(|pr| pr.element_depth_fuel_exhausted += 1);
            panic!(
                "#247 C1 环路硬门：element_depth 自 idx={e_idx} 上溯 {depth} 步超过 fuel 上界 \
                 {fuel}（=elements.len()）⟹ parent 图含环（registry structural_parent_id 无环性\
                 无写入侧保障）。显式失败，不静默钳制。"
            );
        }
        cur = elements.get(p).and_then(|e| e.parent);
    }
    depth
}

/// **全定义策略目标头寸腿集 `q̄_Θ = LegTarget(AncOK[(A∖D)∪B])`**（M28 §十三 总形式）。
///
/// 对祖先闭合活动集 `active`（A_{t+1}）中**每个元素**生成 [`leg_target`]，得目标头寸腿列表
/// （= 分账本目标头寸 q̄_Θ 的腿分解，对齐 `SeparateStrategyTarget.strategyTargetLegs` + M28）。
/// 每条腿带 `(ν(e), ε_e, s_e, role)`，多空独立坐标（分账本 C25），净额抵消在 [`net_target_units`]。
pub(crate) fn strategy_target_legs(
    elements: &ElementView,
    active: &[usize],
    base_units: f64,
    config: &VoiceConfig,
) -> Vec<LegTarget> {
    // ★工位 4d 热点① O(n²) 消除：base（tree 前缀）兄弟索引缓存命中复用 `Rc`（O(1)，§16 不变），
    // 仅 overlay 段（candidate ++ restore，绝大多数 bar 仅 candidate）现建小索引。旧版每 bar
    // build_prev_sibling_index(contiguous=tree+overlay) O(tree)/bar=O(n²)，现 base 段 O(1) 命中。
    // bit-exact：operation_role_two_segment 双段 partition_point == 旧合并 sibling_idx partition_point
    // （base idx 全 < base_len ≤ overlay idx ⟹ 合并列表升序，分段查等价；见 two_segment 函数 doc）。
    let base_len = elements.base.len();
    // overlay 段兄弟索引（全局 idx = base_len + i）。overlay 空（热循环常态）⟹ 空 HashMap，全查 base。
    let mut overlay_sibling: std::collections::HashMap<(Option<usize>, u32), Vec<usize>> =
        std::collections::HashMap::new();
    for (i, e) in elements.overlay.iter().enumerate() {
        overlay_sibling.entry((e.parent, e.level)).or_default().push(base_len + i);
    }
    // base 段兄弟：缓存命中复用 Rc（O(1)），缺失 fallback 现建 O(tree)（测试/无缓存路径）。
    let base_sibling_owned;
    let base_sibling: &std::collections::HashMap<(Option<usize>, u32), Vec<usize>> =
        match &elements.base_sibling_idx {
            Some(rc) => rc.as_ref(),
            None => {
                base_sibling_owned = build_prev_sibling_index(elements.base);
                &base_sibling_owned
            }
        };
    active
        .iter()
        .map(|&e_idx| {
            leg_target_two_segment(elements, e_idx, base_units, config, base_sibling, &overlay_sibling)
        })
        .collect()
}

// ════════════════════════════════════════════════════════════════════════════
//  §6 净额执行（Nautilus 净额兼容：毛分账本腿 → 净持仓 units:f64）
// ════════════════════════════════════════════════════════════════════════════

/// **净额目标持仓 `q̄_net`（毛分账本腿 → 净持仓，Nautilus 净额兼容）**。
///
/// 所有目标腿（[`strategy_target_legs`] 产）合并为**净持仓单位数** `units:f64`（有符号，正=净多/
/// 负=净空），对齐 `plan_and_fill_mtm` 的有符号 `units` 净额账本（runner.rs:306）。多腿（Long）+
/// 单位、空腿（Short）− 单位——**对冲腿部分抵消父仓**（ReverseOpen 反向子声部腿的空单位抵消父多腿，
/// 对齐 M11 短差「父声部不动建反向子声部」的净额体现）。
///
/// ★分账本（毛）→ 净账本的语义降维（M29 §7 诚实声明）：分账本目标头寸 q̄_Θ 是**多空独立坐标**
/// （C25，毛收益级，G_net=0 双开净额退化）；Nautilus 净额账户只持**单一净持仓**——本函数把毛腿
/// 净额化（Σ ε_e·s_e）。净额化**丢失**双开的毛敞口信息（净杠杆 ≤ 毛杠杆，risk.rs `net_le_gross`）
/// ——这是 Nautilus 净额兼容的必然降维，**非** bug。完整毛分账本执行须 hedging 账户（v0 净额）。
pub fn net_target_units(legs: &[LegTarget]) -> f64 {
    legs.iter()
        .map(|leg| match leg.side {
            VoiceSide::Long => leg.units,
            VoiceSide::Short => -leg.units,
            VoiceSide::Flat => 0.0,
        })
        .sum()
}

/// **毛敞口 G = Σ|s_e|**（分账本毛账本，双开两腿都计入；对齐 risk.rs `gross_notional`）。
///
/// 各腿单位数绝对值之和——双开（多腿+空腿）的毛敞口高（净额 [`net_target_units`] 可低）。
/// 用于诊断净额降维丢失的毛敞口（净 ≤ 毛，element-coverage 的分账本毛收益级覆盖见证）。
pub fn gross_target_units(legs: &[LegTarget]) -> f64 {
    legs.iter().map(|leg| leg.units.abs()).sum()
}

/// **K_Θ 毛头寸约束（G7）：legs 折叠成净持仓之前施加毛敞口上限——逐根子树 KKT 投影**。
///
/// > **结果包六要素**
/// > - **结论**：毛敞口 `G=Σ|s_e|` 超上限 `Ḡ=γ̄·U_ℓ`（[`super::super::risk::gross_units_cap`]）时，
/// >   按根子树分组求缩放系数 `c_r∈[0,1]`（water-filling），每腿 `units·=c_{root(leg)}`；
/// >   未超限/空腿集不动（不触发路径 bit-exact）。缩放**先于** [`net_target_units`] 折叠——
/// >   净标量已丢失毛敞口信息，事后诊断门被 #122 终裁拒绝（反例 Long100+Short100：净0毛200）。
/// >   返回被**整体零化**（c_r=0/Ḡ=0）的腿 e_idx——调用方（[`coverage_step_from_buckets`]）据此
/// >   把零化的开仓腿排除出 next_active（裁定4 幽灵腿防护：从未建仓的腿不作持仓身份跨 bar 延续）。
/// > - **定义依据**：strict §11（毛 `G_t=Σ|n_v|` 与净必须**同时**约束，line 359 同单位数双开
/// >   反例）+ §12 K_Θ 17 项约束含杠杆上界 + 互斥 spec M18（`p*=LexArgmin_{p∈K_Θ}J_t(p)`，主键
/// >   `‖p−p̃‖²_W`）。毛约束触发后的处置**不是自由缩放规则**，而是 J_t 主键在 K_Θ 上的投影：
/// >   Θ 钉死腿间比率（同单位数 C04 / κ §6 / depth_weights）⟹ 每根子树只剩整体尺度 c_r 一个
/// >   自由度；跨根无 spec 比率绑定 ⟹ 逐根独立。等权 W（rust 无 per-voice w_v）下投影 =
/// >   `min Σ_r A_r(c_r−1)² s.t. Σ_r G_r·c_r ≤ Ḡ, c_r∈[0,1]`（`G_r=Σ|units|`、`A_r=Σunits²`），
/// >   KKT 闭式 `c_r = max(0, 1−μ/t_r)`（`t_r=2A_r/G_r`），μ 取约束等号（water-filling 扫描）。
/// >   **单根退化为等比例** `c=Ḡ/G`（codex decide 5b46：方案A只是单根退化情形，非多根语义）。
/// > - **边界条件**：① `G=0`（空腿/全零）或 `G≤Ḡ` ⟹ 不动。② `Ḡ=0`（γ̄=0 或 U_ℓ=0）⟹ 全腿
/// >   归零（K_Θ 毛可行集退化 {0}）。③ 若引入真 per-voice `w_v` 或腿级 lot 网格，`A_r=Σunits²`
/// >   须升级为对应主键权重/离散 LexArgmin（decide 5b46 翻转条件）。④ 若 spec 后续明确要求跨根
/// >   比例保持，则翻转为全腿等比例。⑤ 缩放只改 `LegTarget.units`（sizing 层目标敞口），不碰
/// >   活动集身份（`next_active` 不携 units）——跨 bar 状态不受污染。
/// > - **下游推论**：`p̃=net_target_units(legs')` 随缩放变小（毛超限 bar 的净目标被压缩）⟹
/// >   激活时历史成交序列改变（语义修正非重构，#122 §4 预期）；默认 `enforce_gross_cap=false`
/// >   不激活 ⟹ frozen Θ v0 bit-exact。
/// > - **谱系引用**：codex #122 终裁（`codex-q1-spec-rulings-20260703.md` G7 节）+ codex decide
/// >   `codex-decide-20260703-034046-5b46.md`（方案B：逐根 KKT，A/C/D 拒绝理由在卷）+
/// >   formalization-validity-domain 231号（净约束有效域覆盖不了毛约束——信息论层面不可恢复）。
/// > - **影响声明**：仅在 `enforce_gross_cap=true` 且触发时改 legs；判定走
/// >   [`super::super::risk::gross_units_ok`]（units 空间 = strict §11 `L^G≤L̄^G` 单标的精确等价形式，
/// >   coverage 不私写同义比较——risk.rs 死代码毛/净杠杆数学的生产接入点）。
///
/// 根子树分组按 `parent_id` 结构映射（spec §13 `p:C_ℓ→C_{ℓ+1}`，与生产 AncOK（#183 归一后
/// = [`super::super::exit::step_active_set_with_subtree_close`]）同一判据——非 per-bar 索引链；
/// restore 段腿 `parent: None` 但 `parent_id` 携真父，索引链会误判其为独立根）。确定性：
/// 分组按腿序首次出现，排序按 `(t_r, root_id)`（平局按根 id 定序），求和顺序固定 ⟹ bit-exact 可重放。
pub(crate) fn apply_gross_cap(
    elements: &ElementView,
    legs: &mut [LegTarget],
    base_units: f64,
    risk: &RiskConfig,
) -> Vec<usize> {
    let g = gross_target_units(legs);
    // 空腿集或未超限 ⟹ 不动（判定走 risk.rs units 空间 predicate，不私写比较）。
    if g == 0.0 || super::super::risk::gross_units_ok(g, base_units, risk.gamma) {
        return Vec::new();
    }
    let cap = super::super::risk::gross_units_cap(base_units, risk.gamma);
    if cap <= 0.0 {
        // Ḡ=0：毛可行集退化 {0}——全腿归零（安全侧，等价 force_flat 的腿级形式）。
        for leg in legs.iter_mut() {
            leg.units = 0.0;
        }
        return legs.iter().map(|l| l.e_idx).collect();
    }

    // ── 根子树分组（parent_id 结构映射；lookup 双段模式同旧生产 AncOK 判据，#183 归一后判据
    //   本体在 exit::step_active_set_with_subtree_close）──
    let base_id_idx_owned;
    let base_id_idx: &std::collections::HashMap<ElementId, usize> = match &elements.base_id_idx {
        Some(rc) => rc.as_ref(),
        None => {
            base_id_idx_owned = build_tree_id_index(elements.base);
            &base_id_idx_owned
        }
    };
    let base_len = elements.base.len();
    let mut overlay_id_idx: std::collections::HashMap<ElementId, usize> =
        std::collections::HashMap::new();
    for (i, e) in elements.overlay.iter().enumerate() {
        overlay_id_idx.entry(e.id).or_insert(base_len + i);
    }
    let lookup = |pid: &ElementId| -> Option<usize> {
        base_id_idx.get(pid).copied().or_else(|| overlay_id_idx.get(pid).copied())
    };
    // 腿的根键 = parent_id 链顶端 id（无父 ⟹ 自身即根）。链顶可为不在本 view 的 id（registry
    // 祖先未恢复的极端情形）——ElementId 跨 bar 稳定，作组键仍确定。
    let root_key = |e_idx: usize| -> ElementId {
        let e_id = elements.get(e_idx).map(|e| e.id).expect("legs.e_idx 来自 next_idx，必在 view 内");
        ancestors_by_id_lookup(elements, e_idx, &lookup).last().copied().unwrap_or(e_id)
    };

    // 每根聚合 (G_r, A_r)：分组按腿序首次出现（确定序）。
    let mut order: Vec<ElementId> = Vec::new();
    let mut group_of: std::collections::HashMap<ElementId, usize> = std::collections::HashMap::new();
    let mut leg_group: Vec<usize> = Vec::with_capacity(legs.len());
    let mut g_r: Vec<f64> = Vec::new();
    let mut a_r: Vec<f64> = Vec::new();
    for leg in legs.iter() {
        let rk = root_key(leg.e_idx);
        let gi = *group_of.entry(rk).or_insert_with(|| {
            order.push(rk);
            g_r.push(0.0);
            a_r.push(0.0);
            order.len() - 1
        });
        leg_group.push(gi);
        g_r[gi] += leg.units.abs();
        a_r[gi] += leg.units * leg.units;
    }

    // ── water-filling：μ 使 Σ_r G_r·max(0, 1−μ/t_r) = Ḡ（t_r=2A_r/G_r 为 c_r 归零阈值）──
    // G_r=0 组无敞口贡献（units 全零），c_r=1 恒可行，不参与求解。
    let threshold = |r: usize| -> f64 { 2.0 * a_r[r] / g_r[r] };
    let mut idxs: Vec<usize> = (0..order.len()).filter(|&r| g_r[r] > 0.0).collect();
    idxs.sort_by(|&a, &b| {
        threshold(a)
            .partial_cmp(&threshold(b))
            .expect("有限 units 平方和 ⟹ 阈值有限可序")
            .then_with(|| (order[a].level, order[a].ordinal).cmp(&(order[b].level, order[b].ordinal)))
    });
    // 后缀和 SG_i=Σ_{j≥i}G_r、SQ_i=Σ_{j≥i}G_r/t_r（=G_r²/2A_r），单趟反向扫（求和顺序固定）。
    let n = idxs.len();
    let mut sg = vec![0.0f64; n + 1];
    let mut sq = vec![0.0f64; n + 1];
    for i in (0..n).rev() {
        let r = idxs[i];
        sg[i] = sg[i + 1] + g_r[r];
        sq[i] = sq[i + 1] + g_r[r] / threshold(r);
    }
    // 扫描首个 μ_i ≤ t_(i) 的截断点：0..i 组 c=0（阈值低于水位），i.. 组 c_r=1−μ/t_r。
    // 数学保证 i=n−1 必满足（μ_{n−1}=t_{n−1}·(1−Ḡ/G_{n−1})≤t_{n−1}，Ḡ>0）——`i+1==n` 臂
    // 只挡浮点舍入把 ≤ 抖成 >，语义同一。
    let mut c = vec![1.0f64; order.len()];
    for i in 0..n {
        let t_i = threshold(idxs[i]);
        let mu = (sg[i] - cap) / sq[i];
        if mu <= t_i || i + 1 == n {
            for &r in &idxs[..i] {
                c[r] = 0.0;
            }
            for &r in &idxs[i..] {
                c[r] = (1.0 - mu / threshold(r)).clamp(0.0, 1.0);
            }
            break;
        }
    }
    // 返回被整体零化（c_r=0）的腿 e_idx——幽灵腿防护（裁定4）：调用方据此把零化的**开仓**腿
    // 排除出 next_active（从未建仓的腿不得作为持仓身份跨 bar 延续）。部分缩放（c∈(0,1)）的腿
    // 真实开仓（缩小目标流入 p̃→订单），不在此列。
    let mut zeroed = Vec::new();
    for (leg, &gi) in legs.iter_mut().zip(&leg_group) {
        leg.units *= c[gi];
        if c[gi] == 0.0 {
            zeroed.push(leg.e_idx);
        }
    }
    zeroed
}

/// ★P0-3 overlay 净贡献诊断 **ΔN_t**（多空对冲.pdf §5 / codex-f2 问题5,6）——**只读**，非账本。
///
/// `ΔN_t = N^#5_t − N^base_t = net_target(全腿) − net_target(剔 ReverseOpen 腿)`，代数上 = ReverseOpen
/// 反向子声部腿的净贡献（= 多空对冲.pdf 的 overlay 头寸 H_t=−σ_parent·h_t）。∑_t|ΔN_t| = ‖ΔN‖_1，
/// 即「depth>0 子声部是否真改变净头寸」的净额可见层度量（多空对冲.pdf p13 三层判定第二层）。
///
/// ★第三会计范畴（674号：R/TW 账本不同构，overlay 不挂靠任一）：本函数**只读**从 legs 净目标差算，
/// 不建账本、不投影到 R/TW。ΔN_t 只是 sizing 层目标敞口差，非成交 PnL——`Π^overlay=H_tΔP−ΔC`
/// 需 hedge 腿独立成本，**不能从此净目标反推**（codex 问题5）。
///
/// ★升级触发条件：一旦诊断结果驱动**真实对冲交易执行**（ReverseOpen 腿实际下单），必须升级为独立
/// `OverlayState` 账本（第三范畴，标注对 R/TW 的投影损失），不得继续用只读净目标差充当账本。
///
/// > 认识论 L0：纯结构算术（腿方向×单位数的条件求和），不依赖经验数据、不声明 alpha。
pub fn overlay_net_delta(legs: &[LegTarget]) -> f64 {
    legs.iter()
        .filter(|leg| leg.role.v == Vertical::ReverseOpen)
        .map(|leg| match leg.side {
            VoiceSide::Long => leg.units,
            VoiceSide::Short => -leg.units,
            VoiceSide::Flat => 0.0,
        })
        .sum()
}

// ════════════════════════════════════════════════════════════════════════════
//  §7 已删除（GAP-5 收口，no-patch）：旧 `coverage_step`（λ_e 走势边界 per-bar 入场五步合成）
//  曾自称"生产引擎核心入口"，但经 [`active_set_step`] 在 `B_t={e:λ_e=t}` 入场——λ_e=
//  `LeveledMove.start_index`=**走势边界**，正是 GAP-5 判定的错误入场源。spec §13 正典递归用
//  解释器三桶 𝒟_x/ℬ_x=ℛ_Θ(Γ(x))（**买卖点 Γ**），生产入场已收口至 §8 买卖点路径 + §9 π_Θ。
//  （框架纠偏 MEMORY coverage-engine-needs-tower-export-bridge：互斥全定义策略=**买卖点入场**+
//  多级角色/嵌套对冲，**非每元素覆盖**。删除 λ_e 入场组装层，保留 §3 区间递归原语作 Lean 对齐。）
// ════════════════════════════════════════════════════════════════════════════
//  §8 环6：解释器三桶 → 活动集 A_{t+1}=AncOK[(A_t∖𝒟_x^†)∪ℬ_x∪ℛ_x] → 目标头寸 p̃_{t+1}
//  （★#247 缺口一：ℛ_x = RegistryRestore = **第三来源**。正文落点：本 §8 抬头之后的连续 doc 块在
//   rustdoc 上挂在 [`restore_ancestor_chain_from_registry`]，**不在** [`coverage_step_from_buckets_sep`] doc）
//        （spec §13 line 1172 活动集 + §14 line 1243 头寸，七链 **环6** rust 兑现）
// ════════════════════════════════════════════════════════════════════════════


#[cfg(test)]
#[path = "leg_tests.rs"]
mod tests;
