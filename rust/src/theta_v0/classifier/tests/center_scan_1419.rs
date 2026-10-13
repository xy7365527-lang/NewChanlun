//! #1419 TB-02-E：共用扫描 `detect_centers_windowed_resume` 的行为锁。
//!
//! - R8（#812 Z-5，`zhongshu.md` §6.1）：中枢结算后从 last_member+2 续扫，突破段是连接段、不进任何中枢；
//!   窗口开放（延伸吃到序列末尾、没有突破段）时游标停在末尾，不越界。
//! - #804 L2：同一扫描的增量（frontier 协议：pop 末窗 + 自 `resume_from` 重扫）与全量逐位相同。
//! - Zn 外缘（#1419 S2，D-A/D-B，`020-第20课.md:52`）：DD/GG 只聚合与 seed 第一单元同向的成员；
//!   L0 端点相连交替三段上与旧三段外缘恒等，延伸期的非 Zn 成员不再改外缘。
//! - 走势区间 ≠ 中枢外缘（#1419 根裁定 r2 第 1 条）：上级走势的区间取全部子走势极值，中枢外缘只数 Zn。
//! - 扫描事实内核（#1419 S4，#804 L2）：`scan_center_facts` 配 `NoopSink`（塔）与 `FactSink`（S 事实层）
//!   输出逐位相同；事实派生的中枢等于塔中枢；具名切面的六变量、贡献成员、归属、续扫锚等于 oracle。
//!
//! 期待值取自独立工作例（`tb1419-prep/worked-examples.json` 的 UP/DOWN 流，与
//! `.chanlun/review-results/issue1419-tb02e-20261013/` 的 oracle 冻结稿交叉核），不从被测函数推出。
//! E4（结算后 last+2 处 seed 相切失败、+1 后成立、连接段两单元）为本文件手算。

use super::super::super::types::{Center, Direction, Tick};
use super::super::center::{
    center_from_segments, center_from_window, compute_dd, compute_gg, UnitRange,
};
use super::super::center::{CoreOrder, RoleVar, SeedExclusion, SeedRule};
use super::super::center_scan::{
    scan_center_facts_full, FactSink, NoopSink, ScanState, SeedFromSelf, UnitAssignment, WaitReason,
};
use super::super::recursive_tower::{
    compose_level, detect_centers_windowed_resume, project_to_units, scan_center_facts, ElementId,
    LeveledMove,
};

type Build = fn(&UnitRange, &UnitRange, &UnitRange) -> Option<Center>;

/// 端点相连、方向由相邻端点决定的 L0 单元序列（u_k = P_k → P_{k+1}）。
fn units_from_endpoints(p: &[Tick]) -> Vec<UnitRange> {
    p.windows(2)
        .enumerate()
        .map(|(k, w)| UnitRange {
            start_index: k * 4,
            end_index: k * 4 + 4,
            direction: if w[1] > w[0] {
                Direction::Up
            } else {
                Direction::Down
            },
            lo: w[0].min(w[1]),
            hi: w[0].max(w[1]),
        })
        .collect()
}

/// 每个产出中枢的 (成员窗口 (首,末) 单元下标, ZD, ZG)。
fn windows(units: &[UnitRange], build: Build) -> Vec<((usize, usize), Tick, Tick)> {
    detect_centers_windowed_resume(units, build, 0)
        .0
        .iter()
        .map(|(c, w)| (*w, c.zd, c.zg))
        .collect()
}

/// UP 流（PREP E2+E3）：u0..u12 完成、u13 进行中。切面 K_k = u0..u_k 完成。
const UP: [Tick; 15] = [10, 40, 20, 35, 30, 38, 21, 25, 5, 15, 2, 12, 4, 9, 6];

/// K_k 切面的完成单元（u0..=u_k）。
fn up_cut(k: usize) -> Vec<UnitRange> {
    units_from_endpoints(&UP[..k + 2])
}

fn down_cut(k: usize) -> Vec<UnitRange> {
    let mirrored: Vec<Tick> = UP[..k + 2].iter().map(|p| 45 - p).collect();
    units_from_endpoints(&mirrored)
}

/// K10：A（u0..u7，核心 [20,35]）在 K8 由 u8 结算；续扫锚 = 7+2 = 9，u9..u11 尚不足三段
/// ⟹ 只有 A。旧 B 口径（`i = j`，从突破段 u8 续扫）在此已发布 B′ = (u8,u9,u10)、核心 [5,12]。
#[test]
fn r8_e3_k10_has_only_center_a() {
    assert_eq!(
        windows(&up_cut(10), center_from_segments),
        vec![((0, 7), 20, 35)]
    );
}

/// K11：B 的 seed = (u9,u10,u11) = ↓[2,15] ↑[2,12] ↓[4,12]，核心 [4,12]；突破段 u8 不进任何中枢。
#[test]
fn r8_e3_k11_second_center_seeds_at_last_plus_two() {
    assert_eq!(
        windows(&up_cut(11), center_from_segments),
        vec![((0, 7), 20, 35), ((9, 11), 4, 12)]
    );
}

/// K12：u12 ↑[4,9] 与 [4,12] 重叠 ⟹ B 成员 u9..u12（未结算）。
#[test]
fn r8_e3_k12_second_center_extends_without_the_connector() {
    assert_eq!(
        windows(&up_cut(12), center_from_segments),
        vec![((0, 7), 20, 35), ((9, 12), 4, 12)]
    );
}

/// DOWN 镜像（p' = 45 − p）：窗口下标不变，核心镜像。
#[test]
fn r8_down_mirror_k12() {
    assert_eq!(
        windows(&down_cut(12), center_from_segments),
        vec![((0, 7), 10, 25), ((9, 12), 33, 41)]
    );
    assert_eq!(
        windows(&down_cut(10), center_from_segments),
        vec![((0, 7), 10, 25)]
    );
}

