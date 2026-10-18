"""#1527 定向静态核验与纯模型；不执行 Rust、SQLite、产品或历史套件。"""
from pathlib import Path
import difflib
import hashlib
import json
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
BASE = '9d8e5c328a4a15aad1e53dac1a875f788e30b05b'
PARENT = 'ce3dd877359e4ccec76b54f1d3d0f3585d85d78f'
PROPOSAL = 'c1c156ed5851c36395fed35e0dc763f5d5b248e5'
REVIEW = 'b228d8ec05173127f17c4af777b2df51c65aa425'
PRODUCTS = [
    'rust/src/s_content/legacy_evidence.rs',
    'rust/src/s_content.rs',
    'rust/src/bin/s_session_v2/evidence_store.rs',
    'rust/src/bin/s_session_v3/admission_private/mod.rs',
    'rust/src/bin/s_structure_session.rs',
]
checks = []


def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT)


def source(path, revision=BASE):
    return git('show', f'{revision}:{path}')


def sha(data):
    return hashlib.sha256(data).hexdigest()


def check(name, condition, detail=None):
    assert condition, name
    checks.append({'项目': name, '结果': 'PASS', '依据': detail})


def write_json(name, data):
    (OUT / name).write_text(json.dumps(data, ensure_ascii=False, indent=2) + '\n')


def body(text, name):
    start = text.index('fn ' + name + '(')
    brace = text.index('{', start)
    depth, end = 1, brace + 1
    while depth:
        depth += (text[end] == '{') - (text[end] == '}')
        end += 1
    return text[brace:end]


def format_rust(data):
    result = subprocess.run(
        ['rustfmt', '--edition', '2021', '--emit', 'stdout', '--config', 'skip_children=true'],
        input=data, stdout=subprocess.PIPE, stderr=subprocess.PIPE, cwd=ROOT, check=True,
    )
    return result.stdout, result.stderr.decode()


base_spec = json.loads((ROOT.parent / 'LEGACY-PURE-BASE.json').read_text())
check('固定基线及树', git('rev-parse', 'HEAD').decode().strip() == BASE
      and base_spec['source'] == BASE
      and git('rev-parse', 'HEAD^{tree}').decode().strip() == base_spec['tree'])
check('产品自冻结父未变', not git('diff', '--name-only', PARENT, BASE, '--', 'rust', 's_session'))
check('当前分支仍为原 codex checkout', git('branch', '--show-current').decode().strip()
      == 'codex/1527-formal-i1-integration-20261018')

old = source(PRODUCTS[2]).decode()
new = (ROOT / PRODUCTS[2]).read_text()
legacy = (ROOT / PRODUCTS[0]).read_text()
for name in ['node_id', 'verify_node_row', 'hex', 'hex_bytes']:
    check(f'原纯函数 {name} 函数体逐字一致', body(old, name) == body(legacy, name),
          {'原函数体SHA256': sha(body(old, name).encode())})
    check(f'bin 不再保留 {name} 的第二份公式', not re.search(r'\bfn ' + name + r'\(', new))

# 用确定性迁移重建整个 evidence_store，既有 SQL/type/finalize/Reader 都在比较内。
expected = old
for name in ['node_id', 'verify_node_row', 'hex', 'hex_bytes']:
    pattern = r'(?:pub\(super\) )?fn ' + name + r'\('
    start = re.search(pattern, expected).start()
    fnbody = body(expected, name)
    end = expected.index(fnbody, start) + len(fnbody)
    expected = expected[:start] + expected[end + 1:]
expected = expected.replace('const KIND_LEAF: i64 = 0;\nconst KIND_SEQ: i64 = 1;\n', '', 1)
expected = expected.replace('/// 节点种类：0 = 精确字节叶；1 = 有序拼接节点。\n', '', 1)
expected = expected.replace('use super::*;', '''use super::*;
// #1527：库/bin 共用迁移后的唯一原验证器；SQL 类型拒绝仍在原读行位置。
use newchan_rust::s_content::legacy_evidence::{hex, node_id, verify_node_row, KIND_LEAF, KIND_SEQ};
pub(super) use newchan_rust::s_content::legacy_evidence::hex_bytes;''', 1)
while '\n\n\n' in expected:
    expected = expected.replace('\n\n\n', '\n\n')
