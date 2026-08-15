//! #983（N7）`consume_at` 逻辑实装（map #529；#981 冻结签名 + #982 A 签名订正落地）。
//!
//! ## Seam（一行）
//!
//! ```text
//! 现役对象（TowerChainCertificate 存活节点 / event_to_bsp 映射 / prior 幂等映射）
//!   → consume_at 逻辑（Closed 门 + 幂等创建 + BspLink 组内写入 + 四错误分支）
//!   → (ManagedBspCreation[], BspLink[])
//! ```
//!
//! 左半边（现役对象）**零改**：本票只读现役对象公开字段（`TowerChainCertificate` 的
//! `status`/`closed_at`/`key`/`nodes`、`ChainNodeTrace` 的 `key`/`status`、`BspStructuralKey`/
//! `CandidateKey`/`ChainKey` 公开字段），不 mock 现役对象内部、不测私有路径、不改
//! bsp_bridge / chain_cert / parser / 递归塔。右半边（新类型族 + 逻辑）全部落在**本文件**。
//!
//! ## 认识论（formalization-validity-domain）
//!
//! 实装本身 = L1（bit-exact 一致性中的「签名/形状翻译正确」一档）：#981 冻结签名与类型族
//! 逐字段落成 Rust，本票把 `todo!()` 换成真逻辑 + 真值表/幂等/错误分支测试。**不声称 L2/L3**——
//! `consume_at` 消费逻辑在市场上的有效性归后续票；模糊判据（LateAuthorization / InconsistentState）
//! 照实留 TODO 注释，不擅自扩语义。
//!
//! ## 语义来源（自包含）
//!
//! - 谱系综合 §2：`chanlun/review-results/e2eo-lineage-synthesis-20260728.md`
//!   （Consume_at 签名 + 只收 Closed + 投影幂等）。
//! - #980 裁定 A：BspKey = [`BspStructuralKey`]（episode 粒度），残余规则「跨多条链只创建一次、
//!   多条链接写 BspLink 组内」。
//! - #982 裁定 A：加 `event_to_bsp` 参数（调用方在调 `consume_at` 前从 `BspBridgeBook` 预解析
//!   「候选事件 → BSP」映射，`consume_at` 只读不查簿）。
//! - 四错误分支判据：`LateAuthorization` = `closed_at > as_of` 最直白推导（精确判据待冻结语义
//!   细化，见函数内 TODO）；`InconsistentState` = prior 超前条目检查（精确判据待 E2E-O 修订协议
//!   落地后细化，见函数内 TODO）。

use std::collections::{HashMap, HashSet};

use super::super::types::Side;
use super::bsp_bridge::BspStructuralKey;
use super::cand_event::CandidateKey;
use super::chain_cert::{ChainKey, ChainNodeStatus, ChainStatus, TowerChainCertificate};

/// 谱系投影键（roadmap E2E-N4 行「ProjectionKey 含 Lineage/policy/rule/slot」）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectionKey {
    /// LineageKey 现役对应（有序 `CandidateKey` 路径）。
    pub lineage: ChainKey,
    pub policy_id: u32,
    pub rule_id: u32,
    pub slot: u32,
}

/// 链接键 = (BspKey, ProjectionKey)，唯一双向边（roadmap E2E-N4 行）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LinkKey {
    /// #980 裁定 A：BspKey = [`BspStructuralKey`]。
    pub bsp: BspStructuralKey,
    pub projection: ProjectionKey,
}

/// 受管 BSP：既有 BSP 结构身份 + 谱系背书（命名独立：不改 `BspPoint`，只新增户口）。
///
/// 形成级别从 `key.level` 取（LEE Consume_ℓ 投影用，不另存一份可能漂移的级别拷贝——
/// 同 CONTEXT.md「级别身份 = 参照系视图非存储事实」纪律）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ManagedBsp {
    pub key: BspStructuralKey,
    pub created_at: usize,
}

/// 受管 BSP 创建记录（Consume_at 产出之一）。创建即新 BSP，无「先有后贴」。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedBspCreation {
    pub bsp: ManagedBsp,
}

/// 谱系反向边（Consume_at 产出之二）。方向 = side（roadmap「边的方向一致即 side 相等」）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BspLink {
    pub key: LinkKey,
    pub side: Side,
    pub written_at: usize,
}