/// E4：A 同 UP 流（u0..u7，u8 ↑[5,15] 结算）；续扫锚 9 的 seed (u9,u10,u11) = ↓[2,15] ↑[2,18] ↓[15,18]
/// ⟹ ZD = ZG = 15 相切不成立（Z-2），+1；seed (u10,u11,u12) = ↑[2,18] ↓[15,18] ↑[15,17] ⟹ 核心 [15,17]。
/// 连接段 = u8、u9 两单元。旧 B 口径从 u8 续扫得 B′ = u8..u12、核心 [5,15]。
#[test]
fn r8_e4_tangent_seed_at_anchor_steps_one_and_connector_has_two_units() {
    let e4: [Tick; 14] = [10, 40, 20, 35, 30, 38, 21, 25, 5, 15, 2, 18, 15, 17];
    let units = units_from_endpoints(&e4);
    assert_eq!(units.len(), 13);
    assert!(center_from_segments(&units[9], &units[10], &units[11]).is_none());
    assert_eq!(
        windows(&units, center_from_segments),
        vec![((0, 7), 20, 35), ((10, 12), 15, 17)]
    );
}

/// 游标：结算且突破段恰为最后一个完成单元（K8）⟹ 扫描越过突破段、`consumed == len`；
/// 窗口开放（K6，延伸吃到末尾、无突破段）⟹ 停在末尾，不得越界到 `len + 1`。
#[test]
fn r8_cursor_settled_skips_breakout_and_open_window_stays_in_range() {
    let k8 = up_cut(8);
    let (_, _, cur) = detect_centers_windowed_resume(&k8, center_from_segments, 0);
    assert_eq!(
        (cur.consumed, cur.resume_from, cur.last_window_emitted),
        (9, 0, 1)
    );
    let k6 = up_cut(6);
    let (out, _, cur) = detect_centers_windowed_resume(&k6, center_from_segments, 0);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].1, (0, 6));
    assert_eq!(
        (cur.consumed, cur.resume_from, cur.last_window_emitted),
        (7, 0, 1)
    );
}

/// 确定性伪随机：端点相连、方向严格交替的 L0 序列（幅度 1..=9，偶有大跳，产生结算、续扫、相切与开放窗口）。
fn alternating_endpoints(seed: u64, len: usize) -> Vec<Tick> {
    let mut s = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let mut next = || {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (s >> 33) as i64
    };
    let mut p: Vec<Tick> = vec![50];
    for k in 0..len {
        let step = 1 + next().rem_euclid(9) + if next().rem_euclid(7) == 0 { 15 } else { 0 };
        let last = *p.last().unwrap();
        p.push(if k % 2 == 0 { last + step } else { last - step });
    }
    p
}

/// #804 L2：任意切分点 k，前缀扫描 + frontier 协议（pop 末窗 `last_window_emitted` 个中枢、自 `resume_from`
/// 重扫）拼接后与全量逐位相同；L0（完整判据）与上级（几何判据）两个 build 都验。
#[test]
fn r8_incremental_frontier_resume_equals_full_scan() {
    let builds: [(&str, Build); 2] = [("l0", center_from_segments), ("geom", center_from_window)];
    let mut settled_resumes = 0usize;
    for seed in 0..60u64 {
        let units = units_from_endpoints(&alternating_endpoints(seed, 24));
        for (name, build) in builds {
            let (full, full_meta, _) = detect_centers_windowed_resume(&units, build, 0);
            for k in 0..=units.len() {
                let (mut pre, mut pre_meta, cur) =
                    detect_centers_windowed_resume(&units[..k], build, 0);
                assert!(cur.consumed <= k, "{name} seed={seed} k={k}: consumed 越界");
                if cur.resume_from < cur.consumed {
                    if !pre.is_empty() && cur.consumed > pre.last().unwrap().1 .1 + 1 {
                        settled_resumes += 1;
                    }
                    for _ in 0..cur.last_window_emitted {
                        pre.pop();
                        pre_meta.pop();
                    }
                }
                let (tail, tail_meta, _) =
                    detect_centers_windowed_resume(&units, build, cur.resume_from);
                pre.extend(tail);
                pre_meta.extend(tail_meta);
                assert_eq!(
                    pre, full,
                    "{name} seed={seed} k={k}: 增量拼接 centers ≠ 全量"
                );
                assert_eq!(
                    pre_meta, full_meta,
                    "{name} seed={seed} k={k}: 增量拼接 WinMeta ≠ 全量"
                );
            }
        }
    }
    assert!(
        settled_resumes > 0,
        "随机域须含『结算后续扫』的切分点（非真空）"
    );
}

// ── #1419 S2：Zn 外缘 ────────────────────────────────────────────────────────────────

/// 一个中枢的 (成员窗口, ZD, ZG, DD, GG)。
type Row = ((usize, usize), Tick, Tick, Tick, Tick);

fn rows(units: &[UnitRange], build: Build) -> Vec<Row> {
    detect_centers_windowed_resume(units, build, 0)
        .0
        .iter()
        .map(|(c, w)| (*w, c.zd, c.zg, c.dd, c.gg))
        .collect()
}

/// 逐切面比对：K_k = u0..u_k 完成（端点取前 k+2 个）。
fn assert_cuts(name: &str, endpoints: &[Tick], cuts: &[(usize, &[Row])]) {
    for &(k, want) in cuts {
        let units = units_from_endpoints(&endpoints[..k + 2]);
        assert_eq!(
            rows(&units, center_from_segments),
            want.to_vec(),
            "{name} K{k}"
        );
    }
}

