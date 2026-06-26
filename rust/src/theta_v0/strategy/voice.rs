//! Θ_voice 声部树（reference-theta-v0.md:39-42，契约锚 `Origin.VoiceTree`）。
//!
//! ## 范围（声部树 𝒯=(V,p) + 4 互斥动作态）
//!
//! - 根 = 当前最高有效决策级别 L*；最多 `config.voice.max_depth` 层（L*, L*-1, L*-2）。
//! - 声部方向 `σ_v = (-1)^depth`（`dir_of_depth`，对齐 Origin.VoiceTree `dirOfDepth`）：
//!   偶深多（+1）/ 奇深空（-1），根多 → 子空 → 孙多…。子须 `σ_child = -σ_parent`
//!   （`flip_dir`，对齐 Origin.VoiceTree `flipDir` + `alternating`）。
//! - 4 互斥动作态 {close, open, hold, wait}：Σ𝟙=1（对齐 Origin.VoiceTree
//!   `voice_action_exhaustive_exclusive`）；目标仓位 q̃ = 0(close)/b_v(open)/q_v(hold)/0(wait)
//!   （对齐 Origin.VoiceTree `targetPos`）。
//! - 资金帽深度权重 `config.voice.depth_weights`（w=[0.60,0.30,0.10]）；未用部分保留现金
//!   不重分配（reference-theta-v0.md:42）。
//!
//! ## 认识论等级
//!
//! L0（定义内蕴）：σ 推导 / 4 动作态划分 / 深度权重映射是定义层逻辑必然，不依赖经验数据。
//! Rust 实装对齐 Lean 的 L0 定理 = L1 一致性（bit-exact），不冒充 L2。

use super::super::config::VoiceConfig;

/// 声部方向 σ ∈ {+1 多, -1 空, 0 空仓}（reference-theta-v0.md:41）。
///
/// `Long=+1` / `Short=-1` 是 Origin.VoiceTree `σ` 的两态（赋格交替域）；`Flat=0` 是 spec:41 的
/// 空仓极性（声部不开仓时的方向，Origin.VoiceTree 用 `q_v=0` 表达，本 Rust 域显式 `Flat`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceSide {
    Long,
    Short,
    Flat,
}

impl VoiceSide {
    /// 方向翻转（对齐 Origin.VoiceTree `flipDir`）：Long ↔ Short；Flat 不翻转（空仓无方向可翻）。
    ///
    /// 边界条件：`Flat.flip() = Flat`——空仓极性不参与赋格交替（spec:41 的 σ=0 是无方向态，
    /// 不在 (-1)^depth 的两态域内）。只有 Long/Short 满足 `flip(flip(x))=x` 对合（Origin.VoiceTree
    /// `flipDir_flipDir`）。
    pub fn flip(self) -> VoiceSide {
        match self {
            VoiceSide::Long => VoiceSide::Short,
            VoiceSide::Short => VoiceSide::Long,
            VoiceSide::Flat => VoiceSide::Flat,
        }
    }
}

/// 从深度奇偶导出声部方向，**根方向固定 Long**（bit-exact 对齐 Origin.VoiceTree `dirOfDepth`）。
///
/// `dir_of_depth(d) = (-1)^d`：偶深 → Long(+1)，奇深 → Short(-1)。根（depth 0）= Long。
/// 这是赋格交替 σ_v=-σ_{p(v)} 的算术核心（Origin.VoiceTree `dirOfDepth_succ`：相邻深度方向相反），
/// **逐值 bit-exact 镜像 Origin.VoiceTree `dirOfDepth`**（根恒 Long 的绝对版）。
///
/// ★有效域辨识（formalization-validity-domain）：本函数 = [`voice_side`] 在 `root_side=Long`
/// 子域的逐值重合（`voice_side(Long, d) == dir_of_depth(d)`）。Origin.VoiceTree `dirOfDepth`/
/// `depth_parity`（根恒 Long）约束的是**赋格嵌套树**（父子 σ 交替）；[`voice_side`] 覆盖的是
/// StrategyFamily §5 **多独立根**（根方向自由，`long_short_both_open_allowed:570`）。两者是同一
/// 极性律在不同有效域的呈现——`dir_of_depth` 是 root_side=Long + 嵌套的强化子情形。
/// **plan_orders 用 [`voice_side`]**（接通做多+做空侧，对齐 §5）；本函数保留作 Fugue 嵌套树
/// 的直接镜像 + Long 子域校验锚（voice.rs 测试 `voice_side_long_root_equals_dir_of_depth`）。
///
/// 边界条件：此函数只依赖 depth 奇偶——若 Θ 引入非二态方向（如多空中性三态参与交替）
/// 则映射翻转（须 change request，不在 Θ v0 内；Flat 不由 depth 导出，是运行时空仓态）。
pub fn dir_of_depth(depth: u32) -> VoiceSide {
    if depth % 2 == 0 {
        VoiceSide::Long
    } else {
        VoiceSide::Short
    }
}

