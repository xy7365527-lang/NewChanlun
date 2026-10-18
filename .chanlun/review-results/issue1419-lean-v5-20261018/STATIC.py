"""#1419 v5 源码/来源封包；只读 Git，输出仅在本报告目录。无编译或产品执行。"""
import hashlib
import json
import re
import subprocess
from pathlib import Path

ROOT = Path.cwd()
OUT = ROOT / '.chanlun/review-results/issue1419-lean-v5-20261018'
MANAGER = Path('/Volumes/AgentStorage/issue1419-sandcastle-manager-20261018')
BASE = '112e1c3ca49da3c5ff21d0b6fa6a8bcd3abcd7b1'
RUST_BASE = '545e9f75faec49be39de7799e0aaf3f6c8276de9'
DONOR = '634f62cddfaddf656a35ca9b9db20cb716b62bff'
OLD = ROOT / '.chanlun/review-results/issue1419-lean-v4-20261018'
CHANGED = ['formal/Origin/SegmentFeatureSeq.lean', 'formal/Phase2/Claim10_SegmentV1.lean']
COMMANDS = []


def sha(b):
    return hashlib.sha256(b).hexdigest()


def save(name, obj):
    (OUT / name).write_text(json.dumps(obj, ensure_ascii=False, indent=2) + '\n')


def cmd(args, record=True):
    r = subprocess.run(args, capture_output=True, check=False)
    if record:
        COMMANDS.append(dict(argv=args, code=r.returncode, stdout=r.stdout.decode(), stderr=r.stderr.decode()))
    assert r.returncode == 0, (args, r.stderr.decode())
    return r.stdout


def gitblob(revision, path):
    return cmd(['git', 'show', f'{revision}:{path}'], record=False)


def strip_comments(text):
    # 保留行号；处理 Lean 嵌套块注释和字符串，禁词扫描不把历史注释当代码。
    out = list(text)
    i, depth, quoted = 0, 0, False
    while i < len(text):
        if depth:
            if text.startswith('/-', i):
                out[i:i+2] = '  '; depth += 1; i += 2
            elif text.startswith('-/', i):
                out[i:i+2] = '  '; depth -= 1; i += 2
            else:
                if text[i] != '\n': out[i] = ' '
                i += 1
        elif quoted:
            if text[i] == '\\': i += 2
            elif text[i] == '"': quoted = False; i += 1
            else: i += 1
        elif text.startswith('/-', i):
            out[i:i+2] = '  '; depth = 1; i += 2
        elif text.startswith('--', i):
            while i < len(text) and text[i] != '\n': out[i] = ' '; i += 1
        elif text[i] == '"': quoted = True; i += 1
        else: i += 1
    assert depth == 0 and not quoted
    return ''.join(out)


def symbols(text):
    text = strip_comments(text)
    ns = []
    found = []
    for number, line in enumerate(text.splitlines(), 1):
        m = re.match(r'^namespace\s+(\S+)', line)
        if m: ns.append(m[1])
        if re.match(r'^end(?:\s|$)', line):
            if ns: ns.pop()
        m = re.match(r'^(?:@\[[^\]]*\]\s*)?(def|theorem|structure|inductive|abbrev)\s+(\S+)', line)
        if m:
            found.append(dict(kind=m[1], name='.'.join(ns + [m[2]]), line=number))
    return found


