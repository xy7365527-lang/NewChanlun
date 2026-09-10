# #1371 TB-01-B：R11 请求绑定修复与交棒报告

> 编号声明：本文 #1371、#1340、#1323 均指 GitHub Issue。
> 原作者实施报告，不是独立评审。R5 **FAIL（R5-M1，M）** 历史原件不改。
> 已批准范围仍是 SPEC #1340 的 TB-01-B；不表示 B、父 TB 或 #1323 六 Destination 完成。

## 1. 当前提交与限定范围

- 冻结父提交：`a60fdd57e5ebb0e7d6a15b7e1f30994720269ed8`；进入时工作树干净，沿用唯一分支 `codex/1323-tb01-b-1371`。
- 本轮代码提交：`6a4e69e60ffd1baca490f870622c098a14fe7b41`。
- 只改 `s_session/browser/index.html`、`s_session/tests/r9_consumer.cjs`，新报告只用 `implementation-r11.md/json`。
- Rust、Python server、launcher、r9_contract.py、原roster、全部旧实施报告和R1–R5十份独评的字节/Git blob/mode保持。
- **R5-M1已实施并取得下文有限自证；根真实GUI、原生剩余面及原独评复评仍待接回。** 没有push、merge、main/GH写、harvest、全局队列/服务/模型调整或经济外效，没有启动C。

### 超时后的本次续接

上轮已保存代码提交及两份报告草稿，但尚未正常交回。用户本次告知：SDK关闭后根终止滞留wrapper，外层实际退出 **143**；不记成自然超时exit1或作者正常完成。本次从冻结代码与两份未提交稿接续，只核自包含档案、补中断记录并提交报告，**没有重新建立HTTP库或重跑同hash验证**。

本次资料核对：14附件逐长度/SHA通过；三个原反例红灯、28项绿灯、55份真实源HTTP及stream/Gap完整热态保持/健康重试读数、8AC/17引用均已核。下文“本轮实跑”指此前R11已保存的实际执行，不冒称本次续接又跑一遍。原临时路径不作存活承诺。

## 2. 共同绑定与证据等级

每个AC/来源子义务共用本节及JSON `binding`，再按各自证据键区分本轮实跑、旧证据复用和NOT_RUN。

规范清单SHA256：`38e3176b6dfff456b8e3704410401c3bedddf0671751464bb9ea5b829e460260`；六份指定payload已读并逐SHA核存。规则为 `s2-axis-quantifiers`；目录116项和TestOnly profile未改。目录文件SHA为 `938b0ef59282e689c114cdcb211e2709c86e64c618e4ec0573862bda43508069`，profile文件SHA为 `2cd50e43e659dc87498eebba76d88deb4cde276cd51c47fa0c083b07dffbdcfc`。

仍只用逐笔一对一精确退化OHLC、严格无包含、前两点定向、无同价竞争的CC-006域。真实S原始输入依次为10000、11000、10500、第三事件新revision改11500、追加10800，之后两次大revision；收到时间固定，逐input/payload/hash与实际argv见 `source-commands.json`。不造前端结构事实，不修改经济政策或未决G/FU。

本轮Linux aarch64，项目Python3.11.16/HTTP SQLite3.53.1。仅为取得真实HTTP小轨迹重建未变S二进制：SHA256 `ee39fb99e4d5f7d4f9a47aefbb7979ed7a384858dec48149a9794cbefb952842`，与冻结R10/R5 Linux二进制相同。不重新跑35测试、存储大矩阵、三杀或旧writer专项，也不把旧计数写成本轮新跑。

