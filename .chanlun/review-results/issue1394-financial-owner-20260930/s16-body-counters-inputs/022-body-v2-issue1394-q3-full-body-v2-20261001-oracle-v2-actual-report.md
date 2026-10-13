# #1394 S16 独立正文与来源核验

状态：PASS_INDEPENDENT_S16_BODY_SOURCE_HTTP。冻结 fullDB、47,671 原生节点、六族 472 条记录、独立 SQL 来源、每条正文的规范字节、全部实际 HTTP 片段与 Q 投影根均逐项核对。

实测：HTTP 5538 次，唯一 proof 节点 26685，响应 wire 172428851 字节；投影正文 1253831 字节，SHA-256 ec1465f6c20af978e23d8eea7fc6b7ecfb0a0ff8683202e9e5cf2e95e025565a。详细字段见 oracle-v2-actual-report.json 和 oracle-v2-row-report.jsonl。

本报告只证明此冻结 S16 与当前 Q3 全文／来源闭包；Client 原子安装、实际物理 allocation、完整 9011、产品前端 #1460 均未测。