// 下列五条流与逐切面期待值由脚本自 S0 oracle 冻结稿
// `.chanlun/review-results/issue1419-tb02e-20261013/worked-examples.json`（sha256 `7a7064fb…`）
// 的 `streams.*.cuts[*].centers[*]`（first/last 与 roles.CORE.ZD/ZG、roles.OUTER.DD/GG）转录；
// oracle 口径 `ZN_OUTER_C_RESUME`（Zn 外缘 + last+2 续扫）。W9 属 ≥9 升级域（D5，归 #1429），不在此表。

const ORACLE_UP: [Tick; 15] = [10, 40, 20, 35, 30, 38, 21, 25, 5, 15, 2, 12, 4, 9, 6];
const ORACLE_UP_CUTS: &[(usize, &[Row])] = &[
    (1, &[]),
    (2, &[((0, 2), 20, 35, 10, 40)]),
    (3, &[((0, 3), 20, 35, 10, 40)]),
    (4, &[((0, 4), 20, 35, 10, 40)]),
    (5, &[((0, 5), 20, 35, 10, 40)]),
    (6, &[((0, 6), 20, 35, 10, 40)]),
    (7, &[((0, 7), 20, 35, 10, 40)]),
    (8, &[((0, 7), 20, 35, 10, 40)]),
    (9, &[((0, 7), 20, 35, 10, 40)]),
    (10, &[((0, 7), 20, 35, 10, 40)]),
    (11, &[((0, 7), 20, 35, 10, 40), ((9, 11), 4, 12, 2, 15)]),
    (12, &[((0, 7), 20, 35, 10, 40), ((9, 12), 4, 12, 2, 15)]),
];

const ORACLE_DOWN: [Tick; 15] = [35, 5, 25, 10, 15, 7, 24, 20, 40, 30, 43, 33, 41, 36, 39];
const ORACLE_DOWN_CUTS: &[(usize, &[Row])] = &[
    (1, &[]),
    (2, &[((0, 2), 10, 25, 5, 35)]),
    (3, &[((0, 3), 10, 25, 5, 35)]),
    (4, &[((0, 4), 10, 25, 5, 35)]),
    (5, &[((0, 5), 10, 25, 5, 35)]),
    (6, &[((0, 6), 10, 25, 5, 35)]),
    (7, &[((0, 7), 10, 25, 5, 35)]),
    (8, &[((0, 7), 10, 25, 5, 35)]),
    (9, &[((0, 7), 10, 25, 5, 35)]),
    (10, &[((0, 7), 10, 25, 5, 35)]),
    (11, &[((0, 7), 10, 25, 5, 35), ((9, 11), 33, 41, 30, 43)]),
    (12, &[((0, 7), 10, 25, 5, 35), ((9, 12), 33, 41, 30, 43)]),
];

const ORACLE_E4: [Tick; 13] = [10, 40, 20, 35, 5, 15, 2, 18, 15, 17, 1, 8, 6];
const ORACLE_E4_CUTS: &[(usize, &[Row])] = &[
    (1, &[]),
    (2, &[((0, 2), 20, 35, 10, 40)]),
    (3, &[((0, 3), 20, 35, 10, 40)]),
    (4, &[((0, 3), 20, 35, 10, 40)]),
    (5, &[((0, 3), 20, 35, 10, 40)]),
    (6, &[((0, 3), 20, 35, 10, 40)]),
    (7, &[((0, 3), 20, 35, 10, 40)]),
    (8, &[((0, 3), 20, 35, 10, 40), ((6, 8), 15, 17, 2, 18)]),
    (9, &[((0, 3), 20, 35, 10, 40), ((6, 9), 15, 17, 2, 18)]),
    (10, &[((0, 3), 20, 35, 10, 40), ((6, 9), 15, 17, 2, 18)]),
];

const ORACLE_E4_DOWN: [Tick; 13] = [35, 5, 25, 10, 40, 30, 43, 27, 30, 28, 44, 37, 39];
const ORACLE_E4_DOWN_CUTS: &[(usize, &[Row])] = &[
    (1, &[]),
    (2, &[((0, 2), 10, 25, 5, 35)]),
    (3, &[((0, 3), 10, 25, 5, 35)]),
    (4, &[((0, 3), 10, 25, 5, 35)]),
    (5, &[((0, 3), 10, 25, 5, 35)]),
    (6, &[((0, 3), 10, 25, 5, 35)]),
    (7, &[((0, 3), 10, 25, 5, 35)]),
    (8, &[((0, 3), 10, 25, 5, 35), ((6, 8), 28, 30, 27, 43)]),
    (9, &[((0, 3), 10, 25, 5, 35), ((6, 9), 28, 30, 27, 43)]),
    (10, &[((0, 3), 10, 25, 5, 35), ((6, 9), 28, 30, 27, 43)]),
];

const ORACLE_LEAD: [Tick; 8] = [10, 40, 5, 9, 3, 20, 12, 15];
const ORACLE_LEAD_CUTS: &[(usize, &[Row])] = &[
    (1, &[]),
    (2, &[]),
    (3, &[((1, 3), 5, 9, 3, 40)]),
    (4, &[((1, 4), 5, 9, 3, 40)]),
    (5, &[((1, 4), 5, 9, 3, 40)]),
];

/// E2 判别点（UP K7）：u7 ↓[5,25] 触及核心 [20,35] 进中枢，但它与 seed 首单元 u0 ↑ 反向、不是 Zn
/// ⟹ DD 仍是 Zn {u0,u2,u4,u6} 的最低点 10（u0），不被 u7 拉到 5。DOWN 镜像：GG 35，不是 40。
#[test]
fn zn_e2_k7_non_zn_member_does_not_move_outer() {
    let up = rows(&up_cut(7), center_from_segments);
    assert_eq!(up, vec![((0, 7), 20, 35, 10, 40)], "UP K7：DD=10");
    let down = rows(&down_cut(7), center_from_segments);
    assert_eq!(down, vec![((0, 7), 10, 25, 5, 35)], "DOWN K7：GG=35");
}

