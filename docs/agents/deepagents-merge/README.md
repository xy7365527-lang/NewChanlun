# 把已验收的 DeepAgents run 归并到本地组合分支

输入是两份或多份已通过单票验收、绑定冻结 SHA 的 DeepAgents run 产物。
现有公共入口把这些产物按清单顺序归并到本地 `sandcastle/merge-<merge-run-id>` 分支。
首跑和中断恢复使用同一份清单、同一条命令。

## 这份指南不做的事

归并分支只在本地保留。
本流程不推送、不强推、不合 main、不关来源票，也不替代单票验收。
Docker 冲突路径、生产冻结、生产切换与 main 合入的真实执行均不在本指南的验收结论内。

## 确认 run 可以列入 accepted 清单

对每份来源 run 完成以下四项核对后，再生成 accepted 输入。
`awaiting_merge` 只表示执行器完成交付，不等于主控已经验收。

| 前置 | 读取器强制什么 | 主控另行核对什么 |
| --- | --- | --- |
| run 在冻结 head 上处于 `awaiting_merge` | 交付 phase、head、分支和 PR 记录的 head 必须匹配输入。存在 stale 标记或未认回的交付效果时拒绝 | 确认选择的是本次要归并的 run 与冻结 head |
| 冻结 head 有异家族精确 PASS | `deepagents/review` 状态记录必须为该 SHA 的 `done`。review 回执必须为 `completed`、模式为 `review`，`expected_head` 和 `checkout_after.head` 都等于该 SHA。回执结果必须为 `completed`，summary 首行必须逐字为 `VERDICT: PASS` | 核对评审者确属异家族。读取器不核验家族身份 |
| 该 head 的必需 CI 已通过 | 不查询必需 CI，也不读取验证证据正文 | 核对所有必需 CI 通过且绑定同一个冻结 head |
| 单票验收记录已写 | `acceptance.head` 必须等于输入 SHA，`status` 必须为 `accepted`。两个 evidence 字段必须是非空字符串。DeepAgents 的独审指针还须等于固定回执路径 | 写好该 head 的单票验收记录。把记录指针交给 `verificationEvidence`。读取器不核验记录正文 |

按 [EXECUTOR.md](../../../.sandcastle/deepagents-pilot/EXECUTOR.md) 准备 `executor` 命令后，查看每份来源 run 的本机 phase。
使用该文档的 `status --run-id` 接口，把值替换为实际来源 run-id。

```sh
executor status --run-id <source-run-id-a>
executor status --run-id <source-run-id-b>
```

`status` 只读本机记录，不访问 GitHub。
远端事实由归并入口在运行时重新核对。

## 准备归并环境

在仓库根执行本指南的命令。
把 `<repository-root>` 替换为归并宿主仓库的绝对路径。
把 `<fixed-base-sha>` 固定为组合分支基线的 40 位小写十六进制 SHA。
把每个 `<frozen-sha-a>`、`<frozen-sha-b>` 替换为对应来源的 40 位小写十六进制冻结 SHA。

确认本仓已有基线与输入提交对象。
本仓若存在与来源分支同名的本地 ref 或远端跟踪 ref，它们都必须等于该来源的冻结 SHA。
fetch 后也按此条件核对。
不要为通过准入而推送或重置来源 ref。

为实时远端读取准备以下环境变量。
私钥只提供路径。

| 环境变量 | 填入什么 |
| --- | --- |
| `DEEPAGENTS_PYTHON` | `<absolute-python-executable>`，可运行远端事实读取器的 Python 可执行文件绝对路径 |
| `DEEPAGENTS_PILOT_PATH` | `<absolute-pilot-package-root>`，对应 Python 包根目录的绝对路径 |
| `PILOT1742_APP_PRIVATE_KEY` | `<absolute-app-private-key-path>`，App 私钥文件的绝对路径 |

Python 环境与执行器准备沿用 [EXECUTOR.md](../../../.sandcastle/deepagents-pilot/EXECUTOR.md)。
本指南不复制那里的机器路径。

## 生成并冻结 accepted 清单

选择新的 `<merge-run-id>`。
把 `<merge-manifest-root>` 设置为仓库之外的绝对路径。
本指南把清单放在 `<merge-manifest-root>/<merge-run-id>/accepted-manifest.json`。
清单必须位于归并工位之外，入口会拒绝工位内的清单。
本指南还要求清单位于仓库之外，不放进 `.sandcastle/logs` 等宿主受管状态目录。