expected_formatted, _ = format_rust(expected.encode())
check('整个旧 Reader/SQL 拒绝位置除具名移位外逐字保持', expected_formatted == new.encode())
check('S-content 只增加模块声明', (ROOT / PRODUCTS[1]).read_text().replace(
    '\n\n/// #1527：旧证据图纯行验证；独立于 S-content 编码与计量。\npub mod legacy_evidence;', '', 1
).encode() == source(PRODUCTS[1]))
proof = legacy.split('pub struct CheckedLegacyEvidenceRow {', 1)[1].split('}', 1)[0]
check('凭据只有私有固定栈字段', proof.strip().splitlines() == [
    'id: [u8; 32],', '    kind: i64,', '    payload_len: usize,', '    declared: u64,'])
proof_production = legacy.split('#[cfg(test)]', 1)[0]
check('凭据唯一实际验证构造口', proof_production.count('Ok(Self {') == 1
      and 'let declared = verify_node_row(id, kind, payload, len)?;' in proof_production
      and not re.search(r'#\[derive|\b(?:Vec|Arc|Default|Deserialize|unchecked)\s*(?:<|\()', proof_production))

unchanged = [
    'rust/Cargo.toml', 'rust/Cargo.lock', 'rust/src/lib.rs', 'rust/src/s_content/job.rs',
    'rust/src/bin/s_session_v3/admission_anchor.rs', 'rust/src/bin/s_session_v3/admission.rs',
    'rust/src/bin/s_session_v3/admission_private/fd_wal.rs',
    'rust/src/bin/s_session_v3/state.rs', 'rust/src/bin/s_session_v3/audit_owner.rs',
    'rust/src/bin/s_session_v2/cache.rs', 'rust/src/bin/s_session_v2/audit.rs',
    's_session/tests/issue1527_recovery/test_support.rs',
]
for path in unchanged:
    check('保护源不变：' + path, (ROOT / path).read_bytes() == source(path))
cargo = source('rust/Cargo.toml').decode()
lib = source('rust/src/lib.rs').decode()
check('库/bin feature 和依赖静态闭合',
      '#[cfg(feature = "s_session")]\npub mod s_content;' in lib
      and 's_session = ["dep:sha2"' in cargo
      and 'name = "s_structure_session"\npath = "src/bin/s_structure_session.rs"\nrequired-features = ["s_session"]' in cargo,
      '跨 crate 使用 pub 模块/函数；Cargo 与既有 sha2 依赖不变，未作类型或链接验证。')

root_old = source(PRODUCTS[4]).decode()
root_new = (ROOT / PRODUCTS[4]).read_text()
boundary = root_old.index('\nmod tests {')
check('根生产区逐字不变', root_old[:boundary] == root_new[:boundary])
renames = {'cmd_accept': 'accept_business_file', 'cmd_advance': 'advance_business', 'cmd_recover': 'recover_business'}
reverted = root_new[boundary:]
helper_start = reverted.index('\n    // #1527 M-03：')
helper_end = reverted.index('    fn accept(files:', helper_start)
helpers = reverted[helper_start:helper_end]
reverted = reverted[:helper_start] + reverted[helper_end:]
for before, after in renames.items():
    reverted = reverted.replace(after + '(', before + '(')
check('M-03 逆替换后所有旧测试正文及断言逐字不变', reverted == root_old[boundary:])
check('业务夹具只调用既有 core 无生产 scope 后门', all(x in helpers for x in [
    'accept_core(db, &text, profile_path, configured_epoch, None).map(|_| ())',
    'advance_core(db, configured_epoch, None).map(|_| ())',
    'recover_core(db, new_epoch, None).map(|_| ())',
]) and not any(x in helpers for x in ['WriterScope', 'TestOnly', 'ACTIVE', 'read_current_text', 'write_current_result']))
check('旧测试区不再直接调用三个命令外壳', not re.search(r'\bcmd_(accept|advance|recover)\(', root_new[boundary:]))

