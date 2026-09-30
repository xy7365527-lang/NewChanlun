# #1394 财务内容与 E/B owner 接线证据

本封包为 open #1394 的工作草稿，承接已批准 #1340 和 #1394 C 接口增补。后端总图 #1323、goal 保持未完成；前端 #1460 独立。没有合入 main、push、关票或修改 GOAL_SET/SUPERSEDE。

## 固定代码与规范

基线 `7ec958012401acd0515f8be558ae13e925473685`；根产品 `6dcee4da72873ab323132cf2fd64271702a1d8c3`。其全部 Rust 文件与受审 source `d8b4b713e4645d4b7833aca0705b4e35f22ab198` 相同。客户端 source `268f864ed48ef0540637a4073d31e75d04af8705` 的完整独评与后续调用审查保留。

规范为 [已采纳 C 原件](../issue1394-content-v3-20260916/APPROVED-SOURCE.md) §5.1–5.9、[增补登记](../issue1394-content-v3-20260916/SPEC-ADDENDUM.md)，及原 R2 C09。原 SPEC 和封存成果字节未修改。

客户端从当次 S3 basis/content 请求取得完整内容，按一份 Job 累计真实传输、节点和逻辑记录。合法长标量没有额外 4096/8192 字节门；selected 与片段证明在实际 Index 访问前计费，失败与重试不退款。根回归1项通过；新小源110请求、1307节点、386记录，传输与节点规模没有因修复改变。

新 economic-session/2 与 finance/2 有独立版本、13表和本 owner 不可变节点存储。当前和全部规定历史路径的 FinancialBasisEvidence 均完整核验；peer 捕获保持原 image/root。节点发布、经济 image、消息和 receipt 在同一经济事务。首次本地审计、多 peer、S、归约和发布共享 Job；私有入队时间贯穿新 Unix 入口到提交前，公开信封和身份不加时钟字段。失败及恢复只清未发布 staging，不删历史 published 节点。旧 /1 格式及接口保持。

成功存储域明确为每 owner 独占的固定容量 HFS+。新实际载体25124864B，原 frozen staged/store 限额32153600/64307200B；原单域16MiB试验另保留。硬容量和 DB/WAL/SHM st_blocks 实测承担预付/对账，不用 SQLite 逻辑页或节点正文冒物理量。checkpoint下降时的保守账面高估限制未推广为所有配置可用性证明。

## 根工位实际验收

| 验收 | 实际结果与边界 |
|---|---|
| 新 /2 SIGKILL 矩阵 | 6操作 × before/after × 两鲜进程，24/24 exit0；12组原 comparison字段逐项相同。每例有实际PID、SIG9、fresh recover、epoch2、原回执/责任断言。缓存复用，不是24次冷导入 |
| 真半导入 | 新实例删除仅未引用的 setup cache；实际 Prepare 读取真实S，持久 staging=1且commit不增时SIG9。S副本离线，fresh recover后stage0、pub0、经济原像不变、责任20、无Prepared/grant，旧请求StaleWriter。1/1，2.14s |
| 正式CLI/Unix | 固定e_economic_session/b_chong_session的3个serve进程；Prepareα/β→E Reserve→B Commitα/β、同ID重发、QueryDecision/QueryGrant/ReadView/Watch全经单LF+EOF Unix请求。grant30/40、责任90、B Committed/locked30/40、无外效资格；1/1，36.16s |
| 正式离线recover | 三个serve均已停止/reap；仅本实例S binary/DB改名离线，三个正式recover命令均exit0、epoch2、stage0，manifest字节不变 |

Book、Voice和已有20责任的 setup 仍为明确 library/TestOnly 前置，包含受控权威/操作结果。实际金融步骤没有伪造 grant或手工置 Committed；本证据不成为形式化γ、完整经营模型或生产资金政策。CLI运行原件、原始结果及SHA清单见 [RESULTS.json](RESULTS.json)。

## 失败原件与回收

首初始化在合法空hex body处失败，只有E genesis、pub0/stage0，尚未SIGKILL；修夹具后新卷重跑成功。首CLI在socket路径超过SUN_LEN时启动失败，尚未接财务请求；新实例以短别名冻结manifest后成功。全部失败库/映像/日志保留，不覆盖，也不把编译红当行为红。

矩阵工蜂69卷及根成功/失败实例均普通detach、保留dmg；正式3服务明确stop/reap。根已实际执行 hdiutil info 核对，本票新旧所有映像均已卸载，原件保留；核对原件见 inputs/TASK-MOUNTS-FINAL.json。临时验收工位独立于产品，仅追加验收及输入回归测试；经济运行入口保持 ignored，不入产品。

## 仍未完成

S3条件FIFO与serve具名资源/旧缓存门映射、完整来源变化重读与真实ENOSPC最终事务等余项、累计历史/全资源和9011双跑、其它TB与六Destination仍未验收。未跑远端CI或本轮DevSkim，不把既有告警或其它票扫描归零。没有改变1349未定公用资金/cap政策与真实场所权限。
