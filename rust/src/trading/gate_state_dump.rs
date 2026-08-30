//! GUARD-ROLE: organic-fugue-v2-trading-layer——名分：现役（详见 `trading/mod.rs` 头部 GUARD-ROLE 块，#763 C7-E4 核定；零删除/零移入 legacy/）。
//!
//! tape v3 门状态列导出（#1312 · #1282 第二轮预注册执行件）— 纯 Rust，不走 PyO3。
//!
//! ## 职责
//!
//! 从 v2r 磁带（`analysis/_dump_tape_rust.py` 落盘）读事件流，用**生产函数本体**
//! 逐 bar 现算三列门状态，落盘 v3 门状态列文件供 Python 驱动侧读：
//!   - `l2_up_unexhausted`   = [`TrendExhaustion::up_unexhausted`]（41课门，#1267 结构判据版）
//!   - `l2_down_unexhausted` = [`TrendExhaustion::down_unexhausted`]（同门方向镜像）
//!   - `fatigue_state`       = [`FatigueGate`] 点态状态（#1264，C7 步骤5-iv 门开率可观测义务）
//!
//! **零新判据**：本模块不实现任何判定，只做「驱动 + 读数 + 列式落盘」。判据
//! 一律调生产函数（分叉防线：Python 侧不得重实现 41课门——#1312 票面设计要点）。
//!
//! ## 推进序（逐字复刻 `runner.rs` 主循环，同序同时点）
//!
//! ```text
//!   D3 方向行滚动推进 → TrendExhaustion::observe → CenterBook::ingest
//!   → CenterBook::negate_pending_departure → FatigueGate::observe → 读列
//! ```
//!
//! 读数时点在同 bar 全部推进之后——与 runner 把 `l41`/`book` 只读借用交给
//! FLAT/ARMED/LONG 分派的时点一致（`runner.rs` 主循环：三态分派在 ingest /
//! negate / fatigue 块之后）。`hard_type3` 取自在册变体 `V2r`（本磁带的在册
//! 消费配置，见 `trade_behavior.rs`）——不另立配置口径。
//!
//! ## fatigue 列的能力边界（090号声明=能力，不提供代理）
//!
//! [`FatigueGate`] 清空路径(1) 依赖磁带 `run_high` 行；当前信号层
//! （`organic_signals.py`）不产出该行（`SignalTape::run_high = None`，见
//! `tape.rs` 行注），生产 runner 对 `rev_gate=true` 在入口 fail-fast。本模块
//! 同守该界：`run_high` 缺失 ⇒ fatigue 列整列写
//! [`FATIGUE_UNAVAILABLE`] 哨兵，**不**用 close 代理 run_high（代理 = 双真相源，
//! `fatigue_gate.rs` 头部明令）。消费侧（Python 可选配置臂）见列值即 fail-fast。
//!
//! ## 运行（长测试，默认 ignore）
//!
//! ```text
//!   # 前置：analysis/_dump_tape_rust.py 已落 _tape_v2r_{SYM}.bin
//!   cargo test --release gate_state_dump_prereg7 -- --ignored --nocapture
//! ```
#![cfg(test)]

use std::fs;
use std::path::PathBuf;

use super::center_book::CenterBook;
use super::config::variant;
use super::fatigue_gate::FatigueGate;
use super::tape::SignalTape;
use super::trade_behavior::load_tape;
use super::trend_exhaustion::TrendExhaustion;
use super::types::{BspEvent, CenterEvent, DivEvent, FIRST_BSP_LADDER, MAX_LADDER};

/// v3 门状态列文件 magic（"NCTPV3G\0"；v2r 磁带 magic 的代次后继，格式错位即 fail-fast）。
const MAGIC_V3G: u64 = 0x4E43545056334700;

/// E 择时口径的 **L2 层** = ladder 4。
///
/// ladder 映射正本（`analysis/fugue_version_i.py:145`【正文】）：
/// `0=bar 1=bi 2=segment 3=走势(L1)；递归 L → ladder L+2（L2→4 … L8→10）`。
/// E 择时的 `l2_flip_long/short` 由 L1 走势 settle 端点喂 L2 PH 产出
/// （`analysis/m1_e_rust_engine.py` `l2_down`/`l2_up`），即 L1 之上一级 =
/// recL2 = ladder 4。门列与择时列取同一级别，无级别错配。
const L2_LADDER: usize = 4;