# 按根测试模块的四空格函数边界建立调用库存；不执行这些测试或其 Python oracle。
functions = list(re.finditer(r'^    (?:pub\(super\) )?fn (\w+)\(', root_old[boundary:], re.M))
inventory, function_text = [], {}
for i, match in enumerate(functions):
    end = functions[i + 1].start() if i + 1 < len(functions) else len(root_old[boundary:])
    text = root_old[boundary:][match.start():end]
    name = match.group(1)
    function_text[name] = text
    calls = {old_name: len(re.findall(r'\b' + old_name + r'\(', text)) for old_name in renames}
    if any(calls.values()):
        inventory.append({'函数': name, '固定源行': root_old[:boundary + match.start()].count('\n') + 1,
                          '直接替换次数': calls})
changed_helpers = {entry['函数'] for entry in inventory}
transitive = set(changed_helpers)
while True:
    before = set(transitive)
    transitive.update(name for name, text in function_text.items()
                      if any(re.search(r'(?<![\w.])' + target + r'\(', text) for target in before))
    if transitive == before:
        break
test_names = {
    re.search(r'^    (?:pub\(super\) )?fn (\w+)\(', root_old[boundary:][m.end():], re.M).group(1)
    for m in re.finditer(r'^    #\[test\]', root_old[boundary:], re.M)
}
write_json('M03-TEST-INVENTORY.json', {
    '说明': '固定根测试区函数库存；含 helper 与其传递调用者，非已运行测试数。原断言经逆替换逐字相同。',
    '直接替换': inventory, '含传递调用的受影响函数': sorted(transitive),
    '受影响测试': sorted(transitive & test_names), '受影响辅助函数': sorted(transitive - test_names),
    '替换调用总数': sum(sum(x['直接替换次数'].values()) for x in inventory),
    'Rust执行': 'NOT_RUN',
})

mod = (ROOT / PRODUCTS[3]).read_text()
production = mod.split('#[cfg(test)]', 1)[0]
check('原后到门优先表达式已移除', 'gate.and(result)' not in production and 'budget.and(result)' not in production)
check('原旧域及 D2 资格拒绝逐字保留', all(x in production for x in [
    'LegacyAdmissionBudgetAdapterMissing：完整旧域审计已调用，但旧证据图节点与Snapshot/Delta尚无Job同源费用适配，禁止签发准入',
    'D2ProductionCapacityMissing：JobLimits8/SResources7没有已裁的overlay payload、metadata与SQLite heap额度映射；禁止采用D2-SMALL六参数默认',
]))
check('业务结果保存早于回滚', production.index('.returned(ErrorPhase::Business, &result)') < production.index('tx.rollback()'))
check('业务检查结果保存早于关闭连接', production.index('.returned(ErrorPhase::Business, &check)') < production.index('.close()'))
check('每个原收尾均在封口前执行', all(production.index(x) < production.index('errors.into_inner().finish()') for x in [
    'tx.rollback()', '.close()', '.finish()', 'account.reconcile(&grant, 0)', 'account.check_deadline()',
]))
check('原 Job 未替换且没有新增旧域消费口/集合', not re.search(r'Job::new|new_started|BTreeSet|accept_checked_legacy|record_visit|LegacyAuditScope', production))
check('已返回实读拒收由原账增量推导', all(x in production for x in [
    'let before = account.snapshot().wire_bytes;', 'let accepted = account.snapshot().wire_bytes - before;',
    'u128::from(actual).saturating_sub(accepted)', 'self.observed(result).map_err(Into::into)',
]))