/// 声部绝对方向 = 根方向 `root_side` × 深度相对极性（reference spec:41 + StrategyFamily §5）。
///
/// `voice_side(root_side, depth) = root_side · (-1)^depth`：根声部（depth 0）方向 = `root_side`
/// （由信号方向定——3买/底背驰 → Long，3卖/顶背驰 → Short）；子声部按 depth 奇偶相对根翻转
/// （`σ_v = σ_root·(-1)^depth`，保留赋格交替 σ_child=-σ_parent）。**plan_orders 用本函数**
/// （recog 接通做多+做空侧）。
///
/// ★**对齐 StrategyFamily §5（VoiceTree 多独立根，无分叉）**（formalization-validity-domain）：
/// - 本函数对齐 `StrategyFamily.lean:570 long_short_both_open_allowed`——该定理证明体里两声部
///   都是独立根（`parent := fun _ => none`），`side` 为 Long(true) 与 Short(false) 并存，
///   `depth=0`，`alternating` 真空满足。**Short 根在 §5 已证允许**，故本函数的 root_side=Short
///   分支落在 §5 的有效域内（§5 已含多独立根，含 Short 根）——voice_side 的 σ 语义即 §5 的
///   side，bit-exact 对齐 §5（无 Rust/Lean σ 分叉）。
/// - `dir_of_depth` / Origin.VoiceTree `depth_parity` 是 `root_side=Long + 嵌套树`的**强化子情形**
///   （不同有效域）：Fugue 约束的是赋格嵌套树内部 σ=dirOfDepth(depth)（根恒 Long）；本函数
///   覆盖的是 §5 的多独立根（根方向自由，由信号定）。`voice_side(Long, d) == dir_of_depth(d)`
///   逐值成立——二者在 root_side=Long 子域重合，是同一极性律在两有效域（§5 多根 / Fugue 嵌套）
///   的一致呈现。
/// - 依据：reference spec:41「σ=+1多/-1空」未固定根方向 + §5 多独立根并存。赋格交替
///   （σ_child=-σ_parent）对任意 root_side 成立（root_side·(-1)^(d+1) = flip(root_side·(-1)^d)）。
///
/// ★诚实：v0 recognize 产**单声部决策**（depth 0，独立根）——每个买卖点是 §5 的独立根，
/// 方向 = `root_side` = 信号方向。嵌套声部树（depth>0 子对冲）是 Origin.VoiceTree 完整结构，
/// v0 未触发（无嵌套）；本函数对 depth>0 给 §5 一致的极性推广，v0 只用 depth=0 分支。
///
/// 边界条件：`root_side = Flat` ⟹ 返回 Flat（空仓无方向，不参与极性翻转，所有深度均 Flat）。
pub fn voice_side(root_side: VoiceSide, depth: u32) -> VoiceSide {
    match root_side {
        VoiceSide::Flat => VoiceSide::Flat,
        // 偶深 = 根方向；奇深 = 根方向翻转（赋格交替）。
        _ => {
            if depth % 2 == 0 {
                root_side
            } else {
                root_side.flip()
            }
        }
    }
}

/// 单声部状态（对齐 Origin.VoiceTree `FugueTree` 的逐声部字段 + `VoiceEnv` 的判定）。
///
/// 字段语义（Origin.VoiceTree `FugueTree`/`VoiceEnv`）：
/// - `depth`：声部深度（根=0），决定 σ（`dir_of_depth`）。
/// - `b`：开仓基准手数 b_v（Θ_risk 参数，结构承载非缠论可导；Origin.VoiceTree `b_pos`：b_v≥1）。
/// - `q`：当前持仓手数 q_v（q_v=0 = 空仓）。
/// - `exit`：退出态 X_v（emergency ∨ ancestor_close ∨ stop ∨ reverse_nest，Origin.VoiceTree `Exit`）。
/// - `enter_ok`：进场许可 E_v（Permit ∧ same_nest，Origin.VoiceTree `EnterOK`）。
///
/// ★诚实：`b`/`q` 取值不由缠论推导（Θ_risk + 运行时状态）；`exit`/`enter_ok` 的实时取值
/// 来自数据流（区间套/许可判定），本结构作为给定 Bool/手数承载——动作态划分在给定它们后
/// 是 L0 逻辑必然（对齐 Origin.VoiceTree 在给定 env 下的 L0 证明）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceState {
    pub depth: u32,
    pub b: u32,
    pub q: u32,
    pub exit: bool,
    pub enter_ok: bool,
}