/// fatigue 列值：无衰竭（[`super::fatigue_gate::FatigueState::Fresh`]）。
const FATIGUE_FRESH: u8 = 0;
/// fatigue 列值：衰竭段内（[`super::fatigue_gate::FatigueState::Fatigued`]）。
const FATIGUE_FATIGUED: u8 = 1;
/// fatigue 列值：能力缺失哨兵（磁带无 `run_high` 行 ⇒ 不判、不代理）。
const FATIGUE_UNAVAILABLE: u8 = 255;

/// 逐 bar 门状态三列（长度均 = 磁带 bar 数）。
pub(super) struct GateColumns {
    pub up_unexhausted: Vec<bool>,
    pub down_unexhausted: Vec<bool>,
    pub fatigue: Vec<u8>,
    /// fatigue 列是否有能力基础（= 磁带有 `run_high` 行）。false ⇒ 整列哨兵。
    pub fatigue_available: bool,
}

/// 逐 bar 驱动生产门函数，产出 `ladder` 层三列门状态。
///
/// 判据零实现——三列全部是生产函数的读数（[`TrendExhaustion::up_unexhausted`] /
/// [`TrendExhaustion::down_unexhausted`] / [`FatigueGate::fatigued_mask`]）。
pub(super) fn gate_state_columns(tape: &SignalTape, ladder: usize) -> Result<GateColumns, String> {
    if ladder >= MAX_LADDER {
        return Err(format!("ladder={ladder} 越界（MAX_LADDER={MAX_LADDER}）"));
    }
    if !tape.has_dir_rows() {
        return Err(
            "门状态列要求磁带 dir_flips 行（D3）——41课门的走势类型延续项以方向行为\
             数据基础，无行时判据残缺（同 runner.rs 的 l41_gate capability guard，\
             不静默降级）"
                .to_string(),
        );
    }
    // 在册配置：本磁带的在册消费变体（trade_behavior.rs 同源），不另立口径。
    let cfg = variant("V2r").ok_or("V2r 变体未在 config.rs 注册")?;

    let n = tape.bars.len();
    let mut te = TrendExhaustion::new();
    let mut book = CenterBook::new();
    let mut gate = FatigueGate::new();

    let empty_evs: [Vec<BspEvent>; MAX_LADDER] = Default::default();
    let empty_devs: [Vec<DivEvent>; MAX_LADDER] = Default::default();

    let flips: &[(i64, u8, crate::stroke::Direction)] = tape.dir_flips.as_deref().unwrap_or(&[]);
    let mut flip_ptr = 0usize;
    let mut dir_state: [Option<crate::stroke::Direction>; MAX_LADDER] = [None; MAX_LADDER];

    // fatigue 能力：run_high 行存在才驱动（缺失 ⇒ 整列哨兵，见模块头能力边界）。
    let fatigue_available = tape.has_run_high();
    let mut center_evs: [Vec<CenterEvent>; MAX_LADDER] = Default::default();

    let mut up_unexhausted = Vec::with_capacity(n);
    let mut down_unexhausted = Vec::with_capacity(n);
    let mut fatigue = Vec::with_capacity(n);

    for i in 0..n {
        let sig = &tape.bars[i];
        let c = sig.close;

        // ── D3 滚动状态推进（翻转行 bar 升序；同 bar 信号当 bar 可见）──
        while flip_ptr < flips.len() && flips[flip_ptr].0 == i as i64 {
            let (_, lad, dir) = flips[flip_ptr];
            dir_state[lad as usize] = Some(dir);
            flip_ptr += 1;
        }
        let evrows: &[Vec<BspEvent>; MAX_LADDER] = sig.bsp_events.as_deref().unwrap_or(&empty_evs);
        let devrows: &[Vec<DivEvent>; MAX_LADDER] =
            sig.div_events.as_deref().unwrap_or(&empty_devs);

        // ── 走势衰竭追踪（41课门；市场性质，全态每 bar 驱动）──
        te.observe(&dir_state, devrows, c);

        // ── 中枢生命周期账本（每 bar，市场性质）──
        if fatigue_available {
            for row in center_evs.iter_mut() {
                row.clear();
            }
        }
        if sig.bsp_events.is_some() {
            for lad in FIRST_BSP_LADDER..MAX_LADDER {
                let out = if fatigue_available {
                    Some(&mut center_evs[lad])
                } else {
                    None
                };
                book.ingest(lad, &evrows[lad], cfg.hard_type3, out);
            }
        }
        // ── H1 禁令窗口价格否定（每 bar，市场性质）──
        for lad in FIRST_BSP_LADDER..MAX_LADDER {
            book.negate_pending_departure(lad, c);
        }

        // ── FatigueGate（点态；仅 run_high 行存在时驱动）──
        if fatigue_available {
            let rh = tape.run_high.as_deref().expect("fatigue_available 已判定");
            let run_high = &rh[i * MAX_LADDER..(i + 1) * MAX_LADDER];
            for lad in FIRST_BSP_LADDER..MAX_LADDER {
                gate.observe(
                    lad,
                    &evrows[lad],
                    &devrows[lad],
                    &center_evs[lad],
                    sig.up_move_settled.get(lad),
                    c,
                    run_high[lad],
                );
            }
        }

        // ── 读列（同 bar 全部推进之后，与 runner 三态分派时点一致）──
        up_unexhausted.push(te.up_unexhausted(ladder, &book, dir_state[ladder]));
        down_unexhausted.push(te.down_unexhausted(ladder, &book, dir_state[ladder]));
        fatigue.push(if !fatigue_available {
            FATIGUE_UNAVAILABLE
        } else if (gate.fatigued_mask() >> ladder) & 1 == 1 {
            FATIGUE_FATIGUED
        } else {
            FATIGUE_FRESH
        });
    }

    Ok(GateColumns {
        up_unexhausted,
        down_unexhausted,
        fatigue,
        fatigue_available,
    })
}

