//! BSP 谓词族 Lean ↔ Rust 向量级锁（#1294 F-8）——固定输入向量集，Lean 提取值 vs Rust 输出
//! 逐位比对（先例：`theta_v0_lean_parity.rs` §7 的 Interval gap/overlap 段，#248/#319）。
//!
//! ## 覆盖三谓词
//!
//! - `IsType1`（`Origin.BspClassification` :94）↔ rust `is_type1_buy`（`closed_loop/buy.rs` :112）；
//! - `IsType3Buy`（:100，含 `firstRetrace` 必要条件）↔ rust `is_type3_buy`（buy.rs :120）；
//! - `SecondTypeStructure`（`Origin.RMoveCompose` :233，#1289 G4 裁定 a 后 = 首个后继
//!   i2=i1+1）↔ rust `find_second_type_structure`（rmove_compose.rs :205，首个后继 i2=i1+1）：
//!   存在性（`.is_some()` ⇔ Lean 镜像 `secondTypeStructureHolds`，#1294 F-8）+ **见证级**
//!   （i1/i2/second_point/retrace_breaks_extreme ⇔ Lean first-match 见证镜像
//!   `findSecondTypeStructureWitness`，#1289 补——同输入同见证点）。
//!
//! ## 机器耦合（631 铁律，非手填）
//!
//! fixture `fixtures/theta_v0_parity.json` 由 `formal/Origin/ParityFixtureExport.lean` 的 #eval
//! **机器导出**：输入字段（端点语义 / 腿序列 / 中枢 / 力度）与谓词真值全部来自 Lean `decide`
//! 真求值。本文件用 `include_str!` 读同一 fixture，断言 rust 输出 == Lean 导出值。
//! Lean 改值 ⟹ #eval 输出变 ⟹ fixture 变 ⟹ 本测试随之变（漂移消除；drift gate 另守「Lean 源 →
//! fixture」一段）。
//!
//! ## 历史分歧案例回归向量
//!
//! - `type1_broke_not_divergent`：破中枢但力度反超（forceC≥forceA）——`witness_type1_needs_divergence`
//!   反例，证明 IsType1 = 破中枢 ∧ 背驰（非「破中枢即一类」）。
//! - `type3_buy_not_first_retrace`：§10.3 第三类必须第一次回抽（firstRetrace 必要条件反例）。
//! - `type3_buy_reenter_zg`：回抽回到 ZG（retrace==zg）——`type3Buy_rejects_reenter` 反例。
//! - `last_leg_type1_no_successor`：唯一破中枢背驰腿在末位（无后继）——「首个后继」存在性边界，
//!   锁定 `∃ i1`（首个后继 i2=i1+1 存在）与 Rust `i1+1 < len` 在存在性上同真同假。
//! - `first_match_two_type1_legs`：首腿与第三腿都是 type1——锁定 **first-match**（见证取 i1=0 非
//!   i1=2），#1289 G4 裁定 a 的「首个后继」见证级锁。
//!
//! ## 认识论等级（formalization-validity-domain 231号）
//!
//! **L0**：Lean machine-checked 见证 ↔ rust 实装对齐。非 L2 经验断言（无真实行情数据）。

use std::rc::Rc;

use newchan_rust::theta_v0::classifier::descend::RMove;
use newchan_rust::theta_v0::classifier::rmove_compose::find_second_type_structure;
use newchan_rust::theta_v0::closed_loop::buy::{is_type1_buy, is_type3_buy, BuyEndpoint};
use newchan_rust::theta_v0::types::{Center, Direction, Side, Tick};
use serde::Deserialize;

#[derive(Deserialize)]
struct ParityFixture {
    bsp_predicates: BspPredicates,
}

#[derive(Deserialize)]
struct BspPredicates {
    endpoints: Vec<EndpointVector>,
    second_type_structures: Vec<SecondTypeStructureVector>,
}

/// 一条 BSP 端点向量（输入字段 + Lean `decide` 真值）。
#[derive(Deserialize)]
struct EndpointVector {
    name: String,
    side: String,
    broke_center: bool,
    is_divergence: bool,
    #[allow(dead_code)]
    after_type_one: bool,
    left_center: bool,
    first_retrace: bool,
    retrace_price: Tick,
    center_zg: Tick,
    is_type1: bool,
    is_type3_buy: bool,
}

/// 一条次级别线段（rust 重建 RMove::Segment 用）。
#[derive(Deserialize)]
struct Leg {
    direction: String,
    lo: Tick,
    hi: Tick,
}

/// 一条第二类走势结构向量（腿序列 + 中枢 + 力度 + 存在性真值 + 见证级）。
#[derive(Deserialize)]
struct SecondTypeStructureVector {
    name: String,
    side: String,
    center_zd: Tick,
    center_zg: Tick,
    center_dd: Tick,
    center_gg: Tick,
    is_divergence: bool,
    legs: Vec<Leg>,
    holds: bool,
    witness: SecondTypeStructureWitness,
}

/// Lean first-match 见证镜像 `findSecondTypeStructureWitness` 的机器导出（#1289 见证级锁）。
#[derive(Deserialize)]
struct SecondTypeStructureWitness {
    present: bool,
    i1: usize,
    i2: usize,
    second_point: Tick,
    retrace_breaks_extreme: bool,
}

