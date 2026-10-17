# #1304 Phase 0 实施 spec（3 个子票）

> 日期：2026-10-17  
> 来源：`.chanlun/review-results/issue1304-replay-system-frontier-20261017.md` Phase 0  
> 性质：不出图的 spec（实施票在 #1304 内）

---

## 目标

让 `rust/src/trading/replay.rs` (D4 回放引擎) 满足 `delivery-discipline.md` 关票门全子句，补齐生产级验收缺口。

**不依赖 TB-02**：Phase 0 全部任务只涉及 `replay.rs` 与 p123，零依赖 TB-02 的六大 Destination。

---

## 子票 A：回放账本导出

### A.1 目标

为 `ReplayEngine` 添加账本导出接口，补齐关票门子句 ②（验证）的账本可查要求。

### A.2 交付物

1. **接口设计**：
   ```rust
   impl ReplayEngine {
       /// 导出回放账本（类 YieldBook 风格）
       pub fn export_book(&self) -> ReplayBook {
           ReplayBook {
               total_bars: self.processed_bars.len(),
               snapshots: self.state_snapshots.len(),
               layers: self.classification_engine.list_layers()
                   .iter()
                   .map(|(id, name)| (*id, name.clone()))
                   .collect(),
           }
       }
   }
   
   #[derive(Debug, Clone)]
   pub struct ReplayBook {
       pub total_bars: usize,
       pub snapshots: usize,
       pub layers: Vec<(LayerId, String)>,
   }
   ```

2. **单测**：
   ```rust
   #[test]
   fn test_export_book() {
       let engine = Box::new(MinimalEngine);
       let mut replay = ReplayEngine::new(engine);
       
       // replay 3 bars
       for i in 0..3 {
           replay.replay_bar(BarInput {
               timestamp_ms: 1000 + i * 1000,
               open: 100.0,
               high: 101.0,
               low: 99.0,
               close: 100.5,
               volume: 1000.0,
           });
       }
       
       let book = replay.export_book();
       assert_eq!(book.total_bars, 3);
       assert_eq!(book.snapshots, 3);
       assert_eq!(book.layers.len(), 3); // MinimalEngine has 3 layers
   }
   ```

3. **文档**：在 `replay.rs` 模块头添加账本导出说明。

### A.3 验收标准

- ✅ `cargo build --release --lib` 通过
- ✅ `cargo test --release --lib --test replay` 通过
- ✅ `export_book()` 单测覆盖 3 bars 回放场景

### A.4 估算

- 工期：0.5 天
- 档位：LOW（纯接口封装）
- 工蜂：sandcastle AFK

---

## 子票 B：双跑对拍机制

### B.1 目标

实现 `ReplayComparator`，对拍 `ReplayEngine` (D4) vs `p123_fast_replay` (生产级基准)。

### B.2 交付物

1. **对拍器实现**：
   ```rust
   // rust/src/trading/replay_comparator.rs
   
   use crate::trading::replay::{ReplayEngine, StateSnapshot};
   use std::fs::File;
   use std::io::{BufRead, BufReader};
   
   pub struct ReplayComparator {
       d4_snapshots: Vec<StateSnapshot>,
       p123_dump: Vec<P123DumpLine>,
   }
   
   impl ReplayComparator {
       /// 从 D4 快照与 p123 dump 构造
       pub fn new(
           d4_snapshots: Vec<StateSnapshot>,
           p123_dump_path: &str,
       ) -> Result<Self, String> {
           let p123_dump = Self::parse_p123_dump(p123_dump_path)?;
           Ok(Self { d4_snapshots, p123_dump })
       }
       
       /// 对拍（白名单：views= 字段）
       pub fn compare(&self) -> CompareResult {
           let mut diffs = Vec::new();
           
           for (idx, (d4, p123)) in self.d4_snapshots.iter()
               .zip(self.p123_dump.iter())
               .enumerate()
           {
               // 比对 timestamp_ms
               if d4.timestamp_ms != p123.timestamp_ms {
                   diffs.push(Diff {
                       bar_index: idx,
                       field: "timestamp_ms".to_string(),
                       d4_value: d4.timestamp_ms.to_string(),
                       p123_value: p123.timestamp_ms.to_string(),
                   });
               }
               
               // 比对 output（跳过 views= 字段）
               // ... (详细实现)
           }
           
           CompareResult {
               total_bars: self.d4_snapshots.len(),
               diffs,
           }
       }
       
       fn parse_p123_dump(path: &str) -> Result<Vec<P123DumpLine>, String> {
           // 解析 p123 dump（CERT/DIV/TERM 行）
           // ... (实现略)
       }
   }
   
   #[derive(Debug)]
   pub struct P123DumpLine {
       pub timestamp_ms: i64,
       // ... (其他字段)
   }
   
   #[derive(Debug)]
   pub struct CompareResult {
       pub total_bars: usize,
       pub diffs: Vec<Diff>,
   }
   
   #[derive(Debug)]
   pub struct Diff {
       pub bar_index: usize,
       pub field: String,
       pub d4_value: String,
       pub p123_value: String,
   }
   ```

2. **对拍脚本**：
   ```bash
   # scripts/replay_compare.sh
   
   #!/usr/bin/env bash
   set -euo pipefail
   
   DATA=${1:-analysis/data_cache/btc_1m_full.json}
   MAX_BARS=${2:-100}
   
   # 1. 跑 p123 dump
   P123_DUMP=$(mktemp)
   P116_DUMP=$P123_DUMP P116_MAX_BARS=$MAX_BARS \
       cargo run --release --bin p123_fast_replay -- "$DATA"
   
   # 2. 跑 D4 replay + 对拍
   cargo run --release --bin replay_compare -- "$DATA" "$P123_DUMP" "$MAX_BARS"
   
   # 3. 清理
   rm -f "$P123_DUMP"
   ```