#[test]
fn zn_outer_matches_oracle_up_and_down_streams() {
    assert_cuts("UP", &ORACLE_UP, ORACLE_UP_CUTS);
    assert_cuts("DOWN", &ORACLE_DOWN, ORACLE_DOWN_CUTS);
}

/// E4：B = u6..u9，Zn = ↑{u6,u8}；u9 ↓[1,17] 进中枢但非 Zn ⟹ DD 仍 2（塔旧值 1）。镜像 GG 43（旧 44）。
#[test]
fn zn_outer_matches_oracle_e4_streams() {
    assert_cuts("E4", &ORACLE_E4, ORACLE_E4_CUTS);
    assert_cuts("E4_DOWN", &ORACLE_E4_DOWN, ORACLE_E4_DOWN_CUTS);
}

/// LEAD：seed(0) ZD>ZG 失败，中枢 A 自 u1（↓）起，Zn = ↓{u1,u3}；u4 ↑[3,20] 进中枢、非 Zn。
#[test]
fn zn_outer_matches_oracle_lead_stream() {
    assert_cuts("LEAD", &ORACLE_LEAD, ORACLE_LEAD_CUTS);
}

/// D-B 性质锁：L0 端点相连、方向交替的三段上，Zn seed 外缘 == 三段外缘（中间段两端点分别是首、末段的
/// 端点）。穷举端点 0..=8 的全部交替四点序列；`level_view/projection.rs` 经同一 `compute_dd/gg` 取外缘，
/// 故 L0 买卖点投影不因 Zn 口径改变。
#[test]
fn zn_seed_outer_equals_three_segment_outer_on_connected_alternating_l0() {
    let mut checked = 0usize;
    let mut formed = 0usize;
    for p0 in 0..=8 {
        for p1 in 0..=8 {
            for p2 in 0..=8 {
                for p3 in 0..=8 {
                    let p: [Tick; 4] = [p0, p1, p2, p3];
                    let up_first = p1 > p0 && p2 < p1 && p3 > p2;
                    let down_first = p1 < p0 && p2 > p1 && p3 < p2;
                    if !(up_first || down_first) {
                        continue;
                    }
                    let u = units_from_endpoints(&p);
                    let (a, b, c) = (&u[0], &u[1], &u[2]);
                    let gg3 = a.hi.max(b.hi).max(c.hi);
                    let dd3 = a.lo.min(b.lo).min(c.lo);
                    assert_eq!(compute_gg(a, b, c), gg3, "{p:?}: GG");
                    assert_eq!(compute_dd(a, b, c), dd3, "{p:?}: DD");
                    if let Some(center) = center_from_segments(a, b, c) {
                        assert_eq!((center.dd, center.gg), (dd3, gg3), "{p:?}: seed 中枢外缘");
                        formed += 1;
                    }
                    checked += 1;
                }
            }
        }
    }
    assert!(
        checked > 0 && formed > 0,
        "穷举域非空（checked={checked} formed={formed}）"
    );
}

fn unit_at(k: usize, direction: Direction, lo: Tick, hi: Tick) -> UnitRange {
    UnitRange {
        start_index: k * 4,
        end_index: k * 4 + 4,
        direction,
        lo,
        hi,
    }
}

/// D-A 判别例（上级不交替输入，几何 build）：Zn 判据是「与 seed 首单元方向相等」，不是位次奇偶。
/// seed = ↑[10,40] ↑[20,50] ↓[0,30]：方向判据下 Zn = {u0,u1} ⟹ GG=50、DD=10；奇偶判据会取 {u0,u2}
/// ⟹ GG=40、DD=0。延伸成员 u3 ↓[5,25] 非 Zn ⟹ 外缘不动；u4 ↑[15,60] 是 Zn ⟹ GG=60。
#[test]
fn zn_upper_level_uses_direction_not_parity() {
    use Direction::{Down, Up};
    let units = vec![
        unit_at(0, Up, 10, 40),
        unit_at(1, Up, 20, 50),
        unit_at(2, Down, 0, 30),
    ];
    assert_eq!(
        rows(&units, center_from_window),
        vec![((0, 2), 20, 30, 10, 50)]
    );
    let mut extended = units.clone();
    extended.push(unit_at(3, Down, 5, 25));
    assert_eq!(
        rows(&extended, center_from_window),
        vec![((0, 3), 20, 30, 10, 50)]
    );
    extended.push(unit_at(4, Up, 15, 60));
    assert_eq!(
        rows(&extended, center_from_window),
        vec![((0, 4), 20, 30, 10, 60)]
    );
}

// ── #1419 根裁定 r2 第 1 条：走势区间与中枢外缘分开 ─────────────────────────────────────

/// UP K12：中枢 A = u0..u7，Zn = ↑{u0,u2,u4,u6} ⟹ 外缘 [10,40]；但走势 A 的末成员 u7 ↓[5,25] 的低点 5
/// 是走势的实际极值 ⟹ 走势区间 [5,40]。上级投影单元、`envelope()` 缓存与递归 `rmove.lo()/hi()` 三者
/// 都取走势区间（S2 时 `envelope()` 读中枢外缘，此处给 [10,40]）。
#[test]
fn r2_move_interval_is_all_sub_extremes_not_center_outer() {
    let units = up_cut(12);
    let moves: Vec<LeveledMove> = units
        .iter()
        .enumerate()
        .map(|(i, u)| {
            LeveledMove::from_unit(
                u,
                ElementId {
                    level: 0,
                    ordinal: i as u64,
                },
            )
        })
        .collect();
    let (centers, upper, _) = compose_level(&units, &moves, true, 1);
    assert_eq!(
        (centers[0].dd, centers[0].gg),
        (10, 40),
        "中枢 A 外缘只数 Zn"
    );
    assert_eq!(
        upper[0].envelope(),
        (5, 40),
        "走势 A 区间含非 Zn 末成员 u7 的低点"
    );
    for m in &upper {
        assert_eq!(m.envelope(), (m.rmove.lo(), m.rmove.hi()), "缓存 == 递归");
    }
    let projected = project_to_units(&upper, &[]);
    assert_eq!(
        (projected[0].lo, projected[0].hi),
        (5, 40),
        "上级单元 = 走势区间"
    );
}

