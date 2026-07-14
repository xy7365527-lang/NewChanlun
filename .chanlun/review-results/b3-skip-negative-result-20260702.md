# B3 NO-SHIP 负结果：segment_macd_area memo 不落地（2026-07-02）

task #4 / ws-b3 / 认识论 L1（CPU 度量确定性）/ 代码影响=零（探针已 revert，git 干净）

## 结论
memo 不落地。真边际 compute ≈ 1.0–1.5% 墙钟（1M/68s double-work 上估），落任务门槛 marginal 区；且 leaf memo 收益天花板被上游 O(n²) 结构封死。

## 实测（BTC，本 HEAD）
- stage @300K（墙钟 9.54s）：05_compose_resume=4643ms、07b_extract_second=406ms、07a=88ms
- 调用探针 @300K：calls=4,185,846，distinct(start,end)=2,314，跨-bar 冗余=99.94%，|hist| ops=271.5M
- stage-timer 07m=181ms 弃用（4.19M 调用 × Instant::now 噪声主导）
- double-work @1M：+0.68–1.01s / 68s ⟹ ~1.0–1.5%（上估）

## NO-SHIP 推导链
1. leaf memo 封顶 ~1%：99.94% 冗余根因是 extract_second（07b）每 bar 重扫全部历史段对（2314 对被重算 4.19M 次）——memo 只省内层 |hist| 累加，O(n²) 段对走查照旧
2. 275 局部依赖：正解=extract_second 上 frontier-resume 门控（07b/A 泳道，change-point 收敛），非 B3 leaf
3. B2 rung-1 复用：跨-bar 有状态 bit-exact 缓存=bug farm，收益被 #1 封顶 ROI 不成立
4. 禁前缀和差分照旧成立（浮点累加序）

## 翻转条件（重开）
- 全历史 4.6M 实测 area% >2% 墙钟（calls/distinct=1810@300K，随 N 增长可能破门）
- extract_second 拿到 frontier 门控后 area 上位残余热点
- 明确要 O(n²)→O(distinct) 线性化：正解=冻结历史 area 缓存（(start,end)→f64 单调增，2314 项，close_idx 只护 frontier 段）+ bit-exact 单测

## 下游
- B 泳道真价值=07b extract_second per-bar 全史重扫收敛（与 05 同属结构 resume 类）
- #12（673split）不依赖本 memo，独立推进
- 谱系：231/161/090/275；B2 §7 profile-first 纪律兑现
