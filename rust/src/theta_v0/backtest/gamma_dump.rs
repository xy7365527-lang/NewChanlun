//! Gap-2 Γ_t 候选级只读 dump（#71）。
//!
//! `OPSEM_GAMMA_DUMP_DIR=<dir>` 独立启用 `<dir>/gamma_candidates.jsonl`。本模块只读取
//! 生产已经算出的 `step_gamma`、χ 过滤成员关系和 `step_trace.opened`；不重跑 χ 门、不调用
//! `MuEstimator::observe`，任何字段都不进入 `entry_z`/`MuClass`/μ 桶键/`J_Θ`/χ。
//!
//! 认识论等级（formalization-validity-domain 231号）：L1（纯只读外化，零信息增量，同
//! [`super::opsem_dump::OpsemDump`] 先例）。
//!
//! ## 写失败处理策略（#563 L3 订正）
//!
//! 本模块（`from_env`/`at_dir` 建目录建文件失败、`write_step` 落盘失败经 `fill.rs` 调用点）
//! 一律 **panic**（fail loud，security.md「不吞异常」+ no-patch-mentality 诚实强制）——与
//! [`super::opsem_dump::OpsemDump::write_trade`]/`write_tower_event`（`open_ledger.rs` 调用点
//! `write_trade_fail_loud` / `opsem_dump.rs:359` 同款 `unwrap_or_else(|e| panic!(...))`）**一致**。
//! `admission.rs` 的 `t5a_chain_dump`（`#[cfg(test)]` 限定的测试期临时脚手架）是**唯一**例外——
//! 显式声明"诊断臂不得击穿回测"只吞异常 eprintln 一次；二者不是同一政策的两种落地，是**两类
//! 载体**分别定策：`GammaDump`/`OpsemDump` 是生产构建常驻的只读外化通道，写失败即审计链断裂，
//! 必须让运行者立刻知道；`t5a_chain_dump` 是仅 `#[cfg(test)]` 存在的一次性诊断挂件，其写失败
//! 不该拖垮它所依附的真实测试。

use std::io::Write;

use super::super::strategy::interp::Candidate;
use super::super::types::Bar;
use super::admission::ChiFilterCtx;
use super::selector::{self, ZExt};

pub(super) struct GammaBarContext<'a> {
    pub(super) bar: usize,
    pub(super) gamma_raw: &'a [Candidate],
    pub(super) tower:
        &'a [std::rc::Rc<Vec<super::super::classifier::recursive_tower::LeveledMove>>],
    pub(super) bars: &'a [Bar],
    pub(super) ext: &'a ZExt,
}

pub(super) struct GammaChiContext<'a> {
    pub(super) admitted_indices: &'a [usize],
    pub(super) opened_indices: &'a [usize],
    pub(super) chi: Option<&'a ChiFilterCtx<'a>>,
}

struct CandidateMembership {
    admitted: std::collections::HashSet<usize>,
    opened: std::collections::HashSet<usize>,
}

pub(super) struct GammaDump {
    path: std::path::PathBuf,
    buf: std::io::BufWriter<std::fs::File>,
}

#[cfg(test)]
thread_local! {
    /// 测试注入必须线程局部：进程 env 会被并行 fill loop 同时读取并 truncate 同一文件。
    pub(super) static GAMMA_DUMP_DIR_OVERRIDE:
        std::cell::RefCell<Option<std::path::PathBuf>> =
        std::cell::RefCell::new(None);
}

impl GammaDump {
    /// 从独立 env `OPSEM_GAMMA_DUMP_DIR` 构造；未设或空串均关闭。
    pub(super) fn from_env() -> Option<Self> {
        #[cfg(test)]
        {
            if let Some(dir) = GAMMA_DUMP_DIR_OVERRIDE.with(|c| c.borrow().clone()) {
                return Some(Self::at_dir(&dir));
            }
        }
        let dir = match std::env::var("OPSEM_GAMMA_DUMP_DIR") {
            Ok(dir) if dir.is_empty() => return None,
            Ok(dir) => dir,
            Err(std::env::VarError::NotPresent) => return None,
            Err(std::env::VarError::NotUnicode(_)) => {
                panic!("环境变量 OPSEM_GAMMA_DUMP_DIR 含非 UTF-8 值")
            }
        };
        Some(Self::at_dir(std::path::Path::new(&dir)))
    }