/// v3 门状态列落盘（小端，与 `analysis/gate_state_columns.py` 逐字段对齐）：
///
/// ```text
///   header: magic u64 = 0x4E43545056334700 ("NCTPV3G\0")
///           n_bars u64, ladder u64, fatigue_available u64（0/1）
///           first_close f64, last_close f64  ← 与 v2r 磁带/驱动 closes 的接缝校验
///   cols:   up_unexhausted   n*u8（0/1）
///           down_unexhausted n*u8（0/1）
///           fatigue_state    n*u8（0=Fresh 1=Fatigued 255=能力缺失）
/// ```
fn write_gate_columns(sym: &str, tape: &SignalTape, ladder: usize, cols: &GateColumns) -> PathBuf {
    let n = tape.bars.len();
    let mut b: Vec<u8> = Vec::with_capacity(6 * 8 + 3 * n);
    b.extend_from_slice(&MAGIC_V3G.to_le_bytes());
    b.extend_from_slice(&(n as u64).to_le_bytes());
    b.extend_from_slice(&(ladder as u64).to_le_bytes());
    b.extend_from_slice(&(cols.fatigue_available as u64).to_le_bytes());
    b.extend_from_slice(&tape.bars[0].close.to_le_bytes());
    b.extend_from_slice(&tape.bars[n - 1].close.to_le_bytes());
    b.extend(cols.up_unexhausted.iter().map(|&v| v as u8));
    b.extend(cols.down_unexhausted.iter().map(|&v| v as u8));
    b.extend_from_slice(&cols.fatigue);
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../analysis/data_cache/_tape_v3_gates_{sym}.bin"));
    fs::write(&out, &b).unwrap_or_else(|e| panic!("写门状态列 {out:?}: {e}"));
    out
}

