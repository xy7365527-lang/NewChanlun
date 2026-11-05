# #1789 受控 Q 进程的 bytes 分页对照

运行器在同一个 Q 子进程内，依次对第 60、106 代各做一次旧尺寸与优化尺寸读取。每代必须保持投影根一致，并同时降低请求数与响应体 wire 计数。它不修改输入配置或数据库，不生成大夹具。

在已分配的 Linux CI 或远端验证窗口运行。八项参数均须显式给出，四个文件路径和新输出目录须为绝对路径。

```sh
node s_session/tests/run_q3_owned_comparison.cjs \
  "$Q_BINARY" "$Q_DB" "$Q_RESOURCES" "$CLIENT_RESOURCES" \
  "$NEW_OUTPUT_DIR" "$PORT" "$PRODUCER_EPOCH" "$SOURCE_REF"
```

`SOURCE_REF` 记录本次 Q binary 与数据库的已核来源。执行者须先确认二者来自同一生产候选，并且数据库包含指定两代。这个文字参数不替代来源验证。Client 配置沿用该验证窗口已批准的配置，不扩大任何资源帽或期限。

输出目录含以下原件。

- `inputs.json` 保存二进制路径、启动参数、Client 配置和来源记录。
- `q.stderr.log` 保存本次 Q 的真实启动记录。
- `comparison.jsonl` 保存每次读取的根、请求数、wire、完整 Job 账、耗时及部署记录。

服务端资源帽只从本次子进程的冻结启动记录提取。启动 owner 生成 nonce，绑定 Ready、endpoint 和 epoch。子进程退出或重复启动记录会撤销证书。退出时运行器只终止自己启动的 Q。

分配适配器是现有 `TestOnly` 具名模拟账。它可复核客户端计账行为，不代表 Browser 物理内存验收。公共任意 fetch 不具备完整 HTTP 帧证明，不启用本优化。

本入口不能替代 #1419 的五条 real_cut 行为验收。固定来源为 PR #1793 的 `528fb42b4c07eb426bbee8fc50b535dae767cd63`。其原件要求前两条进入普通 CI，后三条逐条进入有 16 GB Node heap 的重型窗口。当前实施基线未包含该整合分支；整合 owner 应消费该固定来源，不复制或重生成 92 MB 夹具，也不把本运行器的结果冒充 real_cut 通过。