| 文件 | 最终SHA256 |
|---|---|
| `s_session/browser/index.html` | `a04ddb1c67e26207cbc9ec4bd72f7cb36fcac30accf4c3f1a49e5ee10e5de94b` |
| `s_session/tests/r9_consumer.cjs` | `c375406cf09a092d451c7b09050fa3eda378b3087ee96d8c033edcc1842d119d` |
| `rust/src/bin/s_structure_session.rs` | `9450f2a9642be70dc5a18f7044fa07312d81d126e25d512e50921105a1f25901` |
| `s_session/s_readonly_server.py` | `9dad379ff1aaa1ff6f9315a877249f4b9003aa39c7d48fdba8016e534c27c65c` |
| `s_session/launch_s.sh` | `65cb1552594dea36f1368d952c0bbc839418d06cab107b0ca8ce594e3a5b7d1d` |
| `s_session/tests/r9_contract.py` | `89eeaff7430a098c4abaf239b71f0d684da6dd1f53038dba9d2f84667cdadc2f` |

本轮全文读正式R5报告，并解码校验其14份附件的长度/SHA；只读正式具名材料，未读独评私有会话/raw/prompt或未挂载宿主文件。

证据名分：**实际S/HTTP**；**实际HTML函数+模拟DOM/TestOnly输送**；**同hash/根给定结果有限复用**；**NOT_RUN**。后三者不得改称真实GUI、TCP故障或本轮独立原生实跑。

## 3. 修复：先证明“属于这次请求”，再显示/提交

### 请求描述与统一验证

新增 `stateRequest` 一次产生URL与期望模式/截止点；`validateRequestedState` **先保留全部原内部状态校验**，再核请求对应。四条读取边界均使用它，不将其中一条换成宽松门：

| 调用 | 必须满足 |
|---|---|
| `load("asof", N)` | 请求前N为规范i64十进制文本；响应为AsKnown，generation/as_of_generation均严格等于N，structure_cut等于cut-N |
| `load("current")` | 响应必须RecomputedWithRevision且as_of_generation为null；**不预定generation，不新增新鲜度/nonce/密码或默认政策** |
| `applyStream` 的权威 `as_of=tmpGen` | 准确AsKnown tmpGen，通过后才进入已有完整publication/六集合对拍与提交 |
| Gap的 `rebuild_cut` 请求 | 准确AsKnown截止点，包含i64范围核验；通过后才能重建绑定/游标/六集合并显示成功 |

前导零、加号、负号、空值、越域或JS number作为历史截止点，均在发HTTP前明确拒绝；复用原 `decimal` 的严格文本规则，不经Number或宽松BigInt归一化。UI既有输入trim仍在请求前，未放宽前导零策略。

### 成功与失败边界

- `pageEpoch` 检查仍在异步响应后、任何本次成功提交前；旧成功/旧失败均不得覆盖新请求。
- `validateState / validateSource / validateDelta / comparePublication / deepClone / canonicalFieldCompare / cmpMaps` 正文未变；不删除profile或六Map核验来过新测试。
- **load** 保持其原失败语义：清currentState/显示并报错；模式/浏览范围标为本次请求失败，不显示本次成功。没有把stream热态保持要求强加到所有load。
- **stream/Gap** 拒绝时，currentState、cursor、binding、historyMode及六Map完整保持，只改错误提示等诊断UI。为落实这一明确要求，会话不匹配拒绝分支也不再调用 `resetCommitted`；保留原切面并提示刷新页面或合法Gap重建，不隐式接受新会话或引入新session协议。

实际位置：
- `s_session/browser/index.html:415`：`stateRequest`。
- `s_session/browser/index.html:421`：`validateRequestedState`。
- `s_session/browser/index.html:445`：`load`。
- `s_session/browser/index.html:507`：`applyStream`。

## 4. 红灯：固定R10 HTML上先重现

`html-red.html` 的SHA为 `9c3d2c3703b9e765f3d65e2cd76759ff5716e83781fd49c6ec5457aad8565498`，与冻结R10 HTML逐字一致。运行同一真实小库上的新请求合同锁，命令exit **1**；这是明确合同断言红灯，不是用探针完成代替实现结论。