# 迁移新增差异面的小输入，不复跑原提案 27/160 套件。
def old_row(digest, kind, payload, length):
    if kind not in (0, 1):
        return f'StorageUnavailable：私有节点 {digest.hex()} 种类 `{kind}` 未声明'
    if hashlib.sha256(bytes([kind]) + payload).digest() != digest:
        return f'StorageUnavailable：私有节点 {digest.hex()} 同 ID 异内容（内容寻址不符）'
    if length < 0:
        return f'StorageUnavailable：私有节点 {digest.hex()} 长度非负越界'
    if kind == 0 and length != len(payload):
        return f'StorageUnavailable：私有叶 {digest.hex()} 声明长度与实际不符'
    return length


row_cases = []
for title, kind, payload, length, wrong_hash, expected_kind in [
    ('空叶', 0, b'', 0, False, '成功'),
    ('非UTF8原字节', 0, b'\x00\xff', 2, False, '成功'),
    ('kind先于坏hash和负len', 256, b'abc', -1, True, '种类 `256` 未声明'),
    ('负kind先拒', -1, b'', -1, True, '种类 `-1` 未声明'),
    ('hash先于负len', 0, b'abc', -1, True, '同 ID 异内容（内容寻址不符）'),
    ('非负长度检查', 0, b'abc', -1, False, '长度非负越界'),
    ('叶长度', 0, b'abc', 4, False, '声明长度与实际不符'),
    ('SEQ行不冒称闭包已核', 1, b'not a sequence', 2**63 - 1, False, '成功'),
]:
    digest = bytes(32) if wrong_hash else hashlib.sha256(bytes([kind]) + payload).digest()
    result = old_row(digest, kind, payload, length)
    if expected_kind == '成功':
        assert result == length
        proof_model = (digest, kind, len(payload), result)
        assert proof_model[2] == len(payload)
    else:
        assert isinstance(result, str) and result.endswith(expected_kind)
    row_cases.append({'用例': title, 'kind': kind, 'payload_hex': payload.hex(), 'len': length,
                      'id': digest.hex(), '结果': result})
check('8项原行/opaque构造纯模型', True, 'Python标准库模型，函数体逐字证明另列；不是原Rust运行对拍。')

# 固定阶段观察模型：主错是最早实际失败；同阶段回调与返回独立保留。
class Errors:
    def __init__(self):
        self.first = None
        self.callback = [None] * 7
        self.returned = [None] * 7
        self.visited = []

    def observe(self, phase, error, callback=False):
        self.visited.append((phase, error, callback))
        if error is None:
            return
        if self.first is None:
            self.first = error
        elif callback:
            if error != self.first and self.callback[phase] is None:
                self.callback[phase] = error
        elif error != self.first and error not in self.callback + self.returned and self.returned[phase] is None:
            self.returned[phase] = error


traces = []
cases = [
    ('先业务后deadline', [(0, '业务原错', False), (2, '晚到deadline', True)], '业务原错'),
    ('先VFS预算后SQL包装', [(0, 'wire cap', True), (0, 'stopped wrapper', True), (0, 'SQL原返回', False)], 'wire cap'),
    ('先progress期限后SQL包装', [(0, '早到deadline', True), (0, 'SQL interrupted', False)], '早到deadline'),
    ('先回滚后期限', [(1, 'rollback原错', False), (2, '晚到deadline', True)], 'rollback原错'),
    ('先关闭后期限', [(3, 'close原错', False), (6, '晚到deadline', False)], 'close原错'),
    ('先结预留后期限', [(5, 'reconcile原错', False), (6, '晚到deadline', False)], 'reconcile原错'),
    ('业务成功但最终超期', [(6, 'deadline', False)], 'deadline'),
    ('全成功', [], None),
    ('原业务错误逐字', [(0, '原业务错误：逐字', False)], '原业务错误：逐字'),
]
for title, events, expected_first in cases:
    errors = Errors()
    for event in events:
        errors.observe(*event)
    for phase in range(1, 7):
        errors.observe(phase, None)
    assert errors.first == expected_first
    assert [x[0] for x in errors.visited[-6:]] == list(range(1, 7))
    if title == '先VFS预算后SQL包装':
        assert errors.returned[0] == 'SQL原返回' and errors.callback[0] == 'stopped wrapper'
    traces.append({'用例': title, '观察': errors.visited, '首错': errors.first,
                   '回调次错': errors.callback, '原返回次错': errors.returned})
