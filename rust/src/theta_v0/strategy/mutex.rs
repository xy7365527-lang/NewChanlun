//! 全互斥买卖点解释器——固定优先级互斥化 `C_j = P_j ∧ ⋀_{k<j} ¬P_k`（买卖点alpha2.pdf
//! Doc2 §5 + Doc3 §6 + Doc2 §9 定理1）。
//!
//! ## 契约锚（`/Users/silencehan/Downloads/买卖点alpha2.pdf` Doc2 §5 / Doc3 §6，逐字）
//!
//! 原始谓词 `P_1..P_m`（m=8）**可重叠**（同一时刻多个 P_j 同时成立）：
//! - `P1` = 风险强平
//! - `P2` = 已有多头且出现卖点，平多
//! - `P3` = 已有空头且出现买点，平空
//! - `P4` = 父声部 active 且出现反父方向证书，开短差
//! - `P5` = 空仓/ambient 状态下出现买点，开多
//! - `P6` = 空仓/ambient 状态下出现卖点，开空
//! - `P7` = 已有同向声部，记录或加仓
//! - `P8` = 无有效动作，保持
//!
//! **固定优先级互斥化**（Doc3 §6）：
//! ```text
//! C_1 = P_1
//! C_j = P_j ∧ ⋀_{k<j} ¬P_k    (j = 2..m)
//! C_0 = ⋀_{j=1}^m ¬P_j         （兜底）
//! ```
//!
//! **定理 1（Doc2 §9 / Doc3 §6）**：`Σ_{j=0}^m 1[C_j] = 1`（全互斥 + 全定义）。
//! 证明：若无 P_j 成立则 C_0 唯一成立；否则取最小成立索引 r，则 C_r 唯一成立
//! （所有 j<r 因 P_j=0 不成立，所有 j>r 因 ¬P_r=0 不成立）。
//!
//! 全定义解释器（Doc2 §5）：`I_Θ(A_t, Γ_t) = (D_t, O_t, L_t)`——D=关闭声部 / O=开启声部 /
//! L=记录但不执行的信号。本模块产出该三元组的语义裁决码（[`MutexClass`]），下游
//! [`super::interp`] 的三桶 fold 据此分流（D↔close / O↔open / L↔record）。
//!
//! ## 与既有模块的关系（no-patch-mentality：不重造，显式化既有隐式优先级）
//!
//! - [`super::intent::action_priority`]：把 10 级优先级**压缩进 match 分支顺序**——优先级隐式编码在
//!   分支序里，输入是 `ClassLabel`（risk_mode×phase 摘要），**无显式可重叠谓词向量**，故无法机器
//!   判定「可重叠谓词互斥化恰好一个成立」（Rust match 穷尽性 = 输入域全覆盖，**不是** alpha2 定理）。
//! - [`super::interp::interpret`]：候选集 Γ 按 ≺_Θ 全序 fold（买卖点**重合时的候选唯一化**），不是
//!   动作意图谓词 P_j 的互斥化。
//!
//! 本模块补的是 alpha2 定理 1 的**显式机器判定**：给定可重叠谓词向量 `P ∈ {0,1}^8`，固定优先级
//! 互斥化为唯一 C_j，可证伪核心 = 穷举 2^8 组合断言 `Σ_j 1[C_j]=1`（[`tests`] 的 `mutex_total`）。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! **L0 结构定理**（非 L2 alpha）：`Σ_{j=0}^m 1[C_j]=1` 是固定优先级互斥化的**组合逻辑恒等式**
//! （alpha2 Doc2§5 / Doc3§6 证毕，零信息增量同义反复）。Rust 穷举 2^8 验证 = **L1 管线正确性**
//! （验证互斥化实装无 bug，不验证谓词 P_j 经验有效——P_j 的市场触发率/盈利性是 L2 未覆盖）。
//!
//! ## 身份：D1 等价测试 oracle，**非生产路径**（codex-q2-d1 §Q2 裁定）
//!
//! 本模块 `mutex_class` **零生产消费者**——生产 π 的候选级裁决走 [`super::interp::interpret`]（≺_Θ
//! 全序三桶）。`mutex_class` 保留的唯一用途是作 **D1 逐候选级等价 property test 的对拍参照**
//! （PDF Part B 四 P1..P8 固定优先级 vs interp ≺_Θ 序，桶级一致性）。[`predicates_of`] 把生产
//! 候选 + fold 状态投影成 PDF 谓词向量，[`bridge_bucket`] 把 `MutexClass` 映射到 interp 三桶
//! （close/open/record），测试断言二者桶级一致（见 `tests::shadow_fold_bucket_equivalence`）。
//! 曾并存的 `closed_loop/mutex_interp.rs`（9 谓词 Lean 镜像）已按 §Q2 删除（no-patch：不留两个
//! 零消费者权威镜像）。**P1 风险强平不在本对拍范围**（由 `KThetaRiskGate.force_flat` 上层独立兜，
//! 见 codex-q2-d1 §3 边界）。