| 原反例 | 修改前实际结果 | 修改后 |
|---|---|---|
| `load(asof,3)`重送完整真实AsKnown4 | 接纳currentState/render为cut4；cursor仍0 | 冷/暖态均拒绝；清当前显示并报错；同页改回健康asof3后成功 |
| `load(current)`重送完整真实AsKnown3 | 接纳历史状态，却标当前；cursor仍0 | 冷/暖态均拒绝；同页健康current重试成功 |
| stream请求asof4，响应改成合法Recomputed/null模式对 | cursor3→4，显示成功 | cursor/currentState/historyMode/binding/六Map全保持3；同页健康重试后提交4 |

前两项**不是提交流游标的证据**；第三项明确改两个模式字段，不冒称单字段反例。错误历史截止点直接使用另一份完整真实状态，不破坏内部字段制造无关拒绝。健康新合同current/AsKnown/stream控制及单profile_hash负例也在红灯阶段通过。

另新增Gap合法错模式对的同类覆盖：旧HTML可接受，新HTML拒绝。**这不是说R5原附件已跑过Gap模式反例。** 完整红灯原始/交付State、逻辑状态、UI字符串与失败断言在 `red-results.json`；对应当时测试源也已保存。

## 5. 绿灯与原有消费者锁

新锁 **28项全部通过**，最终源HTTP记录55份、状态全部200；原始body/hash、请求路径、交付前后完整JSON、差异路径、before/after/retry的完整页面逻辑状态及可用UI字符串全部保存。

- 健康 current、AsKnown0、AsKnown3、stream3→4、合法Gap同cut重建成功。
- 两个load原反例分别覆盖冷态/已有显示的暖态，失败后同一页面上下文健康重试成功。
- stream合法错模式对、另一完整错cut、单profile_hash、受控响应读取失败、会话不匹配均拒绝；Gap的错模式对/完整错cut/受控读取失败均拒绝。热状态逐字段不变，随后同页健康输送重试成功。
- 7种非规范历史输入及未知mode在HTTP发出前拒绝；另以**真实早先current响应**证明current没有新增generation新鲜度限制。
- 晚到成功与晚到失败均不覆盖获胜current。获胜响应和早先current都来自真正的 `/api/state` 正文，不是把AsKnown的两个模式字段手改成当前。
- 现有R9、R10消费者测试均运行通过。约原117–119行的R9晚响应测试保留原generation断言，改用实际current来源，并加模式/null断言；没有删掉隔离断言。

### TestOnly输送准确名分

本轮源库已到gen7，stream健康/负例共用**信封和列表一致裁至gen4**的TestOnly交付。不是说正常HTTP会自行返回错数据，也不将本轮gen7调度冒充R5原附件的gen5调度。

Gap使用服务对 `after_generation=99` 返回的**真实合法Gap消息**，在TestOnly输送中重送给当前消费者；后续请求准确rebuild_cut。原消息、原请求与交付包均记录。此处验证消费者请求cut/mode与原子保持，不宣称实际TCP断连、后台retention或整个C压力矩阵。

`request_failure` 是取得合法源响应后受控抛出读取错误；非法输入测试则检查“未发请求”。这些不是三个M1特异反例的替代依据，也不计成真实网络故障。

## 6. 实际检查/退出码与复跑入口

| 检查 | exit | 完整记录 |
|---|---:|---|
| 未变后端 `cargo build --locked --features s_session --bin s_structure_session --jobs 1` | 0 | `commands.json#/0`，仅供本轮HTTP来源 |
| 固定R10 HTML上的合同红灯锁 | 1 | `commands.json#/2`、`red-results.json` |
| `node --check s_session/tests/r9_consumer.cjs` | 0 | `commands.json#/4` |
| 最终HTML提取script的 `node --check` | 0 | `commands.json#/5` |
| 最终R11新锁（28项） | 0 | `commands.json#/6`、`green-results.json` |
| 现有R9消费者锁 | 0 | `commands.json#/7`、`r9-results.json` |
| 现有R10实际HTTP消费者锁 | 0 | `commands.json#/8`、`r10-results.json` |

