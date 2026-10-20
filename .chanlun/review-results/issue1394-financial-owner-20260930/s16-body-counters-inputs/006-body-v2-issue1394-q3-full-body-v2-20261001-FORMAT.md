# #1394 私有 Q3R2 HTTP 证据载体

本 fixture 归档实际 HTTP/1.0 请求和响应原字节。不改 Q21 限额、Content 码本、产品客户端行为或物理分配账。

每条记录的大端头为 `>4sIIII32s32s`：字面量 `Q3R2`、规范 metadata JSON 字节数、请求字节数、响应字节数、`request || response` 的 CRC32、请求 SHA-256、响应 SHA-256。其后依次是 metadata、请求原字节、响应原字节。metadata 固定为 `serial`、`complete`、`error`、`op`、`code`。`complete=true` 只表示 HTTP 原字节收齐：`code` 必须是 100–599 的整数，允许完整 503；业务仍会失败。`complete=false` 要求非空错误文本，保留 socket 错误或 SIGTERM 前实际收到的所有字节；`code` 可以是 `null` 或已经读到的合法状态码。两者都不能把半包当成功 Content。

最多 256 条且打包总长不超过 16 MiB，形成一个独立 `zstd -3 --long=27 --check` 帧，文件名 `archive-chunk-%06d.zst`。单条请求与响应合计上限 10 MiB；写者先验证解压，再将压缩帧完整写入 `.part` 并 fsync，重命名、fsync 目录，然后完整追加一条规范记录到 `archive-manifest.jsonl` 并 fsync。manifest 最后一行也必须以 LF 结束。缺块、多块、残留 `.part`、CRC/SHA 不符、serial 断档、布尔冒充整数字段、缺尾 LF 或无效 zstd 帧都会使 `archive_v2.py --verify` 失败。

每次发 HTTP 前，写者用 Q21 单响应上限、当前未刷块字节与 2 MiB 元数据/收尾余量，在 50 MiB 私有上限内预留最坏回复；若不足，先冲刷现有块，再决定是否拒绝新请求。SIGTERM 会关闭当前 socket；其半包先入归档，再冲刷末块和关闭 manifest。同步 zstd 编码/解码各有 30 秒上限，信号可能延后，父进程等待同一个自有 `Popen` 最多 180 秒优雅退出，超时才核身份向该进程组 KILL。归档读取器只证明原字节无损恢复；独立 Python 来源 oracle 仍须对拍完整 projection 与六族 472 条记录。本 fixture 不验 Client 原子安装、heap/物理 allocation、全 9011 对拍或 #1460 前端。
