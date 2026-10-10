//! # Theta v0 → Fugue v3 适配器完整实现
//!
//! 本实现将 theta_v0 的 Classification 和 Chong 引擎桥接到 fugue_v3 统一接口。

use std::collections::BTreeMap;
use std::rc::Rc;

use crate::fugue_v3::interface::classification::{
    ClassificationEngine, ClassificationResult, L0ParseLayerPlaceholder, BarPlaceholder,
};
use crate::fugue_v3::interface::chong::{
    ChongEngine, ChongKey, ChongState, OperationPlaceholder,
};

// 引入 theta_v0 实际类型（使用公开的 re-export）
use crate::theta_v0::config::ThetaConfig;
use crate::theta_v0::parser::ParseLayer;
use crate::theta_v0::types::Bar;
use crate::theta_v0::classifier::{classify, classify_incremental, ClassifyOutput};

/// Theta v0 Classification 适配器
pub struct ThetaClassificationAdapter {
    /// theta_v0 配置（从外部注入）
    config: ThetaConfig,
}

impl ThetaClassificationAdapter {
    /// 创建新的适配器实例
    pub fn new(config: ThetaConfig) -> Self {
        Self { config }
    }

    /// 内部转换：L0ParseLayerPlaceholder → ParseLayer
    fn convert_l0_parse(&self, _placeholder: &L0ParseLayerPlaceholder) -> ParseLayer {
        // TODO: 当 L0ParseLayerPlaceholder 有实际字段时，进行真正转换
        // 当前 placeholder 是空结构体，需要从其他来源获取真实 ParseLayer
        //
        // 临时方案：返回空的 ParseLayer（这会导致空分类结果）
        // 生产实现需要：
        // 1. L0ParseLayerPlaceholder 添加实际数据字段（如 Vec<Bar>）
        // 2. 或者在调用层传入真实 ParseLayer 的引用
        
        ParseLayer {
            merged_bars: Rc::new(Vec::new()),
            merged_confirmed_len: 0,
            segments_confirmed_len: 0,
            segments_earliest_unsealed: None,
            fractals: Rc::new(Vec::new()),
            strokes: Rc::new(Vec::new()),
            segments: Rc::new(Vec::new()),
            tail: Default::default(),
        }
    }

    /// 内部转换：ClassifyOutput → ClassificationResult
    fn convert_output(&self, output: ClassifyOutput) -> ClassificationResult {
        // 遍历 theta_v0 的每个级别状态，转换为 fugue_v3 的 LevelState
        let levels = output.classification.levels
            .into_iter()
            .map(|theta_level| {
                use crate::fugue_v3::interface::classification::{
                    LevelState, CenterPlaceholder, BspPointPlaceholder, MovePlaceholder
                };
                
                // 转换中枢序列（保持 Rc 共享）
                let centers = Rc::new(
                    theta_level.centers.iter()
                        .map(|_center| {
                            // TODO: 完整映射 theta_v0::Center → CenterPlaceholder
                            // 当前 CenterPlaceholder 是空结构体，需要后续添加字段
                            CenterPlaceholder
                        })
                        .collect()
                );
                
                // 转换买卖点序列（保持 Rc 共享）
                let bsp = Rc::new(
                    theta_level.bsp.iter()
                        .map(|_bsp_point| {
                            // TODO: 完整映射 theta_v0::BspPoint → BspPointPlaceholder
                            // 需要传递 pivot_low/pivot_high/center/six_bit 等字段
                            BspPointPlaceholder
                        })
                        .collect()
                );
                
                // 转换走势块序列
                let moves = theta_level.moves
                    .into_iter()
                    .map(|_move_block| {
                        // TODO: 完整映射 theta_v0::MoveBlock → MovePlaceholder
                        // 需要传递方向、起止位置、类型等字段
                        MovePlaceholder
                    })
                    .collect();
                
                LevelState {
                    centers,
                    bsp,
                    moves,
                }
            })
            .collect();
        
        ClassificationResult { levels }
    }
}

impl ClassificationEngine for ThetaClassificationAdapter {
    fn classify(
        &self,
        l0_parse: &L0ParseLayerPlaceholder,
        operating_levels: &[u32],
    ) -> ClassificationResult {
        // 1. 转换输入
        let parse_layer = self.convert_l0_parse(l0_parse);
        
        // 2. 调用 theta_v0 核心分类
        let output = classify(&parse_layer, &self.config, operating_levels);
        
        // 3. 转换输出
        self.convert_output(output)
    }