errors = Errors()
errors.observe(0, '业务原错')
for phase in range(1, 7):
    errors.observe(phase, f'回调{phase}', True)
    errors.observe(phase, f'收尾返回{phase}')
assert errors.first == '业务原错'
assert all(errors.callback[phase] and errors.returned[phase] for phase in range(1, 7))
traces.append({'用例': '六收尾均失败仍保主错和每阶段两类次错', '观察': errors.visited, '首错': errors.first})
check('10项首错/全收尾纯事件模型', True, traces)

# 独立账语义模型，额度 3 是 toy 数字。已接受 4，随后已返回 3 拒收 => 总观察 7。
wire, stopped, rejected, t0, reservation = 0, False, 0, 123456, object()
original_reservation = reservation
wire_observations = []
for actual in [4, 3]:
    before = wire
    if not stopped:
        wire += actual
        stopped = wire > 3
    delta = wire - before
    rejected += max(0, actual - delta)
    wire_observations.append({'实读': actual, '已接纳增量': delta, '原wire': wire, '拒收累计': rejected})
assert (wire, rejected, wire + rejected) == (4, 3, 7)
assert t0 == 123456 and reservation is original_reservation and stopped
check('部分实读/已入账超限与停止后拒收分开', True, wire_observations)
write_json('MODEL-RESULTS.json', {'旧行用例': row_cases, '首错事件': traces, '实读量': wire_observations})

parse = []
for path in PRODUCTS:
    data = (ROOT / path).read_bytes()
    _, stderr = format_rust(data)
    parse.append({'路径': path, '输入SHA256': sha(data), '退出码': 0, 'stderr': stderr})
check('5份增量Rust源码经stdin解析', True, parse)
subprocess.run(['git', 'diff', '--check'], cwd=ROOT, check=True)
changed = set(git('diff', '--name-only', BASE, '--').decode().splitlines())
check('已跟踪写集只含五份授权产品', changed <= set(PRODUCTS), sorted(changed))
untracked = git('ls-files', '--others', '--exclude-standard').decode().splitlines()
check('未跟踪文件限定新模块和本报告目录', all(
    path == PRODUCTS[0] or path.startswith(str(OUT.relative_to(ROOT)) + '/') for path in untracked
), untracked)

# 精确产品补丁含未跟踪新模块，内存按旧文本零fuzz复核，不修改Git index。
patch = ''
for path in PRODUCTS:
    new_bytes = (ROOT / path).read_bytes()
    old_bytes = b'' if path == PRODUCTS[0] else source(path)
    diff = list(difflib.unified_diff(old_bytes.decode().splitlines(True), new_bytes.decode().splitlines(True),
                                  fromfile='/dev/null' if not old_bytes else 'a/' + path,
                                  tofile='b/' + path))
    patch += 'diff --git a/' + path + ' b/' + path + '\n'
    if not old_bytes:
        patch += 'new file mode 100644\n'
    patch += ''.join(diff)
    rebuilt, cursor, i = [], 0, 2
    old_lines = old_bytes.decode().splitlines(True)
    while i < len(diff):
        match = re.match(r'@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@', diff[i])
        assert match, (path, diff[i])
        start = int(match[1]) - 1 if int(match[1]) else 0
        rebuilt.extend(old_lines[cursor:start]); cursor = start; i += 1
        while i < len(diff) and not diff[i].startswith('@@ '):
            line = diff[i]
            if line[0] in ' -':
                assert old_lines[cursor] == line[1:]
                cursor += 1
            if line[0] in ' +':
                rebuilt.append(line[1:])
            i += 1
    rebuilt.extend(old_lines[cursor:])
    assert ''.join(rebuilt).encode() == new_bytes
(OUT / 'PRODUCT.patch').write_text(patch)
check('精确五文件产品patch零fuzz内存重建', True, sha(patch.encode()))

