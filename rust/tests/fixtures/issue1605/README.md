# #1605 公开 runner 旧基线夹具

输入取 `analysis/data_cache/btc_1m_full.json` 的 2017-08-17 至 2017-08-31（首尾日期均含），共 21,360 根 BTC 一分钟 bar。首根为 2017-08-17 04:00:00，末根为 2017-08-31 23:59:00。读取切片重新生成连续 source_index，与 `slice_date_window` 一致。

窗口依据为既有 `lee_readonly_layer_btc_structural_witness_644` 及 `chanlun/review-results/issue644-lee-attr-wiring-20260729.md`，不是根据本次候选结果选取。

期望文件由旧提交 `13ed4d45c8ff85175819ca987690328ab2a247a4` 的公开 `run_theta_v0_pi` 生成：默认 `ThetaConfig`、years=1、initial_nav=1,000,000。独立旧基线读数：1,617 次订单、800 笔已实现盈亏、801 条含期末强平的交易记录。

`baseline-13ed4d45-bits.json` 保存权益、逐 bar 收益、已实现及含强平盈亏、交易字段、11 项浮点指标的 `f64::to_bits()`，另存订单数与指标交易数。它不含候选新增的政策身份字段。禁止用候选重新生成期望。

生成脚本、原始运行日志与公开 caller 变异证据保存在 `/Volumes/AgentStorage/issue1605-pstack-pilot-20261025/`。常规测试：`cargo test --locked --lib p1605_`（本项目经 nc-remote 在 Windows WSL 执行）。
