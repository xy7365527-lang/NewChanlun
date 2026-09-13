# 三臂最终候选实际运行交付

三臂均绑定 `3ff8c47ea4ab2aeb4672722e204d9f20b35f9b7f`，每臂 458 份绑定原件尾核一致，runtime 与 consumer 均退出 0。按 AC3→R5-a→R5-b 顺序运行；前两臂采集后均经本臂正式 controller 精确停止 S/Q。未修改 W 产品源码、Git、GH 或原历史证据。

| 臂 | 故障范围 | 关键事实 | HTTP 完整/总数 | 原请求+响应字节 |
|---|---|---|---:|---:|
| trajectory-revision-final-ac3 | 原 S95 故障，无 Q 故障 | head95 保持 cursor95，head99 后 Watch96..99；113 页完整核验后原子安装 cut99，用时 23,254.249667ms，安装时输入已推进 head144 | 278/278 | 29,435,644 |
| trajectory-r5-a | 原 S95 与 Q109 故障 | 普通32→35续接、积压与过期Gap、cut41重建、重启后cut16旧token；修订与紧随AsKnown在Q故障窗口失败，未安装新状态 | 119/121 | 13,752,137 |
| trajectory-r5-b | 原 S95 与 Q109 故障 | 同上；全部495条核心JSON全字段与a相等；修订/AsKnown失败保留 | 111/113 | 13,124,799 |

R5-a/b 核心文件各 161,080,375 bytes，SHA-256 均为 `02fdd97ed3f2efbc2493a44506d1bc33cc3619fe57f0d25a781c39a50edd0442`。逐行解析未删除任何字段，覆盖 160 actual_ingest_result、160 final_receipt、160 authoritative_cut、13 全表、1 schema、1 lifecycle。完整对照见 `R5-CORE-COMPARISON.json`。不把两臂不同 HTTP 时序和页面完成数叫相同；core 的业务/运行诊断区分由冻结原 driver 定义。

AC3 无 Q kill，只支持独立有效跨修订续接，不替代原 Q109 故障双跑。其与旧 R4-a 的对照只称前494条子域，绝不声称全495等价。R5修订命令实际收到97..100有效Watch，但a仅完成G97的24页、b仅16页后遇Q故障，均`not_verified`并保留cut96；紧随AsKnown连接被拒，均未安装。

R5两臂原游标32→35均完成三批、30个同cut分页后安装；raw积压响应为`client_backlog_overflow`、缺37..41，过期响应为`delivery_retention_gap`、缺33..33。Gap返回token所有JSON字段未改地用于实际第一页，fixed_cut逐字段等于rebuild；随后独立公开load完整安装cut41。raw请求自身不安装完整状态。两臂旧cut16 token均所有JSON字段未改地复用，经producer_epoch2实际Q响应，索引frontier与projection digest不变。定点关系在 `RAW-CROSS-RESPONSE-CHECKS.json`，原完整字段在各臂 FACTS 与 HTTP 原件。

每臂 REPORT.md、FACTS.json、EVIDENCE-INDEX.json 保存逐命令事实、源与字节位置。AC3 与 R5-a 已停止并全目录封存；R5-b 初始只封 STATIC-MANIFEST 明列的静态范围，明确排除 state、sqlite、gui-final 与后续停止文件。根正在该臂 gui-final 做实际浏览器补验；收到精确停止回执后另封最终全目录，不改任何既有 manifest。

这是实际运行与字节核对报告，不是最终独立 C 验收裁判。历史r1/r2/r3与本轮故障失败全部保留；不以采集完成状态替代语义通过，不新增运行或放宽资源/期限。