把 `<executor-state-root>/runs/<source-run-id-a>` 和对应的 b 路径替换为来源 run 目录的绝对路径。
把 `<verification-record-a>` 和 `<verification-record-b>` 替换为已写好的单票验收记录指针。
所有占位参数替换完成后，在仓库根执行以下代码。

```sh
npx tsx --input-type=module <<'TS'
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { deepAgentsMergeInput } from './.sandcastle/merge-source.mts';

const runId = '<merge-run-id>';
const manifestDir = '<merge-manifest-root>/<merge-run-id>';
const manifest = {
  runId,
  branch: `sandcastle/merge-${runId}`,
  base: '<fixed-base-sha>',
  inputs: [
    deepAgentsMergeInput(
      '<executor-state-root>/runs/<source-run-id-a>',
      '<frozen-sha-a>',
      '<verification-record-a>',
    ),
    deepAgentsMergeInput(
      '<executor-state-root>/runs/<source-run-id-b>',
      '<frozen-sha-b>',
      '<verification-record-b>',
    ),
  ],
};
mkdirSync(manifestDir, { recursive: true });
writeFileSync(
  join(manifestDir, 'accepted-manifest.json'),
  JSON.stringify(manifest, null, 2) + '\n',
  { flag: 'wx' },
);
TS
```

更多来源就在 `inputs` 中追加 `deepAgentsMergeInput` 调用。
数组顺序就是归并顺序。
`wx` 是本指南防止覆盖已有清单的约定，不是归并入口合同。
生成器只读本地 run 记录，不代替归并运行时的实时 ref 和 PR 复核。
不要手写 DeepAgents 输入，也不要在清单里添加 `admission` 字段或 PR 编号。

清单顶层形状为 `{ runId, branch, base, inputs }`，本例的 `inputs` 是 `DeepAgentsMergeInput[]`。
生成后的字段按下表核对。

| 字段 | 谁填 | 含义 |
| --- | --- | --- |
| `runId` | 主控 | 本次组合归并的标识，与来源 run-id 分开 |
| `branch` | 主控示例代码 | 固定为 `sandcastle/merge-<merge-run-id>` |
| `base` | 主控 | 固定基线 SHA |
| `inputs` | 主控排列 | 有序的固定 SHA 输入数组 |
| `inputs[].source` | `deepAgentsMergeInput` | 固定为 `deepagents` |
| `inputs[].issue` | 生成器从 run 记录读取 | 来源票号，必须为正的安全整数 |
| `inputs[].branch` | 生成器从交付记录读取 | 固定为 `codex/deepagents/t<票号>` |
| `inputs[].sha` | 主控传给生成器 | 来源冻结 SHA |
| `inputs[].run` | 生成器 | 形状为 `{ id, generation, dir }` |
| `inputs[].run.id` | 生成器从 run 记录读取 | 形如 `t<票号>-<正整数>`，票号须匹配 `issue` |
| `inputs[].run.generation` | 生成器从 claim 记录读取 | 正的安全整数代次，运行时须与记录一致 |
| `inputs[].run.dir` | 主控传给生成器 | 来源 run 的绝对路径，目录 basename 必须等于 `run.id` |
| `inputs[].acceptance` | 生成器 | 形状为 `{ head, status, reviewEvidence, verificationEvidence }` |
| `inputs[].acceptance.head` | 生成器 | 必须等于该输入的 `sha` |
| `inputs[].acceptance.status` | 生成器 | 固定为 `accepted`，前提是主控已完成单票验收 |
| `inputs[].acceptance.reviewEvidence` | 生成器 | 固定为 `<executor-state-root>/runs/<source-run-id>/workers/review-<frozen-sha>/result.json` |
| `inputs[].acceptance.verificationEvidence` | 主控传给生成器 | 绑定该 head 的单票验收记录指针，必须为非空字符串 |

## 运行归并

保持冻结清单不变。
在以下命令中填入与准备阶段相同的环境路径。
`SANDCASTLE_SWARM_MERGE_MANIFEST` 使用清单的绝对路径。

```sh
cd <repository-root>
DEEPAGENTS_PYTHON='<absolute-python-executable>' \
DEEPAGENTS_PILOT_PATH='<absolute-pilot-package-root>' \
PILOT1742_APP_PRIVATE_KEY='<absolute-app-private-key-path>' \
SANDCASTLE_SWARM_MERGE_MANIFEST='<merge-manifest-root>/<merge-run-id>/accepted-manifest.json' \
npx tsx .sandcastle/main-swarm.mts
```