/// 单标的：加载 v2r 磁带 → 生产函数现算三列 → 落盘 + 打印开关率读数。
fn dump_gate_columns(sym: &str) {
    let tape = load_tape(sym);
    let n = tape.bars.len();
    let cols = gate_state_columns(&tape, L2_LADDER).unwrap_or_else(|e| panic!("[{sym}] {e}"));
    let up_on = cols.up_unexhausted.iter().filter(|&&v| v).count();
    let dn_on = cols.down_unexhausted.iter().filter(|&&v| v).count();
    let fat_on = cols
        .fatigue
        .iter()
        .filter(|&&v| v == FATIGUE_FATIGUED)
        .count();
    let out = write_gate_columns(sym, &tape, L2_LADDER, &cols);
    println!(
        "[{sym}] {n} bars | L2(ladder={L2_LADDER}) up_unexhausted={up_on} ({:.2}%) \
         down_unexhausted={dn_on} ({:.2}%) | fatigue={} ({fat_on} Fatigued bars) | 落盘 {out:?}",
        up_on as f64 / n as f64 * 100.0,
        dn_on as f64 / n as f64 * 100.0,
        if cols.fatigue_available {
            "可用"
        } else {
            "能力缺失（磁带无 run_high 行）"
        },
    );
}

/// #1282 预注册冻结 7 标的（顺序照票面冻结表）。
const PREREG7: [&str; 7] = ["ES", "GC", "CL", "ZN", "6E", "BRN", "DX"];

#[test]
#[ignore]
fn gate_state_dump_prereg7() {
    for sym in PREREG7 {
        dump_gate_columns(sym);
    }
}

