# #1372 HTTP 介质失败修复独立复验

结论：**HTTP-CAPTURE-01 已在固定 helper `cc0e3dc134deda93fcc64b6d6e2807a1e403ea509f81afbab882b8ff239c7397` 闭合**。

复用原候选文件 EIO 触发，改为真实 CLI 子进程，且事前同时排入 load 和第二条 Watch。正式 Client 经 6 次自有 HTTP 完成 load 私有提交后，helper 明报 `fatal_recording_failure`、`full_state_installed=true`、`committed_but_not_archived=true`，保存已排队未执行命令名单并退出 1；第二条 Watch 没有 HTTP 请求或 RESULT。无注入正控的两条命令均完成且退出 0。共两个 CLI 子进程、18 个自有 HTTP 请求；探针 exit0 表示以上预期断言成立。

已读完整增量：记录层将 write/open/writeSync/closeSync 的异常具名包装；execute 不将介质失败降为普通业务错误继续，队列和 pump 停止。没有伪称私有提交已回滚。原版 FAIL 报告及证据保留。

机器结果、完整 CLI stdout/stderr、HTTP 原件与固定源码在本目录；SOURCE.json 与 REVIEW.json 含 SHA 绑定，源码未漂移。本轮仅该修复和健康正控，不是当前 S/Q、GUI、完整负载或整个 C 的验收。历史 payload 由自有回环端按实际请求重封公共头。身份 `/root/r7_codex_product_review`，运行时 `codex_native_subagent`。
