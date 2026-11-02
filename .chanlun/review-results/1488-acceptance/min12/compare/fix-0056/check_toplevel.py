"""s2-fix-0056/F1 补证：catalog/fixed_cut 中列表差异按多重集复核（消除 semdiff 按下标比对的位移噪声）。
- 字符串列表（evidence.object_ids）与 dict 列表（CC-056 evidence.replaces/withdrawals）解析身份后作多重集；
- 名字中 CC-056.segment_version 的槽位 TB-02-D/TB-02-W 统一为 *（同一对象改名，已在行级配对）。
输出：每个列表路径 old_only / new_only 条目，期望全部是 CC-022.unit_assignment / CC-056.center_version 相关。"""
import collections, json, re, sys
sys.path.insert(0, sys.argv[3])
import semdiff as S

SLOT = re.compile(r'TB-02-[DW]')
CENTER = ('CC-022.unit_assignment', 'CC-056.center_version')


def lists(o, p=''):
    if isinstance(o, dict):
        for k, v in o.items():
            yield from lists(v, p + '.' + k)
    elif isinstance(o, list):
        yield p, o


def norm(v):
    return SLOT.sub('TB-02-*', json.dumps(v, sort_keys=True, ensure_ascii=False))


old, new = S.load(sys.argv[1]), S.load(sys.argv[2])
on, nn = S.names_of(old['rows']), S.names_of(new['rows'])
out = {}
for top in ('fixed_cut', 'catalog'):
    A = dict(lists(S.resolve(old[top], on)))
    B = dict(lists(S.resolve(new[top], nn)))
    for p in sorted(set(A) | set(B)):
        a = collections.Counter(norm(x) for x in A.get(p, []))
        b = collections.Counter(norm(x) for x in B.get(p, []))
        if a != b:
            oo, no = list((a - b).elements()), list((b - a).elements())
            out[top + p] = {'old': sum(a.values()), 'new': sum(b.values()), 'old_only': len(oo), 'new_only': len(no),
                            'old_only_all_center': all(any(c in x for c in CENTER) for x in oo),
                            'new_only_all_center': all(any(c in x for c in CENTER) for x in no),
                            'path_absent_in_new': p not in B,
                            'old_only_sample': oo[:2], 'new_only_sample': no[:2]}
print(json.dumps(out, ensure_ascii=False, indent=1, sort_keys=True))
