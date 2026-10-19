# #1394 读侧 adapter 四处窄返修01（工作草稿）

作者：Sandcastle run17（gpt-6-astra，run `5cc1d593-8c8d-4ca9-a072-8a82dae14c7d`）写完源码与测试后因 Codex 额度在出报告前退出；接续负责人 /root/owner_1394 于 2026-10-19 核实其差异、补跑验证并收包。材料基线 HEAD `490b53c2`，真实 subject `4b3a84ebba`；被修文件字节与集成分支 `codex/1394-acceptance-integration-20261018` 同名文件一致。

| 身份 | 独审 | 修法 |
|---|---|---|
| A1 | P1 `_cli` 清理后按路径读 stdout/stderr，可冒收外来预存原件 | `_StdinProcess` 在取得目录后开只读句柄并核 inode；cleanup 只从持有句柄按帽读取并封口核身份；persist 只核路径仍指向原 inode，不读路径内容 |
| A2 | P2 公开 ProductTransport 接受任意 output | 构造与每次发送前后核 output 绑定根准入且持有本次 `OwnedDirectory` |
| A3 | P2 准入后 binary 漂移仍先占目录 | `ReadAdmission.verify_current()` 在建目录前重核 owner 身份、owner/reader binary SHA、原最早截止；发送前 guard 保留 |
| A4 | P2 工具读预算只计六族 canonical | Ledger 持同一 `ReadBudget`；请求帧、回复+stderr 原字节在解析前计费且失败不退；snapshot/basis/closure/投影/闭包遍历逐检查点计记录 |

验证：四条正式探针在基线红（red-targeted.txt，5 失败含 A2 两子例）；修后 test_product_capture 全 56 项普通与 -O 均通过（有限内存 fake 与临时普通文件，禁 subprocess/socket/sqlite）。未运行产品、cargo、网络、DB、真实进程。

尚欠：独立审阅（作者与本收包人均非独审）。execution_authorized=false；不勾 P11，不关 #1394。
