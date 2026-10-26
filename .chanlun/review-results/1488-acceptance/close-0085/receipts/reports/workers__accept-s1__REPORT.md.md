# 收据 workers/accept-s1/REPORT.md

- 原件：`workers/accept-s1/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`9124ad0106ea1cddefc47080651cdb3137f04bdfbaa611684e197883ffb1e952`
- 字节数：15303；行数：202

## 关键行（行号: 原文）

```
1: # #1488 验收切片 S1 报告：S86 基线 r8 与对拍工具重建
5:   产物为 `/mnt/g/AgentStorage/1488-accept/s1/bin/s_structure_session-6badaa6049`，sha256 `c944bd6bed40dd7e75332d66851ad050178b7ccb31860c1068d749de2726e040`。
7: ## 结论
9: r8 双跑全部逐位一致，确定性证据成立。`compare_s86.py --mode same-binary` 的判定为 **PASS**：INVARIANT 31/31 相等，CONDITIONAL 5/5 相等。
14: ## 1. 输入来源与哈希
17: - 源文件 sha256 `76472995cbf0825ceeeaa94e89bd60d1b1416d6c7e0a7ec3a1b0ec5a90c60087`，9011 根，1,006,026 B。
18: - 生成器 issue1394-gen/2，seed 139605，params sha256 `ffb54d8b92867142bc34b18620af195b96a0bc59d0f2134a97f350e17e328f3f`。
22: 1. 86 条信封逐行 sha256 等于仓内 receipt `018-FULL9011-MATERIAL-RECHECK.json.raw.json` 中 `ordered_request_hashes[0:86]`；
23: 2. 与档案里 9011 行 `envelopes.draft.jsonl`（sha256 `cd69fef24d6e05a9950e14131095eef40dd9c9f16738958ea89d4900a8462283`，取自 content-v3 档案）的前 86 行逐字节相同；全部 9011 行也由同一配方逐字节复现；
24: 3. 86 事件时钟与 `clock-plan.full9011.draft.json`（sha256 `b02ac2444161e1e7e327fcf2a2bb79b294356a3dd95a59a6404c5b92deb04f01`）中同名事件逐值相等，头字段相同。
26: | 入仓文件 | sha256 |
37: ## 2. 脚本说明
49:   - **INVARIANT**：
56:   - **CONDITIONAL**（同二进制必须相等，跨二进制允许不同但须解释）：`s_content_nodes` 全集（含不可达中间根）、逐代 stored_nodes/bytes、DDL、binary sha、meta 环境键。
60: - 自测：r8a 与自身对比 PASS。另做了人工篡改测试：
61:   - 改回复帧或对象行：INVARIANT 报 DIFF，判 FAIL；
62:   - 删除 1000 个不可达节点：same-binary 判 FAIL，cross-binary 判 PASS（只记一条 CONDITIONAL 差异）；
70: ## 3. r8 两跑产物与比对结论
79: | s.sqlite sha256（停服时主文件） | `f8a563a0…76da` | `f8a563a0…76da`（相同） |
80: | s.sqlite-wal sha256 | `4ced153e…a9f4` | `af389149…a9e0`（不同，WAL 尾部属 ALWAYS_DIFFERENT） |
82: 比对报告存放在 `/mnt/g/AgentStorage/1488-accept/s1/r8/compare-r8a-r8b.json`，仓内副本为 `s86/r8/compare-r8a-r8b.json`。结果为 **VERDICT PASS same-binary**，36 项全部相等。抽样根（r8a 与 r8b 相同）：
92: ## 4. 每代读数（供 S5 对照）
```