/// 4 互斥动作态（bit-exact 对齐 Origin.VoiceTree `ActState`：close/open/hold/wait）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActState {
    /// 平仓（退出 X_v 成立，吸收前四优先级：全局强平/祖先关闭/止损/反向区间套）。
    Close,
    /// 开仓（从空仓建仓 b_v：¬Exit ∧ q=0 ∧ EnterOK）。
    Open,
    /// 持仓不动（¬Exit ∧ q>0）。
    Hold,
    /// 空仓观望（¬Exit ∧ q=0 ∧ ¬EnterOK）。
    Wait,
}

/// 选出当前动作态（bit-exact 对齐 Origin.VoiceTree `actState`：基于 (Exit, q=0, EnterOK) 三元分类）。
///
/// 全函数：穷尽 (Exit, q=0?, EnterOK?) 三元判定的 4 种结果——对齐 Origin.VoiceTree
/// `voice_action_exhaustive_exclusive`（Σ𝟙=1，4 态恰一成立）。Rust match 静态保证穷尽。
///
/// 边界条件：close 吸收 Exit（不论 q）；¬Exit 后按 q=0 划分 open/wait（EnterOK 区分）与
/// hold（q>0）。若 Θ 引入第 5 动作态则翻转（不在 Θ v0 内）。
pub fn act_state(v: &VoiceState) -> ActState {
    if v.exit {
        ActState::Close
    } else if v.q == 0 {
        if v.enter_ok {
            ActState::Open
        } else {
            ActState::Wait
        }
    } else {
        ActState::Hold
    }
}

/// 目标仓位 q̃（bit-exact 对齐 Origin.VoiceTree `targetPos`）。
///
/// close → 0；open → b_v；hold → q_v；wait → 0。与 4 动作态一一对应
/// （Origin.VoiceTree `targetPos_spec`）。
pub fn target_pos(v: &VoiceState) -> u32 {
    match act_state(v) {
        ActState::Close => 0,
        ActState::Open => v.b,
        ActState::Hold => v.q,
        ActState::Wait => 0,
    }
}

/// 取深度 `depth` 的资金帽权重 w_depth（reference-theta-v0.md:42）。
///
/// 从 `config.voice.depth_weights` 按深度索引取权重（w=[0.60,0.30,0.10]）。超出权重表长度
/// 的深度返回 `0.0`——**未用部分保留现金不重分配**（spec:42 的核心约束：权重表外的深度
/// 不获得资金，剩余资金留现金，**不**按比例重分配给已有声部）。
///
/// 边界条件：`config.voice.max_depth` 应 ≤ `depth_weights.len()`（spec:40 默认 3 层对应
/// 3 个权重）；若 max_depth 超出权重表则超出层 w=0（自然不开仓，非错误）。
pub fn depth_weight(depth: u32, config: &VoiceConfig) -> f64 {
    config
        .depth_weights
        .get(depth as usize)
        .copied()
        .unwrap_or(0.0)
}

/// 声部是否在树的有效深度范围内（reference-theta-v0.md:40：最多 max_depth 层）。
///
/// `depth < config.voice.max_depth` 才是有效声部——超出 max_depth 的深度不开声部
/// （spec:40 的「最多 3 层 L*,L*-1,L*-2」硬约束）。这关死「无限递归开子声部」。
pub fn within_max_depth(depth: u32, config: &VoiceConfig) -> bool {
    depth < config.max_depth
}

#[cfg(test)]
mod tests {
    use super::*;

    /// bit-exact 对齐 Origin.VoiceTree `dirOfDepth`：偶深 Long，奇深 Short（根多→子空→孙多）。
    #[test]
    fn dir_of_depth_alternates_from_long_root() {
        assert_eq!(dir_of_depth(0), VoiceSide::Long);
        assert_eq!(dir_of_depth(1), VoiceSide::Short);
        assert_eq!(dir_of_depth(2), VoiceSide::Long);
        assert_eq!(dir_of_depth(3), VoiceSide::Short);
    }

    /// 赋格交替 σ_child = -σ_parent（对齐 Origin.VoiceTree `alternating` + `dirOfDepth_succ`）：
    /// 相邻深度方向恒相反。
    #[test]
    fn child_side_is_parent_flipped() {
        for d in 0u32..8 {
            assert_eq!(dir_of_depth(d + 1), dir_of_depth(d).flip());
            assert_ne!(dir_of_depth(d + 1), dir_of_depth(d)); // adjacent_opposite
        }
    }