// ── #1419 S4：扫描事实内核 ────────────────────────────────────────────────────────────

/// #804 L2：同一输入上 `NoopSink`（塔）与 `FactSink`（S 事实层）的扫描输出逐位相同；事实与塔中枢一致：
/// <9 成员窗口的六变量投影 == 塔中枢；每次 seed 评估的 `formed` == 该事实按口径的投影；成立的评估与窗口一一
/// 对应；结算窗口之后的下一次评估在 last+2（R8）。L0 与上级两个 build 都验。
#[test]
fn s4_fact_sink_scan_equals_tower_scan_and_projects_to_tower_centers() {
    let builds: [(&str, Build, SeedRule); 2] = [
        ("l0", center_from_segments, SeedRule::Complete),
        ("geom", center_from_window, SeedRule::Geometric),
    ];
    let (mut settled, mut failed, mut upgraded) = (0usize, 0usize, 0usize);
    for seed in 0..80u64 {
        let units = units_from_endpoints(&alternating_endpoints(seed, 30));
        for (name, build, rule) in builds {
            for k in [units.len(), units.len() / 2, 7] {
                let u = &units[..k.min(units.len())];
                let ctx = format!("{name} seed={seed} len={}", u.len());
                let tower = scan_center_facts(u, build, 0, &mut NoopSink);
                let mut sink = FactSink::default();
                let with_facts = scan_center_facts(u, build, 0, &mut sink);
                assert_eq!(tower, with_facts, "{ctx}: 两种 sink 的扫描输出不同");
                let facts = scan_center_facts_full(u, build);
                assert_eq!(
                    (&facts.centers, &facts.metas, &facts.cursor),
                    (&tower.0, &tower.1, &tower.2),
                    "{ctx}"
                );
                for a in &facts.attempts {
                    assert_eq!(
                        a.facts.center(rule).is_some(),
                        a.formed,
                        "{ctx}: at={}",
                        a.at
                    );
                    failed += usize::from(!a.formed);
                }
                let formed_at: Vec<usize> = facts
                    .attempts
                    .iter()
                    .filter(|a| a.formed)
                    .map(|a| a.at)
                    .collect();
                let window_at: Vec<usize> = facts.windows.iter().map(|w| w.seed_at).collect();
                assert_eq!(formed_at, window_at, "{ctx}: 成立评估与窗口不对应");
                let mut emitted = 0usize;
                for (w, wf) in facts.windows.iter().zip(facts.window_facts(u)) {
                    if wf.settled {
                        settled += 1;
                        if let Some(next) = facts.attempts.iter().find(|a| a.at > wf.last) {
                            assert_eq!(next.at, wf.last + 2, "{ctx}: 结算后续扫锚须 last+2");
                        }
                    }
                    if w.upgraded {
                        upgraded += 1;
                    } else {
                        let c = tower.0[emitted].0;
                        assert_eq!(
                            Center {
                                zd: wf.role(RoleVar::Zd).value,
                                zg: wf.role(RoleVar::Zg).value,
                                dd: wf.role(RoleVar::Dd).value,
                                gg: wf.role(RoleVar::Gg).value,
                                start_index: u[wf.first].start_index,
                                end_index: u[wf.last].end_index,
                            },
                            c,
                            "{ctx}: 事实投影 ≠ 塔中枢"
                        );
                        assert_eq!(tower.0[emitted].1, (wf.first, wf.last), "{ctx}");
                    }
                    emitted += w.emitted;
                }
                assert_eq!(emitted, tower.0.len(), "{ctx}");
                let unit_facts = facts.unit_facts();
                assert_eq!(unit_facts.len(), u.len(), "{ctx}: 每单元恰一状态");
            }
        }
    }
    assert!(
        settled > 0 && failed > 0 && upgraded > 0,
        "随机域须含结算、seed 失败与 ≥9 升级（{settled}/{failed}/{upgraded}）"
    );
}

/// 一个角色变量的 (值, 贡献成员)。
fn role(f: &super::super::center_scan::CenterWindowFacts, v: RoleVar) -> (Tick, Vec<usize>) {
    let r = f.role(v);
    (r.value, r.contributors.clone())
}

/// oracle UP K7：A 开放（u0..u7），Zn = u0,u2,u4,u6；D=30(u4)、G=25(u6)（D>G），DD/GG 只数 Zn = 10/40，
/// 全成员 dd=5 不进外缘；游标 EXTENDING、下一待测 u8。
#[test]
fn s4_up_k7_roles_and_contributors_match_oracle() {
    let units = up_cut(7);
    let facts = scan_center_facts_full(&units, center_from_segments);
    let wf = facts.window_facts(&units);
    assert_eq!(wf.len(), 1);
    let a = &wf[0];
    assert_eq!((a.first, a.last, a.count(), a.settled), (0, 7, 8, false));
    assert_eq!(a.next_tested_unit, Some(8));
    assert_eq!(a.zn_members, vec![0, 2, 4, 6]);
    assert_eq!(role(a, RoleVar::Zd), (20, vec![1, 2]));
    assert_eq!(role(a, RoleVar::Zg), (35, vec![2]));
    assert_eq!(role(a, RoleVar::Dd), (10, vec![0]));
    assert_eq!(role(a, RoleVar::Gg), (40, vec![0]));
    assert_eq!(role(a, RoleVar::D), (30, vec![4]));
    assert_eq!(role(a, RoleVar::G), (25, vec![6]));
    assert_eq!(a.role(RoleVar::Zd).same_price_witness(), &[1, 2]);
    assert!(a.role(RoleVar::Zg).same_price_witness().is_empty());
    assert_eq!(
        facts.scan_state(),
        ScanState::Extending {
            center: 0,
            next_unit: 8
        }
    );
}

