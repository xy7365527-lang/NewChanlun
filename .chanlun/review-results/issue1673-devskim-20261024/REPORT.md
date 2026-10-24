# #1673 DevSkim 红灯处置（PR #1686）

## 输入与结论

- 输入：DevSkim run 39685651726，候选 `04617a17cb`，原始 SARIF 91,794 条结果；门禁报告 97 个基线外唯一键。
- 80 个键来自正式会话运行证据：DS117838 ×8、DS173237 ×71、DS197836 ×1。均为合成数据的内容摘要、事实/对象标识，或内容摘要与并列时间字段；按精确 `rule|path|line` 入 `.github/devskim-baseline.json`。
- 17 个键来自源码摘要锁：`tb03c_fin_runtime.py` ×8、`test_tb03c_fin_runtime_contract.py` ×9。均为 SHA-256 输入/断言锁，逐行添加 DS173237 抑制及理由。
- 未改扫描规则、目录 glob、冻结夹具或财务运行逻辑；未把源码命中灌入基线。

## 凭据核实

`scan_credentials.py` 覆盖本票相关 122 个文件（含证据、驱动、契约测试和新增夹具），固定前缀凭据强特征 0，键名=值弱特征 0。输出随附 `CREDENTIAL-SCAN.txt`。

## 验证

- `python3 s_session/tests/test_tb03c_fin_runtime_contract.py`：6 tests OK。
- `python3 tests/test_devskim_sarif_gate.py`：11 tests OK。
- 两个 Python 文件 `py_compile` 通过。
- 原 SARIF + 新基线重放：唯一剩余恰为上述 17 个源码键，证明 80 个证据键精确覆盖且无额外放行。
- 从原 SARIF 仅移除这 17 个行内抑制对应结果后的本地预期验证：91,777 条结果，门禁 PASS；此步只验证基线差集，不替代扫描器对抑制语法的真实识别。

本机 Docker daemon 不可用，最终全树重扫与行内抑制识别必须由 PR 的 GitHub DevSkim workflow 确认；本地预期结果不冒称 CI 已通过。

## CI 第一轮复核（run 40074198818）

真实全树扫描确认 17 条行内抑制与 80 条证据基线均生效；唯一新增命中是基线更新记录中的完整 40 位 commit SHA。按既有记录格式改为 12 位短 SHA，不为该自触发追加基线。最终门禁待下一轮 CI。
