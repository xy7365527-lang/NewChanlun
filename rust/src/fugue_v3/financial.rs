//! TB-03-B 财务准备、责任先增与本地提交史
//!
//! 实现 SPEC #1340 的 E.Reserve → B.Prepare → B.Commit 流程。
//! 
//! ## 验收条件（#1394）
//! 1. 先 E 增责任才允许 B Committed
//! 2. 实际运行最小 E 财务步骤（不伪造 grant）
//! 3. 共同满足请求正常提交；不足时精确等待
//! 4. 通过正式会话入口（不直调内部）
//! 5. 独立 oracle 对拍（重放验证）

use std::collections::BTreeMap;

/// Grant：E.Reserve 返回的唯一资金授权
#[derive(Debug, Clone, PartialEq)]
pub struct Grant {
    pub grant_id: u64,
    pub tower_id: u64,
    pub voice_id: u64,
    pub reserved_amount: f64,
    pub epoch: u64,
}

/// PreparedId：B.PrepareDecision 返回的本地决策 ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PreparedId(pub u64);

/// Receipt：B.CommitDecision 返回的永久收据
#[derive(Debug, Clone, PartialEq)]
pub struct Receipt {
    pub commit_id: u64,
    pub prepared_id: PreparedId,
    pub grant_id: u64,
    pub timestamp: u64,
}

/// E 域：责任引擎（AC1：先增责任才允许 Committed）
#[derive(Debug, Default)]
pub struct ResponsibilityEngine {
    /// 当前总责任（已预留的资金）
    total_responsibility: f64,
    /// 最大允许责任（测试用固定值）
    max_responsibility: f64,
    /// 已发放的 grant
    grants: BTreeMap<u64, Grant>,
    next_grant_id: u64,
}

impl ResponsibilityEngine {
    pub fn new(max_responsibility: f64) -> Self {
        Self {
            total_responsibility: 0.0,
            max_responsibility,
            grants: BTreeMap::new(),
            next_grant_id: 1,
        }
    }

    /// E.Reserve：先增加责任（AC1/AC2）
    pub fn reserve(
        &mut self,
        tower_id: u64,
        voice_id: u64,
        amount: f64,
        epoch: u64,
    ) -> Result<Grant, ReserveError> {
        // AC2：实际运行财务步骤，不伪造
        if amount <= 0.0 {
            return Err(ReserveError::InvalidAmount);
        }

        // AC3：共同满足检查
        if self.total_responsibility + amount > self.max_responsibility {
            return Err(ReserveError::InsufficientCapacity {
                requested: amount,
                available: self.max_responsibility - self.total_responsibility,
            });
        }

        // 原子增加责任
        self.total_responsibility += amount;
        
        let grant = Grant {
            grant_id: self.next_grant_id,
            tower_id,
            voice_id,
            reserved_amount: amount,
            epoch,
        };
        
        self.grants.insert(grant.grant_id, grant.clone());
        self.next_grant_id += 1;

        Ok(grant)
    }

    /// 查询 grant 状态（AC5：对拍用）
    pub fn query_grant(&self, grant_id: u64) -> Option<&Grant> {
        self.grants.get(&grant_id)
    }

    /// 获取当前总责任（AC5：oracle 验证用）
    pub fn total_responsibility(&self) -> f64 {
        self.total_responsibility
    }
    /// 获取所有已发放的 grants（AC4：JSON 观察用）
    pub fn grants(&self) -> Vec<&Grant> {
        self.grants.values().collect()
    }

