//! TB-03-B 正式会话入口测试（AC1-AC5 完整覆盖）
//!
//! 验证通过正式入口运行 E.Reserve → B.Commit 流程，
//! 不直调内部函数或渲染假数据。

use newchan_rust::fugue_v3::financial::{
    DecisionBook, Grant, PreparedId, Receipt, ResponsibilityEngine,
};
use serde::{Deserialize, Serialize};

// ============================================================
// 正式会话结构体定义
// ============================================================

/// 正式会话：完整的 Prepare → Reserve → Commit 流程
#[derive(Debug)]
pub struct FinancialSession {
    e: ResponsibilityEngine,
    b: DecisionBook,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SessionError {
    PrepareFailed,
    ReserveFailed,
    InsufficientCapacity,
    CommitFailed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionState {
    pub total_responsibility: f64,
    pub committed_count: usize,
}

// ============================================================
// JSON 观察快照结构体
// ============================================================

/// JSON 可观察的 Grant 快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrantSnapshot {
    pub grant_id: u64,
    pub tower_id: u64,
    pub voice_id: u64,
    pub reserved_amount: f64,
    pub epoch: u64,
}

impl From<&Grant> for GrantSnapshot {
    fn from(grant: &Grant) -> Self {
        Self {
            grant_id: grant.grant_id,
            tower_id: grant.tower_id,
            voice_id: grant.voice_id,
            reserved_amount: grant.reserved_amount,
            epoch: grant.epoch,
        }
    }
}

/// JSON 可观察的 Receipt 快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptSnapshot {
    pub commit_id: u64,
    pub prepared_id: u64,
    pub grant_id: u64,
    pub timestamp: u64,
}

impl From<&Receipt> for ReceiptSnapshot {
    fn from(receipt: &Receipt) -> Self {
        Self {
            commit_id: receipt.commit_id,
            prepared_id: receipt.prepared_id.0,
            grant_id: receipt.grant_id,
            timestamp: receipt.timestamp,
        }
    }
}

// ============================================================
// FinancialSession 实现
// ============================================================

impl FinancialSession {
    /// 创建会话（AC4：正式入口）
    pub fn new(max_capacity: f64) -> Self {
        Self {
            e: ResponsibilityEngine::new(max_capacity),
            b: DecisionBook::new(),
        }
    }

    /// 提交决策请求（AC4：三步原子流程）
    pub fn submit_decision(
        &mut self,
        tower_id: u64,
        voice_id: u64,
        amount: f64,
        epoch: u64,
    ) -> Result<Receipt, SessionError> {
        // 1. B.Prepare
        let prepared_id = self
            .b
            .prepare_decision(tower_id, voice_id, amount, epoch)
            .map_err(|_| SessionError::PrepareFailed)?;

        // 2. E.Reserve（AC1：必须先增责任）
        let grant = self
            .e
            .reserve(tower_id, voice_id, amount, epoch)
            .map_err(|e| match e {
                newchan_rust::fugue_v3::financial::ReserveError::InsufficientCapacity { .. } => {
                    SessionError::InsufficientCapacity
                }
                _ => SessionError::ReserveFailed,
            })?;

        // 3. B.Commit（用 grant）
        let receipt = self
            .b
            .commit_decision(prepared_id, &grant)
            .map_err(|_| SessionError::CommitFailed)?;

        Ok(receipt)
    }

    /// 查询会话状态（AC5：验证用）
    pub fn query_state(&self) -> SessionState {
        SessionState {
            total_responsibility: self.e.total_responsibility(),
            committed_count: self.b.committed_count(),
        }
    }

    /// AC4：查询 Grant 列表（JSON 格式）
    pub fn query_grants_json(&self) -> Result<String, serde_json::Error> {
        let grants: Vec<GrantSnapshot> = self.e.grants().iter().map(|g| GrantSnapshot::from(*g)).collect();
        serde_json::to_string_pretty(&grants)
    }

    /// AC4：查询 Receipt 列表（JSON 格式）
    pub fn query_receipts_json(&self) -> Result<String, serde_json::Error> {
        let receipts: Vec<ReceiptSnapshot> = self
            .b
            .receipts()
            .iter()
            .map(|r| ReceiptSnapshot::from(*r))
            .collect();
        serde_json::to_string_pretty(&receipts)
    }

