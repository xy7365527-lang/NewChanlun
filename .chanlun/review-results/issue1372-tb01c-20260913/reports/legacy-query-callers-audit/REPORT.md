旧 Q 调用兼容审计及 R9/R10 补跑：PASS_BOUNDED_LEGACY_R9_R10_COMPATIBILITY。C 最终验收未裁定。

已确认并保留原始 RED：旧 r9_contract.py:71–72 缺 --resource-config / --producer-epoch；Q argparse exit2，HTTP 从未就绪。默认 R9 与 --r10 共用此 Server，均受影响。修复没有放宽 Q 门。

本次只改 s_session/tests/r9_contract.py：声明 TestOnly 查询资源/epoch1、添加显式 --binary 入口、以本人 Popen 的有界清理替代 macOS 不可用的 /proc 读法，并在 R10 异常时保全日志。exercise 与 exercise_r10 两个业务函数 AST 与修复前逐字结构一致，既有结构断言未改变。

| 真实入口 | 正常/历史阶段 | 损坏病例 | HTTP | writer CLI | 已回收 Q | 退出 |
|---|---:|---:|---:|---:|---:|---:|
| r9 | 7 | 44 | 324 | 331 | 45 | 0 |
| r10 | 15 | 21 | 392 | 199 | 43 | 0 |

44 种 R9 与 21 种 R10 损坏全部满足原拒绝且零写断言；R9 七阶段 observation 对拍全相同，R10 十五阶段 AsKnown0 稳定。两进程 stderr 均空；88 个 Q 均由本测试 TERM 并 communicate 回收。所有旧失败、输入、独立数据库、HTTP、CLI 和输出原件保留于本目录，未覆写旧 RED。

调用枚举：现役 CLI 构造只有已修旧 launcher、正确的 service 控制器、此处旧 R9/R10；main 和两份 C 测试的 QueryHTTPServer 构造都传齐资源/epoch。r13_integrity 直接调用 read_state，C query tests 直接构造服务器，旧 Node 消费者使用外部 URL 或 VM；这些都不证明 Q CLI 参数仍兼容。未发现现役 Q 的旧二参 HTTPServer 构造残留。

之前可复用的 root-final-qmemo-v2-integration.json 是 v2 直接构造服务器的 5 ingest/21 行旧 token 集成。根确认此前没有最终候选 R9/R10 CLI 收据，本次补齐。现有 pytest testpaths 与 Rust CI 不执行这两条手动 Python 入口，不能用它们绿色替代此补跑。

固定 writer SHA256：5bb7f1dfd57e1d8e6cfc1097360246ae27333776504767df4aba1b9fba9ce71b。完整 argv、源码/原件摘要、逐调用位置与未覆盖项见 REPORT.json、FIXED-CHECKS.json 和 ARTIFACTS.json。没有访问正在取证的数据库/服务，没有提交产品或裁定自己的 Q 最终验收。
