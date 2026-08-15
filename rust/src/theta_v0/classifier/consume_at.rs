//! #981（N7）`Consume_at` 签名冻结落地（map #529）。
//!
//! ## Seam（一行）
//!
//! ```text
//! 现役对象（BspStructuralKey / ChainKey / TowerChainCertificate）
//!   → 新类型族（ProjectionKey / LinkKey / ManagedBsp / BspLink / ManagedBspPolicy / ConsumeError）
//!   → consume_at 函数签名（stub，逻辑 todo!()）
//! ```
//!
//! 左半边（现役对象）**零改**（唯一的例外是本票允许的 `BspStructuralKey` 增加 `Hash` derive，
//! 见 `bsp_bridge.rs`，不动字段不动结构）；右半边（新类型族）全部新造，落在**本文件**——不在
//! 现役文件里塞类型。本票只做「冻结签名」一层：不实现消费逻辑（归后续票），不 mock 现役对象
//! 内部、不测私有路径、不改 parser / 递归塔。
//!
//! ## 认识论（formalization-validity-domain）
//!
//! 实装本身 = L1（bit-exact 一致性中的「签名/形状翻译正确」一档）：冻结签名与类型族逐字段落成
//! Rust，编译期即锁死签名。**不声称 L2/L3**——`consume_at` 消费逻辑在市场上的有效性归后续票。
//!
//! ## 冻结语义来源（自包含）
//!
//! - 谱系综合 §2：`chanlun/review-results/e2eo-lineage-synthesis-20260728.md`
//!   （Consume_at 签名 + 冻结语义）。
//! - #980 裁定 A：BspKey = [`BspStructuralKey`]（episode 粒度），残余规则「跨多条链只创建一次、
//!   多条链接写 BspLink 组内」。

use std::collections::HashMap;

use super::super::types::Side;
use super::bsp_bridge::BspStructuralKey;
use super::chain_cert::{ChainKey, TowerChainCertificate};

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

/// Consume_at 冻结签名（逻辑 stub，`todo!("consume_at 逻辑归后续票")`）。
///
/// 冻结语义（谱系综合 §2 / roadmap）：
/// - 只接收 `Closed` 谱系（`closed_lineage.status == ChainStatus::Closed`，否则
///   `NoConsumption`/`InvalidPolicy`）；
/// - 授权 BSP 在同一生产事务形成并写入谱系反向边，不是给既有 BSP 事后贴标；
/// - 以旧 BSP/链接映射（`prior_bsp_by_key` / `prior_links_by_key`）保证投影幂等——
///   同一 BspKey 已存在则不重复创建，只补 BspLink（残余规则 #980：跨多条链只创建一次，
///   多条链接写 BspLink 组内）。
pub fn consume_at(
    as_of: usize,
    prior_bsp_by_key: &HashMap<BspStructuralKey, ManagedBsp>,
    prior_links_by_key: &HashMap<LinkKey, BspLink>,
    closed_lineage: &TowerChainCertificate,
    policy: &ManagedBspPolicy,
) -> Result<(Vec<ManagedBspCreation>, Vec<BspLink>), ConsumeError> {
    let _ = (
        as_of,
        prior_bsp_by_key,
        prior_links_by_key,
        closed_lineage,
        policy,
    );
    todo!("consume_at 逻辑归后续票（N7 实装），本票只冻结签名")
}

#[cfg(test)]
mod tests {
    use super::super::bsp_bridge::BspPointClass;
    use super::super::cand_event::{CandidateKey, CandidateKind, ParentFingerprint};
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

    /// 测试脚手架：经 `ChainKey::new` 公共构造点构造一条两节点链（沿用现役 ≥2 节点不变量）。
    fn chain_key() -> ChainKey {
        ChainKey::new(vec![candidate(3, 10), candidate(2, 5)])
    }

    /// 测试脚手架：构造一个合法形状的 BSP 结构键（只借公共字段）。
    fn bsp_key() -> BspStructuralKey {
        BspStructuralKey {
            rule_version: 0,
            level: 2,
            parent: ParentFingerprint {
                center_start: 0,
                zd: 0,
                zg: 0,
            },
            side: Side::Long,
            class: BspPointClass::Buy1,
            anchor: vec![(0, 0)],
        }
    }