3. **对拍 bin**：
   ```rust
   // rust/src/bin/replay_compare.rs
   
   use newchan_rust::trading::replay::ReplayEngine;
   use newchan_rust::trading::replay_comparator::ReplayComparator;
   // ... (实现略)
   
   fn main() -> Result<(), String> {
       let args: Vec<String> = std::env::args().collect();
       let data_path = &args[1];
       let p123_dump_path = &args[2];
       let max_bars: usize = args[3].parse()
           .map_err(|e| format!("MAX_BARS 非法: {}", e))?;
       
       // 1. D4 replay
       let mut replay = create_replay_engine();
       let bars = load_bars(data_path, max_bars)?;
       replay.replay_bars(bars);
       
       // 2. 对拍
       let comparator = ReplayComparator::new(
           replay.all_snapshots().to_vec(),
           p123_dump_path,
       )?;
       let result = comparator.compare();
       
       // 3. 报告
       if result.diffs.is_empty() {
           println!("✅ 对拍通过：{} bars, diff=0", result.total_bars);
           Ok(())
       } else {
           eprintln!("❌ 对拍失败：{} bars, {} diffs", result.total_bars, result.diffs.len());
           for diff in result.diffs.iter().take(10) {
               eprintln!("  bar={} field={} d4={} p123={}", 
                   diff.bar_index, diff.field, diff.d4_value, diff.p123_value);
           }
           Err(format!("{} diffs", result.diffs.len()))
       }
   }
   ```

### B.3 验收标准

- ✅ `scripts/replay_compare.sh analysis/data_cache/btc_1m_full.json 100` 返回 `diff=0`
- ✅ 白名单：`views=` 字段（同 p123 验收面）
- ✅ 失败时打印前 10 个 diff

### B.4 估算

- 工期：1.5 天
- 档位：MED（需解析 p123 dump 格式）
- 工蜂：sandcastle AFK

---

## 子票 C：回放 ADR 落盘

### C.1 目标

起草 `docs/adr/00XX-replay-engine-architecture.md`，补齐关票门子句 ⑤（架构）。

### C.2 交付物

**ADR 内容**（建议结构）：

```markdown
# ADR 00XX: 回放引擎架构

## 状态

Accepted（2026-10-17）

## 背景

#1304 要求"完整历史回放、账目可查、双跑确定"（Destination ④）。现有两档实现：
- D4 基础回放引擎（`replay.rs`，157 行）
- p123 快速回放（`p123_fast_replay.rs`，4,925 行，M2/M3 验收）

需要明确：
1. 两档形态的选择逻辑
2. 接口设计原则
3. 验收标准

## 决策

### 1. 双档并存

- **D4 作为库接口**：`replay.rs` 提供可组合的回放引擎，供其他模块导入
- **p123 作为验收基准**：生产级 bin，M2/M3 双跑背书，是 D4 的对拍参考

**不合并两者**：p123 是 theta_v0 专用、高度优化的验收工具；D4 是通用接口，追求可组合性而非极致性能。

### 2. D4 接口设计原则

- **最小接口**：`replay_bar()` + `replay_bars()` + `all_snapshots()`
- **账本导出**：`export_book() -> ReplayBook`（类 YieldBook 风格）
- **无侧信道**：诊断能力由 p123 提供，D4 保持简洁

### 3. 验收标准

- **D4 vs p123 对拍**：100 bars 截段，diff=0（白名单 `views=`）
- **关票门全子句**：
  - ① 改动：模块头文件清单
  - ② 验证：对拍 + 单测
  - ③ 签字：本 ADR
  - ④ 追溯：模块头引用
  - ⑤ 架构：本 ADR
  - ⑥ 入仓：常驻

## 后果

### 正面

- D4 保持轻量（~200 行），易于理解与扩展
- p123 继续作为生产级验收基准，无需重构
- 双跑对拍机制可复用于未来其他引擎

### 负面

- 双档维护成本（需保持接口兼容）
- 对拍脚本增加 CI 时长（~10 秒/100 bars）

### 风险

- D4 与 p123 账本格式不兼容 → Phase 1 统一接口设计可缓解

## 关联

- #1304：回测重定位（本 ADR 的直接来源）
- Destination ④：完整历史回放、账目可查、双跑确定
```

### C.3 验收标准

- ✅ ADR 文件落盘到 `docs/adr/`
- ✅ 编号按现有 ADR 序列递增
- ✅ 内容覆盖：背景、决策、后果、关联
- ✅ `replay.rs` 模块头引用本 ADR

### C.4 估算

- 工期：0.5 天
- 档位：LOW（只读文书）
- 工蜂：RLM 子代理

---

## 总估算

- **总工期**：2.5 天（可并行派发 3 个工蜂）
- **依赖关系**：3 个子票无依赖，可并行
- **档位分布**：2 LOW + 1 MED

---

## 验收标准（Phase 0 整体）

1. ✅ 3 个子票全部关闭
2. ✅ D4 满足关票门全子句（①②③④⑤⑥）
3. ✅ `scripts/replay_compare.sh` 100 bars 对拍通过
4. ✅ #1304 票面更新：添加 Phase 0 完成记录

---

## 下游（Phase 1）

Phase 0 完成后，Phase 1 可启动：
- 账本接口设计（grilling，1 轮）
- 侧信道插件化（task HIGH，2 天）

**Phase 1 部分依赖 TB-02**：需要稳定基线验收插件化（见 frontier scan §5.2）。