    /// 每次 run 截断重写单一 JSONL；与 `OpsemDump::at_dir` 落盘语义一致。
    pub(super) fn at_dir(dir: &std::path::Path) -> Self {
        std::fs::create_dir_all(dir)
            .unwrap_or_else(|error| panic!("创建 GammaDump 目录 {} 失败：{error}", dir.display()));
        let path = dir.join("gamma_candidates.jsonl");
        let file = std::fs::File::create(&path)
            .unwrap_or_else(|error| panic!("创建 GammaDump 文件 {} 失败：{error}", path.display()));
        Self {
            path,
            buf: std::io::BufWriter::new(file),
        }
    }

    pub(super) fn path(&self) -> &std::path::Path {
        &self.path
    }

    fn write_bar_record(
        &mut self,
        bar_context: &GammaBarContext<'_>,
        chi_context: &GammaChiContext<'_>,
    ) -> std::io::Result<()> {
        serde_json::to_writer(
            &mut self.buf,
            &serde_json::json!({
                "kind": "bar",
                "bar": bar_context.bar,
                "gamma_raw_count": bar_context.gamma_raw.len(),
                "gamma_trade_count": chi_context.admitted_indices.len(),
                "chi_filter_active": chi_context.chi.is_some(),
            }),
        )?;
        self.buf.write_all(b"\n")
    }

    fn admission_fields(
        candidate: &Candidate,
        bar_context: &GammaBarContext<'_>,
        chi_context: &GammaChiContext<'_>,
    ) -> (bool, Option<f64>, Option<f64>) {
        let evaluated = candidate.dir != super::super::strategy::voice::VoiceSide::Flat
            && candidate.bsp_class != u8::MAX;
        match (chi_context.chi, evaluated) {
            (Some(ctx), true) => {
                let z = selector::z_of_candidate(
                    candidate,
                    bar_context.tower,
                    bar_context.bars,
                    bar_context.ext,
                );
                let value = match ctx.shrink_tau_sq {
                    Some(tau_sq) => ctx.est.mu_shrink(&z, tau_sq),
                    None => ctx.est.mu_lcb(&z, ctx.z_alpha),
                };
                (evaluated, value, Some(ctx.theta))
            }
            (Some(ctx), false) => (evaluated, None, Some(ctx.theta)),
            (None, _) => (evaluated, None, None),
        }
    }

    fn write_candidate_record(
        &mut self,
        candidate: &Candidate,
        bar_context: &GammaBarContext<'_>,
        chi_context: &GammaChiContext<'_>,
        membership: &CandidateMembership,
    ) -> std::io::Result<()> {
        let (chi_evaluated, admission_value, theta) =
            Self::admission_fields(candidate, bar_context, chi_context);
        let bsp_class = if candidate.bsp_class == u8::MAX {
            -1
        } else {
            i64::from(candidate.bsp_class)
        };
        serde_json::to_writer(
            &mut self.buf,
            &serde_json::json!({
                "kind": "candidate",
                "bar": bar_context.bar,
                "gamma_index": candidate.gamma_index,
                "level": candidate.level,
                "source_index": candidate.source_index,
                "bsp_bits_class_index": candidate.bits.class_index(),
                "dir": super::opsem_dump::voice_side_str(candidate.dir),
                "bsp_class": bsp_class,
                "nest_confirmed": candidate.nest_confirmed,
                "nest_depth": super::econ_positive::structural_nest_depth(
                    bar_context.tower,
                    candidate.level as usize,
                    candidate.source_index,
                ),
                "role": super::opsem_dump::operation_role_str(candidate.role),
                "chi_filter_active": chi_context.chi.is_some(),
                "chi_evaluated": chi_evaluated,
                "admission_value": admission_value,
                "theta": theta,
                "chi_admit": membership.admitted.contains(&candidate.gamma_index),
                "opened": membership.opened.contains(&candidate.gamma_index),
            }),
        )?;
        self.buf.write_all(b"\n")
    }