fn load_fixture() -> ParityFixture {
    let raw = include_str!("fixtures/theta_v0_parity.json");
    serde_json::from_str(raw).expect("fixture 必须是 Lean #eval 导出的合法 JSON")
}

fn side_from_lean(s: &str) -> Side {
    match s {
        "long" => Side::Long,
        "short" => Side::Short,
        other => panic!("fixture 出现未知 side「{other}」——Lean 枚举与 rust 镜像漂移"),
    }
}

fn direction_from_lean(s: &str) -> Direction {
    match s {
        "up" => Direction::Up,
        "down" => Direction::Down,
        other => panic!("fixture 出现未知 direction「{other}」——Lean 枚举与 rust 镜像漂移"),
    }
}

impl EndpointVector {
    fn to_buy_endpoint(&self) -> BuyEndpoint {
        BuyEndpoint {
            side: side_from_lean(&self.side),
            broke_center: self.broke_center,
            is_divergence: self.is_divergence,
            left_center: self.left_center,
            first_retrace: self.first_retrace,
            retrace_price: self.retrace_price,
            center_zg: self.center_zg,
        }
    }
}

impl SecondTypeStructureVector {
    fn to_parent(&self) -> RMove {
        let subs: Vec<RMove> = self
            .legs
            .iter()
            .map(|leg| RMove::Segment {
                direction: direction_from_lean(&leg.direction),
                lo: leg.lo,
                hi: leg.hi,
            })
            .collect();
        RMove::Compose {
            subs: Rc::new(subs),
            centers: Vec::new(),
            level: 1,
        }
    }

    fn center(&self) -> Center {
        Center {
            zd: self.center_zd,
            zg: self.center_zg,
            dd: self.center_dd,
            gg: self.center_gg,
            start_index: 0,
            end_index: 0,
        }
    }
}

/// IsType1：rust `is_type1_buy` == Lean `decide (IsType1 e)` 逐用例 bit-exact。
#[test]
fn lean_is_type1_vectors_bit_exact() {
    let fx = load_fixture();
    for v in &fx.bsp_predicates.endpoints {
        assert_eq!(
            is_type1_buy(&v.to_buy_endpoint()),
            v.is_type1,
            "{}: rust is_type1_buy == Lean decide(IsType1)（bit-exact）",
            v.name
        );
    }
}

/// IsType3Buy（firstRetrace 必要条件）：rust `is_type3_buy` == Lean `decide (IsType3Buy e)`。
#[test]
fn lean_is_type3_buy_first_retrace_vectors_bit_exact() {
    let fx = load_fixture();
    for v in &fx.bsp_predicates.endpoints {
        assert_eq!(
            is_type3_buy(&v.to_buy_endpoint()),
            v.is_type3_buy,
            "{}: rust is_type3_buy == Lean decide(IsType3Buy)（bit-exact，firstRetrace 必要条件）",
            v.name
        );
    }
}

/// SecondTypeStructure 存在性：rust `find_second_type_structure(...).is_some()` == Lean
/// 可计算判定镜像 `secondTypeStructureHolds`（= `SecondTypeStructure` 存在性，首个后继
/// i2=i1+1，#1289 G4 裁定 a）。
#[test]
fn lean_second_type_structure_existence_bit_exact() {
    let fx = load_fixture();
    for v in &fx.bsp_predicates.second_type_structures {
        let parent = v.to_parent();
        let side = side_from_lean(&v.side);
        let center = v.center();
        let is_divergence = v.is_divergence;
        let rust_holds =
            find_second_type_structure(&parent, side, |_m| center, |_m| is_divergence).is_some();
        assert_eq!(
            rust_holds, v.holds,
            "{}: rust find_second_type_structure.is_some() == Lean SecondTypeStructure 存在性（bit-exact）",
            v.name
        );
    }
}

/// SecondTypeStructure 见证级（#1289 G4 裁定 a）：rust `find_second_type_structure` 返回的
/// `(i1, i2=i1+1, second_point, retrace_breaks_extreme)` == Lean first-match 见证镜像
/// `findSecondTypeStructureWitness` 逐位比对（同输入同见证点；first-match 由
/// `first_match_two_type1_legs` 锁定）。
#[test]
fn lean_second_type_structure_witness_bit_exact() {
    let fx = load_fixture();
    for v in &fx.bsp_predicates.second_type_structures {
        let parent = v.to_parent();
        let side = side_from_lean(&v.side);
        let center = v.center();
        let is_divergence = v.is_divergence;
        let rust = find_second_type_structure(&parent, side, |_m| center, |_m| is_divergence);
        let w = &v.witness;
        match rust {
            None => assert!(
                !w.present,
                "{}: rust None == Lean witness.present=false",
                v.name
            ),
            Some(s) => {
                assert!(
                    w.present,
                    "{}: rust Some == Lean witness.present=true",
                    v.name
                );
                assert_eq!(s.i1, w.i1, "{}: i1（first-match）", v.name);
                assert_eq!(s.i2, w.i2, "{}: i2=i1+1（首个后继）", v.name);
                assert_eq!(s.second_point, w.second_point, "{}: second_point", v.name);
                assert_eq!(
                    s.retrace_breaks_extreme, w.retrace_breaks_extreme,
                    "{}: retrace_breaks_extreme（重合标注）",
                    v.name
                );
            }
        }
    }
}
