//! LEE Consume_ℓ 确定性路由（multi-level-native-execution-design-20260719 §C.3 支柱 3）。
//!
//! Consume_at 解决「BSP 什么时候、凭什么谱系被授权形成」（生产侧）；本模块解决「授权形成的
//! BSP 由谁持有」（执行侧）——二者在 ManagedBspCreation/BspLink 输出处对接。本模块只读
//! `BspStructuralKey.level`（≡ formation_level，LEE 设计「不新增口径」纪律），不做任何判定、
//! 不产订单、不碰声部腿（SepLeg）——那是 M2/M4 sizing 与 fill 的事。
//!
//! 认识论等级：L1（纯分桶，确定性路由，零信息增量，不声明 alpha）。

use std::collections::BTreeMap;

use crate::theta_v0::classifier::consume_at::{ManagedBsp, ManagedBspCreation};

/// 受管 BSP 的级别持有账本：`formation_level → 受管 BSP 列表`。
/// BTreeMap 保确定序（同输入 ⟹ 同输出，bit-exact 可复现，同 level_ledger.rs 确定序纪律）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ManagedBspLedger {
    by_level: BTreeMap<u32, Vec<ManagedBsp>>,
}

impl ManagedBspLedger {
    pub fn new() -> Self {
        Self {
            by_level: BTreeMap::new(),
        }
    }

    /// 该级别的受管 BSP（升序 = 插入序，同一次 route 内按 creations 序）。
    pub fn bsp_at(&self, level: u32) -> &[ManagedBsp] {
        self.by_level
            .get(&level)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// 已占用的 formation_level（BTreeMap 键序 ⟹ 升序）。
    pub fn levels(&self) -> impl Iterator<Item = u32> + '_ {
        self.by_level.keys().copied()
    }

    pub fn total(&self) -> usize {
        self.by_level.values().map(|v| v.len()).sum()
    }

    /// 分桶不变量：每个 BSP 恰好落在其 formation_level 桶（测试锁机器化）。
    pub fn level_invariant_holds(&self) -> bool {
        self.by_level
            .iter()
            .all(|(&lvl, bsps)| bsps.iter().all(|b| b.key.level == lvl))
    }
}

/// 确定性路由：`ManagedBspCreation` 按其 `BspStructuralKey.level`（≡ formation_level）分桶。
/// 不改 Consume_at 生产语义，只改输出的持有方式（纯下游扩展）。
pub fn route_consume(creations: &[ManagedBspCreation]) -> ManagedBspLedger {
    let mut ledger = ManagedBspLedger::new();
    for c in creations {
        ledger
            .by_level
            .entry(c.bsp.key.level)
            .or_default()
            .push(c.bsp.clone());
    }
    ledger
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theta_v0::classifier::bsp_bridge::{BspPointClass, BspStructuralKey};
    use crate::theta_v0::classifier::cand_event::ParentFingerprint;
    use crate::theta_v0::types::Side;

    /// 最小 `BspStructuralKey` 构造器（仅填 `level`，其余身份分量固定占位——
    /// 路由只读 `key.level`，不消费其他身份分量，测试不 mock 现役对象私有）。
    fn bsp_key(level: u32) -> BspStructuralKey {
        BspStructuralKey {
            rule_version: 1,
            level,
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

    /// 最小 `ManagedBspCreation` 构造器（公开字段 struct literal，`created_at` 作逐项区分标）。
    fn creation(level: u32, created_at: usize) -> ManagedBspCreation {
        ManagedBspCreation {
            bsp: ManagedBsp {
                key: bsp_key(level),
                created_at,
            },
        }
    }

    /// 分桶正确性：level ∈ {1,2,3} 各若干，`bsp_at(level)` 内容逐项正确、`total()` = 总数。
    #[test]
    fn route_buckets_by_formation_level() {
        let creations = vec![
            creation(1, 100),
            creation(2, 200),
            creation(3, 300),
            creation(1, 101),
            creation(2, 201),
            creation(3, 301),
            creation(2, 202),
        ];
        let ledger = route_consume(&creations);

        // 总数 = 7（每个 creation 恰好落一桶，无丢失、无重复）。
        assert_eq!(ledger.total(), 7);

        // 逐项：level 1 两桶，按 route 内 creations 序（插入序）。
        let l1 = ledger.bsp_at(1);
        assert_eq!(l1.len(), 2);
        assert_eq!(l1[0].key.level, 1);
        assert_eq!(l1[0].created_at, 100);
        assert_eq!(l1[1].key.level, 1);
        assert_eq!(l1[1].created_at, 101);

        // 逐项：level 2 三桶。
        let l2 = ledger.bsp_at(2);
        assert_eq!(l2.len(), 3);
        assert_eq!(l2[0].key.level, 2);
        assert_eq!(l2[0].created_at, 200);
        assert_eq!(l2[1].key.level, 2);
        assert_eq!(l2[1].created_at, 201);
        assert_eq!(l2[2].key.level, 2);
        assert_eq!(l2[2].created_at, 202);

        // 逐项：level 3 两桶。
        let l3 = ledger.bsp_at(3);
        assert_eq!(l3.len(), 2);
        assert_eq!(l3[0].key.level, 3);
        assert_eq!(l3[0].created_at, 300);
        assert_eq!(l3[1].key.level, 3);
        assert_eq!(l3[1].created_at, 301);

        // 未占级别 ⟹ 空切片；`levels()` 键序升序。
        assert!(ledger.bsp_at(4).is_empty());
        assert_eq!(ledger.levels().collect::<Vec<_>>(), vec![1, 2, 3]);
    }

    /// 确定性：同一批 creations 两次 route → 两个账本 `PartialEq` 相等
    /// （BTreeMap 保序 + 插入序）。
    #[test]
    fn route_is_deterministic() {
        let creations = vec![
            creation(1, 10),
            creation(3, 30),
            creation(2, 20),
            creation(1, 11),
            creation(2, 21),
        ];
        let a = route_consume(&creations);
        let b = route_consume(&creations);
        assert_eq!(a, b);
    }

    /// 分桶不变量：`level_invariant_holds()` 为真（每个 BSP 恰落其 formation_level 桶）。
    #[test]
    fn level_invariant_holds_on_routed_ledger() {
        let creations = vec![
            creation(1, 10),
            creation(2, 20),
            creation(3, 30),
            creation(2, 21),
        ];
        let ledger = route_consume(&creations);
        assert!(ledger.level_invariant_holds());
        // 空账本（未 route 任何输入）不变量真空成立。
        assert!(ManagedBspLedger::new().level_invariant_holds());
    }

    /// 空输入：空 creations → `total()==0`、`levels()` 空。
    #[test]
    fn route_empty_input() {
        let empty: Vec<ManagedBspCreation> = vec![];
        let ledger = route_consume(&empty);
        assert_eq!(ledger.total(), 0);
        assert_eq!(ledger.levels().collect::<Vec<_>>(), Vec::<u32>::new());
        assert!(ledger.level_invariant_holds());
    }
}