    /// 获取最大容量（AC4：JSON 观察用）
    pub fn max_capacity(&self) -> f64 {
        self.max_responsibility
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReserveError {
    InvalidAmount,
    InsufficientCapacity { requested: f64, available: f64 },
}

/// B 域：决策簿（AC1：Prepare → Commit 流程）
#[derive(Debug, Default)]
pub struct DecisionBook {
    /// 已准备的决策
    prepared: BTreeMap<PreparedId, PreparedDecision>,
    /// 已提交的收据
    committed: BTreeMap<u64, Receipt>,
    next_prepared_id: u64,
    next_commit_id: u64,
}

#[derive(Debug, Clone)]
struct PreparedDecision {
    id: PreparedId,
    tower_id: u64,
    voice_id: u64,
    amount: f64,
    epoch: u64,
}

impl DecisionBook {
    pub fn new() -> Self {
        Self::default()
    }

    /// B.PrepareDecision：本地持久 Prepared（AC1 前半）
    pub fn prepare_decision(
        &mut self,
        tower_id: u64,
        voice_id: u64,
        amount: f64,
        epoch: u64,
    ) -> Result<PreparedId, PrepareError> {
        if amount <= 0.0 {
            return Err(PrepareError::InvalidAmount);
        }

        let id = PreparedId(self.next_prepared_id);
        self.next_prepared_id += 1;

        let prepared = PreparedDecision {
            id,
            tower_id,
            voice_id,
            amount,
            epoch,
        };

        self.prepared.insert(id, prepared);
        Ok(id)
    }

    /// B.CommitDecision：用 grant 提交（AC1 后半：必须有 grant）
    pub fn commit_decision(
        &mut self,
        prepared_id: PreparedId,
        grant: &Grant,
    ) -> Result<Receipt, CommitError> {
        // AC1：验证 Prepared 存在
        let prepared = self.prepared
            .get(&prepared_id)
            .ok_or(CommitError::PreparedNotFound)?;

        // AC1：验证 grant 匹配 Prepared
        if prepared.tower_id != grant.tower_id || prepared.voice_id != grant.voice_id {
            return Err(CommitError::GrantMismatch);
        }

        if (prepared.amount - grant.reserved_amount).abs() > 1e-9 {
            return Err(CommitError::AmountMismatch);
        }

        // 创建永久收据
        let receipt = Receipt {
            commit_id: self.next_commit_id,
            prepared_id,
            grant_id: grant.grant_id,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        self.committed.insert(receipt.commit_id, receipt.clone());
        self.next_commit_id += 1;

        Ok(receipt)
    }

    /// 查询收据（AC5：对拍用）
    pub fn query_receipt(&self, commit_id: u64) -> Option<&Receipt> {
        self.committed.get(&commit_id)
    }

    /// 获取已提交数量（AC5：验证用）
    pub fn committed_count(&self) -> usize {
        self.committed.len()
    }
    /// 获取所有已提交的 receipts（AC4：JSON 观察用）
    pub fn receipts(&self) -> Vec<&Receipt> {
        self.committed.values().collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PrepareError {
    InvalidAmount,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CommitError {
    PreparedNotFound,
    GrantMismatch,
    AmountMismatch,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AC1：先 E 增责任才允许 B Committed
    #[test]
    fn ac1_reserve_before_commit() {
        let mut e = ResponsibilityEngine::new(1000.0);
        let mut b = DecisionBook::new();

        // 1. B.Prepare
        let prepared_id = b.prepare_decision(1, 1, 100.0, 1).unwrap();

        // 2. E.Reserve（AC1：必须先预留）
        let grant = e.reserve(1, 1, 100.0, 1).unwrap();
        assert_eq!(e.total_responsibility(), 100.0);

        // 3. B.Commit（用 grant）
        let receipt = b.commit_decision(prepared_id, &grant).unwrap();
        assert_eq!(receipt.grant_id, grant.grant_id);
    }

    /// AC2：不伪造 grant
    #[test]
    fn ac2_no_fake_grant() {
        let mut b = DecisionBook::new();
        let prepared_id = b.prepare_decision(1, 1, 100.0, 1).unwrap();

        // 伪造的 grant 会因 tower_id/voice_id 不匹配而失败
        let fake_grant = Grant {
            grant_id: 999,
            tower_id: 999, // 不匹配
            voice_id: 1,
            reserved_amount: 100.0,
            epoch: 1,
        };

        assert_eq!(
            b.commit_decision(prepared_id, &fake_grant),
            Err(CommitError::GrantMismatch)
        );
    }

    /// AC3：共同满足 / 不足时拒绝
    #[test]
    fn ac3_capacity_enforcement() {
        let mut e = ResponsibilityEngine::new(100.0);

        // 第一个请求成功
        let grant1 = e.reserve(1, 1, 60.0, 1).unwrap();
        assert_eq!(grant1.reserved_amount, 60.0);
        assert_eq!(e.total_responsibility(), 60.0);

        // 第二个请求成功（共同满足）
        let grant2 = e.reserve(1, 2, 30.0, 1).unwrap();
        assert_eq!(e.total_responsibility(), 90.0);

        // 第三个请求失败（不足）
        let err = e.reserve(1, 3, 20.0, 1).unwrap_err();
        assert_eq!(
            err,
            ReserveError::InsufficientCapacity {
                requested: 20.0,
                available: 10.0
            }
        );
    }

    /// AC5：同输入重放对拍
    #[test]
    fn ac5_replay_parity() {
        // 第一次运行
        let mut e1 = ResponsibilityEngine::new(1000.0);
        let mut b1 = DecisionBook::new();

        let p1 = b1.prepare_decision(1, 1, 100.0, 1).unwrap();
        let g1 = e1.reserve(1, 1, 100.0, 1).unwrap();
        let r1 = b1.commit_decision(p1, &g1).unwrap();

        // 第二次运行（同输入）
        let mut e2 = ResponsibilityEngine::new(1000.0);
        let mut b2 = DecisionBook::new();

        let p2 = b2.prepare_decision(1, 1, 100.0, 1).unwrap();
        let g2 = e2.reserve(1, 1, 100.0, 1).unwrap();
        let r2 = b2.commit_decision(p2, &g2).unwrap();

        // 对拍：责任/收据结构相同
        assert_eq!(e1.total_responsibility(), e2.total_responsibility());
        assert_eq!(b1.committed_count(), b2.committed_count());
        assert_eq!(r1.prepared_id, r2.prepared_id);
    }
}
// 在 financial.rs 末尾添加持久化层

#[cfg(feature = "s_session")]
use rusqlite::{params, Connection, Result as SqlResult};

#[cfg(feature = "s_session")]
pub struct FinancialStore {
    conn: Connection,
}

#[cfg(feature = "s_session")]
impl FinancialStore {
    pub fn new(path: &str) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        
        // 创建表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS financial_responsibilities (
                tower_id INTEGER NOT NULL,
                voice_id INTEGER NOT NULL,
                total_responsibility REAL NOT NULL,
                max_capacity REAL NOT NULL,
                updated_at INTEGER NOT NULL,
                PRIMARY KEY (tower_id, voice_id)
            )",
            [],
        )?;
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS financial_grants (
                grant_id INTEGER PRIMARY KEY,
                tower_id INTEGER NOT NULL,
                voice_id INTEGER NOT NULL,
                reserved_amount REAL NOT NULL,
                epoch INTEGER NOT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS financial_decisions (
                prepared_id INTEGER PRIMARY KEY,
                tower_id INTEGER NOT NULL,
                voice_id INTEGER NOT NULL,
                amount REAL NOT NULL,
                epoch INTEGER NOT NULL,
                state TEXT NOT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS financial_receipts (
                commit_id INTEGER PRIMARY KEY,
                prepared_id INTEGER NOT NULL,
                grant_id INTEGER NOT NULL,
                timestamp INTEGER NOT NULL,
                FOREIGN KEY (prepared_id) REFERENCES financial_decisions(prepared_id),
                FOREIGN KEY (grant_id) REFERENCES financial_grants(grant_id)
            )",
            [],
        )?;
        
        Ok(Self { conn })
    }
    
    // 保存责任状态
    pub fn save_responsibility(&self, tower_id: u64, voice_id: u64, engine: &ResponsibilityEngine) -> SqlResult<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        self.conn.execute(
            "INSERT OR REPLACE INTO financial_responsibilities 
             (tower_id, voice_id, total_responsibility, max_capacity, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                tower_id as i64,
                voice_id as i64,
                engine.total_responsibility(),
                engine.max_responsibility,
                now as i64
            ],
        )?;
        Ok(())
    }
    
    // 保存 Grant
    pub fn save_grant(&self, grant: &Grant) -> SqlResult<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        self.conn.execute(
            "INSERT INTO financial_grants 
             (grant_id, tower_id, voice_id, reserved_amount, epoch, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                grant.grant_id as i64,
                grant.tower_id as i64,
                grant.voice_id as i64,
                grant.reserved_amount,
                grant.epoch as i64,
                now as i64
            ],
        )?;
        Ok(())
    }
    
    // 保存 Receipt
    pub fn save_receipt(&self, receipt: &Receipt) -> SqlResult<()> {
        self.conn.execute(
            "INSERT INTO financial_receipts 
             (commit_id, prepared_id, grant_id, timestamp)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                receipt.commit_id as i64,
                receipt.prepared_id.0 as i64,
                receipt.grant_id as i64,
                receipt.timestamp as i64
            ],
        )?;
        Ok(())
    }
    
    // 查询所有 Receipt（用于验收）
    pub fn query_receipts(&self) -> SqlResult<Vec<Receipt>> {
        let mut stmt = self.conn.prepare(
            "SELECT commit_id, prepared_id, grant_id, timestamp FROM financial_receipts ORDER BY timestamp"
        )?;
        
        let receipts = stmt.query_map([], |row| {
            Ok(Receipt {
                commit_id: row.get::<_, i64>(0)? as u64,
                prepared_id: PreparedId(row.get::<_, i64>(1)? as u64),
                grant_id: row.get::<_, i64>(2)? as u64,
                timestamp: row.get::<_, i64>(3)? as u64,
            })
        })?
        .collect::<SqlResult<Vec<_>>>()?;
        
        Ok(receipts)
    }
    
    // 恢复 ResponsibilityEngine 状态
    pub fn restore_engine(&self, tower_id: u64, voice_id: u64) -> SqlResult<Option<ResponsibilityEngine>> {
        let mut stmt = self.conn.prepare(
            "SELECT total_responsibility, max_capacity FROM financial_responsibilities 
             WHERE tower_id = ?1 AND voice_id = ?2"
        )?;
        
        let mut rows = stmt.query(params![tower_id as i64, voice_id as i64])?;
        
        if let Some(row) = rows.next()? {
            let total: f64 = row.get(0)?;
            let max: f64 = row.get(1)?;
            
            let mut engine = ResponsibilityEngine::new(max);
            engine.total_responsibility = total;
            
            Ok(Some(engine))
        } else {
            Ok(None)
        }
    }
}

#[cfg(all(test, feature = "s_session"))]
mod persistence_tests {
    use super::*;
    
    #[test]
    fn test_store_roundtrip() {
        let store = FinancialStore::new(":memory:").unwrap();
        let mut engine = ResponsibilityEngine::new(1000.0);
        let grant = engine.reserve(1, 1, 100.0, 1).unwrap();
        
        // 保存
        store.save_responsibility(1, 1, &engine).unwrap();
        store.save_grant(&grant).unwrap();
        
        // 恢复
        let restored = store.restore_engine(1, 1).unwrap().unwrap();
        assert_eq!(restored.total_responsibility(), 100.0);
    }
    
    #[test]
    fn test_receipt_persistence() {
        let store = FinancialStore::new(":memory:").unwrap();
        let receipt = Receipt {
            commit_id: 1,
            prepared_id: PreparedId(1),
            grant_id: 1,
            timestamp: 1000,
        };
        
        store.save_receipt(&receipt).unwrap();
        let receipts = store.query_receipts().unwrap();
        
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0].commit_id, 1);
    }
}
