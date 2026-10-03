# 订单基底候选：构造义务、反例与公开报价样本观察

关联：[订单基底候选构造与合法性证据](https://github.com/xy7365527-lang/NewChanlun/issues/1467)。名分：研究工作草稿。日期：2026-10-03。生产代码检查基线：`6f99b36c64e0560febdb08e5d1f8e781d2af9299`。

## 本轮回答与范围修正

用户选A表示研究范围包括价格不动时的队列变化，并明确说技术判断需要研究。它不表示已经裁定某个函数、认可队列波动就是缠论走势，或批准生产换装。此前要求用户先凭直觉选择基底再开始研究，倒置了取证顺序。本票已由 grilling 改为 research。

本轮完成候选比较、数学反例与一份公开报价文件的表示层观察。结果支持继续研究队列信息，也暴露了观察量、完整起始构造与递归资格之间的缺口。尚无候选获得完整唯一分解/递归闭合证明；没有调用缠论引擎，没有回测收益，本票保持开放。

## 1. 先拆清四个对象

- `E`：已收到的有类型事件序列。场所、顺序、时间、原始载荷和来源关系保留。
- `B_n`：从具名初始状态和前n个事件重建的订单簿状态。若输入只有报价行，不能冒称已重建逐订单簿。
- `φ(B_0,…,B_n)`：观察量或观察路径，例如报价中点、按队列数量加权的报价、订单流不平衡；它可以有意忽略部分维度。
- `F₁`：真正产生可递归最低层对象的构造。它还要定义分解、方向、区间、组成关系、完成与确认；一个φ公式本身没有完成这些义务。

`[新缠论:候选]` 一种完整候选路线是 `F₁ = F_price ∘ φ`，即先形成具名观察路径，再调用价格结构起始构造。另一种路线直接在有类型的订单簿演化上定义F₁。两条都需研究，不把前者的计算便利当成采纳理由。

**投影非单射不等于分解不唯一。** 两份订单簿投成同一个值，只能否定“这个值完整刻画订单簿”；它不能单独否定该投影路径存在唯一分解。研究必须指明究竟在对什么对象做完全分类。

同样，程序每次返回相同解析，仅证明确定性。要证明定义下的分解唯一，还须排除第二种同样满足定义的分解。

## 2. 可比较的候选

下表是研究分类，不是教义裁定。令最佳买卖价为b、a，对应数量为q_b、q_a。

| 候选 | 构造入口 | 固定报价时能否变化 | 当前主要缺项 |
|---|---|---|---|
| C0 新增委托价格串 | 按消息到达串接委托价 | 能，甚至远离盘口的订单也能造成起落 | 它描述委托报价到达顺序，不能直接当成成交/可执行价格路径；撤单等消息还需独立取值规则 |
| C1 报价价格路径 | `(b,a)`或中点`m=(b+a)/2` | 中点不能；固定b、a时两者都不动 | 作为价格对照；不承担队列数量结构的解释 |
| C2 数量加权报价 | `w=(a q_b+b q_a)/(q_b+q_a)` | 能，只要数量比例改变 | 只是价格单位的观察量，尚需F₁；不能按w假定成交；绝对深度、价差等信息可能被投影掉 |
| C3 订单流坐标 | 最佳报价处OFI增量及累计路径 | 能 | 坐标单位为数量；不能将其区间、波幅、止损距离直接当成货币价格 |
| C4 多变量订单簿演化 | 完整状态及有类型转移，包含数量、价位、原因 | 能 | 尚缺与既有F₂相容的方向、区间、连接、完成和唯一分解；自然的供需偏序会出现不可比状态 |
| C5 经估计的micro-price | 作者模型以中点、价差、不平衡状态作估计 | 可以 | 增加估计与训练的因果隔离义务；与C2的直接公式不同，不是现成缠论基底 |

C2的名称与公式核自 [Stoikov作者仓库的notebook](https://github.com/sstoikov/microprice/blob/4e5f29a843cb59f82bf3c64b0725642792048124/Microprice%20-%20Big%20Data%20Conference.ipynb) 的定义及 `get_df` 单元。作者分别定义weighted mid-price和micro-price，本文不将二者混名；本轮只读取notebook，没有执行其拟合代码。作者主页直链的SSRN全文本轮未成功打开，不宣称已通读该论文。

C3核自 [Cont、Kukanov、Stoikov原文§2.1](https://arxiv.org/html/1011.6402v3#S2.SS1)。其单事件贡献为：

\[
e_n=1_{b_n\ge b_{n-1}}q^b_n-1_{b_n\le b_{n-1}}q^b_{n-1}
    -1_{a_n\le a_{n-1}}q^a_n+1_{a_n\ge a_{n-1}}q^a_{n-1}.
\]

在b、a都不变的特例，直接化简为 `e_n=Δq_b−Δq_a`。论文明确指出，同量的市价卖出与买单撤销可能对该指标产生相同贡献。这个指标的经济解释不是缠论闭合证明；本轮也未将报价采样行之间的差值冒称完整逐事件OFI。

## 3. 可复核的反例与界限

以下为本轮推导或合成样例。它们各自只否定具名过强主张，不整包否定候选。

**Q1，数量脉冲可见。** 固定`b=100、a=100.02、q_a=100`，让`q_b=100→1000→100`。C1恒为100.01，C2为 `100.01 → 5501/55 ≈100.0181818 →100.01`；固定报价特例的累计OFI相对起点为`0→900→0`。这是观察量的起落，还不是已确认的缠论笔、段或中枢。

**Q2，绝对深度不可恢复。** `(q_b,q_a)=(100,100)`与`(1000,1000)`得到同一C2。它们的可见总量不同。更一般地，对正数λ，`w(b,a,λq_b,λq_a)=w(b,a,q_b,q_a)`。单靠w不能恢复绝对深度，但原始状态仍可独立保留。

**Q3，价差不可恢复。** 等量队列下，`(b,a)=(99.99,100.01)`与`(99.9,100.1)`都给出`w=100`，点差却不同。由此不能只凭w计算交易成本。

**Q4，事件原因不可恢复。** 在报价未动且剩余量仍为正时，撤去买一10单位与市价卖出消耗买一10单位可得到同一队列状态。C2和该步OFI相同，而有类型事件及成交流不同。保留事件类型有独立意义。

**Q5，终态不是路径。** 补单再撤回和完全不动，最终订单簿可以一样，中间路径不同。只保存末态不足以研究该段递归结构。采样报价也可能漏掉采样点之间的完整来回。

**Q6，自然偏序不是全序。** 若暂按“买量增、卖量减”定义供需方向，则状态`(100,100)`与`(200,200)`不可比。研究C4必须处理这类情形，不能声称向量已经天然拥有现有一维价格方向。这不证明所有多变量构造都不可能。

**Q7，指标价不是成交价。** Q1中，如果价格一直固定、假设只能主动买入卖出，一次买入100.02再卖出100的往返为`−0.02`，未计费用；w存在起落不改变这个计算。该反例不覆盖被动成交，也不证明C2没有预测用途。

**Q8，空侧与量化有定义责任。** q_b+q_a为0时C2无定义；只有单侧报价时，不能凭空补另一侧。把有理数w变成整数价格单位还需固定量化规则，不能无声抹平差异后宣称与精确值完全相同。

## 4. 公开报价样本上的有限观察

源：[作者公开的BAC报价文件](https://github.com/sstoikov/microprice/blob/4e5f29a843cb59f82bf3c64b0725642792048124/BAC_20110301_20110331.csv)。该仓库README称两份数据用于入门。源commit为 `4e5f29a843cb59f82bf3c64b0725642792048124`，文件16,534,767字节，SHA-256为 `8e683125053558399717971e80a7a6dcec8b4a29120b721d6e1710333162dea0`。

文件无表头，本轮保留第一行，按作者notebook中的字段顺序读取 `date,time,bid,bs,ask,as`；价格转成精确美分整数，数量保持源单位。数量是否以股或手计、场所覆盖和原始采样链未获得具名说明，本文不补猜。源日期字段包含22个不同值；全部同日相邻time差均为1。这是一份报价行序列，没有逐订单ID、事件类型或成交表。

本轮过滤条件：六列可解析、正数量、`0<bid<ask`、报价处于美分格点、源数量为整数。521,128行全部满足这些有限条件。这不等于完成行情完整性认证。相邻比较不跨日期；若某行无效会中断比较，不跨缺项拼接。

| 读数 | 计数 | 分母与含义 |
|---|---:|---|
| 报价行 | 521,128 | 本文件全部数据行 |
| 同日、时间严格递增的原相邻行对 | 521,106 | 下列行情观察的比较域 |
| 买一卖一价格均不变 | 502,272 | 上述521,106对中的子集 |
| 价格不变而至少一侧数量改变 | 447,111 | 上述521,106对的约85.80%，仅描述本文件 |
| 价格不变且精确w改变 | 447,100 | 447,111对中的子集 |
| 数量改变但w没有改变 | 11 | 与上一行合计447,111；反映比例投影的局限 |
| 若按0.01价格单位舍入，w变化被合并 | 425,139 | 447,100个精确变化中的子集，压力情形，不是当前引擎默认配置 |
| 若按1e-8价格单位舍入，w变化被合并 | 5 | 同一447,100对；精确有理数舍入结果，不是Rust浮点路径逐位验收 |

第一对实数来源为date=40603、time=34221→34222：买卖价14.33/14.34不变，数量`(8,468)→(20,471)`；精确w以美分计为`170529/119→703623/491`。

**本轮没有进行**：逐订单簿重建、撤单/成交原因识别、缠论解析、实时延迟测量、成交模拟、样本外预测或收益评价。有限观察说明报价静止时的数量信息在该文件中大量存在，不能据此外推所有市场、推断预测力，或把行对数量叫作撤单次数。

## 5. 与现有递归接口的关系

图工具先定位符号，再以本报告基线文件核行：`rust/src/theta_v0/types.rs:15` 为 `Tick=i64`，`:24`按传入tick_size量化，`:52`为价格OHLC `Bar`；`rust/src/theta_v0/config.rs:18–30` 的默认tick_size是`1e-8`，不是0.01。`rust/src/theta_v0/classifier/center.rs:83–92` 的 `UnitRange` 包含原始索引、Up/Down方向和价格lo/hi。

因此，C2并非因“包含小数”就必然需要重写价格类型；须先确定观察精度、溢出范围与量化影响。样本的0.01结果不能冒充当前默认引擎缺陷。C3或C4即使能编码成整数，也不自动取得价格量纲或一维区间语义。

`[新缠论:候选]` 一个可检验的条件命题是：若φ对已知前缀有确定定义，F_price在该输出域有唯一分解证明，而且其输出满足F₂的前提，则复合构造可以沿用那些已证性质。**本报告只给出这条证明路线，没有声称其前提已经全部核实。** 字段类型、能编译或返回非空结构，都不能代替证明。

原文与正本边界已核：[第84课第52、54、56行](../../docs/chanlun/text/blog/084-第84课.md#L52)允许起始构造变化；[中枢正本第134–158行](../definitions/zhongshu.md#L134)保留当前线段基底及合法备选的名分；[走势正本第443行](../definitions/zoushi.md#L443)要求各层同一递归判据。新订单构造没有因此自动获得生产资格。本轮不修改这些正本。

## 6. 由研究继续承担的义务

1. 固定两条候选路径的完整表达：C2经价格结构起始构造；C4直接从有类型队列演化构造。C1与C3为具名对照。先做低成本证据准备，不把这一路径安排记成生产选型。
2. 对每条写出输入有效域、全部分解规则、待完成/已完成状态、方向、区间、来源关系、确认时刻；查明现有F_price/F₂的实际证明域，逐条检查可迁移的前提。
3. 重复值、补撤往返、同时增厚、价差变化、单侧耗尽、快照恢复和修订分别给出确定结果或反例。还须明确观察时钟：每条消息取值、只在观察值变化时取值、周期采样会生成不同长度的路径。特别要核实重复观察是否影响既有构造中的间隔计数；本轮未作此断言或测试。网络重发与真实的无价格变化事件不能混为一类去重。有限实例验证不能替代一般性的唯一分解证明。
4. 建立数据充分性分层：C2的表示观察可用报价行；若问题涉及撤销和成交原因、队列身份或精确执行，就需相应事件字段。不能把所有研究一刀切成“非L3不可”，也不能把L1样本升级为逐订单实证。
5. 在明确候选之后设计真正的递归增量实验。当前统计只验证输入信息和表示差异，不能用来关闭增量价值票或生产架构票。

需要用户决定的仍是证据无法替代的目标优先级、预算和生产采纳；这些问题应附候选证据与代价再提出。本轮不再索要“队列变化是否合法”的直觉裁决。

## 复现

下附标准库观察程序。只下载上述固定commit的公开文件到所选临时目录，核SHA后计算；不执行第三方notebook，不调用项目引擎。把程序保存为临时目录中的`observe.py`后运行 `python3 observe.py`，结果写到同目录`result.json`。

```python
import csv, hashlib, io, json, urllib.request
from collections import Counter
from fractions import Fraction as F
from pathlib import Path

COMMIT = "4e5f29a843cb59f82bf3c64b0725642792048124"
NAME = "BAC_20110301_20110331.csv"
URL = f"https://raw.githubusercontent.com/sstoikov/microprice/{COMMIT}/{NAME}"
root = Path(__file__).resolve().parent
path = root / NAME
if not path.exists():
    with urllib.request.urlopen(URL, timeout=40) as response:
        raw = response.read(20_000_001)
    if len(raw) > 20_000_000:
        raise RuntimeError("source exceeds 20 MB cap")
    path.write_bytes(raw)
raw = path.read_bytes()
EXPECTED_SHA256 = "8e683125053558399717971e80a7a6dcec8b4a29120b721d6e1710333162dea0"
if hashlib.sha256(raw).hexdigest() != EXPECTED_SHA256:
    raise RuntimeError("source hash mismatch")
counts = Counter()
deltas = Counter()
previous = None
examples = []
dates = set()

def round_positive(num, den):
    return (2*num + den) // (2*den)

for row in csv.reader(io.StringIO(raw.decode(), newline=""), delimiter=','):
    counts['rows'] += 1
    if len(row) != 6:
        counts['bad_columns'] += 1
        previous = None
        continue
    try:
        date, time, bid, qb, ask, qa = map(F, row)
    except (ValueError, ZeroDivisionError):
        counts['bad_number'] += 1
        previous = None
        continue
    if bid <= 0 or ask <= bid or qb <= 0 or qa <= 0:
        counts['invalid_book'] += 1
        previous = None
        continue
    b, a = bid*100, ask*100
    if any(v.denominator != 1 for v in (b, a, qb, qa)):
        counts['not_cent_or_integer_size'] += 1
        previous = None
        continue
    b,a,qb,qa = map(int,(b,a,qb,qa))
    num,den = a*qb+b*qa,qb+qa  # weighted midpoint in cents
    quant = round_positive(num,den)  # exact rational nearest-cent, positive ties up
    quant_fine = round_positive(num*1_000_000,den)
    counts['valid_rows'] += 1
    counts['weighted_off_cent_grid'] += (num % den != 0)
    counts['weighted_off_1e8_grid'] += (num*1_000_000 % den != 0)
    dates.add(str(date))
    current = (date,time,b,a,qb,qa,num,den,quant,quant_fine)
    if previous is not None and previous[0] == date:
        dt=time-previous[1]
        deltas[str(dt)] += 1
        if dt <= 0:
            counts['nonincreasing_time_pairs'] += 1
        else:
            counts['same_day_increasing_adjacent_pairs'] += 1
            stable=(b,a)==previous[2:4]
            changed=(qb,qa)!=previous[4:6]
            weighted_changed=num*previous[7] != previous[6]*den
            if stable:
                counts['stable_quote_pairs'] += 1
                if changed:
                    counts['stable_quote_size_changed_pairs'] += 1
                    if weighted_changed:
                        counts['stable_quote_weighted_changed_pairs'] += 1
                        if quant_fine == previous[9]:
                            counts['stable_quote_weighted_change_erased_by_1e8_rounding'] += 1
                        if quant == previous[8]:
                            counts['stable_quote_weighted_change_erased_by_cent_rounding'] += 1
                    else:
                        counts['stable_quote_size_change_invisible_to_weighted'] += 1
                    if len(examples)<3:
                        examples.append({'date_code':str(date),'time':str(time),
                         'bid_cents':b,'ask_cents':a,'previous_sizes':previous[4:6],
                         'sizes':[qb,qa],'previous_weighted_cents':str(F(previous[6],previous[7])),
                         'weighted_cents':str(F(num,den))})
    previous=current

# Independent algebraic checks and synthetic counterexamples, not Chan parsing.
def w(b,a,qb,qa): return (a*qb+b*qa)/(qb+qa)
b,a=F(100),F('100.02')
synthetic={
 'fixed_quotes_pulse':[str(w(b,a,F(q),F(100))) for q in (100,1000,100)],
 'scale_collision':[str(w(b,a,F(q),F(q))) for q in (100,1000)],
 'same_weighted_different_spread':[str(w(F('99.99'),F('100.01'),F(100),F(100))),str(w(F('99.9'),F('100.1'),F(100),F(100)))],
 'fixed_quote_ofi_pulse':[0,900,0],
 'aggressive_roundtrip_fixed_quotes':str(b-a),
}
assert synthetic['scale_collision'][0] == synthetic['scale_collision'][1]
assert synthetic['same_weighted_different_spread'][0] == synthetic['same_weighted_different_spread'][1]
assert len(set(synthetic['fixed_quotes_pulse'])) == 2
assert counts['rows'] == sum(counts[k] for k in ['bad_columns','bad_number','invalid_book','not_cent_or_integer_size','valid_rows'])
assert counts['stable_quote_size_changed_pairs'] == counts['stable_quote_weighted_changed_pairs'] + counts['stable_quote_size_change_invisible_to_weighted']
result={'source_commit':COMMIT,'url':URL,'bytes':len(raw),'sha256':hashlib.sha256(raw).hexdigest(),
 'counts':dict(counts),'day_count':len(dates),'date_codes':sorted(dates),
 'within_day_time_deltas':dict(deltas),'examples':examples,'synthetic':synthetic,
 'scope':'quote-row observation and exact arithmetic only; no trades/order IDs/event types, Chan parser, signals or profits tested'}
(root/'result.json').write_text(json.dumps(result,ensure_ascii=False,indent=2))
print(json.dumps(result,ensure_ascii=False,indent=2))
```
