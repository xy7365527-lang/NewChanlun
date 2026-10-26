# 诊断限价成交对照（#1718）

在同一冻结订单与行情快照上比较严格穿越和触价成交。

```sh
cd rust
cargo run --bin diagnostic_fill -- compare tests/fixtures/issue1718/boundary.json
```

合成 fixture 中，买单价100、卖单价110。第一根bar仅触价，第二根穿越1 tick。
cross结果在第二根分别以99、110成交；touch在第一根分别以100、110成交。
输出包含模式、具名模型、记录版本、规则、快照、获知前沿、成交订单与差异订单。

## 只生成诊断报告

CLI消费 `diagnostic-fill-request/1`。模型为 `adr0018-integer-bar/1`。
价格全部为整数tick，数量为正整数单位；模型没有费用、资金或持仓账。
`mode` 必须是 `diagnostic`，其他模式拒绝。输出始终标为诊断。

`cross(k)` 要求k≥1，原样使用显式输入k；不自动把更大k改为1。
`compare` 要求输入指定cross(k)，分别运行该规则与touch，不在cross失败后放行。
touch是乐观撮合，存在过拟合代价。诊断报告不得作为正式成交、持仓或验收结果。

订单必须在bar的事件时间之前提交。bar须满足OHLC一致性、事件时间递增、
获知时间不倒退，且获知时间不能越过请求的 `as_of`。
开盘穿越时按更优开盘价成交，买取min、卖取max。模型一次性填满订单，
不模拟队列、成交量约束或部分成交。这些约束共同构成该具名诊断模型的适用域。

## 增量恢复

```sh
cargo run --bin diagnostic_fill -- checkpoint tests/fixtures/issue1718/boundary.json 1 > /tmp/diag-checkpoint.json
cargo run --bin diagnostic_fill -- resume tests/fixtures/issue1718/boundary.json /tmp/diag-checkpoint.json
```

checkpoint保存完整冻结请求、下一bar游标与已有成交。恢复比较完整请求，
模型、记录版本、规则、订单或行情任何变化均拒绝。继续推进只处理剩余bar，
不重做已处理bar，不重复原成交。相同游标可重复调用，倒退游标拒绝。

## 正式验收接线仍待完成

`theta_backtest` 及其NT组件是ADR0048登记的冻结影子，不是正式launcher。
本票保留其Touch/Cross(0)配置拒收回归，但不将它冒称正式结果准入证明。
正式回放及launcher的真实结果消费者归#1381/#1411/#1523原责任线；
其尚未消费诊断报告的拒收证据须由主控衔接，本票在该证据补齐前保持未验收。