    #[test]
    fn projection_key_constructs_and_fields_read() {
        let lineage = chain_key();
        let key = ProjectionKey {
            lineage: lineage.clone(),
            policy_id: 1,
            rule_id: 2,
            slot: 3,
        };
        assert_eq!(key.lineage, lineage);
        assert_eq!(key.policy_id, 1);
        assert_eq!(key.rule_id, 2);
        assert_eq!(key.slot, 3);
    }

    #[test]
    fn link_key_constructs_and_fields_read() {
        let bsp = bsp_key();
        let projection = ProjectionKey {
            lineage: chain_key(),
            policy_id: 1,
            rule_id: 2,
            slot: 3,
        };
        let key = LinkKey {
            bsp: bsp.clone(),
            projection: projection.clone(),
        };
        assert_eq!(key.bsp, bsp);
        assert_eq!(key.projection, projection);
    }

    #[test]
    fn managed_bsp_constructs_and_fields_read() {
        let bsp = bsp_key();
        let managed = ManagedBsp {
            key: bsp.clone(),
            created_at: 42,
        };
        assert_eq!(managed.key, bsp);
        assert_eq!(managed.created_at, 42);
    }

    #[test]
    fn managed_bsp_creation_constructs_and_fields_read() {
        let managed = ManagedBsp {
            key: bsp_key(),
            created_at: 42,
        };
        let creation = ManagedBspCreation {
            bsp: managed.clone(),
        };
        assert_eq!(creation.bsp, managed);
    }

    #[test]
    fn bsp_link_constructs_and_fields_read() {
        let key = LinkKey {
            bsp: bsp_key(),
            projection: ProjectionKey {
                lineage: chain_key(),
                policy_id: 1,
                rule_id: 2,
                slot: 3,
            },
        };
        let link = BspLink {
            key: key.clone(),
            side: Side::Short,
            written_at: 7,
        };
        assert_eq!(link.key, key);
        assert_eq!(link.side, Side::Short);
        assert_eq!(link.written_at, 7);
    }

    #[test]
    fn managed_bsp_policy_constructs_and_fields_read() {
        let rule = PolicyRule {
            rule_id: 2,
            policy_id: 1,
            slot: 3,
        };
        let policy = ManagedBspPolicy {
            rules: vec![rule.clone()],
        };
        assert_eq!(policy.rules.len(), 1);
        assert_eq!(policy.rules[0], rule);
    }

    #[test]
    fn policy_rule_constructs_and_fields_read() {
        let rule = PolicyRule {
            rule_id: 2,
            policy_id: 1,
            slot: 3,
        };
        assert_eq!(rule.rule_id, 2);
        assert_eq!(rule.policy_id, 1);
        assert_eq!(rule.slot, 3);
    }

    #[test]
    fn consume_error_is_copy_and_variants_distinct() {
        let e = ConsumeError::NoConsumption;
        let copied = e;
        assert_eq!(e, copied);
        assert_ne!(ConsumeError::NoConsumption, ConsumeError::InvalidPolicy);
        assert_ne!(ConsumeError::InvalidPolicy, ConsumeError::InconsistentState);
        assert_ne!(
            ConsumeError::InconsistentState,
            ConsumeError::LateAuthorization
        );
    }

    /// 冻结签名的机器锁：把 `consume_at` 绑到与票面逐字一致的函数类型上。
    ///
    /// 只断言「签名可调用」（参数/返回类型与冻结签名完全一致），**不调用**——本票的
    /// `consume_at` 是 `todo!()` stub，调用必 panic，故签名由编译期类型断言锁死，逻辑归后续票。
    #[test]
    fn consume_at_has_frozen_signature() {
        let _f: fn(
            usize,
            &HashMap<BspStructuralKey, ManagedBsp>,
            &HashMap<LinkKey, BspLink>,
            &TowerChainCertificate,
            &ManagedBspPolicy,
        ) -> Result<(Vec<ManagedBspCreation>, Vec<BspLink>), ConsumeError> = consume_at;
    }
}
