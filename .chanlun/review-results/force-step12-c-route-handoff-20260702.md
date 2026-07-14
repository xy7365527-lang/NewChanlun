# 力度代理 Step1/2 — C 路交接（已实装全绿，非待实施）

> 工位：ws-force2 ｜ 2026-07-02 ｜ Lead 裁定走 C 路（GOLDEN 严格不变）已批准
> **状态订正**：Lead 消息前提「context 不足、未落码」已不成立——C 路**已完整落码 + cargo test 全绿（1384 passed）+ GOLDEN 不变**。本文档记录「已实装的 API + 验证证据」，供 #13 消费与新工位复核，**不是待实施清单**。

## 已落码改动（3 文件，未 commit）

| 文件 | 改动 |
|------|------|
| `rust/src/theta_v0/classifier/divergence.rs` | 新增 `ForceProxies { seg_a, seg_c: ForceFeatures }`（derive Debug/Clone/Copy/PartialEq，无 Eq——含 f64） |
| `rust/src/theta_v0/classifier/signal.rs` | ①`judge_first_cached` 加 `dif`/`closes_tick` 参数，复用已算 a_idx/c_idx + `force_features` 算 A/C proxy，返 `(BspPoint, Option<ForceProxies>)`（A/C 同趋势方向）；②`extract_signals_with_hist` 加 `dif`/`closes_tick` 参数、返 `(Vec<BspPoint>, Vec<Option<ForceProxies>>)`（pair 同排 unzip）；③`extract_signals` 传空 dif/closes_tick + `.0`（签名不变，18 个测试调用点零改动）；④新增离线入口 `extract_signals_force(centers, segments, closes, close_src, macd_cfg) -> Vec<(BspPoint, Option<ForceProxies>)>` |
| `rust/src/theta_v0/classifier/mod.rs` | 生产(255)/增量(1094) 两处 `extract_signals_with_hist(.., &hist, &[], &[], &close_src).0`——传空 dif/closes_tick，热路径零行为变化 |

## 为什么 GOLDEN/bit-exact 双不变

- force 是**平行向量**旁挂返回，**BspPoint 结构体零改动** → digest 哈希 `{pts:?}` 不变 → GOLDEN 恒等。
- Classification/LevelState 零改动 → 增量 bit-exact 电池 `assert_eq!(incr_cls, leg_cls)` 恒等。
- 生产/增量热路径传空 dif → force=None 且 `.1` 丢弃 → 不算无消费者的死计算。

## 验证证据（cargo test --lib 全绿）

- `extract_signals_bit_exact_digest_guard`（GOLDEN 守卫）通过 → GOLDEN 不变。
- `bit_exact_synthetic` / `bit_exact_confirmed_len_open_tail`（full==incremental）通过。
- 新增 `force_proxies_juxtaposed_on_first_class_candidate`：验 BspPoint 逐字段==extract_signals + 一类候选 force=Some + C 段面积<A 段（背驰）+ DIF/振幅 proxy 已填 + 填充率报告（一类=1 并置=1 =100%）。

## #13 W-VERIFY 消费入口

离线调 `signal::extract_signals_force(...)` 拿每个候选的 4-proxy（MACD面积/DIF峰/价格振幅/速度）：
- 一类趋势背驰候选 → `Some(ForceProxies{seg_a, seg_c})`；二/三类无 A/C 对 → `None`（诚实缺省）。
- Step3（OR vs Lex 分歧分析）用 `divergence::weak_theta(mode, &seg_a, &seg_c)` 逐 mode 比较。

## 与 Lead 批准 C 路的唯一偏差（ponytail 已justify）

Lead 措辞「离线 assemble_gamma_with_force 填 interp.rs Candidate.force」；实装用 `extract_signals_force` 直返 (BspPoint, ForceProxies) pair。理由（YAGNI）：#13 消费的是 4-proxy **数据**，`extract_signals_force` 直出即满足；interp.rs Candidate.force 字段无当前消费者 → 等 #13 真需要按候选身份索引时再加（避免投机结构）。若 #13 确需 Candidate 层字段，见下方 30 秒补法。

### 若 #13 确需 Candidate.force（备用，当前不做）
`Candidate`（interp.rs:69，无 PartialEq）加 `force: Option<ForceProxies>`；`assemble_gamma_with_force(classification, force_by_source: &HashMap<usize, ForceProxies>)` 用 `extract_signals_force` 产的 source_index→force map 按 `point.source_index` 查填；现有 assemble_gamma/tower 变体 + 测试字面量补 `force: None`。GOLDEN/bit-exact 仍不变（Candidate 不进摘要/不 ==）。
