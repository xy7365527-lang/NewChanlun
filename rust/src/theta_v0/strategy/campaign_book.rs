//! 每重 campaign 账簿（#1191 D01 职责块自 `oscillation_campaign.rs` 迁出，零行为）。
//!
//! 承载 [`CampaignBook`]（BTreeMap 按 [`super::chong::ChongKey`] 分账）。消费面经
//! `oscillation_campaign` 重导出保持原路径；测试套随块归位至本文件 `mod tests`。

use super::super::classifier::center_lifecycle::CenterId;
use super::super::closed_loop::state::RiskMode;
use super::campaign_ledger::{CampaignOutcome, UnclosedReduction};
use super::center_oscillation_trade::CenterOscillationAction;
use super::chong::ChongKey;
use super::oscillation_campaign::{CampaignLifecycleEvent, CampaignViolation, OscillationCampaign};
use super::short_diff_bucket::CoreCostBasisSnapshot;
use super::voice::VoiceSide;
use std::collections::BTreeMap;

/// 每重 campaign 账簿（issue #294 验收①②；★#880 改挂，SPEC #847 S2）：BTreeMap 按
/// [`ChongKey`]（**键 = (标的, 操作级别)**，ADR 0013 裁定二，成本状态不进键）——
/// 三阶段实例单位 = 重：一重内部对所有**结构**级别是总体的、单一的（塔各结构级别的短差
/// 全部汇进这一重的同一个成本），**不是**回退到 per-TInstance（`docs/three_phase_unified_
/// design.md` 反对的「每个塔层一个三阶段」仍然有效，重 ≠ 塔层）。确定性迭代序理由同
/// #292 H1 先例（`BTreeMap` 非 `HashMap`，避免哈希序跨进程/跨版本不确定）。
///
/// **侧（side）不进键**（重内单向，ADR 0014 裁定一）：一重一份筹码一个持仓方向，侧是
/// campaign 开局时的属性（[`OscillationCampaign::side`]）；要同时持多与持空须两份筹码 =
/// 两个重（不同操作级别）。
///
/// 窗口级汇总（如「本窗口总共几个 campaign 到过 II」）**只是派生读数**——从 `campaigns` 现算
/// （遍历/过滤/计数），本结构不额外维护累计计数器（避免双份状态漂移，同 `state.rs` phase 派生
/// 写回的精神）。
#[derive(Debug, Default)]
pub struct CampaignBook {
    campaigns: BTreeMap<ChongKey, OscillationCampaign>,
}

impl CampaignBook {
    pub fn new() -> Self {
        Self::default()
    }

    /// ★#880：键 = [`ChongKey`]（标的, 操作级别）——同一重跨结构级别共享同一 campaign。
    pub fn campaign(&self, key: &ChongKey) -> Option<&OscillationCampaign> {
        self.campaigns.get(key)
    }

    /// 当前存活 campaign 数（诊断/派生读数——非累计维护，见结构体文档）。
    pub fn active_count(&self) -> usize {
        self.campaigns.len()
    }

    /// 现存 campaign 键迭代（BTreeMap 确定序）——`drive_campaign_wiring` 的死亡判据需要
    /// 「簿内现存但本 bar 已无持仓」的键集（空仓快照 sync）。
    pub fn keys(&self) -> impl Iterator<Item = &ChongKey> + '_ {
        self.campaigns.keys()
    }

    /// 同步本重持仓快照 ⟹ 生死判据（High-2 裁定：本方法与 [`OscillationCampaign::reduce_units`]
    /// 共用同一 [`CoreCostBasisSnapshot`] 接口形状，零派生副本）：
    /// - 空仓→有持仓：开仓生（[`CampaignLifecycleEvent::Opened`]，侧取 incoming `side`）。
    /// - 有持仓→空仓：全平死（[`CampaignLifecycleEvent::Died`]，挂起随死；事件侧取自将死
    ///   campaign 自身——空仓快照不携侧，incoming `side` 在该分支不被消费）。
    /// - 仍持仓且同侧：刷新防线基准 + 推进 bar 时钟（见下），no-op（`Ok(None)`）。
    /// - 仍持仓但**侧不一致**（穿零未经过空仓 bar）：`Err(SideConflict)`——重内单向
    ///   （ADR 0014 裁定一）生产不可达，触达即 enforcement 被改坏的警报，不静默翻侧。
    ///
    /// `side` 入参 = incoming 快照的持仓侧，**仅 (false→true) 开仓与 (true,true) 同侧核验
    /// 消费**。★#380 项二：「仍持仓」分支刷新 campaign 的 `current_units`（当时真实持仓，
    /// 防线校验基准）；开局冻结快照（sizing 基准）与 `notional_in` 均不动。
    /// ★#380 项三：`bar` 入参=当前 bar，开局时记为 campaign 的 `opened_bar`。
    ///
    /// 调用方（`fill.rs::drive_campaign_wiring`）负责把各结构级别的快照**按键聚合**后每重
    /// 每 bar 恰调一次（同重跨结构级别共享同一实例的接线侧落点）。
    pub fn sync_position(
        &mut self,
        key: ChongKey,
        side: VoiceSide,
        snapshot: CoreCostBasisSnapshot,
        bar: usize,
    ) -> Result<Option<CampaignLifecycleEvent>, CampaignViolation> {
        let currently_open = self.campaigns.contains_key(&key);
        match (currently_open, snapshot.units() > 0) {
            (false, true) => {
                self.campaigns
                    .insert(key.clone(), OscillationCampaign::open(snapshot, bar, side));
                Ok(Some(CampaignLifecycleEvent::Opened {
                    key,
                    side,
                    notional_in: snapshot.cost_basis(),
                }))
            }
            (true, false) => {
                let campaign = self
                    .campaigns
                    .remove(&key)
                    .expect("contains_key 刚核验为 true");
                // ★#441（ADR 补充十二）：死亡吞挂起在 `close` 内按「未闭合减出」核销，产出
                // 分列呈报（货缺口/桶实收现金）；`None`=死亡时无挂起。
                let died_side = campaign.side; // close 消费 self 前先取侧（事件/dump 用）
                let (_final_tw, settlement, wo_breakdown) = campaign.close();
                // ★#719（L19②）：观测门 dump——逐（重, 中枢）核销明细，服务 #441 移交
                // 「units_gap 按（级别, 中枢）分桶复算」（★#880：分桶键随实例单位改挂从重出）。
                // 观测门（`THETA_DEATH_WO_DUMP`，env_registry 登记）：置位才打，纯 stderr，
                // 不进任何产物文件、不改任何判定。
                if settlement.is_some() {
                    if std::env::var(crate::theta_v0::env_registry::THETA_DEATH_WO_DUMP).is_ok() {
                        for (cid, u, c) in &wo_breakdown {
                            eprintln!(
                                "[death_wo][#719] key={:?} side={:?} center={:?} units_gap={} cash_booked={}",
                                key, died_side, cid, u, c
                            );
                        }
                    }
                }
                Ok(Some(CampaignLifecycleEvent::Died {
                    key,
                    side: died_side,
                    settlement,
                }))
            }
            (true, true) => {
                let campaign = self
                    .campaigns
                    .get_mut(&key)
                    .expect("contains_key 刚核验为 true");
                // ★#880：重内单向——存续 campaign 侧与 incoming 侧不一致（穿零未经过空仓
                // bar）= 结构性不可达，typed 拒绝（详见 [`CampaignViolation::SideConflict`]）。
                if campaign.side != side {
                    return Err(CampaignViolation::SideConflict {
                        campaign_side: campaign.side,
                        incoming: side,
                    });
                }
                // 仍持仓：刷新防线基准（当时真实持仓），不产生生死事件、不动冻结快照。
                // ★#383：同时推进 campaign 的 bar 时钟（`last_sync_bar`）——阶段三「次 bar 换
                // 模式」的唯一时点来源，见 [`OscillationCampaign::earning_active`]。
                campaign.current_units = snapshot.units();
                campaign.last_sync_bar = bar;
                Ok(None)
            }
            (false, false) => Ok(None),
        }
    }

    /// 单次动作应用（透传 [`OscillationCampaign::apply_action`]）——本重该侧须已开局。
    ///
    /// ★#880：`side` 不进键但参与**侧核验**：重有 campaign 但在另一侧 ⟹
    /// [`CampaignViolation::NoActiveCampaign`]（「该侧空仓」的旧观测语义原样保留——#381 前
    /// 的旧键 `(level, side)` 找不到条目与今「键命中但侧不符」同义，都是「该侧重在该侧无
    /// 持仓」）。空仓重调用动作是接线错误，不静默创建幽灵 campaign。
    pub fn apply_action(
        &mut self,
        key: &ChongKey,
        side: VoiceSide,
        action: CenterOscillationAction,
        price: i64,
        risk_mode: RiskMode,
        source_center: CenterId,
    ) -> Result<CampaignOutcome, CampaignViolation> {
        let campaign = self
            .campaigns
            .get(key)
            .filter(|c| c.side == side)
            .ok_or(CampaignViolation::NoActiveCampaign)?;
        let (next, outcome) = campaign.apply_action(action, price, risk_mode, source_center)?;
        self.campaigns.insert(key.clone(), next);
        Ok(outcome)
    }

    /// ★#366/#472/#489：**未闭合减出核销**入口——教义三类点不回补路径与
    /// `RebaseVanished` 工程失踪路径共用的清算落点，透传
    /// [`OscillationCampaign::write_off_unclosed`]。
    ///
    /// 返回 `Ok(None)` 有两种诚实情形，均非错误、均不新增 typed 拒绝：① 该重该侧当前
    /// 无 campaign（该侧空仓——结构信号独立于持仓的既有「无门」设计，同
    /// [`CampaignViolation::NoActiveCampaign`] 的「预期经济场景」定性；★#880 含「重有
    /// campaign 但在另一侧」）；② 有 campaign 但该来源中枢无挂起批次（高抛后已自然收口）。
    /// 调用方（`fill.rs::drive_campaign_wiring`）把两情形合并计入「无可核销」观测桶，不与
    /// 真实核销读数混计。
    ///
    /// `level` = 来源中枢的**结构级别**（归属呈报进 [`UnclosedReduction::level`]，不参与
    /// 定位——定位只认 `key`+侧核验）。
    pub fn write_off_unclosed(
        &mut self,
        key: &ChongKey,
        side: VoiceSide,
        level: u32,
        center: CenterId,
    ) -> Result<Option<UnclosedReduction>, CampaignViolation> {
        let Some(campaign) = self.campaigns.get(key).filter(|c| c.side == side) else {
            return Ok(None);
        };
        let (next, reduction) = campaign.write_off_unclosed(level, center)?;
        self.campaigns.insert(key.clone(), next);
        Ok(reduction)
    }
}

#[cfg(test)]
mod tests {
    use super::super::campaign_ledger::{CoverAssignment, DeathWriteOff};
    use super::super::ledger::{TStage, TwEvent};
    use super::super::oscillation_campaign::{CampaignWiringWitness, StageEventRecord};
    use super::super::short_diff_bucket::ShortDiffViolation;
    use super::*;