    fn write_candidate_records(
        &mut self,
        bar_context: &GammaBarContext<'_>,
        chi_context: &GammaChiContext<'_>,
        membership: &CandidateMembership,
    ) -> std::io::Result<()> {
        for candidate in bar_context.gamma_raw {
            self.write_candidate_record(candidate, bar_context, chi_context, membership)?;
        }
        Ok(())
    }

    /// 写一个决策 bar 及其 χ 前 Γ_t 候选。
    ///
    /// `admitted_indices` 来自生产 `filter_gamma_with_admission` 的返回成员关系；即使下游
    /// Nest/Xzd 门继续收窄，也不会污染 `chi_admit`。`opened` 同理由生产 `step_trace` 导出。
    ///
    /// ★#712 收 #645 LOW-3：下游收窄清单已被 #647 超出——`fill.rs` 在 Nest/Xzd 门之后又多了
    /// #647 入场结构复检门（第三次收窄同一份候选 Vec）。字段语义未破（`chi_admit` 仍是纯 χ
    /// 真值，本文档声明依旧成立），但 `admitted − opened` 差的归因解读须相应补一项：
    /// Nest/Xzd + 入场复检 + 开仓判据三者共同解释，而非仅前两者。
    pub(super) fn write_step(
        &mut self,
        bar_context: GammaBarContext<'_>,
        chi_context: GammaChiContext<'_>,
    ) -> std::io::Result<()> {
        let membership = CandidateMembership {
            admitted: chi_context.admitted_indices.iter().copied().collect(),
            opened: chi_context.opened_indices.iter().copied().collect(),
        };
        self.write_bar_record(&bar_context, &chi_context)?;
        self.write_candidate_records(&bar_context, &chi_context, &membership)?;
        self.buf.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::fill::pi_theta_fill_loop;
    use super::super::mu_estimator::{MuClass, MuEstimator, MuObservation, PositionState};
    use super::*;
    use crate::theta_v0::classifier::bsp::{BspPoint, OwnerRef};
    use crate::theta_v0::classifier::{Classification, LevelState};
    use crate::theta_v0::config::ThetaConfig;
    use crate::theta_v0::types::{BspBits, Center};
    use std::rc::Rc;

    fn gamma_dump_test_dir(tag: &str) -> std::path::PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("系统时间晚于 epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("gamma_dump_{tag}_{}_{}", std::process::id(), nonce))
    }

    fn gamma_dump_rows(dir: &std::path::Path) -> Vec<serde_json::Value> {
        std::fs::read_to_string(dir.join("gamma_candidates.jsonl"))
            .expect("GammaDump 启用 ⟹ gamma_candidates.jsonl 存在")
            .lines()
            .map(|line| serde_json::from_str(line).expect("每行均为合法 JSON"))
            .collect()
    }

    fn px100_bar(i: usize) -> Bar {
        let c = 10_000_000_000i64 + (i as i64) * 10_000_000;
        Bar {
            source_index: i,
            timestamp: i as i64,
            open: c,
            high: c,
            low: c,
            close: c,
            volume: 1,
            untradable: false,
        }
    }

    fn buy1_at(si: usize) -> BspPoint {
        BspPoint {
            source_index: si,
            bits: BspBits {
                buy1: true,
                ..Default::default()
            },
            pivot_low: 9_000_000_000,
            pivot_high: 0,
            center: Some(OwnerRef::Center(Center {
                zd: 9_500_000_000,
                zg: 10_500_000_000,
                dd: 9_000_000_000,
                gg: 11_000_000_000,
                start_index: 0,
                end_index: si,
            })),
            struct_break_dir: None,
            force: None,
        }
    }

    fn buy1_at3_confirmed_at7() -> impl Fn(
        usize,
    ) -> (
        Classification,
        Vec<Rc<Vec<crate::theta_v0::classifier::recursive_tower::LeveledMove>>>,
        Vec<usize>,
        u64,
        u64,
    ) {
        let classification = Classification {
            levels: vec![LevelState {
                bsp: Rc::new(vec![buy1_at(3)]),
                ..Default::default()
            }],
        };
        move |i| {
            if i >= 7 {
                (
                    classification.clone(),
                    Vec::new(),
                    Vec::new(),
                    i as u64,
                    i as u64,
                )
            } else {
                (
                    Classification::default(),
                    Vec::new(),
                    Vec::new(),
                    i as u64,
                    i as u64,
                )
            }
        }
    }

    /// #71 §5-A1：env 未设/空串关闭；线程局部启用只外化，三个生产输出 bit-exact。
    #[test]
    fn gamma_dump_env_gated_bit_exact() {
        std::env::remove_var("OPSEM_GAMMA_DUMP_DIR");
        assert!(GammaDump::from_env().is_none(), "未设 ⟹ None");
        std::env::set_var("OPSEM_GAMMA_DUMP_DIR", "");
        assert!(GammaDump::from_env().is_none(), "空串 ⟹ None");
        std::env::remove_var("OPSEM_GAMMA_DUMP_DIR");

        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        let unset = pi_theta_fill_loop(buy1_at3_confirmed_at7(), &bars, 1.0e6, &config, None);

        let dir = gamma_dump_test_dir("bitexact");
        GAMMA_DUMP_DIR_OVERRIDE.with(|c| *c.borrow_mut() = Some(dir.clone()));
        let enabled = pi_theta_fill_loop(buy1_at3_confirmed_at7(), &bars, 1.0e6, &config, None);
        GAMMA_DUMP_DIR_OVERRIDE.with(|c| *c.borrow_mut() = None);

        assert_eq!(
            unset.typed_ledger, enabled.typed_ledger,
            "typed_ledger bit-exact"
        );
        assert_eq!(unset.n_orders, enabled.n_orders, "n_orders bit-exact");
        assert_eq!(
            unset.trade_pnls_with_forced, enabled.trade_pnls_with_forced,
            "trade_pnls_with_forced bit-exact"
        );
        assert!(!gamma_dump_rows(&dir).is_empty(), "启用 ⟹ 至少有 bar 行");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// #71 §5-A2：合成候选逐字段验证 schema；χ=None 时 χ≡1。
    #[test]
    fn gamma_dump_schema_on_synthetic() {
        let dir = gamma_dump_test_dir("schema");
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        GAMMA_DUMP_DIR_OVERRIDE.with(|c| *c.borrow_mut() = Some(dir.clone()));
        let fill = pi_theta_fill_loop(buy1_at3_confirmed_at7(), &bars, 1.0e6, &config, None);
        GAMMA_DUMP_DIR_OVERRIDE.with(|c| *c.borrow_mut() = None);
        assert!(fill.n_orders > 0, "合成买点应真实开仓");

        let rows = gamma_dump_rows(&dir);
        let bar = rows
            .iter()
            .find(|row| row["kind"] == "bar" && row["gamma_raw_count"].as_u64().unwrap_or(0) > 0)
            .expect("至少一个非空 Γ_t bar");
        assert_eq!(bar["gamma_raw_count"], bar["gamma_trade_count"]);
        assert_eq!(bar["chi_filter_active"], false);

        let candidate = rows
            .iter()
            .find(|row| row["kind"] == "candidate")
            .expect("至少一个候选行");
        for field in [
            "bar",
            "gamma_index",
            "level",
            "source_index",
            "bsp_bits_class_index",
            "dir",
            "bsp_class",
            "nest_confirmed",
            "nest_depth",
            "role",
            "chi_filter_active",
            "chi_evaluated",
            "admission_value",
            "theta",
            "chi_admit",
            "opened",
        ] {
            assert!(candidate.get(field).is_some(), "候选行缺字段 {field}");
        }
        assert_eq!(candidate["chi_filter_active"], false);
        assert_eq!(candidate["chi_admit"], true);
        assert!(candidate["admission_value"].is_null());
        assert!(candidate["theta"].is_null());
        let _ = std::fs::remove_dir_all(dir);
    }

    fn rejecting_mu_estimator() -> MuEstimator {
        let z_buy = MuClass {
            horizontal: Some(crate::theta_v0::strategy::coverage::Horizontal::First),
            sigma_higher: Some(0),
            origin_level: Some(0),
            risk_mode: Some(crate::theta_v0::strategy::risk::RiskMode::Normal),
            t_stage: Some(crate::theta_v0::strategy::ledger::TStage::CostReduction),
            eta_bucket: Some(crate::theta_v0::strategy::ledger::EtaBucket::PositiveSafe),
            ..MuClass::from_certificate(
                0,
                1,
                BspBits {
                    buy1: true,
                    ..Default::default()
                },
                0,
                PositionState::Root,
            )
        };
        let mut est = MuEstimator::new();
        est.observe(MuObservation {
            class: z_buy,
            x_gamma: -50.0,
        });
        est.observe(MuObservation {
            class: z_buy,
            x_gamma: -50.0,
        });
        est
    }

    /// #71 §5-A3：chi_admit 取生产过滤后成员关系；诊断 admission_value 与同门输入一致。
    #[test]
    fn gamma_dump_chi_consistency() {
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
        let est = rejecting_mu_estimator();
        let chi = ChiFilterCtx {
            est: &est,
            theta: 0.0,
            z_alpha: 1.645,
            treat_empty_as_pass: false,
            shrink_tau_sq: None,
        };

        let dir = gamma_dump_test_dir("chi");
        GAMMA_DUMP_DIR_OVERRIDE.with(|c| *c.borrow_mut() = Some(dir.clone()));
        let fill = pi_theta_fill_loop(buy1_at3_confirmed_at7(), &bars, 1.0e6, &config, Some(chi));
        GAMMA_DUMP_DIR_OVERRIDE.with(|c| *c.borrow_mut() = None);
        assert_eq!(fill.n_orders, 0, "生产 χ 门应拒绝唯一买点");

        let rows = gamma_dump_rows(&dir);
        let candidate = rows
            .iter()
            .find(|row| row["kind"] == "candidate" && row["chi_evaluated"] == true)
            .expect("应有 χ 实际求值候选");
        assert_eq!(candidate["chi_admit"], false);
        assert_eq!(candidate["opened"], false);
        assert_eq!(candidate["theta"], 0.0);
        assert!(
            candidate["admission_value"]
                .as_f64()
                .expect("n=2 ⟹ LCB 非 null")
                <= 0.0,
            "诊断准入量须与生产拒绝方向一致"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    /// #71 §5-A4：override 为线程局部；并行跑各写 pid+nonce 唯一目录，不互相 truncate。
    #[test]
    fn gamma_dump_thread_local_override_is_parallel_safe() {
        let dirs: Vec<_> = (0..2)
            .map(|worker| gamma_dump_test_dir(&format!("parallel_{worker}")))
            .collect();
        let handles: Vec<_> = dirs
            .iter()
            .cloned()
            .map(|dir| {
                std::thread::spawn(move || {
                    let config = ThetaConfig::default();
                    let bars: Vec<Bar> = (0..20).map(px100_bar).collect();
                    GAMMA_DUMP_DIR_OVERRIDE.with(|c| *c.borrow_mut() = Some(dir.clone()));
                    let fill =
                        pi_theta_fill_loop(buy1_at3_confirmed_at7(), &bars, 1.0e6, &config, None);
                    GAMMA_DUMP_DIR_OVERRIDE.with(|c| *c.borrow_mut() = None);
                    assert!(fill.n_orders > 0);
                    dir
                })
            })
            .collect();
        let written: Vec<_> = handles
            .into_iter()
            .map(|h| h.join().expect("并行 worker"))
            .collect();
        assert_ne!(written[0], written[1]);
        for dir in written {
            assert!(!gamma_dump_rows(&dir).is_empty());
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}
