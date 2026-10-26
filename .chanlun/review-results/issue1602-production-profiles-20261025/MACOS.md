# macOS 平台补验（复审后取得）

- run：[40130575778](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40130575778)，触发方式 `workflow_dispatch`，ref `sandcastle/issue-1602`，候选 `07e3443c5118bbdc3da2c3978f12576a1e5fece1`。
- `cargo test --locked --features s_session --bin s_structure_session v3::financial_session`：**26 passed / 0 failed / 3 ignored**，431.17s。WSL 排除的 macOS 保留量读数与 peer uid 分支已在真实 macOS 上执行。
- 后续 economic 库测试：222 passed / 3 failed / 9 ignored。三项失败为 main/#1621 已登记的 funding 回归，与基线同名同断言；不归本候选。
- 因此尾部评审中的 `macOS NOT_VERIFIED` 已被本作业的实体结果取代；整套 CI 仍不因此写成全绿。