/// 受管政策：一组 rule，每条 rule 精确投影一个 ProjectionKey（roadmap E2E-N7 行
/// 「合法且 hash 固定的 policy 每条唯一 rule 精确投影一个 ProjectionKey，多点拆多规则」）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedBspPolicy {
    pub rules: Vec<PolicyRule>,
}

/// 单条 policy rule。
///
/// rule 语义（什么 BSP 类/方向/级别 → 什么投影）的**完整判定归后续票**，本票只建形状：
/// `rule_id` + 投影的 `policy_id`/`rule_id`/`slot`（投影的 lineage 由消费时的
/// `closed_lineage.key` 提供，不复制进 rule——同一 rule 可投影多条链，lineage 是投影参数
/// 非 rule 分量）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PolicyRule {
    pub rule_id: u32,
    pub policy_id: u32,
    pub slot: u32,
}

/// Consume_at 错误分支（冻结签名四分支）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsumeError {
    NoConsumption,
    InvalidPolicy,
    InconsistentState,
    LateAuthorization,
}

/// Consume_at 逻辑（#983 实装；#981 冻结签名 + #982 A 加 `event_to_bsp` 参数）。
///
/// 语义（谱系综合 §2 / roadmap / #980 A / #982 A）：
/// - 只接收 `Closed` 谱系（`closed_lineage.status == ChainStatus::Closed`，否则
///   `Err(NoConsumption)`）；
/// - 授权 BSP 在同一生产事务形成并写入谱系反向边，不是给既有 BSP 事后贴标；
/// - 以旧 BSP/链接映射（`prior_bsp_by_key` / `prior_links_by_key`）保证投影幂等——
///   同一 BspKey 已存在则不重复创建，只补 BspLink（残余规则 #980：跨多条链只创建一次，
///   多条链接写 BspLink 组内）；
/// - `event_to_bsp` = 调用方在调本函数前从 `BspBridgeBook` 预解析的「候选事件 → BSP」映射，
///   本函数只读不查簿（#982 A）。
///
/// 输入校验顺序（先到先返回）：NoConsumption → LateAuthorization → InvalidPolicy →
/// InconsistentState。模糊判据（LateAuthorization / InconsistentState）按最直白推导实现并留
/// TODO 注释，不擅自扩语义。
pub fn consume_at(
    as_of: usize,
    prior_bsp_by_key: &HashMap<BspStructuralKey, ManagedBsp>,
    prior_links_by_key: &HashMap<LinkKey, BspLink>,
    closed_lineage: &TowerChainCertificate,
    policy: &ManagedBspPolicy,
    event_to_bsp: &HashMap<CandidateKey, BspStructuralKey>,
) -> Result<(Vec<ManagedBspCreation>, Vec<BspLink>), ConsumeError> {
    // 1. NoConsumption：只接收 Closed 谱系（非 Closed 不可消费 = 空分支，非错误但也不产出）。
    if closed_lineage.status != ChainStatus::Closed {
        return Err(ConsumeError::NoConsumption);
    }
    // 2. LateAuthorization：链在本次 as_of 之后才闭合 ⟹ 本次 as_of 时链尚未可消费。
    // TODO(精确判据待冻结语义细化)：本版 = closed_at > as_of 最直白推导。
    if let Some(closed_at) = closed_lineage.closed_at {
        if closed_at > as_of {
            return Err(ConsumeError::LateAuthorization);
        }
    }
    // 3. InvalidPolicy：空 rules，或两条 rule 投影到同一 (policy_id, rule_id, slot) 三元组。
    if policy.rules.is_empty() {
        return Err(ConsumeError::InvalidPolicy);
    }
    let mut seen_projections = HashSet::new();
    for rule in &policy.rules {
        if !seen_projections.insert((rule.policy_id, rule.rule_id, rule.slot)) {
            return Err(ConsumeError::InvalidPolicy);
        }
    }
    // 4. InconsistentState：prior 状态超前于本次 as_of。
    // TODO(精确判据待 E2E-O 修订协议落地后细化)：本版 = 超前条目检查。
    if prior_bsp_by_key.values().any(|bsp| bsp.created_at > as_of)
        || prior_links_by_key
            .values()
            .any(|link| link.written_at > as_of)
    {
        return Err(ConsumeError::InconsistentState);
    }
    let mut creations: Vec<ManagedBspCreation> = vec![];
    let mut links: Vec<BspLink> = vec![];
    // 防御性去重：同一 bsp_key 被多个节点命中时只产一次 creation；同一 link_key 只产一次 link
    // （event_to_bsp 为单射时不可达，但一次调用内仍用本地集合兜底）。
    let mut created_this_call: HashSet<BspStructuralKey> = HashSet::new();
    let mut linked_this_call: HashSet<LinkKey> = HashSet::new();
    for node in &closed_lineage.nodes {
        // 只授权存活端点；Falsified/Absent 被跨过不授权。
        if node.status != ChainNodeStatus::Alive {
            continue;
        }
        // 查无 = Absent 非证伪，跳过。
        let Some(bsp_key) = event_to_bsp.get(&node.key) else {
            continue;
        };
        // 幂等创建：prior 已有则只补链接，不重复创建。
        if !prior_bsp_by_key.contains_key(bsp_key) && created_this_call.insert(bsp_key.clone()) {
            creations.push(ManagedBspCreation {
                bsp: ManagedBsp {
                    key: bsp_key.clone(),
                    created_at: as_of,
                },
            });
        }
        // 每条 rule 投影一个 ProjectionKey → 一条 BspLink（组内多条链接同事务写入）。
        for rule in &policy.rules {
            let projection = ProjectionKey {
                lineage: closed_lineage.key.clone(),
                policy_id: rule.policy_id,
                rule_id: rule.rule_id,
                slot: rule.slot,
            };
            let link_key = LinkKey {
                bsp: bsp_key.clone(),
                projection,
            };
            // 幂等：已链接跳过。
            if prior_links_by_key.contains_key(&link_key) {
                continue;
            }
            if !linked_this_call.insert(link_key.clone()) {
                continue;
            }
            links.push(BspLink {
                key: link_key,
                side: bsp_key.side,
                written_at: as_of,
            });
        }
    }
    Ok((creations, links))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::super::super::types::Side;
    use super::super::bsp_bridge::{BspPointClass, BspStructuralKey};
    use super::super::cand_event::{
        CandidateKey, CandidateKind, CandidateState, ParentFingerprint,
    };
    use super::super::chain_cert::{
        ChainKey, ChainNodeStatus, ChainNodeTrace, ChainStatus, TowerChainCertificate,
    };
    use super::*;

    /// 测试脚手架：构造一个合法形状的候选键（只借公共字段，不 mock 现役对象内部）。
    fn candidate(level: u32, c_start: usize) -> CandidateKey {
        CandidateKey {
            rule_version: 0,
            level,
            kind: CandidateKind::Trend,
            side: Side::Long,
            previous_center_start: None,
            parent: ParentFingerprint {
                center_start: 0,
                zd: 0,
                zg: 0,
            },
            seg_a: (0, 0),
            c_start,
        }
    }

    /// 测试脚手架：构造一个合法形状的 BSP 结构键（只借公共字段）。
    fn bsp_key() -> BspStructuralKey {
        bsp_key_with(BspPointClass::Buy1, Side::Long, 2)
    }

    fn bsp_key_with(class: BspPointClass, side: Side, level: u32) -> BspStructuralKey {
        BspStructuralKey {
            rule_version: 0,
            level,
            parent: ParentFingerprint {
                center_start: 0,
                zd: 0,
                zg: 0,
            },
            side,
            class,
            anchor: vec![(0, 0)],
        }
    }

    fn rule(rule_id: u32, policy_id: u32, slot: u32) -> PolicyRule {
        PolicyRule {
            rule_id,
            policy_id,
            slot,
        }
    }

    fn policy(rules: Vec<PolicyRule>) -> ManagedBspPolicy {
        ManagedBspPolicy { rules }
    }

    fn alive_node(key: CandidateKey) -> ChainNodeTrace {
        ChainNodeTrace {
            key,
            level: key.level,
            status: ChainNodeStatus::Alive,
            state: Some(CandidateState::Confirmed),
        }
    }

    /// 一条 `Closed` 链（全节点存活），`closed_at == as_of`（不触发 LateAuthorization）。
    fn closed_certificate(as_of: usize, node_keys: Vec<CandidateKey>) -> TowerChainCertificate {
        let key = ChainKey::new(node_keys.clone());
        TowerChainCertificate {
            key,
            extends: None,
            root_level: node_keys[0].level,
            leaf_level: node_keys.last().expect("≥2 节点").level,
            nodes: node_keys.iter().map(|&k| alive_node(k)).collect(),
            edges: vec![],
            extendable: false,
            status: ChainStatus::Closed,
            observed_at: as_of,
            closed_at: Some(as_of),
            invalidated_at: None,
            invalidation_cause: None,
            revision: 0,
            supersedes_revision: None,
            revision_at: as_of,
        }
    }

    /// 真值表：Closed 链 + 两个存活节点各映射一个不同 BSP + 两条 rule。
    /// creation 数 = 唯一 bsp 数；link 数 = bsp 数 × rule 数；字段逐项正确。
    #[test]
    fn truth_table_creates_per_bsp_and_links_per_rule() {
        let as_of = 10;
        let node0 = candidate(3, 10);
        let node1 = candidate(2, 5);
        let certificate = closed_certificate(as_of, vec![node0, node1]);
        let bsp0 = bsp_key();
        let bsp1 = bsp_key_with(BspPointClass::Sell1, Side::Short, 2);
        let mut event_to_bsp = HashMap::new();
        event_to_bsp.insert(node0, bsp0.clone());
        event_to_bsp.insert(node1, bsp1.clone());
        let policy = policy(vec![rule(1, 100, 0), rule(2, 100, 1)]);

        let (creations, links) = consume_at(
            as_of,
            &HashMap::new(),
            &HashMap::new(),
            &certificate,
            &policy,
            &event_to_bsp,
        )
        .expect("Closed + 合法 policy + 空 prior 应产出");

        assert_eq!(creations.len(), 2, "creation 数 = 唯一 bsp 数");
        assert_eq!(links.len(), 4, "link 数 = bsp 数 × rule 数");

        // creation 逐字段：key 覆盖两个 bsp，created_at == as_of。
        let created_keys: HashSet<BspStructuralKey> = creations
            .iter()
            .map(|creation| creation.bsp.key.clone())
            .collect();
        assert_eq!(created_keys, HashSet::from([bsp0.clone(), bsp1.clone()]));
        for creation in &creations {
            assert_eq!(creation.bsp.created_at, as_of);
        }

        // link 逐字段：key 集合 = {bsp0, bsp1} × 两条 rule 的 ProjectionKey；
        // lineage == certificate.key；written_at == as_of；side == 对应 bsp.side。
        let mut expected_links = HashSet::new();
        for bsp in [&bsp0, &bsp1] {
            for r in &policy.rules {
                expected_links.insert(LinkKey {
                    bsp: bsp.clone(),
                    projection: ProjectionKey {
                        lineage: certificate.key.clone(),
                        policy_id: r.policy_id,
                        rule_id: r.rule_id,
                        slot: r.slot,
                    },
                });
            }
        }
        let link_keys: HashSet<LinkKey> = links.iter().map(|link| link.key.clone()).collect();
        assert_eq!(link_keys, expected_links);
        for link in &links {
            assert_eq!(link.key.projection.lineage, certificate.key);
            assert_eq!(link.written_at, as_of);
            let expected_side = if link.key.bsp == bsp0 {
                bsp0.side
            } else {
                bsp1.side
            };
            assert_eq!(link.side, expected_side);
        }
    }

    /// 幂等：prior 已有某 bsp → 不产 creation、但产 link。
    #[test]
    fn idempotent_prior_bsp_skips_creation_but_still_links() {
        let as_of = 10;
        let node0 = candidate(3, 10);
        let certificate = closed_certificate(as_of, vec![node0, candidate(2, 5)]);
        let bsp0 = bsp_key();
        let mut event_to_bsp = HashMap::new();
        event_to_bsp.insert(node0, bsp0.clone());
        let policy = policy(vec![rule(1, 100, 0)]);

        let mut prior_bsp_by_key = HashMap::new();
        prior_bsp_by_key.insert(
            bsp0.clone(),
            ManagedBsp {
                key: bsp0.clone(),
                created_at: 5,
            },
        );

        let (creations, links) = consume_at(
            as_of,
            &prior_bsp_by_key,
            &HashMap::new(),
            &certificate,
            &policy,
            &event_to_bsp,
        )
        .expect("prior 未超前，应产出");

        assert!(creations.is_empty(), "prior 已有 bsp 不产 creation");
        assert_eq!(links.len(), 1, "prior 已有 bsp 仍产 link");
        assert_eq!(links[0].key.bsp, bsp0);
        assert_eq!(links[0].written_at, as_of);
    }

    /// 幂等：prior 已有某 link → 该 link 不重复产（同 bsp 的其余 rule 照产）。
    #[test]
    fn idempotent_prior_link_is_not_reproduced() {
        let as_of = 10;
        let node0 = candidate(3, 10);
        let certificate = closed_certificate(as_of, vec![node0, candidate(2, 5)]);
        let bsp0 = bsp_key();
        let mut event_to_bsp = HashMap::new();
        event_to_bsp.insert(node0, bsp0.clone());
        let policy = policy(vec![rule(1, 100, 0), rule(2, 100, 1)]);

        let rule0 = &policy.rules[0];
        let existing_link_key = LinkKey {
            bsp: bsp0.clone(),
            projection: ProjectionKey {
                lineage: certificate.key.clone(),
                policy_id: rule0.policy_id,
                rule_id: rule0.rule_id,
                slot: rule0.slot,
            },
        };
        let mut prior_links_by_key = HashMap::new();
        prior_links_by_key.insert(
            existing_link_key.clone(),
            BspLink {
                key: existing_link_key,
                side: bsp0.side,
                written_at: 3,
            },
        );

        let (creations, links) = consume_at(
            as_of,
            &HashMap::new(),
            &prior_links_by_key,
            &certificate,
            &policy,
            &event_to_bsp,
        )
        .expect("prior 未超前，应产出");

        assert_eq!(creations.len(), 1, "bsp 不在 prior，仍产 creation");
        assert_eq!(links.len(), 1, "已链接的 rule 不重复产，剩第二条 rule");
        assert_eq!(links[0].key.projection.rule_id, 2);
    }

    /// 防御性去重：同一 bsp 被两个节点命中（event_to_bsp 单射时不可达），
    /// 仍只产一次 creation、每条 rule 只产一次 link。
    #[test]
    fn defensive_dedup_same_bsp_from_two_nodes() {
        let as_of = 10;
        let node0 = candidate(3, 10);
        let node1 = candidate(2, 5);
        let certificate = closed_certificate(as_of, vec![node0, node1]);
        let bsp0 = bsp_key();
        let mut event_to_bsp = HashMap::new();
        event_to_bsp.insert(node0, bsp0.clone());
        event_to_bsp.insert(node1, bsp0.clone());
        let policy = policy(vec![rule(1, 100, 0), rule(2, 100, 1)]);

        let (creations, links) = consume_at(
            as_of,
            &HashMap::new(),
            &HashMap::new(),
            &certificate,
            &policy,
            &event_to_bsp,
        )
        .expect("Closed + 合法 policy + 空 prior 应产出");

        assert_eq!(creations.len(), 1, "同一 bsp 只产一次 creation");
        assert_eq!(links.len(), 2, "同一 bsp × 两条 rule 只产两条 link");
    }

    /// 空产出：Closed 链但 event_to_bsp 全查无 → Ok((空, 空))，非错误。
    #[test]
    fn closed_chain_with_all_missing_events_yields_empty_output() {
        let as_of = 10;
        let certificate = closed_certificate(as_of, vec![candidate(3, 10), candidate(2, 5)]);
        let policy = policy(vec![rule(1, 100, 0)]);

        let (creations, links) = consume_at(
            as_of,
            &HashMap::new(),
            &HashMap::new(),
            &certificate,
            &policy,
            &HashMap::new(),
        )
        .expect("空 event_to_bsp 是 Absent 非证伪，不报错");

        assert!(creations.is_empty());
        assert!(links.is_empty());
    }

    /// 错误分支 1：非 Closed 谱系 → NoConsumption。
    #[test]
    fn non_closed_lineage_returns_no_consumption() {
        let as_of = 10;
        let mut certificate = closed_certificate(as_of, vec![candidate(3, 10), candidate(2, 5)]);
        certificate.status = ChainStatus::Open;
        certificate.closed_at = None;

        let err = consume_at(
            as_of,
            &HashMap::new(),
            &HashMap::new(),
            &certificate,
            &policy(vec![rule(1, 100, 0)]),
            &HashMap::new(),
        )
        .unwrap_err();

        assert_eq!(err, ConsumeError::NoConsumption);
    }

    /// 错误分支 2：closed_at > as_of → LateAuthorization。
    #[test]
    fn closed_after_as_of_returns_late_authorization() {
        let as_of = 10;
        let mut certificate = closed_certificate(as_of, vec![candidate(3, 10), candidate(2, 5)]);
        certificate.closed_at = Some(as_of + 1);

        let err = consume_at(
            as_of,
            &HashMap::new(),
            &HashMap::new(),
            &certificate,
            &policy(vec![rule(1, 100, 0)]),
            &HashMap::new(),
        )
        .unwrap_err();

        assert_eq!(err, ConsumeError::LateAuthorization);
    }

    /// 错误分支 3a：空 rules → InvalidPolicy。
    #[test]
    fn empty_rules_returns_invalid_policy() {
        let as_of = 10;
        let certificate = closed_certificate(as_of, vec![candidate(3, 10), candidate(2, 5)]);

        let err = consume_at(
            as_of,
            &HashMap::new(),
            &HashMap::new(),
            &certificate,
            &policy(vec![]),
            &HashMap::new(),
        )
        .unwrap_err();

        assert_eq!(err, ConsumeError::InvalidPolicy);
    }

    /// 错误分支 3b：两条 rule 投影到同一 (policy_id, rule_id, slot) → InvalidPolicy。
    #[test]
    fn duplicate_projection_rule_returns_invalid_policy() {
        let as_of = 10;
        let certificate = closed_certificate(as_of, vec![candidate(3, 10), candidate(2, 5)]);

        let err = consume_at(
            as_of,
            &HashMap::new(),
            &HashMap::new(),
            &certificate,
            &policy(vec![rule(1, 100, 0), rule(1, 100, 0)]),
            &HashMap::new(),
        )
        .unwrap_err();

        assert_eq!(err, ConsumeError::InvalidPolicy);
    }

    /// 错误分支 4：prior 超前（created_at > as_of）→ InconsistentState。
    #[test]
    fn prior_bsp_ahead_returns_inconsistent_state() {
        let as_of = 10;
        let certificate = closed_certificate(as_of, vec![candidate(3, 10), candidate(2, 5)]);
        let mut prior_bsp_by_key = HashMap::new();
        prior_bsp_by_key.insert(
            bsp_key(),
            ManagedBsp {
                key: bsp_key(),
                created_at: as_of + 1,
            },
        );

        let err = consume_at(
            as_of,
            &prior_bsp_by_key,
            &HashMap::new(),
            &certificate,
            &policy(vec![rule(1, 100, 0)]),
            &HashMap::new(),
        )
        .unwrap_err();

        assert_eq!(err, ConsumeError::InconsistentState);
    }

    /// 错误分支 4b：prior link 超前（written_at > as_of）→ InconsistentState。
    #[test]
    fn prior_link_ahead_returns_inconsistent_state() {
        let as_of = 10;
        let node0 = candidate(3, 10);
        let certificate = closed_certificate(as_of, vec![node0, candidate(2, 5)]);
        let bsp0 = bsp_key();
        let link_key = LinkKey {
            bsp: bsp0.clone(),
            projection: ProjectionKey {
                lineage: certificate.key.clone(),
                policy_id: 100,
                rule_id: 1,
                slot: 0,
            },
        };
        let mut prior_links_by_key = HashMap::new();
        prior_links_by_key.insert(
            link_key,
            BspLink {
                key: LinkKey {
                    bsp: bsp0,
                    projection: ProjectionKey {
                        lineage: certificate.key.clone(),
                        policy_id: 100,
                        rule_id: 1,
                        slot: 0,
                    },
                },
                side: Side::Long,
                written_at: as_of + 1,
            },
        );

        let err = consume_at(
            as_of,
            &HashMap::new(),
            &prior_links_by_key,
            &certificate,
            &policy(vec![rule(1, 100, 0)]),
            &HashMap::new(),
        )
        .unwrap_err();

        assert_eq!(err, ConsumeError::InconsistentState);
    }

    /// `ConsumeError` 是 `Copy`（按值返回的错误枚举，调用侧可随意复制比较）。
    #[test]
    fn consume_error_is_copy() {
        let e = ConsumeError::NoConsumption;
        let copied = e;
        assert_eq!(e, copied);
    }

    /// 签名机器锁：把 `consume_at` 绑到与票面逐字一致的函数类型上（#981 冻结 + #982 A 加参）。
    #[test]
    fn consume_at_has_frozen_signature() {
        let _f: fn(
            usize,
            &HashMap<BspStructuralKey, ManagedBsp>,
            &HashMap<LinkKey, BspLink>,
            &TowerChainCertificate,
            &ManagedBspPolicy,
            &HashMap<CandidateKey, BspStructuralKey>,
        ) -> Result<(Vec<ManagedBspCreation>, Vec<BspLink>), ConsumeError> = consume_at;
    }
}
