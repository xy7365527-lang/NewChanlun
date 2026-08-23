# Agent Roster 2026-08-18

| 类型 | 模型 | 任务 | 状态 |
|---|---|---|---|
| sandcastle 两段（implementer+reviewer） | deepseek-v4-pro | #1053 删除全量 classify 循环（空缓存=全量，七入口签名不动） | 已关票（镜像 f9f1a93b4a，CI 32114036826 success） |
| rlm 子代理 scan-archaeology | deepseek-v4-pro（继承） | #1056 P1/CandDeltaEvent/nest 对照侧历史考古 | 已关票（子代理卡死后本体接手，报告 98bdfa48ea） |
| sandcastle 两段（implementer+reviewer） | deepseek-v4-pro | #1068 Massive 抓取管线深化（policy 模块独立成件 + 失败日显式化） | 已关票（merge fdcc2d3c57，CI 32116231103 success） |
| sandcastle 两段（implementer+reviewer） | deepseek-v4-pro | #1054 classify 入口家族收口（7→2 + 统一输出模块，消费点约 80 处） | 已关票（main/mirror 1994b60c50，CI 32121697512 success） |