/// oracle UP K10 与 K12：K10 只有 A（结算于 u8，break_up=false，续扫锚 9），u8 CONNECTOR(A→pending)，
/// u9/u10 待评估；K12 B = u9..u12 成立，u8 改为 CONNECTOR(A→B)，B 的六变量与贡献成员。
#[test]
fn s4_up_k10_k12_assignment_and_anchor_match_oracle() {
    let k10 = up_cut(10);
    let f10 = scan_center_facts_full(&k10, center_from_segments);
    let w10 = f10.window_facts(&k10);
    assert_eq!(w10.len(), 1);
    assert_eq!(
        (w10[0].break_unit, w10[0].break_up, w10[0].resume_anchor),
        (Some(8), Some(false), Some(9))
    );
    let a10 = f10.unit_facts();
    assert_eq!(
        a10[8].assignment,
        UnitAssignment::Connector {
            left: 0,
            right: None
        }
    );
    assert_eq!(a10[8].seed_from_self, SeedFromSelf::NotEvaluated);
    for k in [9, 10] {
        assert_eq!(
            a10[k].assignment,
            UnitAssignment::NotYetAssigned(WaitReason::SeedNotYetEvaluable),
            "u{k}"
        );
    }
    assert_eq!(
        f10.scan_state(),
        ScanState::SeekingSeed {
            seed_start: 9,
            awaiting_unit: 11
        }
    );

    let k12 = up_cut(12);
    let f12 = scan_center_facts_full(&k12, center_from_segments);
    let w12 = f12.window_facts(&k12);
    assert_eq!(w12.len(), 2);
    let b = &w12[1];
    assert_eq!((b.first, b.last, b.settled), (9, 12, false));
    assert_eq!(b.zn_members, vec![9, 11]);
    assert_eq!(role(b, RoleVar::Zd), (4, vec![11]));
    assert_eq!(role(b, RoleVar::Zg), (12, vec![10, 11]));
    assert_eq!(role(b, RoleVar::Dd), (2, vec![9]));
    assert_eq!(role(b, RoleVar::Gg), (15, vec![9]));
    assert_eq!(role(b, RoleVar::D), (4, vec![11]));
    assert_eq!(role(b, RoleVar::G), (12, vec![11]));
    let a12 = f12.unit_facts();
    assert!((0..=7).all(|k| a12[k].assignment == UnitAssignment::Member { center: 0 }));
    assert_eq!(
        a12[8].assignment,
        UnitAssignment::Connector {
            left: 0,
            right: Some(1)
        }
    );
    assert!((9..=12).all(|k| a12[k].assignment == UnitAssignment::Member { center: 1 }));
    assert_eq!(a12[9].seed_from_self, SeedFromSelf::Formed);
}

/// oracle E4 K7/K8：续扫锚 u5 的 seed 相切（ZD=ZG=15，TANGENT_ZD_EQ_ZG）⟹ +1；K7 时 u4、u5 CONNECTOR(A→pending)
/// （u5 seed_from_self=FAILED），u6、u7 待评估；K8 B 自 u6 成立，u4、u5 改为 CONNECTOR(A→B)。
#[test]
fn s4_e4_tangent_seed_failure_and_two_unit_connector_match_oracle() {
    const E4: [Tick; 13] = [10, 40, 20, 35, 5, 15, 2, 18, 15, 17, 1, 8, 6];
    let k7 = units_from_endpoints(&E4[..9]);
    let f7 = scan_center_facts_full(&k7, center_from_segments);
    let at: Vec<(usize, bool)> = f7.attempts.iter().map(|a| (a.at, a.formed)).collect();
    assert_eq!(at, vec![(0, true), (5, false)]);
    let tangent = &f7.attempts[1].facts;
    assert_eq!(
        (tangent.zd, tangent.zg, tangent.core_order),
        (15, 15, CoreOrder::ZdEqZg)
    );
    assert_eq!(
        tangent.exclusion(SeedRule::Complete),
        Some(SeedExclusion::TangentZdEqZg)
    );
    let a7 = f7.unit_facts();
    for k in [4, 5] {
        assert_eq!(
            a7[k].assignment,
            UnitAssignment::Connector {
                left: 0,
                right: None
            },
            "u{k}"
        );
    }
    assert_eq!(
        (a7[4].seed_from_self, a7[5].seed_from_self),
        (SeedFromSelf::NotEvaluated, SeedFromSelf::Failed)
    );
    for k in [6, 7] {
        assert_eq!(
            a7[k].assignment,
            UnitAssignment::NotYetAssigned(WaitReason::SeedNotYetEvaluable),
            "u{k}"
        );
    }

    let k8 = units_from_endpoints(&E4[..10]);
    let f8 = scan_center_facts_full(&k8, center_from_segments);
    let a8 = f8.unit_facts();
    for k in [4, 5] {
        assert_eq!(
            a8[k].assignment,
            UnitAssignment::Connector {
                left: 0,
                right: Some(1)
            },
            "u{k}"
        );
    }
    let b = &f8.window_facts(&k8)[1];
    assert_eq!(b.first, 6);
    assert_eq!(role(b, RoleVar::Zd), (15, vec![7, 8]));
    assert_eq!(role(b, RoleVar::Zg), (17, vec![8]));
}