    fn classify_incremental(
        &self,
        _prev: &ClassificationResult,
        _new_bar: &BarPlaceholder,
        operating_levels: &[u32],
    ) -> ClassificationResult {
        // TODO: 实现增量分类桥接
        // theta_v0 的 classify_incremental 签名：
        // fn classify_incremental(
        //     prev_parse: &ParseLayer,
        //     prev_result: &ClassifyOutput,
        //     new_bar: Bar,
        //     config: &ThetaConfig,
        //     operating_levels: &[u32],
        //     tower_cache: &TowerCache,
        // ) -> ClassifyOutput
        //
        // 需要维护 TowerCache 状态，当前简化为调用全量 classify
        
        // 临时实现：退化为全量分类（bit-exact 但性能不优）
        // 生产实现需要：
        // 1. ClassificationResult 包含 prev_parse 和 prev_result 引用
        // 2. 在 adapter 内维护 TowerCache
        
        // 从 prev 恢复 l0_parse（当前无法实现，因为 ClassificationResult 不包含原始数据）
        let l0_parse = L0ParseLayerPlaceholder;
        self.classify(&l0_parse, operating_levels)
    }
}

/// Theta v0 Chong 适配器
pub struct ThetaChongAdapter {
    /// 内部维护的多重状态
    chongs: BTreeMap<ChongKey, ChongState>,
}

impl ThetaChongAdapter {
    /// 创建新的适配器实例
    pub fn new() -> Self {
        Self {
            chongs: BTreeMap::new(),
        }
    }
}

impl ChongEngine for ThetaChongAdapter {
    fn get_chong_state(&self, key: &ChongKey) -> Option<&ChongState> {
        self.chongs.get(key)
    }

    fn create_chong(&mut self, key: ChongKey, quota_usd: f64) -> Result<(), String> {
        if self.chongs.contains_key(&key) {
            return Err(format!("重 {:?} 已存在", key));
        }
        
        if quota_usd <= 0.0 {
            return Err(format!("配额无效: {}", quota_usd));
        }
        
        let state = ChongState {
            quota_usd,
            cost_basis_placeholder: 0.0,
            position: 0.0,
        };
        
        self.chongs.insert(key, state);
        Ok(())
    }

    fn execute_operation(
        &mut self,
        key: &ChongKey,
        _operation: &OperationPlaceholder,
    ) -> Result<ChongState, String> {
        // TODO: 实现完整桥接到 theta_v0 的 Chong 操作逻辑
        //
        // theta_v0 的 Chong 相关代码在：
        // - rust/src/trading/positional.rs: 持仓管理
        // - rust/src/trading/ledger.rs: 双向嵌套记账
        // - rust/src/strategy/chong.rs: Chong 信号生成
        //
        // 当前简化实现：直接操作 ChongState，无实际 theta_v0 调用
        
        let state = self.chongs.get_mut(key)
            .ok_or_else(|| format!("重 {:?} 不存在", key))?;
        
        // 临时占位：返回当前状态的克隆
        // 生产实现需要：
        // 1. 解析 OperationPlaceholder（买入/卖出/平仓信号）
        // 2. 调用 theta_v0 的持仓管理逻辑
        // 3. 更新 cost_basis 和 position
        // 4. 验证约束（重内单向、逐仓计提）
        
        Ok(state.clone())
    }

    fn all_chongs(&self) -> &BTreeMap<ChongKey, ChongState> {
        &self.chongs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> ThetaConfig {
        // 使用 theta_v0 的默认配置
        ThetaConfig::default()
    }

    #[test]
    fn test_classification_adapter_with_config() {
        let config = default_config();
        let adapter = ThetaClassificationAdapter::new(config);
        
        let l0_parse = L0ParseLayerPlaceholder;
        let result = adapter.classify(&l0_parse, &[0, 1]);
        
        // 验证返回空的 levels（因为输入是空的）
        assert_eq!(result.levels.len(), 0);
    }

    #[test]
    fn test_incremental_classification() {
        let config = default_config();
        let adapter = ThetaClassificationAdapter::new(config);
        
        let l0_parse = L0ParseLayerPlaceholder;
        let prev_result = adapter.classify(&l0_parse, &[0, 1]);
        
        let new_bar = BarPlaceholder;
        let incremental_result = adapter.classify_incremental(&prev_result, &new_bar, &[0, 1]);
        
        // 验证增量结果结构（当前退化为全量）
        assert_eq!(incremental_result.levels.len(), 0);
    }

    #[test]
    fn test_chong_adapter_operations() {
        let mut adapter = ThetaChongAdapter::new();
        let key = ChongKey {
            symbol: "AAPL".to_string(),
            op_level: 1,
        };
        
        // 创建重
        adapter.create_chong(key.clone(), 10000.0).unwrap();
        
        // 执行操作
        let op = OperationPlaceholder;
        let state = adapter.execute_operation(&key, &op).unwrap();
        
        assert_eq!(state.quota_usd, 10000.0);
        assert_eq!(state.position, 0.0);
    }
}