入口跳过 planner 与认领。
组合 runId 来自清单。
入口在任何 Git 写入之前复核全部 DeepAgents 来源。
它还会在每项归并前、提交前和成功记账前复核该输入。

每步归并后，宿主对精确组合 SHA 执行以下组合验证。

- 对固定基线到组合 head 执行 `git diff --check`。
- 对累计 Rust 差异执行远端 cargo check。
- 涉及 TS、MTS、TSX、`package.json`、`package-lock.json` 或 `tsconfig*.json` 文件时，执行 `npx --no-install tsc -p tsconfig.sandcastle.json --noEmit`。

验证失败时停止后续输入。
无冲突时使用既有宿主提交路径，不调用 Docker。

### 成功时你会看到

`swarm-merge.mts` 的完成打印模板如下，运行时会代入组合分支和 head。

```text
功能归并完成：${manifest.branch} @ ${result.snapshot.head}；仅待验收，不合 main、不关来源票。
```

检查点的 `phase` 为 `done`。
重复 SHA 去重执行，原输入与票号追溯保留。
已在祖先链上的输入只记 `already_integrated`，不增加空提交。
因此，不要按输入数量推算新增提交数量。
把完成报告和单票证据交主控做组合结果的最终验收。

## 处理读取器拒绝

首轮 DeepAgents 来源准入在任何 Git 写入之前完成。
首跑或重入若在这一轮被拒，入口不会改动已有检查点与 `MERGE_HEAD`。
运行中再次准入失败时，保留此前进度并交主控。
清单位置检查在准备工位之后执行，不能把该项拒绝理解为工位一定尚未创建。

按下表识别错误文本子串。
除单独标注的两项外，文本均来自 `merge-source.mts`。

| 情形 | 读取器表现 | 操作员处置 |
| --- | --- | --- |
| 执行器已判 stale | `DeepAgents 交付已被执行器判 stale` | 停下并交主控决定重新评审或释放。保留 stale 标记 |
| 交付效果尚未认回 | `DeepAgents 交付仍有未认回的写效果` | 交原执行器按 `EXECUTOR.md` 读回效果。结果仍不明时由主控对账，不手改 delivery 记录 |
| 精确 SHA 的 review 回执不合格 | `DeepAgents review 回执不是该 SHA 的 VERDICT: PASS` | 交主控核对回执的 head、完成状态与首行结论。新 head 须重新评审和验收 |
| review 状态缺失或不匹配 | `DeepAgents 缺少该 SHA 的 deepagents/review 状态` | 交原执行器核对该 SHA 的交付状态。不要把其他 head 的状态挪来使用 |
| run 不在该 SHA 的交付终态 | `DeepAgents 交付不在该 SHA 的 awaiting_merge` | 先由主控核对来源 phase 与冻结 head，再决定恢复交付或重新验收 |
| 本仓同名来源 ref 漂移 | `DeepAgents 来源分支已不在已审 SHA` | 记录 ref 与观察到的 SHA，交主控核对。fetch 后仍须满足冻结 SHA 条件 |
| 远端 ref 漂移或缺失 | `DeepAgents 远端分支已不在已审 SHA`，缺失时含 `absent` | 交主控核对远端。不要推送或重置远端以凑回旧清单 |
| PR head 漂移、PR 已关、已合或找不到原 PR | `不是指向已审 SHA 的 open PR` | 到 GitHub 分辨具体状态并交主控。该消息共用，不能据此自动重开 PR |
| 同分支另一 open PR 指向不同 SHA | `DeepAgents 分支另有 open PR` | 交主控核对另一 PR 与 head。仅存在另一份同 SHA 的 open PR 不触发这条拒绝 |
| run 与 claim 不一致，或 run 身份与清单不符 | `DeepAgents run/claim 记录不一致`、`DeepAgents run 记录与清单身份不符`、`DeepAgents run 身份与票号不符` | 核对来源目录、run-id、票号和代次。交主控处理身份问题，不改记录迁就清单 |
| run 目录形状不符 | `DeepAgents run 目录必须为该 run 的绝对路径` | 核对绝对路径与目录 basename。按正确来源重新准备新清单 |
| 输入分支、固定 SHA 或证据绑定不符 | `DeepAgents 输入分支必须为 codex/deepagents/t<票号>`、`归并输入必须为固定单票 SHA`、`缺少绑定精确 SHA 的单票验收/独审证据`、`DeepAgents 独审证据必须指向该 SHA 的 review 回执` | 交主控核对验收事实。使用生成器准备新清单，不手工补成 accepted |
| 本机记录不可读或交付缺少 PR 编号 | `DeepAgents 记录不可读`、`DeepAgents 交付缺 PR 编号` | 保留报错路径和 stderr，交主控核对原执行器记录 |
| 远端读取环境缺项 | `DeepAgents 来源需要 App 只读远端事实` | 补齐准备阶段的三个环境变量。私钥变量只给绝对路径 |
| 远端事实读取失败或结果不可用 | `DeepAgents 远端事实读取失败，停止归并`、`DeepAgents 远端事实不是 JSON`、`DeepAgents 远端事实格式不符` | 交主控核对读取环境与 stderr。恢复可用后使用原清单重入，不改准入判据 |
| 清单位于工位内 | `验收清单不得位于工蜂 worktree`，来自 `swarm-merge.mts` | 保留工位。交主控把同一份清单安置到规定的外部位置后重入 |
| 含旧来源的清单被旧入口冻结闸拒绝 | `main-swarm.mts` 捕获 `DrainFrozenError` 后以 `75` 退出 | 按下文旧来源准入条件处理。保留现场，不自行解冻 |