/// oracle LEAD K5：seed(0) 不成立（ZD>ZG，INVERTED_ZD_GT_ZG），u0 记 NO_LEFT_CENTER_IN_WINDOW、seed_from_self=FAILED；
/// A = u1..u4，u5 向上突破（break_up=true），续扫锚 6。
#[test]
fn s4_lead_unit_before_first_center_matches_oracle() {
    const LEAD: [Tick; 8] = [10, 40, 5, 9, 3, 20, 12, 15];
    let k5 = units_from_endpoints(&LEAD[..7]);
    let f = scan_center_facts_full(&k5, center_from_segments);
    assert_eq!(
        f.attempts[0].facts.exclusion(SeedRule::Complete),
        Some(SeedExclusion::InvertedZdGtZg)
    );
    let u = f.unit_facts();
    assert_eq!(
        u[0].assignment,
        UnitAssignment::NotYetAssigned(WaitReason::NoLeftCenterInWindow)
    );
    assert_eq!(u[0].seed_from_self, SeedFromSelf::Failed);
    let a = &f.window_facts(&k5)[0];
    assert_eq!((a.first, a.last), (1, 4));
    assert_eq!(
        (a.break_unit, a.break_up, a.resume_anchor),
        (Some(5), Some(true), Some(6))
    );
    assert_eq!(
        u[5].assignment,
        UnitAssignment::Connector {
            left: 0,
            right: None
        }
    );
}

/// oracle W9 K8：第 9 个成员 u8 加入 ⟹ 窗口 upgrade_pending，u0..u8 记 UPGRADE_PENDING_TB02H（R3/D-I）。
#[test]
fn s4_w9_ninth_member_marks_upgrade_pending() {
    const W9: [Tick; 12] = [10, 40, 20, 35, 25, 38, 22, 36, 24, 37, 26, 28];
    let k7 = units_from_endpoints(&W9[..9]);
    let f7 = scan_center_facts_full(&k7, center_from_segments);
    assert!(!f7.windows[0].upgraded);
    let k8 = units_from_endpoints(&W9[..10]);
    let f8 = scan_center_facts_full(&k8, center_from_segments);
    assert!(f8.windows[0].upgraded);
    assert!(f8
        .unit_facts()
        .iter()
        .all(|u| u.assignment == UnitAssignment::NotYetAssigned(WaitReason::UpgradePendingTb02h)));
}