实际argv/cwd/stdout/stderr由外层调用和内层Node执行分别保存，真实exit不是管道末级值。未运行Rust全量test/fmt/check/clippy：本轮没有Rust/Python/launcher改动，复用已完成同hash检查，而不是机械重复。既存Rust warnings及Node环境NO_COLOR警告原样保留，不称全仓零warning。

实际原生环境变量见JSON binding；本轮只读服务终止前核具体PID/可执行命令/DB归属，由父进程wait取得退出。该清理是readonly服务收尾，**不是S三点SIGKILL**。

## 7. 原票8AC

全部仍保留，公共绑定见第2节。仅影响面新跑，其余明确复用/待验，不缩票或填整票PASS。

| AC | 当前证据与未完边界 | 证据键 |
|---|---|---|
| AC1 | **后端有限证据复用；新源小轨迹实跑**。未变S核init/accept/advance产生七cut：单点/两点/旧TOP/更正RISING/追加TOP/两次大修订；本轮不是新的launcher或三杀验收，正式launcher既有成功沿R5原范围引用。 | HTTP-SOURCE, BACKEND-REUSE |
| AC2 | **未变后端有限复用，消费者profile负例保持**。版本/首获知/固定profile后端未动；新源完整payload含原身份与见证，profile单字段矛盾拒绝不回退。 | HTTP-SOURCE, R11-GREEN, R9-R10-TESTS, BACKEND-REUSE |
| AC3 | **请求绑定新锁通过，真实GUI待根**。load/current/asof及stream/Gap均核本次请求；健康提交、非法不提交、同页面重试成功，六Map完整保持或按同cut提交。 | R11-RED, R11-GREEN |
| AC4 | **未变后端既有有限证据；原生after/GUI待根**。本轮未发S SIGKILL、不借readonly清理代替三杀；原R10作者数据、R5摘要及根crash-before各按原证据强度保留。 | BACKEND-REUSE, ROOT-SUPPLIED |
| AC5 | **未变后端有限复用**。原IDQuery/幂等/epoch逻辑均未改；不重新铸造身份、资格或经济动作，未重复owner/恢复矩阵。 | BACKEND-REUSE |
| AC6 | **消费者请求对应有界通过；GUI待根**。AsKnown0/3、current、另一完整AsKnown误投、合法错模式对均新跑；当前不预定generation，不引入新鲜度策略。后端cut0保持已修。 | R11-GREEN, ROOT-SUPPLIED |
| AC7 | **未变S前件有限复用；消费者失败呈现新跑**。S存储/profile拒绝未改、不重新计数旧矩阵；load清显示报错，stream/Gap错误不污染热状态；本轮仅S/readonly，没有E/B/X。 | R11-GREEN, BACKEND-REUSE |
| AC8 | **本轮完整附件已保存，未等交付批准**。原始HTTP/输入/argv/exit、完整交付前后/逻辑缓存/UI字符串、红绿源码与hash自包含；Node/GUI与资料核对不混。 | ARTIFACTS |

## 8. 17来源子义务

JSON逐项保留signed_pointer/required_result；只承接本片结构子义务，不销完整来源ID的跨域量词。

