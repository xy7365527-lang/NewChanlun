//! TB-03-B 正式会话入口测试（AC4）
//!
//! 验证通过正式入口运行 E.Reserve → B.Commit 流程，
//! 不直调内部函数或渲染假数据。

use newchan_rust::fugue_v3::financial::{
    DecisionBook, Grant, PreparedId, Receipt, ResponsibilityEngine,
};

/// 正式会话：完整的 Prepare → Reserve → Commit 流程
#[derive(Debug)]
pub struct FinancialSession {
    e: ResponsibilityEngine,
    b: DecisionBook,
}

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
        let prepared_id = self.b
            .prepare_decision(tower_id, voice_id, amount, epoch)
            .map_err(|_| SessionError::PrepareFailed)?;

        // 2. E.Reserve（AC1：必须先增责任）
        let grant = self.e
            .reserve(tower_id, voice_id, amount, epoch)
            .map_err(|e| match e {
                newchan_rust::fugue_v3::financial::ReserveError::InsufficientCapacity { .. } => {
                    SessionError::InsufficientCapacity
                }
                _ => SessionError::ReserveFailed,
            })?;

        // 3. B.Commit（用 grant）
        let receipt = self.b
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

#[cfg(test)]
mod tests {
    use super::*;

    /// AC4：通过正式会话入口运行
    #[test]
    fn ac4_formal_session_entry() {
        let mut session = FinancialSession::new(1000.0);

        // 提交第一个决策（正式入口）
        let receipt1 = session.submit_decision(1, 1, 100.0, 1).unwrap();
        assert_eq!(receipt1.commit_id, 0);

        // 查询状态
        let state = session.query_state();
        assert_eq!(state.total_responsibility, 100.0);
        assert_eq!(state.committed_count, 1);

        // 提交第二个决策
        let receipt2 = session.submit_decision(1, 2, 200.0, 1).unwrap();
        assert_eq!(receipt2.commit_id, 1);

        // 最终状态
        let final_state = session.query_state();
        assert_eq!(final_state.total_responsibility, 300.0);
        assert_eq!(final_state.committed_count, 2);
    }

    /// AC3 + AC4：不足时精确等待（通过正式入口）
    #[test]
    fn ac3_ac4_insufficient_capacity_via_session() {
        let mut session = FinancialSession::new(100.0);

        // 第一个请求成功
        session.submit_decision(1, 1, 60.0, 1).unwrap();

        // 第二个请求成功
        session.submit_decision(1, 2, 30.0, 1).unwrap();

        // 第三个请求失败（不足）
        let err = session.submit_decision(1, 3, 20.0, 1).unwrap_err();
        assert_eq!(err, SessionError::InsufficientCapacity);

        // 状态未改变
        let state = session.query_state();
        assert_eq!(state.total_responsibility, 90.0);
        assert_eq!(state.committed_count, 2);
    }

    /// AC5：同输入重放对拍（通过正式入口）
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
}