/// 全量 oracle 核对（证据性，非常驻锁）：逐流逐切面读 S0 冻结稿 `worked-examples.json`，核中枢集合（首末、结算、
/// 突破段、break_up、续扫锚、下一待测、Zn 成员、upgrade_pending、六变量值与贡献成员与同价见证）、扫描游标、
/// 每单元归属与 seed_from_self、流级 seed 尝试（起点、结果、排除原因、三叶、Bool⁵、ZD/ZG 贡献）。
/// 冻结稿是 review-results 工作草稿（活期随票），故 `#[ignore]`：`cargo test center_scan_1419 -- --ignored`。
#[test]
#[ignore = "读 review-results 冻结稿；收据见 tb1419-impl/receipts"]
fn s4_full_oracle_sweep_worked_examples() {
    use serde_json::Value;
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../.chanlun/review-results/issue1419-tb02e-20261013/worked-examples.json"
    );
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("读冻结稿"))
        .expect("冻结稿是 JSON");
    let ids = |idx: usize| ((b'A' + idx as u8) as char).to_string();
    let ints = |v: &Value| -> Vec<usize> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_u64().unwrap() as usize)
            .collect()
    };
    let opt_usize = |v: &Value| v.as_u64().map(|x| x as usize);
    let mut checked = 0usize;
    for (stream, st) in doc["streams"].as_object().unwrap() {
        let endpoints: Vec<Tick> = st["endpoints"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_i64().unwrap())
            .collect();
        let mut last_facts = None;
        for cut in st["cuts"].as_array().unwrap() {
            let k = cut["k"].as_u64().unwrap() as usize;
            let ctx = format!("{stream} {}", cut["cut"]);
            let units = units_from_endpoints(&endpoints[..k + 2]);
            let f = scan_center_facts_full(&units, center_from_segments);
            let wf = f.window_facts(&units);
            let centers = cut["centers"].as_array().unwrap();
            assert_eq!(wf.len(), centers.len(), "{ctx}: 中枢数");
            for (idx, (w, c)) in wf.iter().zip(centers).enumerate() {
                let cc = format!("{ctx} {}", ids(idx));
                assert_eq!(c["id"].as_str().unwrap(), ids(idx), "{cc}");
                assert_eq!(
                    ints(&c["seed"]),
                    vec![w.first, w.first + 1, w.first + 2],
                    "{cc}"
                );
                assert_eq!(
                    (w.first, w.last, w.count(), w.settled),
                    (
                        c["first"].as_u64().unwrap() as usize,
                        c["last"].as_u64().unwrap() as usize,
                        c["count"].as_u64().unwrap() as usize,
                        c["settled"].as_bool().unwrap()
                    ),
                    "{cc}"
                );
                assert_eq!(
                    w.break_unit,
                    opt_usize(&c["break_unit"]),
                    "{cc}: break_unit"
                );
                assert_eq!(w.break_up, c["break_up"].as_bool(), "{cc}: break_up");
                assert_eq!(w.resume_anchor, opt_usize(&c["resume_anchor"]), "{cc}");
                assert_eq!(
                    w.next_tested_unit,
                    opt_usize(&c["next_tested_unit"]),
                    "{cc}"
                );
                assert_eq!(w.zn_members, ints(&c["zn_members"]), "{cc}: Zn");
                assert_eq!(
                    w.upgrade_pending,
                    c["upgrade_pending"].as_bool().unwrap(),
                    "{cc}"
                );
                if w.upgrade_pending {
                    assert!(c["roles"].is_null(), "{cc}: ≥9 不发布六变量");
                    continue;
                }
                for var in RoleVar::ALL {
                    let o = &c["roles"][var.role()][var.as_str()];
                    let r = w.role(var);
                    assert_eq!(
                        r.value,
                        o["value"].as_i64().unwrap(),
                        "{cc} {}",
                        var.as_str()
                    );
                    assert_eq!(
                        r.contributors,
                        ints(&o["contributors"]),
                        "{cc} {} 贡献",
                        var.as_str()
                    );
                    assert_eq!(
                        r.same_price_witness().to_vec(),
                        ints(&o["same_price_witness"]),
                        "{cc} {} 同价",
                        var.as_str()
                    );
                    assert_eq!(var.frozen(), o["frozen"].as_bool().unwrap(), "{cc}");
                }
            }
            let sc = &cut["scan_cursor"];
            let want_state = match sc["state"].as_str().unwrap() {
                "EXTENDING" => ScanState::Extending {
                    center: (sc["center"].as_str().unwrap().as_bytes()[0] - b'A') as usize,
                    next_unit: sc["next_unit"].as_u64().unwrap() as usize,
                },
                "SEEKING_SEED" => ScanState::SeekingSeed {
                    seed_start: sc["seed_start"].as_u64().unwrap() as usize,
                    awaiting_unit: sc["awaiting_unit"].as_u64().unwrap() as usize,
                },
                other => panic!("{ctx}: 未知游标 {other}"),
            };
            assert_eq!(f.scan_state(), want_state, "{ctx}: 游标");
            let uf = f.unit_facts();
            for a in cut["assignment"].as_array().unwrap() {
                let k = a["unit"].as_u64().unwrap() as usize;
                let (got, sfs) = if k < uf.len() {
                    (uf[k].assignment, uf[k].seed_from_self)
                } else {
                    (
                        UnitAssignment::NotYetAssigned(WaitReason::UnitNotCompleted),
                        SeedFromSelf::NotEvaluated,
                    )
                };
                let want = match a["status"].as_str().unwrap() {
                    "CENTER_MEMBER" => UnitAssignment::Member {
                        center: (a["center"].as_str().unwrap().as_bytes()[0] - b'A') as usize,
                    },
                    "CONNECTOR" => UnitAssignment::Connector {
                        left: (a["left"].as_str().unwrap().as_bytes()[0] - b'A') as usize,
                        right: match a["right"].as_str().unwrap() {
                            "PENDING" => None,
                            r => Some((r.as_bytes()[0] - b'A') as usize),
                        },
                    },
                    "NOT_YET_ASSIGNED" => {
                        UnitAssignment::NotYetAssigned(match a["reason"].as_str().unwrap() {
                            "NO_LEFT_CENTER_IN_WINDOW" => WaitReason::NoLeftCenterInWindow,
                            "SEED_NOT_YET_EVALUABLE" => WaitReason::SeedNotYetEvaluable,
                            "UPGRADE_PENDING_TB02H" => WaitReason::UpgradePendingTb02h,
                            "UNIT_NOT_COMPLETED" => WaitReason::UnitNotCompleted,
                            r => panic!("{ctx}: 未知原因 {r}"),
                        })
                    }
                    st => panic!("{ctx}: 未知状态 {st}"),
                };
                assert_eq!(got, want, "{ctx} u{k}");
                assert_eq!(
                    sfs.as_str(),
                    a["seed_from_self"].as_str().unwrap(),
                    "{ctx} u{k} seed_from_self"
                );
                checked += 1;
            }
            last_facts = Some(f);
        }
        let f = last_facts.expect("流非空");
        let attempts = st["seed_attempts"].as_array().unwrap();
        assert_eq!(f.attempts.len(), attempts.len(), "{stream}: seed 尝试数");
        for (a, o) in f.attempts.iter().zip(attempts) {
            let ctx = format!("{stream} seed@{}", a.at);
            assert_eq!(a.at, o["start"].as_u64().unwrap() as usize, "{ctx}");
            assert_eq!(a.formed, o["result"] == "FORMED", "{ctx}");
            assert_eq!(
                a.facts
                    .exclusion(SeedRule::Complete)
                    .map(SeedExclusion::as_str),
                o["exclusion_reason"].as_str(),
                "{ctx}"
            );
            assert_eq!(
                a.facts.core_order.as_str(),
                o["core_order"].as_str().unwrap(),
                "{ctx}"
            );
            let v = a
                .facts
                .condition_vector(super::super::center::SeedInput::TOWER);
            let ov = &o["vector"];
            assert_eq!(
                (
                    v.completed,
                    v.consecutive,
                    v.same_level,
                    v.alternating,
                    v.strict
                ),
                (
                    ov["completed"].as_bool().unwrap(),
                    ov["consecutive"].as_bool().unwrap(),
                    ov["same_level"].as_bool().unwrap(),
                    ov["alternating"].as_bool().unwrap(),
                    ov["strict"].as_bool()
                ),
                "{ctx}: Bool⁵"
            );
            let units = units_from_endpoints(&endpoints[..a.at + 4]);
            let seed = &units[a.at..a.at + 3];
            let roles = super::super::center::CenterRoles::of_members(seed);
            for (var, key) in [(RoleVar::Zd, "ZD"), (RoleVar::Zg, "ZG")] {
                let got: Vec<usize> = roles
                    .contributors(seed, var)
                    .into_iter()
                    .map(|k| a.at + k)
                    .collect();
                assert_eq!(got, ints(&o[key]["contributors"]), "{ctx} {key}");
                assert_eq!(
                    roles.value(var),
                    o[key]["value"].as_i64().unwrap(),
                    "{ctx} {key}"
                );
            }
            checked += 1;
        }
    }
    assert!(checked > 300, "核对项 {checked}");
    eprintln!("s4_full_oracle_sweep_worked_examples: checked={checked}");
}
