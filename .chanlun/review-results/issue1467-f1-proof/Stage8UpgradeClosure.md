# 第八轮：九段触发不自动给出三个合法子核心

日期：2026-10-03。承接math-kit-goal-reframe-v3，关联研究票1467。冻结问题为 `DC-UPGRADE-CLOSURE-v1`，完整目标见 `Stage8WorkCard.md` 和 `UpgradeSpec.lean`。

本轮在DC-v0的合法实例内反驳了一个额外的充分性假设：**九个已完成、有波动、且各自与初始核心严格相交的单元，不保证固定三段重切后得到三个合法严格子核心。** 反例第三子窗是真空交，排除了“仅是相切端点问题”的解释。

同时证明一个足够条件：若三个单元都覆盖同一个非退化核心，其三段核心也覆盖该核心且非退化。该条件不必要，本轮不据它缩小原始研究域、修改DC-v0或增加生产准入门。

## 与已有规则、Stage5的区别

中枢正本§4.4–4.7已经区分S-4的触发计数与Z-4的触发后计算，Z-4明确允许不产出没有严格核心的子窗；既有[重切探针](https://github.com/xy7365527-lang/NewChanlun/issues/821)也以该差异为题。因此本轮不宣称新发现了教义矛盾，不重裁“9”，不声称原义F₂不可能。

Stage5反例的三个子窗各有合法核心，只是旧实现复制了错误母核心。本轮进一步说明：**即使改为正确重算，也可能只剩两个合法局部核心**。数值纠正和语义组装仍是两项义务。

## 具名输入与子窗

固定bid=10000、ask=10200、卖量10000，买量为：

`1429,2903,1429,2903,1765,3333,2500,2739,2121,2270,1976`

精确数量加权、半值向上量化后，观察列为：

`10025,10045,10025,10045,10030,10050,10040,10043,10035,10037,10033`

δ=2。九个完成DC单元分别连接观察0→1、…、8→9，确认序号分别为2、…、10。初始核心为 `[10025,10045]`。所有九个单元都与它**严格**相交，单元幅度至少2。

| 子窗 | 三个成员区间 | 重算严格核心 |
|---|---|---|
| 0–2 | [10025,10045]、[10025,10045]、[10025,10045] | [10025,10045] |
| 3–5 | [10030,10045]、[10030,10050]、[10040,10050] | [10040,10045] |
| 6–8 | [10040,10043]、[10035,10043]、[10035,10037] | 下沿10040 > 上沿10037，不成立 |

这不是三个无关区间的拼接。Lean证明该完整观察路径确实满足已冻结的 `DCSpec.Whole`，结果恰有九个完成单元和指定活动尾部。原定义没有改动，证明没有引用Rust的返回值等式。

Lean还逐点证明所有11个买量为正，按冻结的精确量化公式得到上述观察值。因此正数量L1状态和抽象价格路径之间的这一个具体桥，也在形式化根目标内。仍不把合成L1状态证明冒充某交易所完整订单ID/撮合协议证明。

## 数学证据

`DCFiniteCertificates.lean`把有限检查与旧关系连接起来：有限前缀内的回撤/振幅存在量词与旧Drop/Wide等价；最早极值证书、首次命中证书和无后续回撤证书推出原Extreme/First/尾部关系。它没有更改原DC语义，也没有只证明一个较弱“看起来像”关系。

`UpgradeResearch.witness_whole`逐项构造9次原 `Run.next` 和最终 `Run.stop`，不借永远输出空集逃避验证。`counterexample`一并证明完整分解、长度9、全部波动/严格相交条件、末子窗空交、正量/投影关系，以及否定冻结的AutomaticSuccess命题。

正面定理 `common_core`使用以下关系：若seed严格非退化，且每个a/b/c均覆盖seed，则

`max(a.lo,b.lo,c.lo) ≤ seed.lo < seed.hi ≤ min(a.hi,b.hi,c.hi)`。

因此三元核心仍覆盖seed。它可作为几何保持量的候选，但没有证明所有DC输入都满足它，也没有证明实际F₂的完成语义或所有输出会保留它。

冻结根 `UpgradeResearch.exact_root` 的类型为 `COMMON_CORE_TARGET ∧ COUNTEREXAMPLE_TARGET`。实际传递假定仅为标准 `propext/Quot.sound`。没有sorryAx、自定假定或native_decide。使用新lean-verify的精确声明检查成功，`machine_verification_passed=true`、`exact_root_passed=true`；`semantic.status=not_reviewed`，二者不混同。

## 两个实际执行与成员保存

Rust探针用真实 `center_from_segments`重算各子窗，对照两种具名输入：

- 上述弱条件反例：2个合法局部核心，第3窗记为NoCore。
- `[10025,10045]`往返的11观察正例：9个单元共同覆盖同一核心，3个子窗都合法。

程序检查完整DC输出的端点与确认序号等于形式化见证所指定的序列。每个子窗保留原成员ID：LocalCore和NoCore两类合计仍恰好覆盖0–8，没有丢掉失败子窗的3个成员。NoCore不生成Center、不改层号，也不参与准入；它不是“严格失败后改宽规则放行”。

这只是诊断证据的角色分开，不是完整F₂实现。两个局部核心和三个保留的低级成员不能被当作同层五个走势去继续递归，合法连接、同级组装和完成证书仍须研究。

## 复现与状态

实际运行并exit=0：

```sh
cargo run --offline --locked --manifest-path .chanlun/review-results/issue1467-f1-proof/stage8-probe/Cargo.toml --target-dir /tmp/nc1467-cargo-target --bin order-base-upgrade-closure
```

Lean检查使用已安装 `lean-verify/scripts/verify_lean_project.py`：project为本研究包、target-file=`UpgradeProof.lean`、declaration=`UpgradeResearch.exact_root`、expected-type=`UpgradeResearch.COMMON_CORE_TARGET ∧ UpgradeResearch.COUNTEREXAMPLE_TARGET`，固定Lean/Lake4.31、direct、60秒构建上限、strict-exit。原始证据在 `/Users/silencehan/Documents/Codex/research-evidence/issue1467/upgrade-closure-v1/`。其后保存证据核对为current，不是第二次内核证明。

对应摘要为 `stage8-lean-verification-summary.json`，Rust原始输出为 `upgrade-closure-stdout.txt`。`upgrade-spec-v1.sha256`保存证明前的定义版本，本轮未改变；源与证据hash见 `source-sha256-stage8.json`。新增Rust代码初次编译仅修正了Center的导入路径，没有修改候选或数据。

没有重跑Stage5旧实现、旧全枚举或市场实验。生产和formal源未改；安装配置不入提交。独立形式读回/比较包已准备，因当前父模型换模工具缺失而未派发，未伪造独评通过。

下一项应在保留未构成子核心的低级片段、避免混级、明确活动/完成状态的前提下，定义上递归状态转换。不能通过删掉反例输入、单成员升格或把“尝试”改称“完成”来取得闭合。原始P1–P4与两条F₁路线继续保留。