区分两个入口的退出码 `75`。
`EXECUTOR.md` 中执行器的 `75` 表示该来源 run 已有控制进程。
`main-swarm.mts` 捕获 `DrainFrozenError` 时的 `75` 表示旧入口冻结。
按触发退出的入口判断，不套用另一入口的恢复动作。

## 中断后恢复

### 用同一清单重入

使用原命令和原清单文件重跑。
不要重新生成清单。
先通过全部来源的实时准入，再由入口收养检查点中的进度。
检查点不能绕过实时准入。

来源 PR 若已被人合入或关闭，同一清单重入也会被拒。
这不表示组合分支损坏。
把远端变化和已有进度交主控核对。

已提交但未验证的进度只补验证。
已通过的进度不重复提交或检查。
已为 `done` 的同一清单重入不增加提交。
尚未完成的输入仍照常产生其首次归并提交，重复 SHA 和已在祖先链上的输入按前述规则处理。

### 换清单时换 run-id

增删输入、调整顺序、改变基线或更换 SHA 时，使用新的 `<merge-run-id>`。
`runId` 是组合分支、工位和检查点的命名空间。
新 run-id 对应新的分支、工位与检查点。
已有检查点时发生合同变化，按 `.sandcastle/README.md` 停下并交 owner。
含旧来源的清单或混合清单还须重新准入，冻结中没有匹配事实时会被拒。
把旧工位留给 owner，不执行 reset、clean 或 abort。

### 冲突停下后交给主控

首次冲突需要默认 Docker 环境，使用 `main-swarm.mts` 指定的 `sandcastle:newchanlun` 镜像。
本进程只在首次冲突时执行环境与可达性预检。
进入预检前，`MERGE_HEAD` 与检查点已经保存。
预检通过后，入口会继续尝试冲突工蜂。
不要把首次冲突理解为必然立即停下。

预检失败，或冲突工蜂失败、受阻时，进程停下并保留现场。
此时把以下材料交给主控。

- 组合 run-id。
- 归并工位路径。
- 检查点路径。
- 当前输入 SHA。
- 以下只读命令的输出。
- 本次 stderr。

```sh
git -C <merge-worktree> rev-parse MERGE_HEAD
```

保留现场，不手工解冲突，不执行 abort、reset 或 clean。
Docker 冲突路径的真实执行尚未验收。

### 处理遗留锁与未知 WIP

发现未知 WIP 或合同变化时，停下并交 owner。
进程硬退出留下 `.lock` 时，由 owner 先核实锁内 PID 确实已经结束。
确认没有另一运行持锁后，由 owner 移除该锁，再用原命令恢复。
不得按超时强抢仍活着的工位。
本指南不提供锁文件精确路径或删除命令。

## 含旧来源输入的清单

旧来源输入没有 `source` 字段，分支形如 `sandcastle/issue-<票号>`。
只有 `inputs` 含这类输入时，入口才写入或依赖冻结准入事实。
这包括纯旧来源清单与旧来源、DeepAgents 混合清单。
纯 DeepAgents 清单不写入，也不依赖这份事实。

