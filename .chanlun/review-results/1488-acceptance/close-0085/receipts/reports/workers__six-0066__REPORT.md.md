# 收据 workers/six-0066/REPORT.md

- 原件：`workers/six-0066/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`8cf9c99e285d0f75ade8dce2bc348a87fa392e411e01b6144ce4e34c656a6b55`
- 字节数：10219；行数：50

## 关键行（行号: 原文）

```
1: # #1488 0066 REPORT：0059 宽对拍 v3:: 其余失败的“相对 main”归属（只诊断，未改源码）
3: ## 0. 口径与环境
4: - main = 6badaa60494740702c217abc9921809de3899ce0（refs/uo1488/main）；#1488 头 = budget-handoff-fix-v2-checkout HEAD 727b3bfb4568955f78534a6cffb1b54369a01826（refs/uo1488/s66-head）。merge-base 25bc128d88；头经 57f9680c4e 合入 main，合入前 #1488 尖为 fff166540e。
9: ## 1. 总表
21: ## 2. financial_session 22 件名单（main 与头一致）
24: ## 3. 二分细节（first-parent，fp.txt 共 263 个，25bc128d88..fff166540e）
40: ## 4. 日志路径（均在 /tmp/nc-remote/）
43: - state 不带 testonly feature：main `20261020-214534-6badaa6049.log`、头 `20261020-214617-727b3bfb45.log`
47: ## 5. 未做与待办
48: - #2 r12_cold 的 `Resource(Budget)` 未定位到具体拒绝点：需加临时探针（随即 revert）或打开 BudgetExceeded 来源，之后才能在 (a)/(b) 间裁定。这是 main 绿本票红的三件中唯一未给出成因的一件，**优先**。
```