    fn snapshot(units: i64, cost_basis: i64) -> CoreCostBasisSnapshot {
        CoreCostBasisSnapshot::new(units, cost_basis)
    }

    /// ★#880：测试重键——`(标的="T", 操作级别)`。键 = (标的, 操作级别)（ADR 0013 裁定二），
    /// 成本状态不进键。
    fn k(op_level: u8) -> ChongKey {
        ChongKey {
            symbol: "T".to_string(),
            op_level,
        }
    }

    /// 来源中枢身份（#380 项四）：`start_index` 区分不同中枢实例，其余字段固定。
    fn cid(start_index: usize) -> CenterId {
        CenterId::of(&crate::theta_v0::types::Center {
            zd: 100,
            zg: 200,
            dd: 98,
            gg: 202,
            start_index,
            end_index: start_index + 50,
        })
    }

    // ── 生命周期：开仓生 / 全平死 ────────────────────────────────────────

    #[test]
    fn sync_position_opens_campaign_on_transition_from_flat_to_held() {
        let mut book = CampaignBook::new();
        let outcome = book
            .sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap();
        assert_eq!(
            outcome,
            Some(CampaignLifecycleEvent::Opened {
                key: k(0),
                side: VoiceSide::Long,
                notional_in: 3_000
            })
        );
        let campaign = book.campaign(&k(0)).expect("开仓后应存在 campaign");
        assert_eq!(
            campaign.tw().notional_in,
            3_000,
            "notional_in=本仓成本基（缠师口径成本入账）"
        );
        assert_eq!(
            campaign.tw().holding,
            3_000,
            "holding=本仓成本基（与 CoreCostBasisSnapshot 同步）"
        );
        assert_eq!(
            campaign.tw().stage,
            TStage::CostReduction,
            "开局即降成本阶段"
        );
    }

    #[test]
    fn sync_position_is_noop_while_still_flat_or_still_held() {
        let mut book = CampaignBook::new();
        assert_eq!(
            book.sync_position(k(0), VoiceSide::Long, snapshot(0, 0), 0)
                .unwrap(),
            None,
            "仍空仓 ⟹ no-op"
        );
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap();
        assert_eq!(
            book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
                .unwrap(),
            None,
            "仍持仓 ⟹ no-op（加仓重算不在本模块范围）"
        );
        assert_eq!(book.active_count(), 1);
    }