def main():
    assert cmd(['git', 'rev-parse', 'HEAD']).decode().strip() == BASE
    cmd(['git', 'rev-parse', 'HEAD^{tree}'])
    assert cmd(['git', 'branch', '--show-current']).decode().strip() == 'codex/1419-lean-v5-20261018'
    cmd(['git', 'diff', '--check'])
    changed = cmd(['git', 'diff', '--name-only', BASE]).decode().splitlines()
    assert sorted(changed) == sorted(CHANGED), changed
    untracked = cmd(['git', 'ls-files', '--others', '--exclude-standard']).decode().splitlines()
    assert all(p.startswith(str(OUT.relative_to(ROOT)) + '/') for p in untracked)
    cmd(['git', 'diff', '--stat', BASE])
    cmd(['git', 'status', '--short'])
    previous = json.loads((OLD / 'SOURCE-MANIFEST.json').read_text())
    sources = []
    for entry in previous:
        path = entry['path']
        before, donor, now = gitblob(BASE, path), gitblob(DONOR, path), (ROOT / path).read_bytes()
        assert sha(before) == entry['final_sha256']
        assert sha(donor) == entry['donor_sha256']
        sources.append(dict(path=path, base_112e_sha256=sha(before), donor_634f_sha256=sha(donor),
                            current_sha256=sha(now), current_bytes=len(now), changed=now != before))
    assert sum(x['changed'] for x in sources) == 2
    save('SOURCE-MANIFEST.json', sources)
    rust_paths = ['rust/src/theta_v0/types.rs', 'rust/src/theta_v0/parser/feature_seq.rs',
                  'rust/src/theta_v0/parser/second_fold.rs']
    rust = []
    for path in rust_paths:
        before, now = gitblob(RUST_BASE, path), (ROOT / path).read_bytes()
        assert before == gitblob(BASE, path) == now
        rust.append(dict(path=path, sha256=sha(now), bytes=len(now), same_545e_112e_current=True))
    save('RUST-IDENTITY.json', rust)
    decls, preservation = [], []
    forbidden = re.compile(r'\b(sorry|admit|axiom|unsafe)\b')
    for path in CHANGED:
        before, now = gitblob(BASE, path).decode(), (ROOT / path).read_text()
        old_symbols = {x['name'] for x in symbols(before)}
        new_symbols = symbols(now)
        decls.extend(dict(file=path, **x) for x in new_symbols if x['name'] not in old_symbols)
        bcode, ncode = strip_comments(before), strip_comments(now)
        assert len(forbidden.findall(ncode)) == len(forbidden.findall(bcode)) == 0
        # 所有旧非注释行按原顺序存在；本轮未删除/替换旧定义或证明代码。
        old_lines = [s.strip() for s in bcode.splitlines() if s.strip()]
        new_lines = iter(s.strip() for s in ncode.splitlines() if s.strip())
        for line in old_lines:
            assert any(other == line for other in new_lines), (path, line)
        preservation.append(dict(path=path, old_code_lines_preserved=True, forbidden_code_tokens=0))
    save('DECLARATIONS.json', decls)
    save('PRESERVATION.json', preservation)
    # 新增 import 的静态依赖闭包与环检测；不是 Lean 类型/导出检查。
    paths = [p for p in cmd(['git', 'ls-files', 'formal'], record=False).decode().splitlines() if p.endswith('.lean')]
    graph = {}
    for path in paths:
        module = path[len('formal/'):-len('.lean')].replace('/', '.')
        graph[module] = re.findall(r'^import\s+([\w.]+)', strip_comments((ROOT / path).read_text()), re.M)
    done, active = set(), set()
    def visit(m):
        if m not in graph or m in done: return
        assert m not in active, m
        active.add(m)
        for dep in graph[m]: visit(dep)
        active.remove(m); done.add(m)
    for m in graph: visit(m)
    affected = {'Origin.SegmentFeatureSeq', 'Phase2.Claim10_SegmentV1'}
    while True:
        nxt = affected | {m for m, deps in graph.items() if affected.intersection(deps)}
        if nxt == affected: break
        affected = nxt
    save('IMPORTS.json', dict(modules=len(graph), acyclic=True, affected=sorted(affected),
                              new_import='Phase2.Claim10_SegmentV1 -> Origin.SegmentFeatureSeq',
                              typecheck='NOT_RUN'))
    refs = []
    for lesson, numbers in [(65, [28]), (67, [38,46]), (71, [30,32,40,42]), (78,[52,54])]:
        path = f'docs/chanlun/text/blog/{lesson:03d}-第{lesson}课.md'
        data = (ROOT / path).read_bytes(); lines = data.decode().splitlines()
        boundary = next(i for i, s in enumerate(lines, 1) if '↑正文' in s)
        for number in numbers:
            text = lines[number-1]
            assert number < boundary
            assert not re.search(r'[（(](?:娇)?注[:：]', text)
            refs.append(dict(path=path, line=number, exact_text=text, source_sha256=sha(data),
                             body_boundary=boundary, before_boundary=True, leading_note=False,
                             inline_note=False, source='正本 blog；逐行原文保留，无删改引文'))
    save('REFERENCES.json', refs)
    xerj_path = MANAGER / 'lean-same-source-preparation/XERJ-REFERENCE-ATTEMPT.json'
    xbytes = xerj_path.read_bytes(); x = json.loads(xbytes); response = json.loads(x['response'])
    assert sha(xbytes) == '39ac6dd99a589973ec0bb1489426b9f0aa638300c8b9a04a708e0622353c6a1a'
    assert x['status'] == 200
    reusable = []
    for path in sorted(OLD.iterdir()):
        if not path.is_file(): continue
        b = path.read_bytes(); rel = str(path.relative_to(ROOT))
        assert b == gitblob(BASE, rel)
        reusable.append(dict(path=rel, sha256=sha(b), bytes=len(b)))
    save('REUSED-EVIDENCE.json', dict(
        formal_unchanged=[x['path'] for x in sources if not x['changed']],
        old_reports_unchanged=reusable, review_path=str(MANAGER / 'LEAN-V4-INDEPENDENT-REVIEW.json'),
        scope='仅复用 C/E/F 未变源的静态证据；D 新差异另验。旧报告的完整前缀措辞由 v5 明确撤销。',
        rerun_C_E_F_models=False, qualification='不继承旧 Lean/Rust/同树执行资格；这些仍 NOT_RUN'))
    inputs = [MANAGER / 'LEAN-V4-TASK.md', MANAGER / 'LEAN-V5-TASK.md',
              MANAGER / 'LEAN-V4-INDEPENDENT-REVIEW.json', MANAGER / 'issue1419-live.json',
              MANAGER / 'lean-same-source-preparation/INDEX.json', xerj_path,
              ROOT / 'AGENTS.md', ROOT / 'CLAUDE.md', ROOT / 'docs/agents/wayfinder-workflow.md',
              ROOT / 'docs/agents/delivery-discipline.md', ROOT / 'docs/agents/generation-constitution.md',
              ROOT / '.chanlun/definitions/xianduan.md', ROOT / 'formal/Origin/ChanlunElements.lean',
              ROOT / 'formal/Origin/SourceAxioms.lean', ROOT / 'formal/lean-toolchain',
              ROOT / 'formal/lakefile.toml', ROOT / 'scripts/check_fixture_drift.py',
              Path('/Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/src/lean/Init/Data/List/Basic.lean'),
              Path('/Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/src/lean/Init/Data/Int/Order.lean')]
    save('INPUTS.json', dict(inputs=[dict(path=str(p), bytes=len(p.read_bytes()), sha256=sha(p.read_bytes())) for p in inputs],
         xerj=dict(status=x['status'], hits=response['hits']['total'], indexes=[h['_index'] for h in response['hits']['hits']],
                   lean_specific_result=False, refetched=False)))
    patch = cmd(['git', 'diff', '--binary', '--full-index', BASE, '--'] + CHANGED, record=False)
    (OUT / 'DELTA.patch').write_bytes(patch)
    cmd(['git', 'apply', '--reverse', '--check', str(OUT / 'DELTA.patch')])
    save('STATIC-COMMANDS.json', COMMANDS)
    summary = dict(status='PASS_SCOPED_STATIC', changed_formal=2, unchanged_formal=9,
                   new_named_declarations=len(decls), new_named_theorems=sum(x['kind']=='theorem' for x in decls),
                   old_code_preserved=True, dependency_cycle=False, rust_545e_112e_same=True,
                   references=len(refs), patch_sha256=sha(patch), lean='NOT_RUN', rust='NOT_RUN')
    save('STATIC-RESULTS.json', summary)
    print(json.dumps(summary, ensure_ascii=False))


if __name__ == '__main__':
    main()
