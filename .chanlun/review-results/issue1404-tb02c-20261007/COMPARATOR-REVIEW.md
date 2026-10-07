# #1404 Python 比较器独立补审

结论：PASS（仅本次比较器与安装接线差异）。初审发现的 live-0 操作身份错误、共同删去顶层数据字段仍可放行的问题，已在当前所审字节修复；未发现剩余具体阻断。本文件是本票工作草稿，不证明正式双跑或整图完成。

## 所审文件与 SHA-256

工作树：`/Users/silencehan/.codex/worktrees/issue1404-seed-first/NewChanlun`。

| 文件 | SHA-256 |
|---|---|
| `s_session/tests/tb02c_compare.py` | `96c7bb453aa2d89d5d71d2b95ad3e67ccd9ef2fd0d4734ca2a1cf6cefa785c56` |
| `s_session/tests/test_tb02c_compare.py` | `83de9b5a19a25a08465ab6ea4083c35903f6c9469a7ee7fee456eb2a285068f3` |
| `s_session/tests/tb02c_harness.py` | `d81001cf46b2e4d9199d8386aa07ff15aee8e79e3421429d1ca94a86deb3bf1d` |
| `s_session/tests/requirements-tb02c.txt` | `a906fc959c048a5535ca832ecc293f29bb6d13f7f47ee8ee75ba59f5cb62b3a5` |
| `.github/workflows/ci.yml` | `dcee555aae331c833fd23e66573c87822d4008c8edb815e5394ff5bb98010082` |

## 具体核验

- `tb02c_compare.py:42`：live-0 固定为 load，其余 live 为 watch，与冻结 collector 及 R3-COMPLETENESS-REVIEW.md 一致；新增测试同时锁住两者。
- `tb02c_compare.py:22、104`：按已声明 kind 核顶层完整键集，拒绝共同删除 result/request/response/candidate/columns/rows/before/after；仅身份壳不能成为完整轨迹。
- `tb02c_compare.py:59、79、128`：逐块硬比全部原始字节；逐层检查重复键、数字类型、JSON 完整性、身份与次序；缺计划记录、额外尾部、读取期间文件变化均不能 PASS。字节不同比较结果仅为 NOT_VERIFIED。
- 161条布局来自28输入和冻结表清单，与事前完整性说明一致。harness 核 fixture 指纹、双向双 arm 次序，以及采集/oracle/浏览器/正常恢复收据后才比较，任一比较未 PASS 不报告 passed。
- 依赖固定为 ijson==3.5.1，CI s-observer 在 TB-02-C 测试前安装对应 requirements。

## 实际验证与边界

使用父代理指定的 compare-deps 与项目 .venv Python，执行 test_tb02c_compare 的6项测试全部通过。另在临时普通文件中构造完整161条仅身份壳，两侧相同仍返回 NOT_VERIFIED（顶层字段缺失）。测试前将 sqlite3.connect 改为立即抛异常，确保补审未打开任何 SQLite；未启动服务、未运行 Cargo、未改受测源码。

本审阅确认字节一致与具名记录完整性；字段的语义有效性仍由冻结 collector、独立 oracle、公共 HTTP/浏览器证据承担。内存随最长标量与对象键集合增长；不据此声明 S7 或 S/Q 资源资格。