    #[test]
    fn sync_position_kills_campaign_on_transition_to_flat() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap();
        let outcome = book
            .sync_position(k(0), VoiceSide::Long, snapshot(0, 0), 0)
            .unwrap();
        assert_eq!(
            outcome,
            Some(CampaignLifecycleEvent::Died {
                key: k(0),
                side: VoiceSide::Long,
                settlement: None
            }),
            "无挂起在途量的全平死亡（无可核销 ⟹ 不产核销记录，#441）"
        );
        assert!(book.campaign(&k(0)).is_none(), "全平后 campaign 实例被移除");
        assert_eq!(book.active_count(), 0);
    }

    /// ★挂起随死（issue #294 验收②「全平=campaign 终结含挂起随死」）：全平死亡时若短差盈亏桶
    /// 仍有挂起在途量（未及收口的半轮往返），死亡照实记录该量，不强求先 assert_conserved。
    ///
    /// ★#441 复审建议3：测试名与 [`DeathWriteOff`]（未闭合减出核销）语义保持一致。
    #[test]
    fn sync_position_death_writes_off_open_suspended_units_without_requiring_conservation() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap(); // avg_cost=10
        book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(0),
        )
        .unwrap(); // 卖 100（1/3）
        assert_eq!(
            book.campaign(&k(0))
                .unwrap()
                .short_diff()
                .bucket()
                .open_units(),
            100,
            "挂起在途量=100（半轮往返）"
        );
        let outcome = book
            .sync_position(k(0), VoiceSide::Long, snapshot(0, 0), 0)
            .unwrap(); // 仓位全平（未先回补）
        assert_eq!(
            outcome,
            Some(CampaignLifecycleEvent::Died {
                key: k(0),
                side: VoiceSide::Long,
                settlement: Some(DeathWriteOff {
                    units_gap: 100,
                    cash_booked: 1_200,
                    bucket_rejected: false,
                    centers: 1
                }),
            }),
            "★#441：挂起随死按未闭合减出核销并分列呈报（货缺口 100 股 / 桶实收 100·12=1200）"
        );
        assert!(book.campaign(&k(0)).is_none());
    }

    #[test]
    fn each_level_has_independent_campaign_no_cross_book_offset() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap();
        book.sync_position(k(1), VoiceSide::Long, snapshot(500, 20_000), 0)
            .unwrap();
        assert_eq!(book.active_count(), 2);
        book.sync_position(k(0), VoiceSide::Long, snapshot(0, 0), 0)
            .unwrap();
        assert_eq!(
            book.active_count(),
            1,
            "0 级死亡不影响 1 级 campaign（禁跨仓冲减，每级一本账强制）"
        );
        assert!(book.campaign(&k(1)).is_some());
    }

    // ── sizing=当时持仓 1/3（issue #348） ───────────────────────────────

    #[test]
    fn reduce_sizes_to_one_third_of_currently_held_units() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap(); // avg_cost=10
        let outcome = book
            .apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Reduce,
                12,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
        assert_eq!(
            outcome.units, 100,
            "sizing=开局冻结快照(300)/3=100（#348 裁定 + #380 项二基准=快照，整数除法）"
        );
    }

    /// #924 区分见证：快照 ≠ 当时持仓时，sizing 吃快照（sync 后仓位变化不改变 reduce 量）。
    #[test]
    fn reduce_sizing_uses_frozen_snapshot_not_current_units() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap(); // 快照 300
                       // 当时持仓涨到 600（外部加仓后 sync）——reduce 仍按快照 300/3=100。
        book.sync_position(k(0), VoiceSide::Long, snapshot(600, 6_000), 5)
            .unwrap();
        let outcome = book
            .apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Reduce,
                12,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
        assert_eq!(
            outcome.units, 100,
            "sizing=快照(300)/3=100，与当时持仓(600)无关（#380 项二）"
        );
    }

    #[test]
    fn replenish_sizes_to_full_open_units_hard_conservation_in_cost_reduction() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap(); // avg_cost=10
        book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(0),
        )
        .unwrap(); // 卖 100
        let outcome = book
            .apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Replenish,
                9,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
        assert_eq!(
            outcome.units, 100,
            "阶段一恒仓约束硬：Replenish 全额买回挂起在途量（同股数进出）"
        );
        assert!(
            book.campaign(&k(0))
                .unwrap()
                .short_diff()
                .assert_conserved()
                .is_ok(),
            "整轮收口 ⟹ 恒仓断言过"
        );
    }

    #[test]
    fn sizing_rounding_to_zero_is_explicit_violation_not_silent_minimum() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(2, 20), 0)
            .unwrap(); // 持仓仅 2 ⟹ 2/3=0
        assert_eq!(
            book.apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Reduce,
                15,
                RiskMode::Normal,
                cid(0)
            ),
            Err(CampaignViolation::SizingRoundsToZero { held: 2 }),
            "1/3 取整到 0 时显式拒绝，不静默钳制为 1"
        );
    }

    #[test]
    fn action_on_level_without_open_campaign_is_explicit_wiring_error() {
        let mut book = CampaignBook::new();
        assert_eq!(
            book.apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Reduce,
                10,
                RiskMode::Normal,
                cid(0)
            ),
            Err(CampaignViolation::NoActiveCampaign)
        );
    }

    // ── 本仓成本基不动（修7）+ P2-D 双账入口对齐 ──────────────────────────

    #[test]
    fn cost_basis_untouched_and_ledger_pi_mirrors_tw_realize() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap(); // avg_cost=10
        let start_tw = book.campaign(&k(0)).unwrap().tw(); // 开局 tw()=holding(3000)+free(0)+withdrawn(0)=3000
        let original_cost_basis = book.campaign(&k(0)).unwrap().short_diff().cost_basis();

        let reduce = book
            .apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Reduce,
                12,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
        assert_eq!(
            reduce.ledger.pi,
            100 * (12 - 10),
            "R 账本 Π 镜像本轮 Realize=units·(price−avg_cost)=200"
        );
        assert!(reduce.ledger.inv_holds(), "R 账本恒等 R=Π-A-W 全程成立");

        let replenish = book
            .apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Replenish,
                9,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
        assert_eq!(
            replenish.ledger.pi,
            100 * (12 - 10) + 100 * (10 - 9),
            "R 账本 Π 累计两笔 Realize（P2-D：TW/R 双账入口对齐，同一 Realize 分量镜像）"
        );
        assert_eq!(
            replenish.tw.tw() - start_tw.tw(),
            replenish.ledger.pi,
            "campaign 开局以来 TW 净漂移=ΣRealize=R 账本 Π（两账本各自入账同一已实现值，非同构但同源）"
        );

        assert_eq!(
            book.campaign(&k(0)).unwrap().short_diff().cost_basis(),
            original_cost_basis,
            "本仓成本基（均价口径）全程不动（修7）"
        );
    }

    // ── 首见证：到 0 转移（issue #294 验收④，阶段机第一次真正到达 II） ────

    /// ★首见证核心用例：构造能到 0 的场景（多轮高抛低吸，模拟震荡+趋势窗内反复高抛低吸积累
    /// 现金）——断言 `RecoverCapital` 派发（阶段机到达 `CapitalRecovered`=II）、恒仓守恒逐笔、
    /// 本仓成本基不动、campaign 全平即死亡。
    ///
    /// 场景：本仓成本基 3_000（300 股，avg_cost=10）。每轮高抛低吸赚取「卖价−买价」的净现金，
    /// 需要 free 累计到 ≥ notional_in(3_000) 才能触发足额退本金（`stage_progression` 的
    /// cash-tight 门：`free≥recover_target`，见 `closed_loop::transition` 文档）。每轮 sizing=
    /// 100（300/3），净赚 4/股（卖12买8）⟹ 每轮 400，需 8 轮（3200≥3000）达门槛。
    #[test]
    fn first_witness_campaign_reaches_recover_capital_stage_ii() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap();
        assert_eq!(
            book.campaign(&k(0)).unwrap().tw().stage,
            TStage::CostReduction,
            "起点：阶段机在 I（降成本）"
        );

        let mut recovered_at_round = None;
        for round in 1..=8 {
            let sell = book
                .apply_action(
                    &k(0),
                    VoiceSide::Long,
                    CenterOscillationAction::Reduce,
                    12,
                    RiskMode::Normal,
                    cid(0),
                )
                .unwrap();
            assert_eq!(
                sell.units, 100,
                "每轮 sizing 恒=当时持仓(300)/3=100（本仓成本基不动，见下）"
            );
            // 每轮卖出后立即核验恒仓状态：本轮挂起=100（尚未回补）。
            assert_eq!(
                book.campaign(&k(0))
                    .unwrap()
                    .short_diff()
                    .bucket()
                    .open_units(),
                100
            );

            let cover = book
                .apply_action(
                    &k(0),
                    VoiceSide::Long,
                    CenterOscillationAction::Replenish,
                    8,
                    RiskMode::Normal,
                    cid(0),
                )
                .unwrap();
            assert_eq!(
                cover.units, 100,
                "整轮收口：买回等量 100（阶段一恒仓约束硬）"
            );
            assert!(
                book.campaign(&k(0))
                    .unwrap()
                    .short_diff()
                    .assert_conserved()
                    .is_ok(),
                "第 {round} 轮往返闭合 ⟹ 恒仓断言逐笔过"
            );

            if let Some(TwEvent::RecoverCapital(_)) = cover.stage_event {
                recovered_at_round = Some(round);
                break;
            }
        }

        let round =
            recovered_at_round.expect("★首见证：8 轮内必须派发 RecoverCapital（阶段机到达 II）");
        assert_eq!(
            round, 8,
            "净赚 4/股 x100 units x8 轮=3200≥notional_in(3000) ⟹ 第 8 轮足额退本金"
        );

        let campaign = book
            .campaign(&k(0))
            .expect("退本金不终结 campaign（仓位仍在，只是阶段推进）");
        assert_eq!(
            campaign.tw().stage,
            TStage::CapitalRecovered,
            "★阶段机第一次真正到达 II"
        );
        assert_eq!(
            campaign.tw().withdrawn,
            3_000,
            "退本金额=notional_in（足额一次性退回）"
        );
        assert_eq!(
            campaign.tw().l_wc(),
            0,
            "在险本金归零（缠师「成本为 0」的现金口径落地）"
        );
        assert_eq!(
            campaign.short_diff().cost_basis(),
            snapshot(300, 3_000),
            "本仓成本基（均价口径）全程不动——「成本为 0」是现金口径,非均价口径（修7）"
        );

        // 全平即 campaign 死亡（联动 #274 出口规则：仓位归零时终结，不因阶段推进而提前终结）。
        let death = book
            .sync_position(k(0), VoiceSide::Long, snapshot(0, 0), 0)
            .unwrap();
        assert_eq!(
            death,
            Some(CampaignLifecycleEvent::Died {
                key: k(0),
                side: VoiceSide::Long,
                settlement: None
            })
        );
        assert!(book.campaign(&k(0)).is_none(), "全平即 campaign 死亡");
    }

    /// 趋势窗对照：卖出与买回同价（11，无价差可赚）——每轮 `Reduce` 的 `Realize=units·(price−
    /// avg_cost)=100·(11−10)=100` 与 `Replenish` 的 `Realize=units·(avg_cost−price)=100·(10−11)
    /// =−100` 恰好相消，本轮净 Realize=0——现金积累不到 notional_in ⟹ 阶段机停留在 I（不到 0，
    /// 诚实退化，非 bug——见 `closed_loop::transition`「free 不足 ⟹ 等待」注记）。
    #[test]
    fn trend_window_without_enough_realized_profit_stays_at_stage_one() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap(); // avg_cost=10
        for _ in 0..8 {
            book.apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Reduce,
                11,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
            let cover = book
                .apply_action(
                    &k(0),
                    VoiceSide::Long,
                    CenterOscillationAction::Replenish,
                    11,
                    RiskMode::Normal,
                    cid(0),
                )
                .unwrap();
            assert_eq!(
                cover.stage_event, None,
                "同价往返净 Realize=0 ⟹ free 不积累 ⟹ 不推进"
            );
        }
        assert_eq!(
            book.campaign(&k(0)).unwrap().tw().stage,
            TStage::CostReduction,
            "趋势单边同价窗：诚实停留在 I，非硬凑到 0"
        );
    }

    // ── #380 项一：亏损往返如实入账 ─────────────────────────────────────

    /// ★#380 项一核心用例（campaign 层）：卖 100@12 后急拉回补 100@50（买贵 40/股）——旧口径
    /// 被 `cash_sound_gate` 整笔拒绝回滚（亏钱的往返在账本里根本不存在，报告层上偏），新口径
    /// 如实入账：`realized_cash` 收负、桶现金（`tw.free`）为负、`loss_accounted` 置真。
    #[test]
    fn loss_round_trip_is_accounted_with_negative_bucket_cash() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap(); // avg_cost=10
        book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(1),
        )
        .unwrap();
        let cover = book
            .apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Replenish,
                50,
                RiskMode::Normal,
                cid(1),
            )
            .expect("★亏损往返如实入账——不再整笔拒绝");
        assert!(
            cover.loss_accounted,
            "本次落账后桶现金为负 ⟹ 亏损入账标志置真"
        );
        assert_eq!(
            cover.tw.free,
            100 * 12 - 100 * 50,
            "桶现金=-3800（短差累计倒贴）"
        );
        assert_eq!(
            book.campaign(&k(0))
                .unwrap()
                .short_diff()
                .bucket()
                .realized_cash(),
            100 * 12 - 100 * 50,
            "报告层短差盈亏如实收负（不再系统性上偏）"
        );
        assert!(
            book.campaign(&k(0))
                .unwrap()
                .short_diff()
                .assert_conserved()
                .is_ok(),
            "亏损往返同样闭合（恒仓断言不受影响）"
        );

        let mut w = CampaignWiringWitness::new();
        w.record_loss_accounted(VoiceSide::Long);
        assert_eq!(
            w.loss_round_trip_accounted_count.get("long"),
            Some(&1),
            "亏损入账计数桶接替退役的 free<0 拒绝桶（按侧）"
        );
    }

    // ── #380 项二：sizing 基准冻结 vs 防线读当前 ─────────────────────────

    /// ★#380 项二核心用例：campaign 开局冻结 300 股（sizing 基准恒为 300/3=100），其间主仓被减
    /// 到 50 股——`sync_position` 刷新 `current_units`（不产生生死事件、不动冻结快照），
    /// `Reduce` 按冻结基准算 100 但当时真实持仓只有 50 ⟹ 防线显式拒绝（超卖），
    /// `held` 报当前值 50。
    #[test]
    fn defense_reads_current_holding_while_sizing_stays_frozen() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap();
        assert_eq!(
            book.campaign(&k(0)).unwrap().current_units(),
            300,
            "开局：当前持仓=冻结快照"
        );

        // 主仓被减到 50（仍持仓 ⟹ 无生死事件），冻结快照与 notional_in 均不动。
        assert_eq!(
            book.sync_position(k(0), VoiceSide::Long, snapshot(50, 500), 3)
                .unwrap(),
            None,
            "仍持仓 ⟹ 无生死事件"
        );
        let campaign = book.campaign(&k(0)).unwrap();
        assert_eq!(campaign.current_units(), 50, "防线基准刷新为当时真实持仓");
        assert_eq!(
            campaign.short_diff().cost_basis(),
            snapshot(300, 3_000),
            "sizing 基准=开局冻结快照，不动"
        );
        assert_eq!(
            campaign.tw().notional_in,
            3_000,
            "notional_in 不因持仓变动而改写"
        );

        let result = book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(1),
        );
        assert_eq!(
            result,
            Err(CampaignViolation::ShortDiff(
                ShortDiffViolation::UnitsExceedCostBasis {
                    held: 50,
                    attempted: 100,
                }
            )),
            "按冻结基准算量(100)>当时真实持仓(50) ⟹ 「不卖没有的货」显式拒绝"
        );

        let mut w = CampaignWiringWitness::new();
        w.record_violation(VoiceSide::Long, result.unwrap_err());
        assert_eq!(
            w.defense_units_exceed_current_holding_count.get("long"),
            Some(&1),
            "防线拒绝落独立分桶（按侧，预期读数）"
        );
        assert_eq!(w.other_violation_count, 0, "不混入接线错误警报桶");
    }

    // ── #380 项三：stage_event 见证 ─────────────────────────────────────

    /// ★#380 项三核心用例：阶段推进事件（`RecoverCapital`）落 witness——计数 + campaign 标识
    /// （级别）+ 开局以来 bar 数 + 金额。旧版 `fill.rs` 的 `Ok(_outcome) => {}` 把该事件整个
    /// 丢弃，#368 切换开关无物可读。
    ///
    /// ★#383 评审挂账（本用例手调 `record_stage_event` 绕过了生产接线）：本用例保留为
    /// **记录形状**的锁；「生产路径确实会把该事件记进 witness」现由 `fill.rs` 的
    /// `drive_campaign_wiring_lands_stage_events_and_earning_readings_end_to_end` 端到端覆盖
    /// （整支经 `drive_campaign_wiring`，无任何 witness 手调）。
    #[test]
    fn stage_event_lands_in_witness_with_level_and_bars_since_open() {
        let mut book = CampaignBook::new();
        let mut w = CampaignWiringWitness::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 5)
            .unwrap(); // 开局 bar=5
        let mut recovered_bar = None;
        for round in 1..=8u32 {
            let bar = 5 + round as usize * 10;
            book.apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Reduce,
                12,
                RiskMode::Normal,
                cid(1),
            )
            .unwrap();
            let cover = book
                .apply_action(
                    &k(0),
                    VoiceSide::Long,
                    CenterOscillationAction::Replenish,
                    8,
                    RiskMode::Normal,
                    cid(1),
                )
                .unwrap();
            if let Some(ev) = cover.stage_event {
                let bars_since_open = book.campaign(&k(0)).unwrap().bars_since_open(bar);
                w.record_stage_event(0, VoiceSide::Long, bar, bars_since_open, ev);
                recovered_bar = Some(bar);
                break;
            }
        }
        let bar = recovered_bar.expect("8 轮内必派 RecoverCapital（#294 首见证场景）");
        assert_eq!(w.stage_recover_capital_count, 1, "RecoverCapital 计数");
        assert_eq!(w.stage_enter_earning_count, 0, "本场景不到 III");
        assert_eq!(
            w.stage_events,
            vec![StageEventRecord {
                level: 0,
                side: "long",
                bar,
                bars_since_open: bar - 5,
                kind: "recover_capital",
                amount: 3_000,
            }],
            "逐条明细：级别/bar/开局以来 bar 数/金额（=notional_in 足额退回）"
        );
    }

    // ── #380 项四：挂起归属（中枢标签 + 同中枢优先冲抵 + 触发但货满） ────

    /// ★#380 项四核心用例（★★#414 改判）：三笔 `Reduce` 分属两个中枢（cid1 seq0、cid2 seq1、
    /// cid1 seq2），一次 cid1 触发的回补 ⟹ **只冲抵 cid1 的两批**（seq0、seq2），cid2 的挂起
    /// 一股不动。旧口径的「余按时间序冲抵其余中枢」在本票被禁（异中枢静默冲抵），cid2 的
    /// 挂起只能等自己那个中枢的回补或三类点清算。
    #[test]
    fn cover_only_settles_same_center_batches_in_arrival_order() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap(); // avg_cost=10，sizing=100
        book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(1),
        )
        .unwrap();
        book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(2),
        )
        .unwrap();
        book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(1),
        )
        .unwrap();

        let campaign = book.campaign(&k(0)).unwrap();
        assert_eq!(
            campaign
                .suspension()
                .batches()
                .iter()
                .map(|b| (b.center, b.units, b.seq))
                .collect::<Vec<_>>(),
            vec![(cid(1), 100, 0), (cid(2), 100, 1), (cid(1), 100, 2)],
            "挂起批次带来源中枢标签，按到达序排列"
        );
        assert_eq!(
            campaign.suspension().open_units(),
            campaign.short_diff().bucket().open_units(),
            "归属账总量与短差桶标量恒等（同一总量的两种口径，非两套记账）"
        );

        let outcome = book
            .apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Replenish,
                9,
                RiskMode::Normal,
                cid(1),
            )
            .unwrap();
        assert_eq!(
            outcome.units, 200,
            "★#414：算量只看同中枢（cid1 两批 100+100），不含 cid2"
        );
        assert_eq!(
            outcome.cover,
            vec![
                CoverAssignment {
                    center: cid(1),
                    units: 100,
                    seq: 0,
                    same_center: true
                },
                CoverAssignment {
                    center: cid(1),
                    units: 100,
                    seq: 2,
                    same_center: true
                },
            ],
            "★#414：只冲抵同中枢（cid1）两批，按到达序；cid2 不被静默冲抵"
        );
        let remaining = book.campaign(&k(0)).unwrap();
        assert_eq!(
            remaining
                .suspension()
                .batches()
                .iter()
                .map(|b| (b.center, b.units))
                .collect::<Vec<_>>(),
            vec![(cid(2), 100)],
            "★#414：cid2 的挂起原样留着，等自己中枢的回补/三类点清算"
        );
        assert_eq!(
            remaining.suspension().open_units(),
            remaining.short_diff().bucket().open_units(),
            "归属账与桶标量在部分收口后仍恒等（不外溢即不失同步）"
        );

        let mut w = CampaignWiringWitness::new();
        w.record_cover(VoiceSide::Long, &outcome.cover);
        assert_eq!(
            w.cover_by_side.get(&("long", "same_center")),
            Some(&2),
            "同中枢冲抵计数（按侧）"
        );
        assert_eq!(
            w.cover_by_side.get(&("long", "other_center")),
            None,
            "★#414：异中枢冲抵桶恒 0（非 0 = 中枢绑定被改坏的警报读数）"
        );
    }

    /// ★★#414 项三：桶里**有**挂起、但没有一批来自本次触发的来源中枢 ⟹ typed
    /// `ReplenishForeignCenter` + 独立分桶，不静默冲抵别的中枢（旧口径此路径会外溢）。
    #[test]
    fn replenish_from_foreign_center_is_rejected_and_bucketed_separately() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap();
        book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(1),
        )
        .unwrap();
        let result = book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Replenish,
            9,
            RiskMode::Normal,
            cid(2),
        );
        assert_eq!(
            result,
            Err(CampaignViolation::ReplenishForeignCenter {
                open_units_total: 100
            }),
            "有货但不是这个中枢的 ⟹ 拒绝，不拿 cid2 的回补抹平 cid1 的挂起"
        );
        assert_eq!(
            book.campaign(&k(0)).unwrap().suspension().open_units(),
            100,
            "拒绝后账本一分不动"
        );

        let mut w = CampaignWiringWitness::new();
        w.record_violation(
            VoiceSide::Long,
            CampaignViolation::ReplenishForeignCenter {
                open_units_total: 100,
            },
        );
        assert_eq!(
            w.replenish_foreign_center_count.get("long"),
            Some(&1),
            "独立分桶如实计数（按侧）"
        );
        assert_eq!(
            w.replenish_triggered_but_full_count.get("long"),
            None,
            "不与「货本就满」混桶"
        );
        assert_eq!(w.other_violation_count, 0, "预期经济场景，不落警报桶");
    }

    /// ★#380 项四：来源中枢后续买点触发回补、但**货已满**（挂起在途量=0）⟹ typed
    /// `ReplenishWhileFull` + 独立分桶「触发但货满」，不与 sizing 取整异常混计、不落接线错误桶。
    #[test]
    fn replenish_while_full_is_typed_and_bucketed_separately() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap();
        let result = book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Replenish,
            9,
            RiskMode::Normal,
            cid(1),
        );
        assert_eq!(
            result,
            Err(CampaignViolation::ReplenishWhileFull),
            "无挂起可回补 ⟹ 触发但货满"
        );

        let mut w = CampaignWiringWitness::new();
        w.record_violation(VoiceSide::Long, CampaignViolation::ReplenishWhileFull);
        assert_eq!(
            w.replenish_triggered_but_full_count.get("long"),
            Some(&1),
            "「触发但货满」独立分桶如实计数（按侧）"
        );
        assert_eq!(w.other_violation_count, 0, "非记账错误，不落警报桶");
        w.record_violation(
            VoiceSide::Long,
            CampaignViolation::SizingRoundsToZero { held: 2 },
        );
        assert_eq!(
            w.other_violation_count, 1,
            "sizing 取整异常仍是警报，两者不混计"
        );
        assert_eq!(
            w.replenish_triggered_but_full_count.get("long"),
            Some(&1),
            "货满桶不被误增"
        );
    }

    // ── ★#366 未闭合减出核销（补充裁定 2026-07-27）：货缺口/桶实收现金分列，不冲销 ────

    /// 核心用例：cid(1) 高抛两笔 + cid(2) 高抛一笔后，cid(1) 三卖终局 ⟹ 只核销 cid(1) 的两批
    /// （货缺口 200 股、桶实收现金 200·12=2400），cid(2) 的挂起**不连坐**（那个中枢未死，
    /// 「挂起继续等」）。货缺口与桶实收现金**分列返回，不相减**。
    #[test]
    fn write_off_settles_only_the_terminated_center_and_reports_gap_and_cash_separately() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap(); // avg_cost=10，sizing=100
        book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(1),
        )
        .unwrap();
        book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(2),
        )
        .unwrap();
        book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(1),
        )
        .unwrap();

        let reduction = book
            .write_off_unclosed(&k(0), VoiceSide::Long, 0, cid(1))
            .expect("核销不应报错")
            .expect("cid(1) 有两批挂起可核销");
        assert_eq!(reduction.units_gap, 200, "货缺口=cid(1) 两批减出股数");
        assert_eq!(
            reduction.cash_booked,
            2_400,
            "多头侧桶实收=Σ[units·avg_cost+units·(price−avg_cost)]=Σ units·price=200·12（与货缺口分列，不相减）"
        );
        assert_eq!(reduction.center, cid(1));
        assert_eq!(reduction.side, VoiceSide::Long);

        let campaign = book.campaign(&k(0)).unwrap();
        assert_eq!(
            campaign
                .suspension()
                .batches()
                .iter()
                .map(|b| (b.center, b.units))
                .collect::<Vec<_>>(),
            vec![(cid(2), 100)],
            "只核销终局中枢那两批，cid(2) 的挂起继续等"
        );
        assert_eq!(
            campaign.suspension().open_units(),
            campaign.short_diff().bucket().open_units(),
            "核销后归属账与桶标量仍恒等（同步推进，不失同步）"
        );
        assert_eq!(
            campaign.short_diff().bucket().written_off_units(),
            200,
            "核销量留痕（不装没发生）"
        );
    }

    /// ★不冲销的行为化断言：核销**不产任何 TW 事件**——TW 三量与 `realized_cash` 在核销前后
    /// 逐字节不变。冲销（造一笔虚拟回补把货补平）会改动 `holding`/`free`，本测试正是它的反例锚。
    #[test]
    fn write_off_does_not_offset_anything_tw_and_cash_are_byte_identical() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap();
        book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(1),
        )
        .unwrap();
        let before = book.campaign(&k(0)).unwrap().clone();

        book.write_off_unclosed(&k(0), VoiceSide::Long, 0, cid(1))
            .unwrap()
            .expect("有挂起可核销");
        let after = book.campaign(&k(0)).unwrap();
        assert_eq!(
            after.tw(),
            before.tw(),
            "核销不产 TwEvent ⟹ TW 三量不变（货缺口留在账上）"
        );
        assert_eq!(after.ledger(), before.ledger(), "R 账本同样不动");
        assert_eq!(
            after.short_diff().bucket().realized_cash(),
            before.short_diff().bucket().realized_cash(),
            "桶实收现金照留（减出腿当时已入账），不被核销冲掉"
        );
        assert_eq!(
            after.short_diff().bucket().open_units(),
            0,
            "在途量归位（承诺已终局，不再等回补）"
        );
    }

    /// 无挂起可核销的两种诚实情形均返回 `Ok(None)`（非错误）：① 该中枢本就没有挂起批次；
    /// ② 该 (重, 侧) 无 campaign（空仓；★#880 含「重有 campaign 但在另一侧」）。
    #[test]
    fn write_off_with_nothing_to_settle_is_ok_none_not_an_error() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap();
        assert_eq!(
            book.write_off_unclosed(&k(0), VoiceSide::Long, 0, cid(1))
                .unwrap(),
            None,
            "该中枢无挂起批次"
        );
        assert_eq!(
            book.write_off_unclosed(&k(0), VoiceSide::Short, 0, cid(1))
                .unwrap(),
            None,
            "该侧无 campaign"
        );
    }

    /// 核销后**同中枢的幽灵回补路径**也随之关闭：在途量已归位 ⟹ 再来一次 `Replenish` 落
    /// 「触发但货满」桶（`ReplenishWhileFull`），不会凭空补回已核销的货。
    #[test]
    fn replenish_after_write_off_is_rejected_as_full_not_resurrected() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap();
        book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(1),
        )
        .unwrap();
        book.write_off_unclosed(&k(0), VoiceSide::Long, 0, cid(1))
            .unwrap()
            .unwrap();
        assert_eq!(
            book.apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Replenish,
                9,
                RiskMode::Normal,
                cid(1)
            ),
            Err(CampaignViolation::ReplenishWhileFull),
            "已核销的货不得被后续回补复活"
        );
    }

    /// 空头侧对称：核销口径对两侧同样适用（★#880：侧不进键，两侧独立 = 分属不同重）。
    ///
    /// ★口径订正（2026-07-27，评审 §2.4）：现金那一笔不是 `Σ units·price`（旧值 800）——空头
    /// 「减」= 回补空头（买回），成交腿是现金**支出** `units·price=800`，把它记成「卖出成交额/
    /// 现金盈余」值与符号双错。正确口径 = **桶实收现金**：`ShortDiff(units·avg_cost)=1000`
    /// （在险成本基 holding→free）+ `Realize(units·(avg_cost−price))=100·(10−8)=200`
    /// = `units·(2·avg_cost−price)=1200`。本测试同时对桶 `realized_cash` 增量取锚——两者恒等
    /// 是「现金一笔取桶实收、不另立公式」的行为化保证，改断言即须同时改语义。
    #[test]
    fn write_off_applies_symmetrically_to_short_side() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Short, snapshot(300, 3_000), 0)
            .unwrap(); // avg_cost=10，sizing=100
        let cash_before = book
            .campaign(&k(0))
            .unwrap()
            .short_diff()
            .bucket()
            .realized_cash();
        book.apply_action(
            &k(0),
            VoiceSide::Short,
            CenterOscillationAction::Reduce,
            8,
            RiskMode::Normal,
            cid(1),
        )
        .unwrap();
        let cash_after = book
            .campaign(&k(0))
            .unwrap()
            .short_diff()
            .bucket()
            .realized_cash();
        assert_eq!(
            cash_after - cash_before,
            100 * (2 * 10 - 8),
            "空头减出腿桶实收=units·avg_cost+units·(avg_cost−price)=units·(2·avg_cost−price)"
        );

        let reduction = book
            .write_off_unclosed(&k(0), VoiceSide::Short, 0, cid(1))
            .unwrap()
            .expect("空头侧同样可核销");
        assert_eq!(reduction.side, VoiceSide::Short);
        assert_eq!(reduction.units_gap, 100);
        assert_eq!(
            reduction.cash_booked, 1_200,
            "桶实收=100·(2·10−8)=1200，非成交额 100·8=800"
        );
        assert_eq!(
            reduction.cash_booked,
            cash_after - cash_before,
            "现金一笔恒等于桶实收增量，不另立公式"
        );
        assert_ne!(
            reduction.cash_booked,
            100 * 8,
            "空头侧不得回退到 Σ units·price 旧口径"
        );
    }

    // ── ★#441 死亡吞挂起 = 未闭合减出核销（ADR 补充十二，2026-07-27 用户裁定） ────

    /// 核心用例：cid(1) 高抛一笔 + cid(2) 高抛一笔后 campaign 全平死亡 ⟹ 两个中枢的挂起
    /// **一并**按「未闭合减出」核销（主仓全平即「回补进主仓」灭失，不留无主欠账），货缺口
    /// 与桶实收现金**分列呈报、不相减**（同 #366 三卖终局口径）。
    #[test]
    fn death_writes_off_open_suspensions_across_all_centers_and_reports_gap_and_cash_separately() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap(); // avg_cost=10，sizing=100
        book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(1),
        )
        .unwrap();
        book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(2),
        )
        .unwrap();

        let outcome = book
            .sync_position(k(0), VoiceSide::Long, snapshot(0, 0), 1)
            .unwrap();
        assert_eq!(
            outcome,
            Some(CampaignLifecycleEvent::Died {
                key: k(0),
                side: VoiceSide::Long,
                settlement: Some(DeathWriteOff { units_gap: 200, cash_booked: 2_400, bucket_rejected: false, centers: 2 }),
            }),
            "死亡吞挂起：两个来源中枢的挂起一并核销（货缺口 200 股 / 桶实收 200·12=2400，分列不相减）"
        );
        assert!(
            book.campaign(&k(0)).is_none(),
            "全平后 campaign 实例被移除（生死语义不变）"
        );
    }

    /// 无挂起可核销的死亡照实记 `None`（不编造零读数记录）——高抛已自然收口，或本就没做过短差。
    #[test]
    fn death_without_open_suspension_reports_no_settlement() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap();
        assert_eq!(
            book.sync_position(k(0), VoiceSide::Long, snapshot(0, 0), 1)
                .unwrap(),
            Some(CampaignLifecycleEvent::Died {
                key: k(0),
                side: VoiceSide::Long,
                settlement: None
            }),
            "无挂起 ⟹ 无核销记录"
        );
    }

    /// 空头侧对称：现金一笔同样取**桶实收**（`units·(2·avg_cost−price)`），非成交额
    /// `units·price`（#366 §2.4 订正在死亡路径同样适用——两条路径共用同一算料）。
    #[test]
    fn death_write_off_applies_symmetrically_to_short_side() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Short, snapshot(300, 3_000), 0)
            .unwrap(); // avg_cost=10，sizing=100
        book.apply_action(
            &k(0),
            VoiceSide::Short,
            CenterOscillationAction::Reduce,
            8,
            RiskMode::Normal,
            cid(1),
        )
        .unwrap();
        let outcome = book
            .sync_position(k(0), VoiceSide::Short, snapshot(0, 0), 1)
            .unwrap();
        assert_eq!(
            outcome,
            Some(CampaignLifecycleEvent::Died {
                key: k(0),
                side: VoiceSide::Short,
                settlement: Some(DeathWriteOff {
                    units_gap: 100,
                    cash_booked: 1_200,
                    bucket_rejected: false,
                    centers: 1
                }),
            }),
            "空头侧桶实收=100·(2·10−8)=1200，不得回退到 Σ units·price=800 旧口径"
        );
    }

    /// ★死亡优先于延续（#414/ADR 补充十一）：campaign 没了，挂起不可能延续到三类买卖点——
    /// 死亡后原中枢的三类点清算请求到达时无物可清（`Ok(None)`），不会把已核销的挂起复活。
    #[test]
    fn death_takes_precedence_over_continuation_third_class_settlement_finds_nothing() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap();
        book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(1),
        )
        .unwrap();
        book.sync_position(k(0), VoiceSide::Long, snapshot(0, 0), 1)
            .unwrap(); // 死亡吞挂起

        assert_eq!(
            book.write_off_unclosed(&k(0), VoiceSide::Long, 0, cid(1))
                .unwrap(),
            None,
            "死亡优先：挂起已随死核销，其后原中枢三类点清算无物可清"
        );
    }

    // ── #381：空头 campaign（镜像减补 + 多空分重不污染；★#880：侧不进键） ──────────

    /// ★#381 验收①②：纯空头持仓开局 campaign 并按**镜像**记账——空头侧「减」=回补空头
    /// （买回，跌了才赚：`Realize=units·(avg_cost−price)`），「补」=加回空头（重新卖空：
    /// `Realize=units·(price−avg_cost)`）。整轮往返累计 = `units·(p_补 − p_减)`，即低吸高抛
    /// 回加的真实盈利；`ShortDiff` 两腿与多头侧逐字节相同（成本基划转与方向无关，往返相消）。
    #[test]
    fn short_campaign_mirrors_long_with_reduce_as_cover_and_replenish_as_re_short() {
        let mut book = CampaignBook::new();
        let opened = book
            .sync_position(k(0), VoiceSide::Short, snapshot(300, 3_000), 0)
            .unwrap(); // avg_cost=10
        assert_eq!(
            opened,
            Some(CampaignLifecycleEvent::Opened {
                key: k(0),
                side: VoiceSide::Short,
                notional_in: 3_000
            }),
            "空头侧开局生（生死事件带侧）"
        );
        // ★#880：侧不进键——「多头侧无仓」的新表述 = 对该重派多头侧动作落 NoActiveCampaign。
        assert_eq!(
            book.apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Reduce,
                8,
                RiskMode::Normal,
                cid(1),
            ),
            Err(CampaignViolation::NoActiveCampaign),
            "重在空头侧 ⟹ 多头侧无 campaign（侧核验，不进键）"
        );
        let campaign = book.campaign(&k(0)).expect("空头 campaign 存在");
        assert_eq!(campaign.side(), VoiceSide::Short);
        let start_tw = campaign.tw();

        // 减=回补空头 @8（低于均价 10 ⟹ 空头获利）。
        let reduce = book
            .apply_action(
                &k(0),
                VoiceSide::Short,
                CenterOscillationAction::Reduce,
                8,
                RiskMode::Normal,
                cid(1),
            )
            .unwrap();
        assert_eq!(reduce.units, 100, "sizing 口径不分侧：当时持仓(300)/3");
        assert_eq!(
            reduce.ledger.pi,
            100 * (10 - 8),
            "空头减 Realize=units·(avg_cost−price)=+200（跌了才赚）"
        );

        // 补=加回空头 @12（高于均价 ⟹ 卖得更高，同样为赚）。
        let replenish = book
            .apply_action(
                &k(0),
                VoiceSide::Short,
                CenterOscillationAction::Replenish,
                12,
                RiskMode::Normal,
                cid(1),
            )
            .unwrap();
        assert_eq!(replenish.units, 100, "阶段一恒仓约束硬：全额加回挂起在途量");
        assert_eq!(
            replenish.ledger.pi,
            100 * (10 - 8) + 100 * (12 - 10),
            "空头补 Realize=units·(price−avg_cost)=+200（镜像多头补侧）"
        );
        assert_eq!(
            replenish.tw.tw() - start_tw.tw(),
            100 * (12 - 8),
            "整轮 TW 漂移=units·(p_补−p_减)=400，即低吸高抛回加的真实盈利"
        );

        let after = book.campaign(&k(0)).unwrap();
        assert_eq!(
            after.short_diff().bucket().realized_cash(),
            100 * (12 - 8),
            "收口后累计=该侧真实盈利"
        );
        assert!(
            after.short_diff().assert_conserved().is_ok(),
            "同股数进出 ⟹ 恒仓断言过"
        );
        assert_eq!(
            after.tw().holding,
            start_tw.holding,
            "ShortDiff 两腿相消 ⟹ 在险成本基回原值（与多头侧同构）"
        );
        assert_eq!(
            after.short_diff().cost_basis(),
            snapshot(300, 3_000),
            "本仓成本基全程不动（修7，两侧同）"
        );
    }

    /// ★#381 镜像的反面：空头侧「减」在价格**高于**均价时如实亏损入账（涨了回补空头=亏），
    /// 与多头侧的亏损入账口径（#380 项一）对称适用——不整笔拒绝、不静默上偏。
    #[test]
    fn short_campaign_loss_is_accounted_symmetrically() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Short, snapshot(300, 3_000), 0)
            .unwrap(); // avg_cost=10
        book.apply_action(
            &k(0),
            VoiceSide::Short,
            CenterOscillationAction::Reduce,
            8,
            RiskMode::Normal,
            cid(1),
        )
        .unwrap();
        let cover = book
            .apply_action(
                &k(0),
                VoiceSide::Short,
                CenterOscillationAction::Replenish,
                4,
                RiskMode::Normal,
                cid(1),
            )
            .expect("★亏损往返如实入账（两侧对称）");
        assert_eq!(
            cover.ledger.pi,
            100 * (10 - 8) + 100 * (4 - 10),
            "加回价低于均价 ⟹ 本腿 Realize 为负"
        );
        assert_eq!(
            book.campaign(&k(0))
                .unwrap()
                .short_diff()
                .bucket()
                .realized_cash(),
            100 * (4 - 8),
            "整轮累计=units·(p_补−p_减)=−400，报告层如实收负"
        );
    }

    /// ★#381 验收③ 改写（★#880，ADR 0014 裁定一）：多空并存 = **两个重**（不同操作级别）
    /// 各自独立生命周期——重内单向，同一份筹码（同一键）不可多空并存；`notional_in` 各按各重，
    /// 一重全平不牵连另一重。本条同时是 #880 验收单测之一：**同标的不同操作级别各自独立
    /// 推进三阶段**（键 = (标的, 操作级别)，ADR 0013 裁定二）。
    #[test]
    fn long_and_short_campaigns_are_independent_per_chong() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap();
        book.sync_position(k(1), VoiceSide::Short, snapshot(50, 1_000), 0)
            .unwrap();
        assert_eq!(
            book.active_count(),
            2,
            "两个重（不同操作级别）各一本账，不覆盖"
        );
        assert_eq!(book.campaign(&k(0)).unwrap().tw().notional_in, 3_000);
        assert_eq!(book.campaign(&k(1)).unwrap().tw().notional_in, 1_000);

        // 两重各减一次：sizing 各按各重冻结快照，互不冲抵。
        book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(1),
        )
        .unwrap();
        book.apply_action(
            &k(1),
            VoiceSide::Short,
            CenterOscillationAction::Reduce,
            8,
            RiskMode::Normal,
            cid(1),
        )
        .unwrap();
        assert_eq!(
            book.campaign(&k(0))
                .unwrap()
                .short_diff()
                .bucket()
                .open_units(),
            100
        );
        assert_eq!(
            book.campaign(&k(1))
                .unwrap()
                .short_diff()
                .bucket()
                .open_units(),
            16,
            "空头重 sizing=50/3=16，不被多头重的 100 污染"
        );

        let died = book
            .sync_position(k(1), VoiceSide::Short, snapshot(0, 0), 9)
            .unwrap();
        assert_eq!(
            died,
            Some(CampaignLifecycleEvent::Died {
                key: k(1),
                side: VoiceSide::Short,
                // ★#441：空头重 avg_cost=1000/50=20、减出价 8 ⟹ 桶实收=16·(2·20−8)=512
                // （非成交额 16·8——空头「减」是买回，见 `UnclosedReduction` 分列声明）。
                settlement: Some(DeathWriteOff {
                    units_gap: 16,
                    cash_booked: 16 * (2 * 20 - 8),
                    bucket_rejected: false,
                    centers: 1
                }),
            }),
            "空头重全平死（挂起随死，事件带键与将死 campaign 的侧）"
        );
        assert_eq!(book.active_count(), 1);
        assert!(book.campaign(&k(0)).is_some(), "多头重不受牵连");
    }

    /// ★#880：重内单向（ADR 0014 裁定一）——同一键存续多头 campaign 时，空头侧持仓快照
    /// （穿零未经过空仓 bar）落 typed 拒绝 `SideConflict`，状态不被改写（不静默翻侧）。
    #[test]
    fn side_conflict_on_same_chong_is_rejected_without_mutating() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap();
        let before = book.campaign(&k(0)).unwrap().clone();
        assert_eq!(
            book.sync_position(k(0), VoiceSide::Short, snapshot(50, 1_000), 1),
            Err(CampaignViolation::SideConflict {
                campaign_side: VoiceSide::Long,
                incoming: VoiceSide::Short,
            }),
            "穿零未经过空仓 bar ⟹ SideConflict（重内单向警报）"
        );
        assert_eq!(
            book.campaign(&k(0)).unwrap(),
            &before,
            "拒绝不改写存续 campaign"
        );
        // 合法翻面路径：先全平（死），再开空头（生）——穿零必须经过空仓 bar。
        book.sync_position(k(0), VoiceSide::Long, snapshot(0, 0), 2)
            .unwrap();
        assert!(book.campaign(&k(0)).is_none());
        book.sync_position(k(0), VoiceSide::Short, snapshot(50, 1_000), 3)
            .unwrap();
        assert_eq!(book.campaign(&k(0)).unwrap().side(), VoiceSide::Short);
    }

    // ── #383：阶段三 EarningShares 等金额回补 ────────────────────────────

    /// 把一本多头 campaign 驱到 `EarningShares`（阶段三）并让模式**生效**（次 bar）。
    ///
    /// 场景同 #294 首见证：成本基 3_000（300 股 @10），每轮 `Reduce`@12 / `Replenish`@8 净赚
    /// 400 ⟹ 第 8 轮的**回补腿**触发 `RecoverCapital(3000)`；第 9 轮同样在**回补腿**上
    /// `EnterReady` 成立（W≥I0 ∧ legs=0 ∧ RiskNormal ∧ κ=0 基线 η⋆=0）⟹ 派 `EnterEarning`。
    ///
    /// ★2026-07-27 订正（ADR 补充十「到 0 判据 = 挂起空 ∧ free≥本金」）：阶段推进事件只可能
    /// 落在**收口后挂起为空**的那一刻——`Reduce` 腿刚落下挂起，前置过滤必拦（旧版本此处断言
    /// 「第 9 轮 Reduce 上派 EnterEarning」，判据订正后不再成立）。
    ///
    /// 返回 `(book, 事件 bar, 生效 bar)`——事件 bar 上模式**尚未**生效（题三：同 bar 不换尺），
    /// 且此刻挂起为空（判据前置的直接推论）。
    fn book_at_earning_stage() -> (CampaignBook, usize, usize) {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap();
        for round in 1..=8usize {
            book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), round)
                .unwrap();
            book.apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Reduce,
                12,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
            book.apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Replenish,
                8,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
        }
        assert_eq!(
            book.campaign(&k(0)).unwrap().tw().stage,
            TStage::CapitalRecovered,
            "前置：8 轮后阶段机在 II"
        );
        // 第 9 轮：Reduce 落下挂起 ⟹ 前置过滤拦下（free 已够但挂起非空）；同轮 Replenish 收口
        // 后挂起归零 ⟹ 才派 EnterEarning（事件 bar=9）。
        let event_bar = 9usize;
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), event_bar)
            .unwrap();
        let sell = book
            .apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Reduce,
                12,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
        assert_eq!(
            sell.stage_event, None,
            "★挂起非空 ⟹ 阶段推进被前置拦下（判据订正）"
        );
        assert!(
            sell.stage_progress_suspended,
            "★拦下这一次可观测（free 够本金但挂起非空）"
        );
        let out = book
            .apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Replenish,
                8,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
        assert_eq!(
            out.stage_event,
            Some(TwEvent::EnterEarning),
            "收口后挂起归零 ⟹ 派 EnterEarning"
        );
        assert!(!out.stage_progress_suspended, "放行的那次不计入被拦桶");
        (book, event_bar, event_bar + 1)
    }

    /// ★题三（切换时点）：`EnterEarning` 的**事件 bar 上不换尺**，次 bar 才换——事件 bar 上
    /// 落下的挂起批次因此按**阶段一等量**口径收口（旧账按卖出时锁定）。
    #[test]
    fn earning_mode_switches_on_next_bar_not_the_event_bar() {
        let (mut book, event_bar, effective_bar) = book_at_earning_stage();
        let c = book.campaign(&k(0)).unwrap();
        assert_eq!(c.earning_event_bar(), Some(event_bar), "事件 bar 已记录");
        assert!(
            !c.earning_active(),
            "★同 bar 不换尺：事件 bar 上等金额 sizing 尚未生效"
        );
        assert_eq!(
            c.suspension().open_units(),
            0,
            "判据前置：派事件的那一刻挂起必为空"
        );

        // 事件 bar 上再落一笔减出（100 股 @12）——尺未换 ⟹ 按阶段一锁定，不进等金额现金池。
        book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(0),
        )
        .unwrap();
        assert_eq!(
            book.campaign(&k(0)).unwrap().earning_pool(),
            0,
            "事件 bar 的减出按阶段一锁定 ⟹ 不进等金额现金池"
        );

        // 该挂起在同 bar 收口 ⟹ 等量买回 100，无净增股数。
        let cover = book
            .apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Replenish,
                8,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
        assert_eq!(
            cover.units, 100,
            "旧账按卖出时锁定：阶段一卖出 ⟹ 回补永远等量"
        );
        assert!(!cover.earning_replenish, "非等金额回补");
        assert_eq!(cover.earning_units_gained, 0, "等量口径无净增股数");

        // 次 bar：sync 推进 bar 时钟 ⟹ 模式生效。
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), effective_bar)
            .unwrap();
        assert!(
            book.campaign(&k(0)).unwrap().earning_active(),
            "★次 bar 换模式"
        );
    }

    /// ★题一（等金额 sizing）核心用例：阶段三生效后 `Reduce` 100@12 收 1_200 现金，
    /// `Replenish`@8 ⟹ `floor(1200/8)=150` 股——收口 100 + **净增 50 股**（挣股数）；
    /// 高抛侧仍是持仓 1/3（100，与阶段一同尺，只改回补侧）。
    #[test]
    fn earning_replenish_sizes_by_equal_cash_and_gains_shares() {
        let (mut book, _event_bar, effective_bar) = book_at_earning_stage();
        // 判据前置后进阶段三时挂起已为空——无须先收口旧账。
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), effective_bar)
            .unwrap();

        let sell = book
            .apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Reduce,
                12,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
        assert_eq!(
            sell.units, 100,
            "高抛侧维持持仓 1/3（阶段三只改回补侧，补充八题一）"
        );
        assert_eq!(
            book.campaign(&k(0)).unwrap().earning_pool(),
            1_200,
            "阶段三锁定的减出把桶实收现金(100·12)记入等金额现金池"
        );

        let free_before = book.campaign(&k(0)).unwrap().tw().free;
        let cover = book
            .apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Replenish,
                8,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
        assert_eq!(
            cover.units, 150,
            "★等金额回补 floor(1200/8)=150 股（抛出所得现金全额回补）"
        );
        assert!(cover.earning_replenish, "本次是等金额口径");
        assert_eq!(
            cover.earning_units_gained, 50,
            "★净增 50 股 = 150−100（挣股数）"
        );
        assert_eq!(
            cover.tw.free,
            free_before - 1_200,
            "桶现金恰扣 150·8=1200（现金不透支）"
        );
        let c = book.campaign(&k(0)).unwrap();
        assert_eq!(c.earning_pool(), 0, "整除 ⟹ 无零头");
        assert!(
            c.short_diff().assert_conserved().is_ok(),
            "挂起在途量归零（收口腿等量冲抵）"
        );
        assert_eq!(
            c.short_diff().cost_basis(),
            snapshot(300, 3_000),
            "本仓成本基（均价口径）仍不动——增股数是旁路账本读数，不改主账本（修7 不变）"
        );
    }

    /// ★题一（零头留桶累计）：回补价 7 ⟹ `floor(1200/7)=171` 股、耗 1_197，**零头 3 留桶**，
    /// 下一轮与新减出的现金合并计算。
    #[test]
    fn earning_replenish_leaves_remainder_in_pool_for_next_round() {
        let (mut book, _event_bar, effective_bar) = book_at_earning_stage();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), effective_bar)
            .unwrap();

        book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(0),
        )
        .unwrap();
        let cover = book
            .apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Replenish,
                7,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
        assert_eq!(cover.units, 171, "floor(1200/7)=171");
        assert_eq!(cover.earning_units_gained, 71);
        assert_eq!(
            book.campaign(&k(0)).unwrap().earning_pool(),
            3,
            "★零头 3 留桶"
        );

        // 下一轮：新减出 1_200 + 零头 3 = 1_203 ⟹ floor(1203/8)=150（零头累计进下次算量）。
        book.sync_position(
            k(0),
            VoiceSide::Long,
            snapshot(300, 3_000),
            effective_bar + 1,
        )
        .unwrap();
        book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(0),
        )
        .unwrap();
        assert_eq!(
            book.campaign(&k(0)).unwrap().earning_pool(),
            1_203,
            "零头累计进下次"
        );
        let cover2 = book
            .apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Replenish,
                8,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
        assert_eq!(cover2.units, 150, "floor(1203/8)=150");
        assert_eq!(
            book.campaign(&k(0)).unwrap().earning_pool(),
            3,
            "1203−150·8=3 再留桶"
        );
    }

    /// ★题三（旧账按卖出时锁定）：阶段三生效**前**卖出的挂起 + 生效**后**卖出的挂起并存时，
    /// 同一次回补里前者走等量、后者走等金额——两族分开算量后合并成交。
    ///
    /// ★2026-07-27 订正后两族并存的**唯一**入口：判据前置（挂起空 ∧ free≥本金）使派事件那一刻
    /// 挂起必为空，故阶段一锁定批次只能诞生在 `EnterEarning` 的**事件 bar 上**（尺未换，题三
    /// 机制保留）——本用例即构造该残余窗口。
    #[test]
    fn replenish_splits_legacy_and_earning_batches_by_sell_time_lock() {
        let (mut book, _event_bar, effective_bar) = book_at_earning_stage();
        // 事件 bar 上落一笔阶段一锁定的挂起（100 股），**不收口**，留到阶段三与新挂起并存。
        // ★#414：两笔减出同属 cid(1)——本用例考的是「按卖出时模式分族算量」，与中枢绑定正交；
        // 分属两中枢会被新的同中枢绑定切开，掩盖本用例真正要锁的两族分流。
        book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(1),
        )
        .unwrap();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), effective_bar)
            .unwrap();
        // 阶段三锁定的第二笔减出：受累计防线约束（open 100 + 本笔 100 ≤ 当前 300）。
        book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(1),
        )
        .unwrap();
        let c = book.campaign(&k(0)).unwrap();
        assert_eq!(c.suspension().open_units(), 200, "两族挂起并存");
        assert_eq!(c.earning_pool(), 1_200, "只有阶段三锁定那笔进池");

        let cover = book
            .apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Replenish,
                8,
                RiskMode::Normal,
                cid(1),
            )
            .unwrap();
        // 等量族 100（旧账锁定）+ 等金额族 floor(1200/8)=150（收口 100 + 净增 50）。
        assert_eq!(cover.units, 250, "★两族分开算量：100（等量）+150（等金额）");
        assert_eq!(cover.earning_units_gained, 50);
        assert!(
            book.campaign(&k(0))
                .unwrap()
                .short_diff()
                .assert_conserved()
                .is_ok(),
            "两族均已收口"
        );
    }

    /// ★题二（阶段三唯一硬门）：桶现金永不为负——违规显式失败，整笔不落账、状态不变。
    ///
    /// 构造：`EnterEarning` 的**事件 bar 上**（尺未换）落一笔阶段一锁定的挂起（100 股 @12
    /// 卖出），次 bar 在**远高于卖出价**的 900 上收口 ⟹ 该等量腿要付 90_000，桶现金转负。
    /// 阶段一/二 下这是 #380 的「亏损如实入账」，阶段三下被硬门拒绝。
    ///
    /// ★2026-07-27 订正：这条路径是判据前置后**唯一**能撞到本门的残余窗口（等金额腿由 floor
    /// 取整保证结构性不可达），见 `CampaignViolation::EarningCashUnsound` 的有效域注。
    #[test]
    fn stage_three_hard_gate_rejects_negative_bucket_cash_without_mutating_state() {
        let (mut book, _event_bar, effective_bar) = book_at_earning_stage();
        book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(0),
        )
        .unwrap();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), effective_bar)
            .unwrap();
        let before = book.campaign(&k(0)).unwrap().clone();

        let result = book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Replenish,
            900,
            RiskMode::Normal,
            cid(0),
        );
        assert!(
            matches!(result, Err(CampaignViolation::EarningCashUnsound { free }) if free < 0),
            "★阶段三硬门：桶现金转负 ⟹ 显式失败（实得 {result:?}）"
        );
        assert_eq!(
            book.campaign(&k(0)).unwrap(),
            &before,
            "违规显式失败 ⟹ 整笔不落账，campaign 状态逐字段不变"
        );

        let mut w = CampaignWiringWitness::new();
        w.record_violation(VoiceSide::Long, result.unwrap_err());
        assert_eq!(
            w.earning_cash_unsound_count.get("long"),
            Some(&1),
            "硬门独立分桶"
        );
        assert_eq!(w.other_violation_count, 0, "归属明确，不落混合警报桶");
    }

    /// ★#383：等金额腿现金池买不起一股（`pool < price`）且无等量族可收口 ⟹ 独立 typed 拒绝，
    /// 不与「货已满」/「持仓过小」混计；挂起继续挂着，不静默补齐。
    #[test]
    fn earning_replenish_below_one_share_is_its_own_typed_rejection() {
        let (mut book, _event_bar, effective_bar) = book_at_earning_stage();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), effective_bar)
            .unwrap();
        book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(0),
        )
        .unwrap();

        // 池=1_200，回补价 5_000 ⟹ floor(1200/5000)=0 股。
        let result = book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Replenish,
            5_000,
            RiskMode::Normal,
            cid(0),
        );
        assert_eq!(
            result,
            Err(CampaignViolation::EarningSizingRoundsToZero {
                pool: 1_200,
                price: 5_000
            })
        );
        assert_eq!(
            book.campaign(&k(0)).unwrap().suspension().open_units(),
            100,
            "挂起继续挂着（不静默补齐、不当作已收口）"
        );

        let mut w = CampaignWiringWitness::new();
        w.record_violation(VoiceSide::Long, result.unwrap_err());
        assert_eq!(w.earning_sizing_rounds_to_zero_count.get("long"), Some(&1));
        assert_eq!(
            w.replenish_triggered_but_full_count.get("long"),
            None,
            "不与「货已满」混计"
        );
        assert_eq!(w.other_violation_count, 0, "预期经济场景，非警报");
    }

    /// ★回归锁：阶段一（无阶段三批次）时回补量 = 全部挂起在途量——#348 口径逐字节不变。
    /// ★#414：两笔减出改为**同一来源中枢**——「全部挂起在途量」的口径自本票起限定在同中枢内
    /// （异中枢挂起不参与本次算量，见 `cover_only_settles_same_center_batches_in_arrival_order`）。
    #[test]
    fn replenish_plan_in_stage_one_is_full_open_units() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap();
        book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(0),
        )
        .unwrap();
        book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(0),
        )
        .unwrap();
        let cover = book
            .apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Replenish,
                9,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
        assert_eq!(cover.units, 200, "阶段一：全额买回挂起在途量（同股数进出）");
        assert!(!cover.earning_replenish);
        assert_eq!(cover.earning_units_gained, 0);
    }

    // ── #383 订正（2026-07-27 用户裁定，ADR 补充十）：到 0 判据 = 挂起空 ∧ free≥本金 ──

    /// ★核心用例：**挂起非空 ∧ free≥本金 ⟹ 不派发阶段事件**，且被拦一次可观测；同一笔挂起
    /// 回补收口后 ⟹ 正常派发（前置只推迟到「货买回来了」那一刻，不吞事件）。
    ///
    /// 场景：#294 首见证的 8 轮（每轮净赚 400）后阶段机在 II，第 9 轮的 `Reduce` 落账后
    /// `EnterReady` 五合取已成立（`stage_progression` 会派 `EnterEarning`），但挂起 100 股尚未
    /// 买回 ⟹ 拦下；随后的 `Replenish` 收口后挂起归零 ⟹ 正常派 `EnterEarning`。
    ///
    /// 为何取 II→III 这一步做见证：I→II 的 `RecoverCapital` 判据另含 `holding≥notional_in`，
    /// 减出腿上 `holding` 本就下探 ⟹ 该步的算子侧判据在减出腿恒不成立，前置过滤在那一步不
    /// binding（同一次订正下两步行为一致，binding 的是本步）。
    #[test]
    fn stage_progress_is_gated_by_empty_suspension_and_observed_when_blocked() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap();
        for round in 1..=8usize {
            book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), round)
                .unwrap();
            book.apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Reduce,
                12,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
            book.apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Replenish,
                8,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
        }
        assert_eq!(
            book.campaign(&k(0)).unwrap().tw().stage,
            TStage::CapitalRecovered,
            "前提：8 轮后阶段机在 II，本金已足额退回"
        );

        // 第 9 轮减出腿：free≥本金（本金已全退，EnterReady 成立），但挂起 100 股在途。
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 9)
            .unwrap();
        let sell = book
            .apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Reduce,
                12,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
        assert_eq!(
            book.campaign(&k(0)).unwrap().suspension().open_units(),
            100,
            "前提：挂起非空（100 股卖出去还没买回来）"
        );
        assert_eq!(
            sell.stage_event, None,
            "★挂起非空 ⟹ 不派发阶段推进事件（哪怕 free 已够本金）"
        );
        assert!(sell.stage_progress_suspended, "★被拦这一次可观测，不静默");
        assert_eq!(
            book.campaign(&k(0)).unwrap().tw().stage,
            TStage::CapitalRecovered,
            "阶段机停在 II（未被推进）"
        );
        assert!(!book.campaign(&k(0)).unwrap().earning_active(), "更未换尺");

        let mut w = CampaignWiringWitness::new();
        w.record_profit_ready_but_suspended(VoiceSide::Long);
        assert_eq!(
            w.profit_ready_but_suspended.get("long"),
            Some(&1),
            "★观测桶=1（分侧）"
        );
        assert_eq!(
            w.profit_ready_but_suspended.get("short"),
            None,
            "两侧分列，不混计"
        );
        assert_eq!(w.other_violation_count, 0, "判据的正面读数，不是警报");

        // 回补收口 ⟹ 挂起归零 ⟹ 同一份 free 现在算数。
        let cover = book
            .apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Replenish,
                8,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
        assert_eq!(
            cover.stage_event,
            Some(TwEvent::EnterEarning),
            "★收口后正常派发"
        );
        assert!(!cover.stage_progress_suspended, "放行的那次不计入被拦桶");
        assert_eq!(
            book.campaign(&k(0)).unwrap().tw().stage,
            TStage::EarningShares
        );
    }

    /// ★对照（既有判据不破）：**挂起空 ∧ free<本金 ⟹ 仍不派发**，且不落被拦桶——「挂起空」是
    /// 新增的**合取项**，不是替代品（free 门仍由 `stage_progression` 单一来源把守）。
    #[test]
    fn empty_suspension_without_enough_free_still_does_not_progress() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), 0)
            .unwrap();
        book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(0),
        )
        .unwrap();
        let cover = book
            .apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Replenish,
                8,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
        assert_eq!(
            book.campaign(&k(0)).unwrap().suspension().open_units(),
            0,
            "前提：挂起已收口为空"
        );
        assert_eq!(
            cover.tw.free, 400,
            "前提：free=400 < 本金 3_000（一轮只赚 4/股×100）"
        );
        assert_eq!(
            cover.stage_event, None,
            "★free 不足 ⟹ 仍不推进（既有判据不破）"
        );
        assert!(
            !cover.stage_progress_suspended,
            "不是「被挂起拦下」——不落该观测桶"
        );
        assert_eq!(
            book.campaign(&k(0)).unwrap().tw().stage,
            TStage::CostReduction
        );
    }

    /// ★回归锁（评审 QUESTION 4.1，多头侧）：**增股腿下 TW 漂移 = ΣRealize**（ADR 补充九
    /// 两侧同定理）。`ShortDiff` 是 TW 中性构造子（free⇄holding 同价转换），增股腿只产
    /// `ShortDiff(−extra·price)` 而无 `Realize` ⟹ 净增股数不改变 TW，ΣRealize 仍是唯一漂移源。
    ///
    /// 场景：阶段三生效后 `Reduce` 100@12（Realize=100·(12−10)=200）+ `Replenish` 150@8
    /// （收口腿 Realize=100·(10−8)=200；增股 50 股无 Realize）⟹ ΣRealize=400。
    #[test]
    fn tw_drift_equals_sum_realize_on_the_extra_units_leg_long_side() {
        let (mut book, _event_bar, effective_bar) = book_at_earning_stage();
        book.sync_position(k(0), VoiceSide::Long, snapshot(300, 3_000), effective_bar)
            .unwrap();
        let c0 = book.campaign(&k(0)).unwrap();
        let (tw0, pi0) = (c0.tw(), c0.ledger().pi);

        book.apply_action(
            &k(0),
            VoiceSide::Long,
            CenterOscillationAction::Reduce,
            12,
            RiskMode::Normal,
            cid(0),
        )
        .unwrap();
        let cover = book
            .apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Replenish,
                8,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
        assert!(
            cover.earning_units_gained > 0,
            "前提：本次走的是增股腿（实得 {}）",
            cover.earning_units_gained
        );

        let c2 = book.campaign(&k(0)).unwrap();
        let sum_realize = 100 * (12 - 10) + 100 * (10 - 8);
        assert_eq!(
            c2.ledger().pi - pi0,
            sum_realize,
            "ΣRealize=400（增股腿不产 Realize）"
        );
        assert_eq!(
            c2.tw().tw() - tw0.tw(),
            sum_realize,
            "★TW 漂移 = ΣRealize（增股腿只改 TW 构成 free→holding，不改 TW）"
        );
    }

    /// ★回归锁（评审 QUESTION 4.1，空头侧）：同定理镜像——空头侧「减=回补空头@8」
    /// Realize=100·(10−8)=200，「补=加回空头@10」收口腿 Realize=100·(10−10)=0，净增 20 份
    /// 无 Realize ⟹ ΣRealize=200 = TW 漂移。
    #[test]
    fn tw_drift_equals_sum_realize_on_the_extra_units_leg_short_side() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Short, snapshot(300, 3_000), 0)
            .unwrap();
        for round in 1..=8usize {
            book.sync_position(k(0), VoiceSide::Short, snapshot(300, 3_000), round)
                .unwrap();
            book.apply_action(
                &k(0),
                VoiceSide::Short,
                CenterOscillationAction::Reduce,
                8,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
            book.apply_action(
                &k(0),
                VoiceSide::Short,
                CenterOscillationAction::Replenish,
                12,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
        }
        book.sync_position(k(0), VoiceSide::Short, snapshot(300, 3_000), 9)
            .unwrap();
        book.apply_action(
            &k(0),
            VoiceSide::Short,
            CenterOscillationAction::Reduce,
            8,
            RiskMode::Normal,
            cid(0),
        )
        .unwrap();
        let out = book
            .apply_action(
                &k(0),
                VoiceSide::Short,
                CenterOscillationAction::Replenish,
                12,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
        assert_eq!(
            out.stage_event,
            Some(TwEvent::EnterEarning),
            "前提：空头侧已进阶段三"
        );
        book.sync_position(k(0), VoiceSide::Short, snapshot(300, 3_000), 10)
            .unwrap();

        let c0 = book.campaign(&k(0)).unwrap();
        let (tw0, pi0) = (c0.tw(), c0.ledger().pi);
        book.apply_action(
            &k(0),
            VoiceSide::Short,
            CenterOscillationAction::Reduce,
            8,
            RiskMode::Normal,
            cid(0),
        )
        .unwrap();
        let cover = book
            .apply_action(
                &k(0),
                VoiceSide::Short,
                CenterOscillationAction::Replenish,
                10,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
        assert_eq!(
            cover.earning_units_gained, 20,
            "前提：净增 20 份（floor(1200/10)=120，收口 100）"
        );

        let c2 = book.campaign(&k(0)).unwrap();
        let sum_realize = 100 * (10 - 8) + 100 * (10 - 10);
        assert_eq!(
            c2.ledger().pi - pi0,
            sum_realize,
            "ΣRealize=200（空头镜像，增股腿不产 Realize）"
        );
        assert_eq!(
            c2.tw().tw() - tw0.tw(),
            sum_realize,
            "★TW 漂移 = ΣRealize（两侧同定理，ADR 补充九）"
        );
    }

    // ── #366/#381 评审挂账：空头侧阶段机零覆盖补齐 ────────────────────────

    /// ★评审挂账（空头阶段机零覆盖）：**空头 campaign 推进到阶段 II**（`RecoverCapital`）。
    ///
    /// 空头镜像（ADR 补充九）：「减=回补空头（买回）」赚 `avg_cost−price`，「补=加回空头
    /// （卖出）」赚 `price−avg_cost` ⟹ 整轮 Σ Realize = `units·(p_补 − p_减)`，低吸高抛回加
    /// 才是空头侧的盈利形态。本仓在险市值 3_000（300 份 @10），每轮减@8 / 补@12 净赚
    /// `100·4=400` ⟹ 第 8 轮足额退本金（3_200 ≥ 3_000），与多头侧同定理、同轮次。
    #[test]
    fn short_side_campaign_reaches_recover_capital_stage_ii() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Short, snapshot(300, 3_000), 0)
            .unwrap();
        assert_eq!(
            book.campaign(&k(0)).unwrap().tw().stage,
            TStage::CostReduction
        );

        let mut recovered_at_round = None;
        for round in 1..=8usize {
            let sell = book
                .apply_action(
                    &k(0),
                    VoiceSide::Short,
                    CenterOscillationAction::Reduce,
                    8,
                    RiskMode::Normal,
                    cid(0),
                )
                .unwrap();
            assert_eq!(sell.units, 100, "空头侧 sizing 同尺：在险份数 300/3");
            let cover = book
                .apply_action(
                    &k(0),
                    VoiceSide::Short,
                    CenterOscillationAction::Replenish,
                    12,
                    RiskMode::Normal,
                    cid(0),
                )
                .unwrap();
            assert!(
                book.campaign(&k(0))
                    .unwrap()
                    .short_diff()
                    .assert_conserved()
                    .is_ok(),
                "第 {round} 轮往返闭合"
            );
            if let Some(TwEvent::RecoverCapital(_)) = cover.stage_event {
                recovered_at_round = Some(round);
                break;
            }
        }
        assert_eq!(
            recovered_at_round,
            Some(8),
            "★空头侧同样在第 8 轮足额退本金（与多头侧同定理）"
        );
        let c = book.campaign(&k(0)).unwrap();
        assert_eq!(
            c.tw().stage,
            TStage::CapitalRecovered,
            "★空头阶段机第一次到达 II"
        );
        assert_eq!(
            c.tw().withdrawn,
            3_000,
            "退本金额=notional_in（足额一次性退回）"
        );
        assert_eq!(c.tw().l_wc(), 0, "空头侧在险本金同样归零");
        assert_eq!(c.side(), VoiceSide::Short);
        // ★#880：侧不进键——「多头侧不被凭空开局」的新表述 = 该重仍是空头侧 campaign，
        // 多头侧动作落 NoActiveCampaign。
        let c = book.campaign(&k(0)).unwrap();
        assert_eq!(c.side(), VoiceSide::Short, "空头侧推进不翻侧");
        assert_eq!(
            book.apply_action(
                &k(0),
                VoiceSide::Long,
                CenterOscillationAction::Reduce,
                8,
                RiskMode::Normal,
                cid(0),
            ),
            Err(CampaignViolation::NoActiveCampaign),
            "多头侧不因空头侧推进而被凭空开局"
        );
    }

    /// ★评审挂账续：空头 campaign 继续推进到**阶段 III** 并做等金额回补——空头侧「补=加回
    /// 空头」的等金额算料同样取**桶实收现金**（ADR 补充九已定的两侧对称符号锚），净增的是
    /// 空头份数。
    #[test]
    fn short_side_campaign_enters_earning_and_gains_short_units() {
        let mut book = CampaignBook::new();
        book.sync_position(k(0), VoiceSide::Short, snapshot(300, 3_000), 0)
            .unwrap();
        for round in 1..=8usize {
            book.sync_position(k(0), VoiceSide::Short, snapshot(300, 3_000), round)
                .unwrap();
            book.apply_action(
                &k(0),
                VoiceSide::Short,
                CenterOscillationAction::Reduce,
                8,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
            book.apply_action(
                &k(0),
                VoiceSide::Short,
                CenterOscillationAction::Replenish,
                12,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
        }
        book.sync_position(k(0), VoiceSide::Short, snapshot(300, 3_000), 9)
            .unwrap();
        let sell = book
            .apply_action(
                &k(0),
                VoiceSide::Short,
                CenterOscillationAction::Reduce,
                8,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
        assert_eq!(sell.stage_event, None, "挂起非空 ⟹ 前置拦下（两侧同判据）");
        assert!(sell.stage_progress_suspended);
        let out = book
            .apply_action(
                &k(0),
                VoiceSide::Short,
                CenterOscillationAction::Replenish,
                12,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
        assert_eq!(
            out.stage_event,
            Some(TwEvent::EnterEarning),
            "★空头阶段机到达 III（收口后）"
        );

        book.sync_position(k(0), VoiceSide::Short, snapshot(300, 3_000), 10)
            .unwrap();
        assert!(
            book.campaign(&k(0)).unwrap().earning_active(),
            "次 bar 生效（两侧同规矩）"
        );
        // 空头侧「减=回补空头@8」的桶实收 = units·avg_cost + units·(avg_cost−price)
        // = 100·10 + 100·2 = 1_200（补充九口径：单腿桶实收≠单笔成交现金）。
        book.apply_action(
            &k(0),
            VoiceSide::Short,
            CenterOscillationAction::Reduce,
            8,
            RiskMode::Normal,
            cid(0),
        )
        .unwrap();
        assert_eq!(
            book.campaign(&k(0)).unwrap().earning_pool(),
            1_200,
            "空头侧算料=桶实收现金"
        );
        let cover = book
            .apply_action(
                &k(0),
                VoiceSide::Short,
                CenterOscillationAction::Replenish,
                10,
                RiskMode::Normal,
                cid(0),
            )
            .unwrap();
        assert_eq!(cover.units, 120, "floor(1200/10)=120 份");
        assert_eq!(
            cover.earning_units_gained, 20,
            "★空头侧净增 20 份（挣的是空头份数）"
        );
        assert!(cover.earning_replenish);
    }
}
