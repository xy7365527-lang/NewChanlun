# #1404：A3 oracle 覆盖失效只读诊断

结论：当前 CI 新增失败是旧合成夹具失去覆盖的证据；该日志没有报全量/增量不等，不能据此裁为缓存实现回归。严格 construction.valid() 准入应保留。已找到 267 根 raw 的确定性替代夹具，既有多产 pop 重扫，又有上级空投影，当前构建逐根全量/增量对拍全部通过。

## 旧夹具与精确条件

- 诊断开始时的 `rust/src/theta_v0/backtest/incremental.rs` 中 `a3_oracle_pop_rescan_empty`：旧 fixture2 是 `pseudo_walk(9000,0xD1B5_4A32_D192_ED03,46,500,3500)`，每根全部 OHLC 加 raw 序号，然后 `without_cross_bar_price_ties` 细化；注释“多尺度锐锯齿”并非独立实现的另一夹具。根任务随后已开始接入本报告的267根生成器，旧行号不再作为当前代码引用。
- `classifier/pipeline.rs:1002` 的 `had_emitted_window` 必须为真；`:1158` 才按 `tail_upper.len()` 增加 `t_gt1`。T>1 不是“产生任意上级中枢”，而是已有末窗被 pop 后，本次重扫返回至少两个输出对象。
- 最直接的 T>1 输入是至少九个已准入 L0 段围绕同一核心。`recursive_tower.rs:523` 吸收触及核心的延伸，`:534` 九段以下只产一窗，`:553` 九段起按三段重切，首次得到三个子对象；也可由停止延伸后重扫产多个不同窗口触发。
- empty 的条件是该级输入已达到默认 min_parts=3，完成本级 compose 后上级投影仍空；`pipeline.rs:1570` 才走 empty break。少于三单元在 `:763` 就走 minparts，不能作为 empty 夹具。
- 已给 CI 日志 `n=9000,max_depth=1,T==1=8596,T>1=0,minparts_break=17620,empty_break=0`：它仍有常态单窗 pop 覆盖；既没有多产重扫，也没有输入达门槛后零中枢的 empty 覆盖。不能删断言或删严格准入来掩盖。
- 已存 `s_session/tests/fixtures/tb02c/raw-ledger.json` 的 first_up/down 每例 28 raw，只足够确认首个三笔段，无法直接补这两项覆盖。

## 替代 raw 夹具

所有 raw 都走公开生产 parser 与 classifier，没有直接构造 Segment，没有改阈值、故障注入、服务或 SQLite。

共同生成器见控制目录 `oracle-frontier-probe.rs`：先放 raw0 close=500 和 raw1 close=1；每对相邻 knots 用四根整数线性插值 `a*100+(b-a)*100*t/4+raw_index`，t=1..4；open=close，high=close+1，low=close-1。四根间隔满足默认新笔排除两端至少三根要求；raw 序号显式细化同价。生成器末尾增加向上一根确认最后的底分型。

|夹具|knots|raw 数|当前构建实测|
|---|---|---:|---|
|valid-nine|`[0]` 后重复五次 `[10,4,20,8,16,0]`|123|9 个有效段，max_depth=2；raw122 原 cursor 已有窗口，重扫起点0，现9个L0单元，重扫产3窗|
|pop-empty|`[0]` 后 j=0..11 各追加 `[8*j+10,8*j+4,8*j+20,8*j+8,8*j+16,8*j]`，取生成结果前267根|267|11 个有效段，max_depth=2；raw170、266 各重扫产2窗；raw266 的 level_idx=1 有3个输入但无中枢，空投影成立|

第二夹具的最终上级输入外缘为 `[0,4486]`、`[2496,7782]`、`[5792,10254]`；`max(lo)=5792 > min(hi)=4486`，三者无公共核心，故 empty 是真实几何输入而不是修改计数器。原始 raw JSON 已保存为控制目录 `pop-empty-267.json`，SHA-256 `9492b70b8f744178127c5678eba2dd3122407b933f8f9a26f4993a38ba1b1f74`。单独九段 fixture 是 `valid-nine-123.json`，SHA-256 `3f2304852f32e02fc79de4e04b0a126c18cf4d87b5dae996e83f3f949eee9b5f`。

## 验证边界与建议

链接当前 r5 debug `libnewchan_rust.rlib`，未触发 Cargo 并发构建。123+267 根每根均断言增量 parser.segments == 全量 parser.segments，增量 classification == 全量 classification，增量 tower == 全量 tower。全部成功，短窗总运行约0.5秒。公开只读 cache 的旧 cursor 与输入扫描前缀用于复算 `detect_centers_windowed_resume`，证实 L0 pop 后 T>1；上级三个输入和空中枢直接来自生产返回对象。

rlib 非 cfg(test)，不含 oracle_probe，故此报告不伪称已读到精确 t_eq1/t_gt1/empty_break 计数。根任务应把 pop-empty 的生成器放入原有 always-run 测试，复用 `run_oracle` 跑原三条覆盖断言；保留 bit-exact 比较与原有覆盖门。该267根候选同时补齐两项目标；可保留 valid-nine 为更易理解的九段升级独立见证。不需改准入，也不需9K随机重扫或大窗全回放。

认识论范围：这是夹具适配与短窗管线一致性证据（L1），不证明所有输入都无实现回归，也不宣称真实行情完整覆盖或交易有效性。123/267 是已找到的短窗见证长度，未声称全局最短。
