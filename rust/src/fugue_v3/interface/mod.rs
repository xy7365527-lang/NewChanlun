//! fugue_v3 核心接口层——D2逐bar贯通的生产职责接口。
//!
//! 本模块定义 Classification 和 Chong 的生产接口，作为 theta_v0 → fugue_v3 迁移的
//! 接口层。当前为骨架，完整实现在后续迭代中逐步迁移。
//!
//! ## 设计原则
//!
//! 1. **接口优先**：先定义清晰的 trait 边界，再填充实现体
//! 2. **向后兼容**：保持与 theta_v0 相同的输入输出类型签名
//! 3. **渐进迁移**：允许 adapter wrapper 桥接旧实现，避免大爆炸式重写
//!
//! ## 模块组织
//!
//! - `classification` - Classification 生产接口（逐bar分类、多级递归）
//! - `chong` - Chong（重）持久操作单位接口（专属筹码、成本状态）

pub mod classification;
pub mod chong;

pub use classification::{ClassificationEngine, ClassificationResult};
pub use chong::{ChongEngine, ChongKey, ChongState};

// Theta v0 适配器（过渡层，#1323 D1）
pub mod theta_adapter;
pub use theta_adapter::ThetaClassificationAdapter;