use super::interp::{ActiveLeg, Candidate};
use super::voice::VoiceSide;

/// 谓词数 m=8（P_1..P_8）。
pub const M: usize = 8;

/// 互斥化裁决类号（Doc3 §6）。`C_0`=兜底（无谓词成立），`C_j`(j=1..8)=最小成立谓词索引。
///
/// `Σ_{j=0}^8 1[C_j]=1`：给定任意谓词向量恰好对应一个 `MutexClass`（全互斥 + 全定义）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutexClass {
    /// C_0：⋀_j ¬P_j（无任何谓词成立，兜底）。
    C0,
    /// C_j：P_j ∧ ⋀_{k<j} ¬P_k（最小成立索引 j∈1..=8，存 1-based 谓词号）。
    Cj(u8),
}

/// 原始谓词向量 `P ∈ {0,1}^8`（可重叠——多个分量可同时为 true，alpha2 §5）。
///
/// 字段名对齐 alpha2 §5/§6 谓词语义（1-based 谓词号见各字段）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Predicates {
    /// P1：风险强平。
    pub risk_liquidate: bool,
    /// P2：已有多头且出现卖点，平多。
    pub close_long: bool,
    /// P3：已有空头且出现买点，平空。
    pub close_short: bool,
    /// P4：父声部 active 且出现反父方向证书，开短差。
    pub open_short_diff: bool,
    /// P5：空仓/ambient 出现买点，开多。
    pub open_long: bool,
    /// P6：空仓/ambient 出现卖点，开空。
    pub open_short: bool,
    /// P7：已有同向 slot 占用 ⟹ 记录。**诚实命名**（codex-q2-d1 §4）：PDF P7 字面是「记录**或加仓**」，
    /// 但生产 `interp::interpret` 只有 record 桶、无「加仓」子动作，本字段只覆盖「记录」半句——故命名
    /// `same_slot_record`（非 `same_dir_record_add`），不暗示不存在的加仓语义。
    pub same_slot_record: bool,
    /// P8：无有效动作，保持。
    pub hold: bool,
}

impl Predicates {
    /// 谓词向量按 1-based 索引读 `P_j`（j∈1..=8）。索引越界 ⟹ panic（内部不变量，j 来自 0..M 循环）。
    fn p(&self, j: usize) -> bool {
        match j {
            1 => self.risk_liquidate,
            2 => self.close_long,
            3 => self.close_short,
            4 => self.open_short_diff,
            5 => self.open_long,
            6 => self.open_short,
            7 => self.same_slot_record,
            8 => self.hold,
            _ => unreachable!("谓词索引 j={j} 越界（合法 1..={M}）"),
        }
    }
}

/// 固定优先级互斥化（Doc3 §6）：`C_j = P_j ∧ ⋀_{k<j} ¬P_k`，兜底 `C_0 = ⋀_j ¬P_j`。
///
/// 实装即定理证明的构造形式——取**最小成立谓词索引** r（C_r 唯一成立）；无成立谓词 ⟹ C_0。
/// 全定义（任意输入返回唯一 [`MutexClass`]）+ 全互斥（`Σ_j 1[C_j]=1`，构造保证恰一个）。
///
/// **边界条件**：全 false ⟹ C0；存在 true ⟹ Cj(最小成立索引)。优先级**不可交换**——更小索引的
/// 谓词成立时屏蔽所有更大索引（P1 风险强平 ≻ … ≻ P8 保持，alpha2 §16 优先级链）。
///
/// **认识论 L0**：纯组合逻辑（无数据依赖），互斥性是定义内蕴的同义反复。
pub fn mutex_class(p: &Predicates) -> MutexClass {
    for j in 1..=M {
        if p.p(j) {
            return MutexClass::Cj(j as u8);
        }
    }
    MutexClass::C0
}