    /// flip 对合（对齐 Origin.VoiceTree `flipDir_flipDir`）：Long/Short 翻两次还原；Flat 不变。
    #[test]
    fn flip_involutive_on_directional_sides() {
        assert_eq!(VoiceSide::Long.flip().flip(), VoiceSide::Long);
        assert_eq!(VoiceSide::Short.flip().flip(), VoiceSide::Short);
        assert_eq!(VoiceSide::Flat.flip(), VoiceSide::Flat);
    }

    /// voice_side bit-exact 子域：`voice_side(Long, d) == dir_of_depth(d)`（Fugue 是 Long 特例）。
    #[test]
    fn voice_side_long_root_equals_dir_of_depth() {
        for d in 0u32..8 {
            assert_eq!(voice_side(VoiceSide::Long, d), dir_of_depth(d));
        }
    }

    /// voice_side 推广：Short 根 = Long 根的镜像（spec:41 顶背驰做空，root_side=Short）。
    #[test]
    fn voice_side_short_root_mirrors_long() {
        // 根（depth 0）= root_side 本身。
        assert_eq!(voice_side(VoiceSide::Short, 0), VoiceSide::Short);
        assert_eq!(voice_side(VoiceSide::Long, 0), VoiceSide::Long);
        // 偶深 = 根方向，奇深 = 根翻转（赋格交替对两种根都成立）。
        assert_eq!(voice_side(VoiceSide::Short, 1), VoiceSide::Long); // 子对冲
        assert_eq!(voice_side(VoiceSide::Short, 2), VoiceSide::Short);
        // Short 根 = Long 根逐深度翻转。
        for d in 0u32..8 {
            assert_eq!(voice_side(VoiceSide::Short, d), voice_side(VoiceSide::Long, d).flip());
        }
    }

    /// voice_side 保留赋格交替（σ_child=-σ_parent）：相邻深度方向恒相反，对任意根方向。
    #[test]
    fn voice_side_preserves_alternation() {
        for &root in &[VoiceSide::Long, VoiceSide::Short] {
            for d in 0u32..8 {
                assert_eq!(voice_side(root, d + 1), voice_side(root, d).flip());
                assert_ne!(voice_side(root, d + 1), voice_side(root, d));
            }
        }
    }

    /// voice_side 边界：Flat 根 ⟹ 所有深度均 Flat（空仓无方向）。
    #[test]
    fn voice_side_flat_root_stays_flat() {
        for d in 0u32..4 {
            assert_eq!(voice_side(VoiceSide::Flat, d), VoiceSide::Flat);
        }
    }

    /// bit-exact 对齐 Origin.VoiceTree `voice_action_exhaustive_exclusive`：4 态 Σ𝟙=1
    /// （穷尽 + 互斥）。逐 (exit, q, enter_ok) 组合枚举验证恰一态。
    #[test]
    fn act_state_partition_exhaustive_exclusive() {
        // exit=true ⟹ Close（不论 q/enter_ok）。
        for &q in &[0u32, 5] {
            for &e in &[false, true] {
                let v = VoiceState { depth: 0, b: 1, q, exit: true, enter_ok: e };
                assert_eq!(act_state(&v), ActState::Close);
            }
        }
        // ¬exit ∧ q=0 ∧ enter_ok ⟹ Open。
        let v = VoiceState { depth: 0, b: 3, q: 0, exit: false, enter_ok: true };
        assert_eq!(act_state(&v), ActState::Open);
        // ¬exit ∧ q=0 ∧ ¬enter_ok ⟹ Wait。
        let v = VoiceState { depth: 0, b: 3, q: 0, exit: false, enter_ok: false };
        assert_eq!(act_state(&v), ActState::Wait);
        // ¬exit ∧ q>0 ⟹ Hold（不论 enter_ok）。
        for &e in &[false, true] {
            let v = VoiceState { depth: 0, b: 3, q: 7, exit: false, enter_ok: e };
            assert_eq!(act_state(&v), ActState::Hold);
        }
    }

    /// bit-exact 对齐 Origin.VoiceTree `targetPos_spec`：close/wait→0，open→b_v，hold→q_v。
    #[test]
    fn target_pos_matches_act_state() {
        let close = VoiceState { depth: 0, b: 3, q: 7, exit: true, enter_ok: false };
        assert_eq!(target_pos(&close), 0);
        let open = VoiceState { depth: 0, b: 3, q: 0, exit: false, enter_ok: true };
        assert_eq!(target_pos(&open), 3);
        let hold = VoiceState { depth: 0, b: 3, q: 7, exit: false, enter_ok: false };
        assert_eq!(target_pos(&hold), 7);
        let wait = VoiceState { depth: 0, b: 3, q: 0, exit: false, enter_ok: false };
        assert_eq!(target_pos(&wait), 0);
    }
}