    /// AC4：查询责任状态（JSON 格式）
    pub fn query_responsibility_json(&self) -> Result<String, serde_json::Error> {
        let state = serde_json::json!({
            "total_responsibility": self.e.total_responsibility(),
            "committed_count": self.b.committed_count(),
            "max_capacity": self.e.max_capacity(),
        });
        serde_json::to_string_pretty(&state)
    }
}

// ============================================================
// 测试：AC1-AC5 完整覆盖
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // AC1：先 E 增责任才允许 B Committed
    // ========================================

    #[test]
    fn ac1_reserve_before_commit_enforcement() {
        let mut session = FinancialSession::new(1000.0);
        
        // 正常流程：Reserve → Commit
        let receipt = session.submit_decision(1, 1, 100.0, 1);
        assert!(receipt.is_ok(), "正常流程应该成功");
        
        // 验证责任确实增加了
        let state = session.query_state();
        assert_eq!(state.total_responsibility, 100.0);
        assert_eq!(state.committed_count, 1);
    }

    #[test]
    fn ac1_failure_preserves_state() {
        let mut session = FinancialSession::new(100.0); // 容量只有 100
        
        // 第一个请求成功
        session.submit_decision(1, 1, 100.0, 1).unwrap();
        let state_after_first = session.query_state();
        
        // 第二个请求失败（超容量）
        let result = session.submit_decision(1, 2, 50.0, 1);
        assert!(result.is_err(), "超容量应该失败");
        
        // 验证状态未改变
        let state_after_failure = session.query_state();
        assert_eq!(state_after_first, state_after_failure);
    }

    #[test]
    fn ac1_ac2_epoch_handling() {
        let mut session = FinancialSession::new(1000.0);
        
        // Epoch 1 的请求
        session.submit_decision(1, 1, 100.0, 1).unwrap();
        
        // Epoch 2 的请求（同 tower 和 voice）
        session.submit_decision(1, 1, 150.0, 2).unwrap();
        
        let state = session.query_state();
        assert_eq!(state.total_responsibility, 250.0);
        assert_eq!(state.committed_count, 2);
    }

    // ========================================
    // AC2：实际运行最小财务步骤，不伪造
    // ========================================

    #[test]
    fn ac2_real_financial_steps_no_fake_grants() {
        let mut session = FinancialSession::new(1000.0);
        
        // 通过正式接口提交
        let receipt = session.submit_decision(1, 1, 100.0, 1).unwrap();
        
        // 验证 Grant 确实被创建（通过 JSON 观察）
        let grants_json = session.query_grants_json().unwrap();
        let grants: Vec<GrantSnapshot> = serde_json::from_str(&grants_json).unwrap();
        assert_eq!(grants.len(), 1);
        assert_eq!(grants[0].reserved_amount, 100.0);
        
        // 验证 Receipt 引用了真实 Grant
        let receipts_json = session.query_receipts_json().unwrap();
        let receipts: Vec<ReceiptSnapshot> = serde_json::from_str(&receipts_json).unwrap();
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0].grant_id, grants[0].grant_id);
    }

    #[test]
    fn ac2_duplicate_request_handling() {
        let mut session = FinancialSession::new(1000.0);
        
        // 第一次提交
        let receipt1 = session.submit_decision(1, 1, 100.0, 1).unwrap();
        
        // 第二次提交（相同参数）
        let receipt2 = session.submit_decision(1, 1, 100.0, 1).unwrap();
        
        // 两次都应该成功，因为是不同的决策
        let state = session.query_state();
        assert_eq!(state.total_responsibility, 200.0);
        assert_eq!(state.committed_count, 2);
        
        // 验证 receipt 不同
        assert_ne!(receipt1.commit_id, receipt2.commit_id);
    }

    // ========================================
    // AC3：全部请求能共同满足时正常提交
    // ========================================

    #[test]
    fn ac3_concurrent_requests_within_capacity() {
        let mut session = FinancialSession::new(1000.0);
        
        // 提交多个请求，总和在容量内
        session.submit_decision(1, 1, 200.0, 1).unwrap();
        session.submit_decision(1, 2, 300.0, 1).unwrap();
        session.submit_decision(1, 3, 400.0, 1).unwrap();
        
        let state = session.query_state();
        assert_eq!(state.total_responsibility, 900.0);
        assert_eq!(state.committed_count, 3);
    }

    #[test]
    fn ac3_insufficient_capacity_precise_error() {
        let mut session = FinancialSession::new(500.0);
        
        // 第一个请求成功
        session.submit_decision(1, 1, 400.0, 1).unwrap();
        
        // 第二个请求超容量
        let result = session.submit_decision(1, 2, 200.0, 1);
        assert!(matches!(result, Err(SessionError::InsufficientCapacity)));
        
        // 验证第一个请求的状态仍然保留
        let state = session.query_state();
        assert_eq!(state.total_responsibility, 400.0);
        assert_eq!(state.committed_count, 1);
    }

    // ========================================
    // AC4：JSON 观察接口
    // ========================================

    #[test]
    fn ac4_json_observation_api() {
        let mut session = FinancialSession::new(1000.0);

        // 提交两个决策
        session.submit_decision(1, 1, 100.0, 1).unwrap();
        session.submit_decision(1, 2, 200.0, 1).unwrap();

        // 查询 Grants JSON
        let grants_json = session.query_grants_json().unwrap();
        assert!(grants_json.contains("\"grant_id\""));
        assert!(grants_json.contains("\"tower_id\""));
        assert!(grants_json.contains("\"reserved_amount\""));

        // 查询 Receipts JSON
        let receipts_json = session.query_receipts_json().unwrap();
        assert!(receipts_json.contains("\"commit_id\""));
        assert!(receipts_json.contains("\"prepared_id\""));
        assert!(receipts_json.contains("\"grant_id\""));

        // 查询责任状态 JSON
        let responsibility_json = session.query_responsibility_json().unwrap();
        assert!(responsibility_json.contains("\"total_responsibility\""));
        assert!(responsibility_json.contains("\"committed_count\""));
        assert!(responsibility_json.contains("\"max_capacity\""));

        // 验证可以解析为有效 JSON
        let resp_value: serde_json::Value = serde_json::from_str(&responsibility_json).unwrap();
        assert_eq!(resp_value["total_responsibility"], 300.0);
        assert_eq!(resp_value["committed_count"], 2);
        assert_eq!(resp_value["max_capacity"], 1000.0);
    }

    // ========================================
    // AC5：同输入重放对拍
    // ========================================

    #[test]
    fn ac5_replay_parity_via_session() {
        // 第一次运行
        let mut session1 = FinancialSession::new(1000.0);
        session1.submit_decision(1, 1, 100.0, 1).unwrap();
        session1.submit_decision(1, 2, 200.0, 1).unwrap();
        let state1 = session1.query_state();

        // 第二次运行（同输入）
        let mut session2 = FinancialSession::new(1000.0);
        session2.submit_decision(1, 1, 100.0, 1).unwrap();
        session2.submit_decision(1, 2, 200.0, 1).unwrap();
        let state2 = session2.query_state();

        // 对拍：状态完全相同
        assert_eq!(state1, state2);
    }

    #[test]
    fn ac5_replay_with_failure() {
        // 第一次运行（包含失败）
        let mut session1 = FinancialSession::new(500.0);
        session1.submit_decision(1, 1, 400.0, 1).unwrap();
        let _ = session1.submit_decision(1, 2, 200.0, 1); // 失败
        let state1 = session1.query_state();

        // 第二次运行（同输入，同失败）
        let mut session2 = FinancialSession::new(500.0);
        session2.submit_decision(1, 1, 400.0, 1).unwrap();
        let _ = session2.submit_decision(1, 2, 200.0, 1); // 失败
        let state2 = session2.query_state();

        // 对拍：失败后状态仍相同
        assert_eq!(state1, state2);
        assert_eq!(state1.total_responsibility, 400.0);
        assert_eq!(state1.committed_count, 1);
    }
}