事实文件为 `.sandcastle/logs/merges/<merge-run-id>/admission.json`。
它的形状为 `{ v, repo, cwd, checkId, manifest }`。
`v` 为 `1`，`repo` 为仓库的真实路径，`cwd` 为归并工位路径，`checkId` 为当前 `MERGE_CHECK_ID`，`manifest` 为完整冻结清单。
入口只认与本次仓库、工位、清单和验证配置匹配的事实。

按以下条件处理历史 run。

| 条件 | 入口行为与操作员处置 |
| --- | --- |
| 只有工位或检查点，没有匹配准入事实 | 不算已准入。保留现场，由主控核对闸状态 |
| 冻结中没有匹配准入事实 | 在首个副作用前拒绝，零写入并保留现场。不要靠已有工位或检查点绕过冻结 |
| 闸仍开放，或已经获批解冻 | 由原公共入口重新取得准入事实。恢复仍使用原清单。若清单改变，按前述操作规则改用新 run-id |
| 冻结前已写入匹配准入事实 | 冻结后可续做。即使首个检查点尚未写出，匹配事实也允许继续 |

本指南不提供冻结或解冻操作。
这里描述的是入口合同，不表示生产冻结已经实施或验收。

## 状态放在哪

下表中，以 `.sandcastle/` 开头的路径均相对 `<repository-root>`。

| 对象 | 位置 |
| --- | --- |
| 组合工位 | `.sandcastle/worktrees/sandcastle-merge-<merge-run-id>` |
| 组合分支 | 本地 `sandcastle/merge-<merge-run-id>` |
| 检查点 | `.sandcastle/logs/merges/<merge-run-id>/checkpoint.json` |
| 旧来源准入事实 | `.sandcastle/logs/merges/<merge-run-id>/admission.json`，仅用于含旧来源输入的清单 |
| 归并事件日志 | `.sandcastle/logs/workers.jsonl`，归并事件的 `phase` 为 `merger`，记录检查点状态、head 与位置 |
| 来源 run 目录 | `<executor-state-root>/runs/<source-run-id>/`，保留 `run.json`、`claim.json`、`delivery/` 与独审回执 |
| accepted 清单 | `<merge-manifest-root>/<merge-run-id>/accepted-manifest.json`，本指南约定放在仓库与归并工位之外 |

## 依据

接口、字段、路径和拒绝条件依据以下六份文件。

- [merge-source.mts](../../../.sandcastle/merge-source.mts)。`deepAgentsMergeInput`、`readDeepAgentsDelivery`、`validateMergeInputSource`、`appRemoteFacts` 和 `admitMergeInput`。
- [swarm-merge.mts](../../../.sandcastle/swarm-merge.mts)。公共归并实现、实时准入、组合验证、状态路径、旧来源准入事实和首次冲突预检。
- [main-swarm.mts](../../../.sandcastle/main-swarm.mts)。清单环境变量入口、跳过 planner 与认领、`DrainFrozenError` 的退出码。
- [accepted-merge-drain.test.mts](../../../.sandcastle/accepted-merge-drain.test.mts)。旧来源冻结准入与恢复的六条用例。
- [README.md](../../../.sandcastle/README.md)。本地归并边界、检查点恢复、未知 WIP、遗留锁与去重规则。
- [EXECUTOR.md](../../../.sandcastle/deepagents-pilot/EXECUTOR.md)。来源 run 状态、交付效果恢复、执行器退出码和状态目录。

`accepted-merge-drain.test.mts` 的六条用例名原文如下。

1. `冻结后旧 sandcastle 清单的新归并在首个副作用前拒绝，解冻后同一清单正常完成`
2. `冻结前已准入的归并 run 在冻结后可续做收尾，不重复提交`
3. `冻结后预先存在但无准入事实的归并工位不能续做：拒绝且零新提交，现场保留`
4. `首次输入不存在而失败留下工位后，冻结中换成另一份清单被拒；解冻后重新准入可完成`
5. `准入后、首个检查点前崩溃的同一清单 run 在冻结后可续做，不再申请准入`
6. `已保存 prepared 检查点的已准入 run 冻结后恢复到 done，重入不增加提交`

这些用例覆盖旧来源、临时锁根与无冲突归并路径。
它们不覆盖 DeepAgents 远端读取或真实 Docker 执行。