old_reports = []
for path in git('ls-tree', '-r', '--name-only', BASE, '--', '.chanlun/review-results').decode().splitlines():
    if '/issue1527-' in path:
        data = source(path)
        current = (ROOT / path).read_bytes()
        assert current == data, path
        old_reports.append({'路径': path, 'blob': git('rev-parse', BASE + ':' + path).decode().strip(),
                            'SHA256': sha(data)})
check('旧#1527报告及证据逐文件哈希不变', bool(old_reports), {'文件数': len(old_reports)})
write_json('CHECKS.json', {
    '票号': '#1527', '范围': '纯旧验证器移位、首错窄修、M-03根业务回归调用对齐',
    '结果': 'PASS_STATIC_AND_MODEL_ONLY', '检查': checks,
    '复用': '原27/160与其他历史套件未重跑，不累加成本轮成绩。',
    'NOT_RUN': ['cargo/rustc/build', '全部Rust测试、类型/借用/ABI/Drop/链接', 'SQLite/正式S/Q/旧binary/历史重型',
                '原Rust验证器执行对拍、真实恢复、OS EINTR/取消、金融/隔离双跑'],
})

inputs = []
for path in [*PRODUCTS[1:], *unchanged, 'AGENTS.md', 'docs/agents/generation-constitution.md']:
    data = source(path)
    inputs.append({'source': BASE, '路径': path, 'blob': git('rev-parse', BASE + ':' + path).decode().strip(),
                   'SHA256': sha(data)})
for revision, paths in [
    (PROPOSAL, ['PROPOSAL.md', 'interfaces.rs', 'CHECKS.json', 'MANIFEST.json']),
    (REVIEW, ['REVIEW.md', 'CHECKS.json', 'MANIFEST.json']),
]:
    directory = 'issue1527-legacy-job-proposal-20261018' if revision == PROPOSAL else 'issue1527-rust-h1m02-r2-review-20261018'
    for filename in paths:
        path = f'.chanlun/review-results/{directory}/{filename}'
        data = source(path, revision)
        inputs.append({'source': revision, '路径': path,
                       'blob': git('rev-parse', revision + ':' + path).decode().strip(), 'SHA256': sha(data)})
parent_inputs = []
for name in ['LEGACY-PURE-BASE.json', 'ROOT-LEGACY-G1-G4-DECISION.md', 'ROOT-I1-IMPLEMENTATION-SCOPE.md',
             'I1-RESPONSIBILITY-AND-REUSE.md', 'PRIOR-R4-V3-DESIGN-TRANSCRIPT.md', 'PRIOR-R4-PROVENANCE.json',
             'xerj-reference-checkpoint-query.json']:
    data = (ROOT.parent / name).read_bytes()
    parent_inputs.append({'路径': '../' + name, 'SHA256': sha(data)})
write_json('MANIFEST.json', {
    '票号': '#1527', '名分': '绑定OPEN #1527的工作草稿；不是I1完成或生产验收',
    'source': BASE, 'tree': base_spec['tree'], 'product_parent': PARENT,
    '父输入': parent_inputs, '固定源': inputs, '旧报告不变': old_reports,
    '产品': [{'路径': path, 'SHA256': sha((ROOT / path).read_bytes())} for path in PRODUCTS],
    '包内文件': [{'路径': str(path.relative_to(ROOT)), 'SHA256': sha(path.read_bytes())}
               for path in sorted(OUT.iterdir()) if path.is_file() and path.name != 'MANIFEST.json'],
    '说明': 'MANIFEST不自我hash；本脚本只操作固定白名单与本报告目录；无联网/索引/安装/凭据。',
})
print(json.dumps({'结果': 'PASS_STATIC_AND_MODEL_ONLY', '检查数': len(checks), '旧报告文件': len(old_reports),
                  'M03直接改动函数': len(inventory), 'M03调用替换': sum(sum(x['直接替换次数'].values()) for x in inventory),
                  'Rust测试': 'NOT_RUN'}, ensure_ascii=False))
