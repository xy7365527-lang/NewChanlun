# #1394 完整 9011 S runner Python 终审

**运行门：PASS（仅离线 runner 首试准入）。** 未启动 S/Q；没有真实 9011 双跑结果。C、整图与完整资源容量仍未完成，严格 RSS/heap/IO/内部阶段峰值为 `UNMEASURED`。

冻结候选：helper commit `a1dd0a82d37915a34ae053d3ac034b402b0f169f`；Root HEAD `3e5a5ba31dbe5c59beeee03de227fd8408eaac55`；Rust tree `7196a77fef56cf87cbe91dc4b135326a142e2395`。

| 文件 | SHA256 |
|---|---|
| `runner.py` | `155d22522d74471705ed6bb6c61499ea0dfb4e0ddf7403d4f48cffe60ae55335` |
| `evidence.py` | `8603beeeca192c2b74635b27f640dcb34332b104868a8b931ed21b1f48ad6a04` |
| `runtime.py` | `1bcae2f7dfedcf7e83fef406840b07fe6250b56433d379f497a2fb62f35d67b6` |
| `test_offline.py` | `992a18f1439cee178a52ede9e4b8aca40cc02c2900935a615a96c6eccb8dae4d` |
| `README.md` | `222a8bf08ceed9b6a7c438ccd3428545b92a905abaeb301a6979aa3fc9106a1c` |
| `contract.schema.json` | `3853d7ca2edc5ebf046b28ad84d87746d06f29579798ef7f9fd15cb6172b15d3` |

manager `FINAL-STATUS-v2.json` SHA256 `478d2b18c286ddaf858f328118d2ab075a8c2a2c64d969db777775db346566b8` 记录 50 项离线测试通过、9011 材料 prepare 通过、只读 gate `GATE_PASSED`；prepared SHA256 `24dd300a31c8ae8beb78c842185664e39b9584f6712be42ec6581843284c28ff` 已核。另独立运行两条无服务否定例，均通过；四个 Python 文件 AST 可解析。正式 runtime/socket 路径仍不存在。

最后一项阻断已修：`runtime.py:565-572` 在保存每个 case 的 `final.fullDB.json` 后，于同一清理期限内运行 `validate_final`；失败进入 `preserve` 错误记录，`CASE-RESULT` 保持 `PARTIAL`。`runtime.py:645-651` 只在 A 为 `VERIFIED` 时调用 B。两条否定例分别核实 A 最终 raw/source 错误仍保留快照与 `PARTIAL`，以及 A 为 `PARTIAL` 时绝不调用 B。

Root 另负责在真实运行前后逐项核 27 份原件，基线 `PRE-RUN-PRESERVATION.json` SHA256 `3bca89e81f1bfcf00f1614abfecd344bd439f45c98e2a8cfb3808d5728af901b`。本报告不把离线准入提升为 S 实跑、C 或整图验收。
