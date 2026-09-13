# Q 与 root 修复独立增量复核

结论：**PASS_BOUNDED_FIX_REVIEW**。原 PY-XR-01/02/03 在本次具名范围内闭合；这不代表 #1372 通过。根已报告完整 R1-A 在 seed 阶段失败，本报告不覆盖或改写该事实。

身份：`/root/r7_codex_product_review`，`codex_native_subagent`。精确输入是 `SOURCE.json` 六份副本；原报告保持不变。

- **PY-XR-01：CLOSED_BOUNDED**。按唯一 Delta.index_frontier 找批次；完整 BLOB 保留类型、base64、bytes、SHA；全 Delta 160 行与全 batch ID 集 160 闭合。实际采集代码离线模型：健康 174 个 DB 记录，孤儿批次和额外 Delta 拒绝。495 = 160 输入 + 160 原身份收据 + 174 DB记录 + 1 生命周期；未执行完整轨迹。 位置 `source/tb01c_runtime.py:256-299`；证据 `RESULT.json#/collector`。
- **PY-XR-02：CLOSED_BOUNDED**。Ready 完整头/nonce/活PID与启动身份复核后登记 inode；清理要求停止记录、同inode、本人socket、ECONNREFUSED、删除前重核。四臂：旧自有失活路径允许，未知登记/替换inode/活监听拒绝。Ready/PID绑定为代码核对与旧真实收据，不冒充本轮新启动。 位置 `source/s_service_control.py:193-266;317-388`；证据 `RESULT.json#/socket_cleanup`。
- **PY-XR-03：CLOSED_BOUNDED**。所有 raw 含 pending 的 price/ts/volume 在hash/receipt前核规范signed-i64。原三条自洽坏值拒绝，三字段各-1/MIN/MAX九个合法控制通过实际capture→verify；未收窄为非负。 位置 `source/s_query_integrity.py:286-306;321-327`；证据 `RESULT.json#/numeric`。

独立执行 `probe.py` exit 0：12 数值臂、4 socket 清理臂、3 SQLite 采集臂，共 19 项符合。没有启动产品服务、访问根运行库或修改产品。采集模型保留于 `collector-healthy-model.json`。

Ready 的完整头、nonce、活 PID/启动身份与剩余总期限核对见 `s_service_control.py:317-367`。旧真实重启收据中 Q 的 before/during/after 完全一致，S PID 76544→78307；清理的是原 inode 307103256，新 nonce 与新登记一致。这是旧 P2 具名复用，不是新 R1 证据。

最终文件核对发现控制器另有 stop 增量（SHA 9f4008c9…）：发信号后身份暂变只记录并继续在原 deadline 内观察，不换 PID/重发信号/认定退出。已静态核对，保留 `source/final-s_service_control.py`，不把旧测试副本称为现文件完全同字节。其余五文件同 SHA。

限制：socket 模型替代 status，未新执行 PID/nonce 故障注入或穷尽并发路径替换。174 条采集是合成模型；Q owner 的 78 组合与 24 检查仅作外部复用。本轮不重审 Q 全域、不自评 Rust。完整轨迹尚无通过结论。证据 SHA、路径、JSON pointer 与计数见 `REVIEW.json`、`MANIFEST.json`。
