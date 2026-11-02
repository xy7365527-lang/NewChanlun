# PR #1846 / #1845 非作者独审（45X）

结论：**源审 PASS**，精确 head `57c6d7deb85710fbcdc4d5d577dd8f95ed6719e5`，相对 main `784bc6d77016a216d085d0a63564c7937e325ed1`。动态验证尚无：CI run 41356423469 查询时 queued，本审未运行 Rust。PASS 不覆盖必要 CI、合入或关票。

审查者 Opus（作者链均 gpt-6.1-sol），异家族非作者。审了两个提交的完整差异（f441e9f0 实装、57c6d7de 封包修正），6 个文件。工位干净，HEAD 与 PR head 一致。

## 逐项核对（对照票面验收 1–9）
- 目标形状：`production_session.rs` 删除 `check_existing_s`、`immutable_uri` 与 rusqlite 引入；检索生产代码无 `immutable=1`、`check_existing_s`（验收 7）。launcher 每次启动写 `<state_dir>/structure-source.json`，init 与 serve 用同一文件（`production_session.rs:277-279, 411-413`）。
- S 自核：`writer_cli.rs:388` 可选 `--structure-source`；两处调用 `writer_cli.rs:673`（诊断夹具）、`:804`（serve_with_policy，s_incremental 与非 s_incremental 两分支都汇到此行），均在 Driver 打开取写锁并完成原 admission 之后、`finish_startup` 与 `serve_unix` 之前，失败即返回，不 bind。
- 读取：`driver.rs:322-332` 走 `admission::read_startup`（`admission.rs:489-555`）：pre_io_check、期限检查、`O_NOFOLLOW`、非普通文件拒绝、逐块 `charge_wire_bytes`。比较在 `audit_owner.rs:323-338` 借 S 已持锁连接，按 canonical 逐字比较，不一致错误点名两侧规范声明；缺声明 InvalidDomain，库内损坏由 `structure_source::stored` 报 StorageUnavailable，均 fail-closed。
- standalone：不传参时 `check_structure_source(None)` 直接 Ok，库缺或损坏声明仍按原合同监听（验收 5）。
- 测试（`production_session_entry.rs:657-880`）：S 直接 serve 的缺声明、不一致（断言两侧 canonical）、文件损坏、文件缺失、库内损坏、超启动帽，均断言非零、无 socket、raw 行数不变；匹配监听且持锁；standalone 两项保持监听；launcher 缺/不一致/损坏三项断言 S critical exit、stopped.failed、退出码 1、无 socket、行数不变；新库 init+serve 共用声明文件与库内声明一致。覆盖验收 1–6。删除的 `s_init_v1` 旧夹具仅被原 launcher 缺声明测试使用，其断言由新 v3 等价测试承接并加强，未放宽。
- 验收 8：本 PR 未改 manifest 校验、资金、隔离、admission 判据或恢复路径；base 未含 #1781 布局改动，后续若同文件冲突需按实际差异再审。

## 封包与隔离
`SEALED.json` 记录 head 57c6d7de、committed=true、文件清单仅两文件。`seal.mts` 只用同一 `STAGE.json` 与 r1 stdout 调现役 `finishManagedStage`，未绕过校验；提交带 `Sandcastle-Stage: 1845/implementer/f441e9f0` trailer，父为 f441e9f0。runner 用 `noSandbox`，与现役 `.sandcastle/main.mts:112` 及 H1 runner 同一后端，环境未加 GitHub 凭据，未扩权。

## 非阻断观察
- launcher 写声明文件用 `std::fs::write`（跟随符号链接），S 读取侧 O_NOFOLLOW，若被替换为链接则 S fail-closed，不会误放；属原有写法扩展到每次启动，不构成本票缺陷。

## 剩余义务
1. 精确 head 必要 CI 全绿，日志中新增与既有入口测试逐名 ok（验收 9），含生产构建与 testonly 构建相关 job。
2. 合入时消息不含自动关票词；关票按完整关票门与工位处置。
3. 与 #1781 同文件的合入顺序按真实冲突决定。
