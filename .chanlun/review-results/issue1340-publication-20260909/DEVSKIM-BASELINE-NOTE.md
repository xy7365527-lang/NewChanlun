# #1340 归档的 DevSkim 精确误报登记

名分：绑定开放票 #1340 的工作草稿，说明本次归档配套检查。已签规范、报告原字节及其归档索引保持原样。

本次为 `.github/devskim-baseline.json` 增加 5,321 个已核实的归档误报键。原 265 个键以及 #1194 的 `schema/generated_at/source_head/note` 历史元数据全部保留；新增来源单独列在 `updates`。没有排除目录、关闭规则或修改扫描器、workflow、gate。

取证来自固定扫描器 `1.0.90+fb2d676ce4` 的三次实际 SARIF：

| 对象 | 实际运行 | SARIF 条目 | 唯一键 | 原 baseline 外键 |
|---|---|---:|---:|---:|
| main 基线 644075a6abcb | [9 月 7 日扫描](https://github.com/xy7365527-lang/NewChanlun/actions/runs/34152777363) | 276 | 264 | 8 |
| 首轮归档 efc1ddc4d6c0 | [首轮 PR 扫描](https://github.com/xy7365527-lang/NewChanlun/actions/runs/34313815417) | 4424 | 4412 | 4156 |
| 完整发布归档 e7034688cc92 | [本次 PR 扫描](https://github.com/xy7365527-lang/NewChanlun/actions/runs/34318774726) | 5597 | 5585 | 5329 |

当前 head 与 main 的新增差集严格等于 5,321 个 `ruleId|path|line` 键，全部位于两个本次新增归档目录的 89 个 JSON 文件。5,320 条 DS173237 分别是 5,247 个 64 位 SHA 摘要和 73 个 40 位 Git 标识；另一条 DS176209 是 `issue1339-r2-spec-20260909/payload/spec/inputs/TEST-SEAMS.json:341` 记录既有 `confirm_fill` 接线缺口的 TODO 文本。

作者逐位置验证 SARIF 匹配文本仍在对应源行，并核所有摘要 token 的形状；独立工位从三次扫描重新分类、验证 89 个源文件与固定提交一致，核候选新增集合与其独立导出的 5,321 键逐项相同。独立候选复核结论为 PASS，覆盖原键/元数据、去重、路径域、误报依据和旧告警保留。此 PASS 仅表示登记范围准确。

使用这份 5,586 键候选运行原 gate，对同一份实际 SARIF 的结果仍为 **退出码 1、FAIL、剩余旧 8 键**。扫描器构建、扫描、上传均成功，失败不是工具或网络问题。本说明不声称新 CI 或整个 PR 全绿。

旧 8 键全部保留：`analysis/p3_random_gate_control.py` 的 141、197、251、344 行，以及 `rust/src/theta_v0/nautilus/strategy.rs` 的 19、104、113、314 行。两文件均未由此 PR 修改；其中 `entry_tick_for` 的限价腿仍是既有缺口，不因本次归档登记而算完成。其实现与旧入口处置仍须按完整 SPEC 和对应票的真实义务完成。

此登记随 PR #1341 提交供审阅；main 合入和整图验收仍按既有批准门处理。
