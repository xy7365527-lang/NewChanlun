# #92 裁定后因果 prefix 重放 — NestCertificate 交易证书产量复测

日期：2026-07-16  
状态：**PASS（实现、库测试与全量重放完成）**

## 1. 裁定口径与实现边界

本次迁移只新增 typed `NestCertificate` provider/装配路径，不修改既有裁定文档与
`chanlun/definitions/`。实现严格采用以下口径：

1. D1/Cand：`dir ∧ Comparable ∧ Extreme`。结构候选不消费 MACD；`Weak` 只在②背驰段确认层独立合取。
2. ②B：生产区间为完整背驰段 C；leave→retest 的 A 口径仅并行诊断。
3. ③：终端确认继续调用真实 `BspBits::confirm_side`，盘背 typed 事件不伪造同级 B1/S1。
4. ⑤：趋势背驰与盘整背驰分别标记 `Trend` / `Consolidation`，再进入同一 typed 装配器。
5. ④：`judge_at` 来自同一对象在逐 prefix 重放中的首次可证 bar；snapshot 只读当前 prefix 的
   MACD/坐标。`created_at_reads=0`，无挂钟、无随机数。
6. D3：父/子首次可证钟只进入 sidecar 计数，不参与 Cand、Weak、闭包含或终端确认真值。

涉及代码：

- `rust/src/theta_v0/classifier/signal.rs`：拆出盘背纯结构 A/C 定位与 Extreme 判定；旧 `judge_pan_div`
  复用同一结构定位后继续执行原 Weak 判据。
- `rust/src/theta_v0/classifier/level_view.rs`：新增 typed provider，输出结构 Cand、独立 Weak、A/B 区间和
  Trend/Consolidation 类型。
- `rust/src/theta_v0/classifier/nest.rs`：新增 A/B typed 证书装配与 D3 sidecar；终端仍由 BspBits 复验。
- `rust/src/theta_v0/classifier/mod.rs`：只读暴露当前 prefix 同源 MACD hist/close_src。
- `rust/src/bin/p92_nest_replay_postruling.rs`：两阶段因果重放、迁移前基线、R7 与 bit-exact 审计。

## 2. 测试与数据

执行命令：

```text
cargo test --manifest-path rust/Cargo.toml --lib
cargo run --release --manifest-path rust/Cargo.toml \
  --bin p92_nest_replay_postruling -- analysis/data_cache/btc_1m_full.json
```

库测试结果：`1637 passed; 0 failed; 127 ignored`。ignored 均为仓库原有显式长测，本次未降低断言强度。

重放输入：`btc_1m_full.json`，共 **4,613,599** 根 1m bar；数据范围
`2017-08-17 04:00:00` 至 `2026-05-31 23:59:00`。未设置 `P92_MAX_BARS`。

重放采用两遍同源增量分类：第一遍固定终态对象集合；第二遍从 bar 0 逐 prefix 追踪这些对象的
首次 Cand/Weak 可证钟。`turn_source > as_of` 的终态对象按 snapshot 无前视定理跳过，50 万根对拍中
除 provider view 次数外所有产量、时钟审计与 bit-exact 字段逐项一致。

## 3. 产量

### 3.1 主漏斗

| 阶段 | 总数 | Trend | Consolidation（盘背） | 相对上一步 |
|---|---:|---:|---:|---:|
| 结构 Cand | 3,807 | 894 | 2,913 | — |
| ② Weak/背驰确认 | 727 | 1 | 726 | 19.096% / Cand |
| BspBits 终端确认 | 19 | 0 | 19 | 2.613% / Weak；0.499% / Cand |

### 3.2 A/B 分段证书

| 区间口径 | 证书数 | Trend-only | Pan-only | Mixed |
|---|---:|---:|---:|---:|
| A：leave→retest（诊断） | 23 | 0 | 23 | 0 |
| B：完整背驰段 C（生产） | 19 | 0 | 19 | 0 |

B 比 A 少 4 张（`-17.391%`）。生产结论以 B 的 **19 张**为准；A 不进入生产声明。

## 4. D3 sidecar

| 可评估父子边 | 违反数 | 违反率 |
|---:|---:|---:|
| 0 | 0 | N/A |

本次 19 张 B 证书均未形成跨级父子边，因此没有 D3 违反率的有效分母。程序输出的数值字段为
`rate=0.000000000`，但报告不把它解释成“有样本的 0%”。当前既不能标注异常高，也不能据此证明
递降经验规律；后续出现跨级证书时再按 sidecar 统计，仍不得回写成主链硬门。

## 5. 与迁移前基线对比

迁移前基线来自同一终态 snapshot 的旧 `cand_delta_tower_cached` +
`assemble_certificates_snapshot` 路径；迁移后来自裁定 typed provider。两侧定义不同，差值用于迁移审计，
不作同口径收益解释。

| 指标 | 迁移前旧路径 | 裁定后 typed 路径 | 差值 |
|---|---:|---:|---:|
| 候选 | 483 | 3,807 | +3,324（+688.199%） |
| 终端确认 | 483 | 19 | -464 |
| 生产证书 | 483 | 19（B） | -464（-96.066%） |

候选扩张符合 D1 去除 Weak 前置税的预期；终端与证书收缩来自新路径继续独立执行 Weak、真实
`BspBits::confirm_side`、typed 分流和 B 区间闭包含，不能把旧路径的 483 直接视作新定义漏失。

## 6. R7 三项验证

| R7 检查 | 结论 | 证据 |
|---|---|---|
| (a) provider 完整性 | **FAIL / 能力边界** | `unresolved_targets=0`，但 exact-three 投影有 215 个 `InvalidSeed`；依 #90 fail-closed，不得静默收复或当作合法产零 |
| (b) 定义忠实性 | **PASS** | D1 结构 Cand、Weak 独立层、B=C、typed 分流、BspBits 终端、D3 sidecar 均由独立字段/测试覆盖 |
| (c) snapshot 无前视 | **PASS** | `future_violations=0`，`created_at_reads=0`；首次钟来自逐 prefix 首见 |

本次 `B_zero=false`（B 段有 19 张证书），故 R7 的零产量签字分支未触发。另因 (a) 未通过，
即使未来某次 B=0，也只能写 provider 能力边界/未决，不能写“正确市场答案：该段无证书”。

## 7. bit-exact 声明

全量终态对拍：

```text
old_path_diff=0
tower_diff=0
moves_centers_bsp_pan_diff=0
lifecycle_cp_ownership_diff=0
classification_total_diff=0
```

因此本次可签字：**不受影响的旧路径回归 0 diff**。新增 typed 路径未改写旧 moves、centers、BSP、
pan-div、tower 或 ownership 终态；没有读取 `created_at`，没有使用 `Math.random`、Rust 随机源或挂钟。

## 8. 结论

#92 的裁定后迁移、完整库测试与 4,613,599 根因果 prefix 重放均已完成。生产 B 口径产出
**19 张 NestCertificate**；D3 本样本没有跨级边，违反率为 N/A；旧路径 bit-exact 为 0 diff。
R7 不触发零产量签字，且 provider 完整性仍受 215 个 fail-closed `InvalidSeed` 限制，必须保留为能力边界。
