//! #1419 TB-02-E：共用扫描 `detect_centers_windowed_resume` 的行为锁。
//!
//! - R8（#812 Z-5，`zhongshu.md` §6.1）：中枢结算后从 last_member+2 续扫，突破段是连接段、不进任何中枢；
//!   窗口开放（延伸吃到序列末尾、没有突破段）时游标停在末尾，不越界。
//! - #804 L2：同一扫描的增量（frontier 协议：pop 末窗 + 自 `resume_from` 重扫）与全量逐位相同。
//!
//! 期待值取自独立工作例（`tb1419-prep/worked-examples.json` 的 UP/DOWN 流，与
//! `.chanlun/review-results/issue1419-tb02e-20261013/` 的 oracle 冻结稿交叉核），不从被测函数推出。
//! E4（结算后 last+2 处 seed 相切失败、+1 后成立、连接段两单元）为本文件手算。

use super::super::super::types::{Center, Direction, Tick};
use super::super::center::{center_from_segments, center_from_window, UnitRange};
use super::super::recursive_tower::detect_centers_windowed_resume;

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