| ID | 本轮结果与证据限制 | 证据键 |
|---|---|---|
| A-CC-006-03 | **后端数据有限复用**。同核/输入/历史恢复数据沿原范围保留；本轮无新真杀原始证据，不把R5遗失raw重构为独立原件。 | BACKEND-REUSE |
| A-CC-006-04 | **新源数据/HTML函数实跑；GUI未验**。旧TOP→更正RISING→新TOP和三根见证/比较来自S；页面函数渲染字符串已记录，不等实际DOM截图。 | HTTP-SOURCE, R11-GREEN |
| A-ST-044-02 | **消费者有界修复**。持久原子载荷逻辑未动；指定请求模式/cut前置于完整内部/六集合验证与原子提交。 | R11-GREEN, BACKEND-REUSE |
| A-ST-044-03 | **协议有界；C/真实网络未验**。stream/Gap指定cut重建、错误保持热态及同页重试已跑；Gap为真实源消息TestOnly重送，非实际TCP断连。 | R11-GREEN |
| A-ES-15-01 | **仅S子域**。本轮数据源只启S/readonly，economic not_started保持；不将结构frontier当经济ack。 | HTTP-SOURCE, BACKEND-REUSE |
| A-ES-15-02 | **后端有限复用**。Begin/batch/Commit/epoch未改；不把本轮读取服务清理当恢复迁移验收。 | BACKEND-REUSE |
| A-RA-04-01 | **仅S有限域**。未改旧TOP身份/first_known/撤回史；历史响应不再被请求错误切面接受。经济责任未销。 | HTTP-SOURCE, R11-GREEN |
| A-RA-10-01 | **消费者有界修复**。明确请求AsKnown N必须对应N，当前只接受Recomputed/null；完整经济历史未验。 | R11-GREEN |
| A-I-02-01 | **后端有限复用，UI负例新跑**。未动存储拒绝路径；本轮模拟响应读取失败不当S/经济故障乘积试验。 | R11-GREEN, BACKEND-REUSE |
| A-OB-004-01 | **未变数据有限复用**。源/对象/结构版本与profile字段保持；现有R9大整数及R10profile消费者锁通过，不改历史字节。 | R9-R10-TESTS, BACKEND-REUSE |
| A-OB-008-01 | **请求/响应绑定新锁通过**。三个原反例在R10 HTML先复现，修后特异拒绝；健康完整状态成功，重复/Gap/旧响应旧锁保留。 | R11-RED, R11-GREEN, R9-R10-TESTS |
| A-OB-009-01 | **后端有限复用/消费者正常历史新跑**。缺右邻和cut0后端未变；AsKnown0/3真实载荷成功，错cut整体替换不能回填本次请求。 | HTTP-SOURCE, R11-GREEN |
| A-OB-014-01 | **经济部分NOT_RUN**。没有经济事件/计划/可能已发/成交实例，不用空域宣称通过。 | BACKEND-REUSE |
| A-OB-015-01 | **请求绑定有界修复**。期望mode/cut与pageEpoch共同约束；晚到成功/失败不覆盖真正current获胜请求；session不匹配拒绝也保留热态。 | R11-GREEN, R9-R10-TESTS |
| A-OB-016-01 | **函数/协议有界，GUI及原生after待根**。stream/Gap拒绝后的同页健康重试和完整缓存对拍通过；不冒称真恢复页面已验。 | R11-GREEN, ROOT-SUPPLIED |
| A-OB-017-01 | **S数据有限复用，切面绑定新跑**。不归一化语义身份/获知时间；本轮源gen7的TestOnly输送裁到gen4明确记录，不说正常HTTP自动错投。 | HTTP-SOURCE, R11-GREEN |
| A-OB-020-01 | **本轮完整原始材料归档**。14附件带bytes/SHA/可恢复正文，原数据/输送/缓存/UI/argv/exit完整；R5旧独立raw遗失如实保留。 | ARTIFACTS |

## 9. 旧证据与最新根状态

- A严格核/目录/profile及Rust/Python全部未动。原R10自包含作者证据保持作者名分；R5超时前独立/tmp原始矩阵已丢，只能按正式报告保存的捕获摘要强度援引，不能拿作者文件冒充独立原件。
- 根最新R11反馈已说明：R10原生build/profile/cut0/旧writer整数专项，以及crash-before21路既存原始证据核对已有结果。按未变源码和该明确强度有限复用；这是**根给定完成范围，不是本容器重新读取宿主原件或亲跑macOS**。
- R5报告中的较早“专项待跑”文字保持历史原样，不再当作最新状态。真实GUI、原生三库recovery/crash-after、TestOnly GUI仍待根完成，不能从完成的build/before数据推成after/GUI通过。
- 本轮没有重开R4-H1/profile存储或后端cut0反例；M1只补当前响应与当前请求的对应。R3及R4既有修复按原有限证据保留，不写“所有义务都已闭合”。

