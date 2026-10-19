# TB-02-V 同价见证夹具（#1405 / #1343）

六个切片原样取自 #1343 诊断 `btc-zero-stroke-diag/witness/`（schema 同 `backtest::data::load_symbol`，坐标 = 切片内 raw 序号）。`real_*` 是 `analysis/data_cache/btc_1m_full.json` 尾 5 万根内的真实 BTC 1m 切片（`meta.full_index` 为全文件下标），`syn*` 是整数价合成构造。

期望输出（oracle）是 #1343 裁定包的 `WITNESS-WALK.md`，由 `rust/src/theta_v0/parser/stroke_tb02v_tests.rs` 的 `WALKS` 逐件抄录；教义正本见 `.chanlun/definitions/bi.md` §7.1。

| 文件 | sha256 |
|---|---|
| real_A_bars14_30_formed_then_popped.json | b4eaec3ad3d1fc3b0c925bbd4e51088051dc1320a91711880d0663c72a678859 |
| real_B_bars25_30_minimal_eq.json | e3c30584b1c55cc0b253ebed114a024a093231a27de3e955ecd4ac2ecb426dd5 |
| real_C_bars14_40_multi_root_second_gate.json | e5dff3622d8d2d42b029c62127f59b6a735170bf4f0b04f03cc3d7a5087c5779 |
| syn1_same_kind_equal_price.json | 47a800ee471185cf304ff9e883cb91209df1ec133d1c1bc2b0774864ef155b77 |
| syn2_multi_root_group.json | 1c22bdee093569fcfc9cf73e1fb2efba76cfd32cf77ac273940a197906ace9e0 |
| syn3_multi_root_replace.json | ad67f939ed11ea214f922c6f497033a6ab6aac9ce1ea08b820665dd4c44a8412 |
