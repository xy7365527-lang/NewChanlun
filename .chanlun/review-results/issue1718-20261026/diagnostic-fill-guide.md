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

bar显式记录opened_at与收盘event_time，要求opened_at<event_time，后一根opened_at不早于上一根收盘。
订单仅从submitted_at≤opened_at的bar开始生效，bar中途提交不能使用该根开盘价或极值。
bar须满足OHLC一致性、获知时间不倒退，且获知时间不能越过请求的 `as_of`。
开盘穿越时按更优开盘价成交，买取min、卖取max。模型一次性填满订单，
不模拟队列、成交量约束或部分成交。这些约束共同构成该具名诊断模型的适用域。

## 增量恢复

```sh
cargo run --bin diagnostic_fill -- checkpoint tests/fixtures/issue1718/boundary.json 1 > /tmp/diag-checkpoint.json
cargo run --bin diagnostic_fill -- resume tests/fixtures/issue1718/boundary.json /tmp/diag-checkpoint.json
```

checkpoint保存完整冻结请求、下一bar游标与已有成交。恢复比较完整请求，
模型、记录版本、规则、订单或行情任何变化均拒绝。恢复时用同一诊断核重算已处理前缀，
逐位核对完整首次成交序列，漏单、非首次命中或顺序变化均拒绝。此成本只发生在诊断checkpoint恢复。
核验后只增量处理剩余bar，不重复原成交。相同游标可重复调用，倒退游标拒绝。

## 正式验收接线仍待完成

`theta_backtest` 及其NT组件是ADR0048登记的冻结影子，不是正式launcher。
本票保留其Touch/Cross(0)配置拒收回归，但不将它冒称正式结果准入证明。
正式回放及launcher的真实结果消费者归#1381/#1411/#1523原责任线；
其尚未消费诊断报告的拒收证据须由主控衔接，本票在该证据补齐前保持未验收。