/// interp 三桶动作类（[`super::interp::interpret`] 的候选归属：close/open/record）。
///
/// D1 对拍的比较粒度——`mutex_class` 只能充当**三桶级 oracle**（不区分「记录」vs「加仓」子动作，
/// codex-q2-d1 §1c）。故等价断言在桶级，不在精确 Cj。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionBucket {
    /// 𝒟_x：关闭活动腿（interp 规则2 反向平仓）。
    Close,
    /// ℬ_x：开启新 slot（interp 规则3）。
    Open,
    /// 𝒦_x：记录不执行（interp 规则1 无类/无向、规则4 slot 冲突）。
    Record,
}

/// `MutexClass` → interp 三桶（codex-q2-d1 §1b 桥接表）。
///
/// `C1(风险,本范围不触发)/C2/C3 → Close`；`C4/C5/C6 → Open`；`C7/C8(hold)/C0 → Record`。
pub fn bridge_bucket(c: MutexClass) -> ActionBucket {
    match c {
        MutexClass::Cj(1) | MutexClass::Cj(2) | MutexClass::Cj(3) => ActionBucket::Close,
        MutexClass::Cj(4) | MutexClass::Cj(5) | MutexClass::Cj(6) => ActionBucket::Open,
        _ => ActionBucket::Record, // Cj(7) 记录 / Cj(8) 保持 / C0 兜底
    }
}

