# #103 中段截断重放：区间套双测（展开完备性 + D3 逐边时序）

日期：2026-07-17 ｜ 数据：btc_1m_full.json（4,613,599 bars）｜ 状态：双测完成

## 输入与侧信道

- 二进制：`rust/src/bin/p92_nest_replay_postruling.rs`，环境变量 `P92_CKPT=250000`
  开启 #103 侧信道：每 250,000 bars 做一次全量快照装配并 dump（只写不判，
  不触碰 book/seen 主路径状态）。
- 转储：`/tmp/p92_ckpt_dump.txt`（CKPT 行 = 各检查点在场证书键集合；
  CERT 行 = 终端 as_of=4613598 集合，66 键）。
- 分析器：`/tmp/p103_ckpt_analyze.py`（键 = caliber,exec,top,side,bucket,ids 身份向量）。
- 检查点：18 个（250000..4500000）；键并集 67 = 终端 66 + 幽灵 1。

## 测 1：展开完备性（UNFOLD）

    P103_SUMMARY vanish=1 ghost=1 unfold_exact=66 unfold_late=0 unfold_early=0 uncovered=0

- **66/66 终端证书全部在 judge_max 之后的首个检查点即完整出现**，无迟到、
  无早现（不存在 judge 前可见）、无网格未覆盖。
- **1 例中段消失（幽灵）**，归因为**链打包身份迁移**、非判定撤销：
  - 750000 时：`3:725489:724160-725489|2:704358|1:706241`（A, exec=1, top=3, Long, pan）；
  - 1000000 起至终端：同 base（1:706241, judge_at=707523 不变）、同 rung-2，
    顶层身份迁至 `3:689893:688090-689893`（Trend），bucket pan→mixed；
  - 即 level-3 结构在 750000 前沿尚未定型，后续数据令包含该链的三级事件重新
    锚定。**exec-base 判定本身 append-only 成立**；消失的是 top 链身份。
- 条款建议（并入接入裁定 DRAFT 附注）：对外发布/打标以 exec-base 判定事件为
  稳定主键；top 链身份仅作归因字段，中段快照间允许迁移。

## 测 2：D3 逐边时序形态

    P103_D3_SHAPE asc(top<base)=7 desc(top>base)=7 mixed=4 single=48

- 多 rung 链 18 条中升序/降序/混合均有分布，**rung 间 turn_source 无单调约束**：
  区间套逐层判定不隐含 top→base 时钟单调，D3 逐边确认必须按边独立校验
  （p92 主路径 d3_violations=0 与此一致）。

## 口径差异记载（非新增异常）

- p92 终端 66 键（A 41 / B 25）≠ nest 91 张（A46/B45）：末端 `2:4613084` 簇
  （39 张、judge=4613104）在 p92 装配集合中为 0 条——p92 走 `terminal_bits_new`
  且无 intake fallback，与 #100 对账已记录的口径差异一致；双测结论不依赖该簇。

## 复现

    P92_CKPT=250000 ./target/release/p92_nest_replay_postruling \
      analysis/data_cache/btc_1m_full.json > /tmp/p92_ckpt_dump.txt
    python3 /tmp/p103_ckpt_analyze.py /tmp/p92_ckpt_dump.txt 250000
