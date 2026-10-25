# #1488 S5（0075）终面运行收据

终面 = 合并面 3ee05a27137fd4a655cc83a8503d989b62a28f15；基线 = main 6badaa6049 r8a/r8b（`../s86/`）。
输入 = `../s86/` 冻结 S86 输入（INPUT-MANIFEST 同 S1；每臂 run-manifest.json 内复记 sha256）。
二进制见 `BINARIES.txt`。大文件（s.sqlite*、births.tsv、responses.jsonl、serve.stderr）留 WSL
`/mnt/g/AgentStorage/1488-accept/s5/<臂>/`，sha256 见 `runs/<臂>/LARGE-FILES.sha256`（n1 失败臂的 serve.stderr/responses.jsonl 小，随仓）。

## 臂

| 臂 | 二进制 | 模式 | DRIVER-EXIT | 用途 |
|---|---|---|---|---|
| p1、p2 | prod-3ee05a2713 | ordinary | 0 / 0 | A①：对 r8 冻结哈希 cross-binary，两跑 same-binary；R1 |
| n1 | testonly-3ee05a2713 | native | 1 | 第 2 条消息 Connection reset（serve 仍活，stderr 无错误行）；4 条前缀复现不出，按偶发处理、保留产物 |
| n1b | testonly-3ee05a2713 | native（生产默认帽 268435456） | 0 | R1 对照 |
| d1 | — | — | 1 | 无意义臂：首个诊断构建（0c981ad3f8）编译失败，被 `\| tail` 掩盖退出码，假二进制已删，链式 d1 立即失败 |
| d2 | diag2-c081bfdeee | native | 0 | D7 读数（只多一个 s5_charge 诊断字段；d2 与 n1b 对拍 PASS、节点集相同） |

## 脚本

- `run_s5.py` / `run_wsl.sh`：驱动（派生自 `../s86/run_s86.py`；增加 births.tsv、发前负载、native 开关）。
- `../s86/compare_s86.py`：本切片修 `diff_list`（元素为 list 时取 canon 串入集合；只影响差异摘要，不影响 equal 判定），原实现在 CONDITIONAL 逐代 stored 列表有差时抛 TypeError → STRUCTURE_ERROR。**需审阅确认。**
- `nodeset_s5.py`：CONDITIONAL「节点全集」差异分解（差集是否全在裸序列顶点之下）。
- `unreach_s5.py`：不可达节点按无父顶点的形状/键族分类并向下传播。
- `load_s5.py`：负载与计时摘要。`peak_s5.py`：D7 handoff 峰值读数。
- 派生诊断提交 c081bfdeee（`refs/uo1488/s75-diag2`，只存在于 NewChanlun 本地，不在本分支）：audit_owner.rs:727 在
  NativeTestOnlyPublication 诊断行追加 `s5_charge`（charge、limit、resources total_limit/peak/request_peak）。

## 收据（receipts/）

`cmp-*`：compare_s86 输出；`nodeset-*`：节点集分解；`unreach-*`：不可达分解（含 r8a 与 0065 nat2 复算）；
`peak-d2.json`：D7 逐代读数；`load-timing.json`：负载与计时。结论见 `workers/s5-0075/REPORT.md`。

## 已知限制

- 安静窗口未达成：p1/p2 期间宿主 1 分钟负载中位 9.8/11.0（8 核），其他代理进程在跑；见 `runs/*/HOST-*.txt`。
- 0c981ad3f8 上构建的生产二进制与终面 prod 差 1,244 字节（含 build-id），未深查；生产臂只用终面 prod 二进制，不主张生产构建逐位可复现。