/// **D1 桥接：生产候选 + fold 状态 → PDF 谓词向量 P∈{0,1}^8**（codex-q2-d1 §1a 修正签名）。
///
/// 独立于 `interp::interpret` 的规则序，从 PDF 谓词语义**重新推导**每个 P_j——这样
/// `bridge_bucket(mutex_class(predicates_of(..)))` 与 interp 实际桶归属的对拍才非循环（若 interp
/// 的 ≺_Θ 序与 PDF P1..P8 优先级分叉，对拍会红，见 `tests::shadow_fold_bucket_equivalence`）。
///
/// 参数（`working`/`opened_slots` 是 interp fold 的**同一份**内部状态快照，§1a 修正——缺 `opened_slots`
/// 则 fold 内已开 slot 无法判 P7）：
/// - `working`：`(ActiveLeg, closed)`——A_t 的工作拷贝，`closed=true` 表本 fold 已关。
/// - `opened_slots`：`(level, dir)`——本 fold 已开过的 slot（interp.rs `opened`）。
///
/// **P1（风险强平）恒 false**——不在 D1 范围（codex-q2-d1 §3；由 `KThetaRiskGate` 上层兜）。
/// **P8（保持）不主动置位**——真实候选若无类/无向已落 record（C0），有类必触发 P2..P7 之一。
pub fn predicates_of(
    c: &Candidate,
    working: &[(ActiveLeg, bool)],
    opened_slots: &[(u32, VoiceSide)],
) -> Predicates {
    let mut p = Predicates::default();
    // 规则1 语义：非方向 / 无类候选 ⟹ 无买卖点动作 ⟹ C0（record）。
    if c.dir == VoiceSide::Flat || c.bsp_class == u8::MAX {
        return p; // 全 false ⟹ C0
    }
    // P2/P3 平多/平空：同级别有未关闭 Long/Short 腿 ∧ 候选携反向信号（reverse_signal 复用 §9）。
    let has_open_long =
        working.iter().any(|(l, closed)| !closed && l.level == c.level && l.dir == VoiceSide::Long);
    let has_open_short =
        working.iter().any(|(l, closed)| !closed && l.level == c.level && l.dir == VoiceSide::Short);
    p.close_long = has_open_long && super::exec::reverse_signal(VoiceSide::Long, &c.bits);
    p.close_short = has_open_short && super::exec::reverse_signal(VoiceSide::Short, &c.bits);
    if p.close_long || p.close_short {
        return p; // 平仓优先（P2/P3 ≺ 开仓/记录），mutex_class 取最小索引
    }
    // slot 占用：同级别同向未关闭腿 ∨ 本 fold 已开同 slot。
    let slot_occupied = working
        .iter()
        .any(|(l, closed)| !closed && l.level == c.level && l.dir == c.dir)
        || opened_slots.iter().any(|&(lv, d)| lv == c.level && d == c.dir);
    if slot_occupied {
        p.same_slot_record = true; // P7：同向 slot 已占 ⟹ 记录（无加仓子动作，见字段诚实声明）
        return p;
    }
    // slot 空 ⟹ 开仓。P4 开短差（ShortDiff 角色）/ P5 开多 / P6 开空——三者同属 Open 桶。
    match c.role.v {
        super::coverage::Vertical::ShortDiff => p.open_short_diff = true, // P4
        _ => match c.dir {
            VoiceSide::Long => p.open_long = true,   // P5
            VoiceSide::Short => p.open_short = true, // P6
            VoiceSide::Flat => {}                    // 不可达（上方已 return）
        },
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ★可证伪核心（alpha2 Doc2§9 定理 1）：穷举全部 2^8=256 谓词组合，断言每个组合
    /// **恰好一个** C_j 成立（`Σ_{j=0}^8 1[C_j]=1`）。若存在组合 Σ≠1 则 fail。
    ///
    /// 互斥化由 [`mutex_class`] 产唯一 [`MutexClass`]——本测试独立重算每个 C_j 的指示函数
    /// `1[C_j]`（不调 `mutex_class`，避免循环论证），逐组合求和断言 ==1，并交叉验证求和命中的
    /// 那个 j 与 `mutex_class` 返回的类号一致（实装 == 独立定义）。
    #[test]
    fn mutex_total_exhaustive_2pow8() {
        for bits in 0u32..(1 << M) {
            let p = decode(bits);

            // 独立重算指示函数（不调 mutex_class）。
            // 1[C_j] = P_j ∧ ⋀_{k<j} ¬P_k；1[C_0] = ⋀_j ¬P_j。
            let mut sum = 0usize;
            let mut hit_class: Option<MutexClass> = None;

            // C_0 兜底。
            let any = (1..=M).any(|j| p.p(j));
            if !any {
                sum += 1;
                hit_class = Some(MutexClass::C0);
            }
            // C_j。
            for j in 1..=M {
                let prefix_all_false = (1..j).all(|k| !p.p(k));
                let cj = p.p(j) && prefix_all_false;
                if cj {
                    sum += 1;
                    hit_class = Some(MutexClass::Cj(j as u8));
                }
            }

            assert_eq!(
                sum, 1,
                "bits={bits:08b}: Σ_j 1[C_j]={sum} ≠ 1（互斥性/全定义破裂，alpha2 定理 1 反例）"
            );
            // 实装 == 独立定义（交叉验证）。
            assert_eq!(
                mutex_class(&p),
                hit_class.unwrap(),
                "bits={bits:08b}: mutex_class 实装 ≠ 独立重算的命中类号"
            );
        }
    }

    /// 全定义：任意输入返回唯一 MutexClass（穷举已覆盖；此处显式断言无 panic + 全 false → C0）。
    #[test]
    fn mutex_total_definedness() {
        // 全 false → 兜底 C0。
        assert_eq!(mutex_class(&Predicates::default()), MutexClass::C0);
        // 穷举不 panic（决定性全函数）。
        for bits in 0u32..(1 << M) {
            let _ = mutex_class(&decode(bits));
        }
    }

    /// 优先级屏蔽：P1（风险强平）成立时屏蔽所有更高索引（即使 P2..P8 全 true）⟹ C_1。
    #[test]
    fn priority_p1_masks_all() {
        let p = Predicates {
            risk_liquidate: true,
            close_long: true,
            close_short: true,
            open_short_diff: true,
            open_long: true,
            open_short: true,
            same_slot_record: true,
            hold: true,
        };
        assert_eq!(mutex_class(&p), MutexClass::Cj(1), "P1 成立 ⟹ C_1（屏蔽 P2..P8）");
    }

    /// 优先级取最小成立索引：P1 false、P4 起为 true ⟹ C_4（P2/P3 false 不屏蔽，P4 最小成立）。
    #[test]
    fn priority_min_index() {
        let p = Predicates {
            open_short_diff: true, // P4
            open_long: true,       // P5
            hold: true,            // P8
            ..Default::default()
        };
        assert_eq!(mutex_class(&p), MutexClass::Cj(4), "最小成立索引 r=4 ⟹ C_4");
    }

    /// bits → Predicates 解码（bit j-1 ↔ P_j，1-based）。
    fn decode(bits: u32) -> Predicates {
        let b = |j: usize| (bits >> (j - 1)) & 1 == 1;
        Predicates {
            risk_liquidate: b(1),
            close_long: b(2),
            close_short: b(3),
            open_short_diff: b(4),
            open_long: b(5),
            open_short: b(6),
            same_slot_record: b(7),
            hold: b(8),
        }
    }

    // ──────────────────────────────────────────────────────────────────────
    //  D1 逐候选级等价 shadow-fold property test（codex-q2-d1 §1b）
    //  断言：interp::interpret 的 ≺_Θ 三桶归属 == mutex P1..P8 优先级桶归属（桶级）。
    //  桶级一致必须过；精确 Cj 一致为可选诊断（本测试只查桶级——§1c 第一层）。
    // ──────────────────────────────────────────────────────────────────────
    use super::super::super::classifier::recursive_tower::ElementId;
    use super::super::coverage::{Dir, Horizontal, OperationRole, Vertical};
    use super::super::exec::reverse_signal;
    use super::super::interp::{interpret, theta_key, ActiveLeg, Candidate};
    use super::super::super::types::BspBits;
    use super::VoiceSide;
    use std::collections::HashSet;

    fn role(v: Vertical) -> OperationRole {
        OperationRole { h: Horizontal::First, v, delta: Dir::Plus }
    }

    #[allow(clippy::too_many_arguments)]
    fn cand(
        gi: usize,
        level: u32,
        src: usize,
        dir: VoiceSide,
        bsp_class: u8,
        bits: BspBits,
        v: Vertical,
    ) -> Candidate {
        Candidate {
            level,
            source_index: src,
            bits,
            dir,
            bsp_class,
            role: role(v),
            nest_confirmed: true,
            gamma_index: gi,
        }
    }

    fn leg(level: u32, dir: VoiceSide, src: usize) -> ActiveLeg {
        ActiveLeg {
            level,
            dir,
            source_index: src,
            lambda: src,
            id: ElementId { level, ordinal: src as u64 },
            parent_id: None,
            is_boundary_root: true,
            op_parent: None,
        }
    }

    fn buy(k: u8) -> BspBits {
        match k {
            1 => BspBits { buy1: true, ..Default::default() },
            2 => BspBits { buy2: true, ..Default::default() },
            _ => BspBits { buy3: true, ..Default::default() },
        }
    }
    fn sell(k: u8) -> BspBits {
        match k {
            1 => BspBits { sell1: true, ..Default::default() },
            2 => BspBits { sell2: true, ..Default::default() },
            _ => BspBits { sell3: true, ..Default::default() },
        }
    }

    /// shadow fold：按 theta_key 排序，逐候选对拍 mutex 桶 vs 真 interp 桶。interp 桶从
    /// `interpret` 聚合输出按 `gamma_index` 反查（open/record 桶含候选；否则该候选被消费为关闭者
    /// ⟹ Close）。shadow `working`/`opened` 用 authoritative interp 桶推进（镜像 interp 规则2/3）。
    fn assert_bucket_equiv(gamma: &[Candidate], active: &[ActiveLeg]) {
        let real = interpret(gamma, active);
        let open_idx: HashSet<usize> = real.open.iter().map(|c| c.gamma_index).collect();
        let rec_idx: HashSet<usize> = real.record.iter().map(|c| c.gamma_index).collect();

        let mut ordered: Vec<&Candidate> = gamma.iter().collect();
        ordered.sort_by(|a, b| theta_key(a).cmp(&theta_key(b)));

        let mut working: Vec<(ActiveLeg, bool)> = active.iter().map(|&l| (l, false)).collect();
        let mut opened: Vec<(u32, VoiceSide)> = Vec::new();

        for c in ordered {
            let mbucket = bridge_bucket(mutex_class(&predicates_of(c, &working, &opened)));
            let ibucket = if open_idx.contains(&c.gamma_index) {
                ActionBucket::Open
            } else if rec_idx.contains(&c.gamma_index) {
                ActionBucket::Record
            } else {
                ActionBucket::Close // 候选被消费为关闭者（不在 open/record 桶）
            };
            assert_eq!(
                mbucket, ibucket,
                "候选 gamma_index={} 桶级分叉：mutex(P1..P8)={:?} vs interp(≺_Θ)={:?}（真矛盾 ⟹ /escalate）",
                c.gamma_index, mbucket, ibucket
            );
            // 推进 shadow 状态镜像 interp（authoritative = ibucket）。
            match ibucket {
                ActionBucket::Close => {
                    if let Some(i) = working
                        .iter()
                        .position(|(l, cl)| !cl && l.level == c.level && reverse_signal(l.dir, &c.bits))
                    {
                        working[i].1 = true;
                    }
                }
                ActionBucket::Open => opened.push((c.level, c.dir)),
                ActionBucket::Record => {}
            }
        }
    }

    /// ★D1 桶级等价（生成域覆盖：空/同级同向/同级反向/双向腿/跨级/fold内重复slot/ShortDiff/无类无向）。
    #[test]
    fn shadow_fold_bucket_equivalence() {
        use Vertical::{Ambient, ShortDiff};
        let l = VoiceSide::Long;
        let s = VoiceSide::Short;
        let f = VoiceSide::Flat;

        // S1 空 active，两 slot 开仓（跨级 + 多空）。
        assert_bucket_equiv(
            &[cand(0, 0, 10, l, 1, buy(1), Ambient), cand(1, 1, 11, s, 1, sell(1), Ambient)],
            &[],
        );
        // S2 fold 内重复同 slot：第一开，第二记录（slot_this_fold）。
        assert_bucket_equiv(
            &[cand(0, 0, 10, l, 1, buy(1), Ambient), cand(1, 0, 12, l, 2, buy(2), Ambient)],
            &[],
        );
        // S3 反向平仓：持多 L0 + 卖候选 ⟹ close。
        assert_bucket_equiv(&[cand(0, 0, 20, s, 1, sell(1), Ambient)], &[leg(0, l, 5)]);
        // S4 同向 slot 占用 ⟹ record（buy 候选不反向持多腿）。
        assert_bucket_equiv(&[cand(0, 0, 20, l, 1, buy(1), Ambient)], &[leg(0, l, 5)]);
        // S5 无类无向候选 ⟹ record（规则1 / C0）。
        assert_bucket_equiv(&[cand(0, 0, 20, f, u8::MAX, BspBits::default(), Ambient)], &[]);
        // S6 ShortDiff 角色开仓（P4）⟹ open。
        assert_bucket_equiv(&[cand(0, 0, 20, s, 1, sell(1), ShortDiff)], &[]);
        // S7 跨级同向不冲突：持多 L0 + buy L1 ⟹ open（不同 slot）。
        assert_bucket_equiv(&[cand(0, 1, 20, l, 1, buy(1), Ambient)], &[leg(0, l, 5)]);
        // S8 双向腿共存 + 反向候选：sell 候选反向平掉 Long 腿（P2 优先）⟹ close。
        assert_bucket_equiv(&[cand(0, 0, 20, s, 1, sell(1), Ambient)], &[leg(0, l, 5), leg(0, s, 6)]);
        // S9 多候选同刻混合：L0 反向平 + L1 开 + L0 无类记录（三桶同刻）。
        assert_bucket_equiv(
            &[
                cand(0, 0, 30, s, 1, sell(1), Ambient),        // 平 L0 Long
                cand(1, 1, 31, l, 1, buy(1), Ambient),          // 开 L1
                cand(2, 0, 32, f, u8::MAX, BspBits::default(), Ambient), // 记录
            ],
            &[leg(0, l, 5)],
        );
    }
}
