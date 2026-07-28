# wave-1+5a/5b 全量双跑合并封印报告（#69，2026-07-27）

- 依据：`.chanlun/scene-ledger.md` :16-:20（中止令「先完全解决超线性再做一次性总重跑」+ :17 欠 4.6M dump diff=0 + 两卡验收节「豁免不覆盖 5a/5b」）。
- 两侧：pre = `/tmp/wt69-5a-pre/p123_fast_replay`（分支基 4d0c3101c2 构建，SHA e3dca86e…）；post = `/tmp/wt69-5b-post/p123_fast_replay`（ticket-69 尖 e8d5a47f06，SHA 69b14254…）。
- 数据：`analysis/data_cache/btc_1m_full.json` 全量 **4,613,599 bars**，无 MAX_BARS 截段。
- 产物：`/tmp/wt69-seal-{pre,post}-{dump.txt,stdout,stderr}`。

## 封印读数

- **dump diff=0**：SHA-256 双侧 `7e2b7614c9c1a9faf20d81243c288470ee5778ab4b87271e03d5760e986cc1c1`，各 1,703,528 字节。stdout 双侧 13 行逐字一致。
- **里程碑逐字一致**：0.5M–4.0M 各 500k 共同里程碑 pending/views/triggers/reevals 双侧全同（如 4.0M：pending=3256/18297 views=68687 triggers=34401 reevals=68687）。
- **终态 SPARSE_SUMMARY 语义计数双侧全同**：triggers=39448 reevals=79290 reuses=68749 syncs_self=46167 syncs_lower=79290 wm_cross_with_lower=58070 wm_cross_without_lower=0 term_rechecks=18301 **term_skips=2（双侧同值，R1 已接受 TURN 残余双胞胎化）** shadow_checks=0。
- post 侧 additive 计数（5b 新仪表，不进对拍面）：pan_entries=41121 pan_hits=528,329,208 pan_misses=164,371,148 pan_writes=164,371,148 pan_invalidations=164,330,027。

## 照实登记（090）

- **墙钟（并发竞争口径）**：pre 16,149.9s（4.49h）/ post 20,050.6s（5.57h），post +24%。两翼同机并发 + 同口径补跑穿插，带宽互染，单跑口径未测——+24% 只作方向读数。
- **超线性尾部两翼同存**：4.0M→4.5M 段 pre 4,192s（为 0.5M→1M 段的 ~20×）；5a/5b 语义等价性成立，但「杀超线性」在 4.6M 尺度未显效（主导项在 5a/5b 作用面之外）。memo 记账面：writes 164.37M 中 99.97% 被 invalidate，hits 528M——收益形态待评，不在本票验收面。
- 同口径 250k/1M base→final 补跑：dump 双零（SHA 614b8332…/ffadc4ef…），stdout 差仅墙钟 + additive pan_*（Spec 轴 MED 口径错位据此关闭）。

## 结论

**合并封印成立**：wave-1+5a/5b 全量 4,613,599 bars 双跑 dump diff=0，scene-ledger :17 欠账清讫。
