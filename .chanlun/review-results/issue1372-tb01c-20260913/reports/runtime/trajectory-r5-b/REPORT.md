# 最终候选 trajectory-r5-b 实际采集事实

候选 3ff8c47ea4ab2aeb4672722e204d9f20b35f9b7f；原 160 输入、500ms 节拍、S95 afterBegin 故障及 Q109 SIGKILL 均按冻结计划执行。runtime/consumer 退出均为 0，495 条核心记录；458 份绑定源码尾核不变。

HTTP 113 次：111 完整，2 未完整；原请求加响应共 13,124,799 字节。918 个原件逐长度及 SHA-256 对照一致。完整数字与失败原文见 FACTS.json，字节索引见 EVIDENCE-INDEX.json。

| 读取命令 | 实际结果 | HTTP 完整/总数 | 已安装/保留 cut | 毫秒 |
|---|---|---:|---|---:|
| reconnect/complete-rebuild | captured_validated_commit | 13/13 | 41 | 184.583 |
| reconnect/expired-original-cursor | captured_envelope_validated | 1/1 | — | 6.823 |
| reconnect/gap-rebuild-first-page | captured_envelope_validated | 1/1 | — | 14.626 |
| reconnect/initial | captured_validated_commit | 10/10 | 32 | 250.786 |
| reconnect/open-backlog-lease | captured_envelope_validated | 1/1 | — | 21.280 |
| reconnect/overflow-five-unconsumed | captured_envelope_validated | 1/1 | — | 100.285 |
| reconnect/valid-three-commit-reconnect | captured_validated_commit | 31/31 | 35 | 662.855 |
| revisions/as-known-before-revision | not_verified | 0/1 | — | 1.248 |
| revisions/before-revision | captured_validated_commit | 29/29 | 96 | 3730.342 |
| revisions/revision-three-commit-reconnect | not_verified | 17/18 | 96 | 2757.668 |
| slow-pages/old-token-after-q-restart | captured_envelope_validated | 1/1 | — | 353.086 |
| slow-pages/slow-fixed-cut | captured_validated_commit | 6/6 | 16 | 4334.966 |

`captured_envelope_validated` 只指正式 Client.request 公共头、因果、哈希与请求响应身份校验；raw 命令没有安装完整状态。Gap 原因、缺失范围、重建身份、首分页原件在 FACTS.json 完整保存。

修订 lane 在 Q109 故障窗口的 `not_verified` 不计作跨修订续接通过；旧 cut 是否保持由各命令原件给出。有效跨修订续接由独立 `trajectory-revision-final-ac3` 的公开 Client 95→99、113 页完整提交事实支持；该臂无 Q 故障，不替代本臂 495 条故障轨迹。

本报告只记采集与字节核对，不是 GUI 或完整 C 的最终独立裁判。停止和最终封存分别见后续回执；本报告不宣称活动状态目录已静止。