/// 单标的入口（符号取环境变量 `GATE_DUMP_SYM`）——预注册 7 标的以外的抽验/
/// 跨语言接缝核验用（Python 侧 `gate_state_columns.load_state_gate` 读同一文件）。
#[test]
#[ignore]
fn gate_state_dump_one() {
    let sym = std::env::var("GATE_DUMP_SYM")
        .expect("需设 GATE_DUMP_SYM=<符号>（对应 _tape_v2r_<符号>.bin）");
    dump_gate_columns(&sym);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::divergence::DivKind;
    use crate::stroke::Direction;
    use crate::trading::tape::BarSig;
    use crate::trading::types::{BspClass, LadderMask};

    fn consol(direction: Direction) -> DivEvent {
        DivEvent {
            kind: DivKind::Consolidation,
            direction,
            seg_idx: 0,
            force_a: 2.0,
            force_c: 1.0,
            price: 0.0,
        }
    }

    fn bar(close: f64) -> BarSig {
        BarSig {
            close,
            ..Default::default()
        }
    }

    fn with_div(mut b: BarSig, lad: usize, d: DivEvent) -> BarSig {
        b.div_events
            .get_or_insert_with(|| Box::new(<[Vec<DivEvent>; MAX_LADDER]>::default()))[lad]
            .push(d);
        b
    }

    fn with_bsp(mut b: BarSig, lad: usize, e: BspEvent) -> BarSig {
        b.bsp_events
            .get_or_insert_with(|| Box::new(<[Vec<BspEvent>; MAX_LADDER]>::default()))[lad]
            .push(e);
        b
    }

    /// 已知趋势延续段的门 sanity（#1312 要做 ④）：L2 层方向行持续 Up、段内无
    /// 盘整背驰 ⇒ `up_unexhausted` 整段恒 true（门在拦逆势空腿）；盘整背驰
    /// 落地那 bar 起门开；下一 Up 段起点重置回 true（背驰逐段对判定）。
    #[test]
    fn up_trend_continuation_window_stays_unexhausted() {
        let bars: Vec<BarSig> = vec![
            bar(10.0),
            bar(10.5),
            bar(11.0),
            // bar3：L2 层盘整背驰（Consolidation×Up）= 衰竭证据
            with_div(bar(11.5), L2_LADDER, consol(Direction::Up)),
            bar(11.6),
            // bar5：翻 Down（Up 段结束）
            bar(11.0),
            // bar6：翻回 Up = 新 Up 段起点，衰竭证据重置
            bar(11.4),
            bar(11.8),
        ];
        let tape = SignalTape {
            bars,
            dir_flips: Some(vec![
                (0, L2_LADDER as u8, Direction::Up),
                (5, L2_LADDER as u8, Direction::Down),
                (6, L2_LADDER as u8, Direction::Up),
            ]),
            ..Default::default()
        };
        let cols = gate_state_columns(&tape, L2_LADDER).expect("dir 行齐备");
        // 趋势延续窗口（bar0..2）：无衰竭迹象 ⇒ 门关，逆势空腿被拦
        assert_eq!(
            &cols.up_unexhausted[0..3],
            &[true, true, true],
            "趋势延续段内 up_unexhausted 必须恒 true（门在拦）"
        );
        // 盘整背驰 bar 起：衰竭证据成立 ⇒ 门开
        assert_eq!(
            &cols.up_unexhausted[3..6],
            &[false, false, false],
            "盘整背驰落地后同段内门保持开"
        );
        // 新 Up 段起点：证据按段对重置 ⇒ 门重新关
        assert_eq!(
            &cols.up_unexhausted[6..8],
            &[true, true],
            "新 Up 段起点重置衰竭证据 ⇒ 门重新关"
        );
    }

    /// 镜像：Down 段窗口的 `down_unexhausted`（拦逆势多腿）。
    #[test]
    fn down_trend_continuation_window_stays_unexhausted() {
        let bars: Vec<BarSig> = vec![
            bar(11.0),
            bar(10.5),
            with_div(bar(10.0), L2_LADDER, consol(Direction::Down)),
            bar(9.9),
        ];
        let tape = SignalTape {
            bars,
            dir_flips: Some(vec![(0, L2_LADDER as u8, Direction::Down)]),
            ..Default::default()
        };
        let cols = gate_state_columns(&tape, L2_LADDER).expect("dir 行齐备");
        assert_eq!(&cols.down_unexhausted[0..2], &[true, true]);
        assert_eq!(&cols.down_unexhausted[2..4], &[false, false]);
    }

    /// 列 = 生产函数读数（零重实现锁）：同一事件流手工驱动 `TrendExhaustion`
    /// 逐 bar 比对，任一 bar 不等即红。
    #[test]
    fn columns_equal_direct_production_readings() {
        let bars: Vec<BarSig> = vec![
            bar(10.0),
            with_div(bar(10.4), L2_LADDER, consol(Direction::Up)),
            bar(10.2),
            with_div(bar(10.0), L2_LADDER, consol(Direction::Down)),
            bar(10.6),
        ];
        let flips = vec![
            (0i64, L2_LADDER as u8, Direction::Up),
            (2, L2_LADDER as u8, Direction::Down),
            (4, L2_LADDER as u8, Direction::Up),
        ];
        let tape = SignalTape {
            bars: bars.clone(),
            dir_flips: Some(flips.clone()),
            ..Default::default()
        };
        let cols = gate_state_columns(&tape, L2_LADDER).expect("dir 行齐备");

        // 手工驱动（生产函数本体，不复制判据）
        let mut te = TrendExhaustion::new();
        let book = CenterBook::new();
        let empty: [Vec<DivEvent>; MAX_LADDER] = Default::default();
        let mut dir: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
        let mut ptr = 0usize;
        for (i, b) in bars.iter().enumerate() {
            while ptr < flips.len() && flips[ptr].0 == i as i64 {
                dir[flips[ptr].1 as usize] = Some(flips[ptr].2);
                ptr += 1;
            }
            let devs = b.div_events.as_deref().unwrap_or(&empty);
            te.observe(&dir, devs, b.close);
            assert_eq!(
                cols.up_unexhausted[i],
                te.up_unexhausted(L2_LADDER, &book, dir[L2_LADDER]),
                "bar{i} up 列 ≠ 生产函数读数"
            );
            assert_eq!(
                cols.down_unexhausted[i],
                te.down_unexhausted(L2_LADDER, &book, dir[L2_LADDER]),
                "bar{i} down 列 ≠ 生产函数读数"
            );
        }
    }

    /// 中枢账本参与判据（结构判据三项的 OR 结构）：confirmed Sell3 坐实后，
    /// 方向行非 Up ⇒ `up_unexhausted` 转 false。锁 `CenterBook` 真的被喂到。
    #[test]
    fn center_book_participates_in_gate_reading() {
        let anchored = |class: BspClass| BspEvent {
            class,
            seg_idx: 0,
            confirmed: true,
            cs: Some(10),
            zd: Some(9.0),
            zg: Some(9.5),
            price: 9.2,
        };
        let bars: Vec<BarSig> = vec![
            bar(9.2),
            with_bsp(bar(9.2), L2_LADDER, anchored(BspClass::Buy1)),
            with_bsp(bar(9.2), L2_LADDER, anchored(BspClass::Sell3)),
            bar(9.2),
        ];
        let tape = SignalTape {
            bars,
            dir_flips: Some(vec![(0, L2_LADDER as u8, Direction::Down)]),
            ..Default::default()
        };
        let cols = gate_state_columns(&tape, L2_LADDER).expect("dir 行齐备");
        // bar0/1：三卖未坐实（或中枢未死）⇒ 未完成 ⇒ 门关
        assert!(cols.up_unexhausted[0] && cols.up_unexhausted[1]);
        // bar2 起：confirmed Sell3 杀中枢 + dead_down ⇒ 三项皆假 ⇒ 门开
        assert!(!cols.up_unexhausted[2] && !cols.up_unexhausted[3]);
    }

    /// 能力边界锁（090号声明=能力）：磁带无 `run_high` 行 ⇒ fatigue 整列哨兵，
    /// 不用 close 代理（代理 = 双真相源，`fatigue_gate.rs` 头部明令）。
    #[test]
    fn fatigue_column_is_unavailable_sentinel_without_run_high() {
        let tape = SignalTape {
            bars: vec![bar(10.0), bar(10.1)],
            dir_flips: Some(vec![(0, L2_LADDER as u8, Direction::Up)]),
            ..Default::default()
        };
        assert!(!tape.has_run_high());
        let cols = gate_state_columns(&tape, L2_LADDER).expect("dir 行齐备");
        assert!(!cols.fatigue_available);
        assert!(cols.fatigue.iter().all(|&v| v == FATIGUE_UNAVAILABLE));
    }

    /// `run_high` 行齐备时 fatigue 列走生产 `FatigueGate` 点态状态（能力在即判）。
    #[test]
    fn fatigue_column_tracks_production_state_with_run_high() {
        let sell1 = BspEvent {
            class: BspClass::Sell1,
            seg_idx: 0,
            confirmed: false,
            cs: None,
            zd: None,
            zg: None,
            price: 10.0,
        };
        let mut b1 = with_bsp(bar(9.8), L2_LADDER, sell1);
        b1.up_move_settled = LadderMask(0);
        let mut b2 = bar(9.9);
        b2.up_move_settled = LadderMask(1 << L2_LADDER); // 路径(3)：MoveSettled{Up} 清空
        let bars = vec![bar(9.7), b1, b2];
        let n = bars.len();
        let tape = SignalTape {
            bars,
            dir_flips: Some(vec![(0, L2_LADDER as u8, Direction::Up)]),
            run_high: Some(vec![10.0; n * MAX_LADDER]),
            ..Default::default()
        };
        let cols = gate_state_columns(&tape, L2_LADDER).expect("dir 行齐备");
        assert!(cols.fatigue_available);
        assert_eq!(
            cols.fatigue,
            vec![FATIGUE_FRESH, FATIGUE_FATIGUED, FATIGUE_FRESH],
            "fatigue 列须逐 bar 跟随生产 FatigueGate 点态状态"
        );
    }

    /// dir 行缺失 ⇒ fail-fast（同 runner 的 l41_gate capability guard，不静默降级）。
    #[test]
    fn missing_dir_rows_is_fail_fast() {
        let tape = SignalTape {
            bars: vec![bar(10.0)],
            ..Default::default()
        };
        assert!(gate_state_columns(&tape, L2_LADDER).is_err());
    }
}