明确NOT_RUN/未销项：

- 最终HTML的真实GUI/DOM及真实TCP断连/重送：NOT_RUN。只有实际HTML函数、真实HTTP源、模拟DOM与TestOnly输送；容器未发现浏览器可执行文件。
- 原生三库recovery/crash-after、TestOnly GUI仍待根完成。根最新R11反馈已说明R10原生build/profile/cut0/旧writer整数专项及crash-before 21路材料核对已有结果；仅按未变后端和该明确强度复用，不把R5较早“待跑”文字作为当前事实，也不冒称本容器重跑。
- 原独评对本轮提交的复评与根验待接回；R5 FAIL原件不改，作者不能改判独评PASS。
- 未重跑Rust35测试、后端损坏大矩阵/三杀/两活writer/旧writer专项。后端/launcher/r9_contract.py完全同hash，本轮只重建本平台binary以取得真实HTTP小轨迹；既有证据限定复用。
- R5超时前独立/tmp原始矩阵已遗失，只能按正式报告记录的捕获摘要强度引用；作者R10自包含附件不能冒称那些独立原件。
- C1372分页/保留/过期/慢消费/长历史、全调度、ST044高级扩展、其余CC/G/FU/非退化市场、经济责任/计划/可能已发/成交、完整经济历史/全经济故障分区均未销项；B/父TB/#1323及main合入未获批准。

## 10. 自包含材料与根接回

14份文本附件嵌入 `implementation-r11.json.evidence_artifacts`，每份带bytes/SHA及可恢复正文。包括本轮输入/CLI命令、完整HTTP、红绿交付/缓存/UI结果、现有消费者结果、当时红灯HTML/测试源与临时驱动。没有DB、二进制、会话导出、prompt或密钥；旧/tmp路径只是当时实际来源，不承诺容器结束后仍存在。

```python
import base64, hashlib, json, lzma, pathlib, tempfile
r = json.loads(pathlib.Path('.chanlun/review-results/issue1371-tb01-b/implementation-r11.json').read_text())
out = pathlib.Path(tempfile.mkdtemp(prefix='issue1371-r11-evidence-'))
for name, a in r['evidence_artifacts'].items():
    assert pathlib.Path(name).name == name and a['encoding'] == 'xz+base64'
    b = lzma.decompress(base64.b64decode(a['data']))
    assert len(b) == a['bytes'] and hashlib.sha256(b).hexdigest() == a['sha256']
    (out / name).write_bytes(b)
print(out)
```

本平台重跑仅需：项目Python运行附件 `driver.py <新空私有目录> init` 建一次小正常库并取得真实current/早先current；再运行 `driver.py <同目录> green-final`。它调用既有消费者路径的 `--r11` 模式，启动/清理自有只读服务。带 `/proc` 的驱动是Linux取证面，不冒称原样适配macOS。不要在已存在库上重复init；后端大矩阵不由本脚本展开。

现有R9模式需要 `api-cuts.json` 中**真正current和earlier_current**，不能用历史响应改模式字段补造；另需从R3正式报告按其格式恢复的 `probe-results.json`（旧schema拒绝锁不作M1特异证明）。现有R10模式通过附件 `r10-driver.py` 使用同一小库真实HTTP。

根接回只需关注本轮HTML影响面：真实页面健康current/AsKnown0/非零历史、三原反例及Gap错模式/错cut，冷暖load失败呈现、stream/Gap完整热态保留与同页重试，以及真current获胜的晚响应隔离。再完成已在进行的原生after/GUI、交原独评会话复审。不得用作者28项通过代替独评或未完成的真实GUI。

**COMPLETE只表示原作者代码/报告交回，不表示独评PASS、GUI/原生验收、main批准或B/父TB/#1323完成。**
