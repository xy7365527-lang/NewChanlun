"""旧 known-N 候选 vs main oracle g=N+1 全量逐项对照（v2，#1488 s2-fix-0056 修 0055/F1 盲区）。

v1 盲区（已修）：递归剥除 semantic/policy/schema/domain 版本字段；flat 时空容器不产生叶子（`[]`/`{}` 新增不可见）；
按 fact_key 用 dict 配对（同 fact_key 多条折叠）；不比 fixed_cut/catalog/counts。

v2 口径：
- 不剥任何字段、不改写 raw_position；只把身份哈希（obj/batch/wit/obs/rcpt-<hex>）解析为
  `<kind|fact_key|r<revision>|g<first_known_generation>|<lifecycle>>`（对象）或 `<batch>` 等占位，使跨二进制可比。
- 行：family:kind 内做规范化记录多重集；残差按身份键（fact_key+revision+first_known_generation；关系为解析后的
  subject/relation_type/object；其余见 ident）分组，同键多条按规范串排序后逐对配对（不折叠）；未配上的整条列出。
- 字段差：flat 保留空容器叶子；路径中列表下标归一为 []；每个 (kind, 路径) 记条数与全部不同的 (old,new) 取值对（至多 8 种）。
- 顶层：fixed_cut、catalog（items 按 id 配对）、counts、scope、omissions、order_version 全部 flat 逐路径比对；
  标量字符串列表（如 evidence.object_ids）解析后排序再比。
用法：python3 semdiff.py <known-N/committed-candidate.json> <oracle/g-(N+1).json>
"""
import collections, json, re, sys

ID = re.compile(r'\b(obj|batch|wit|obs|rcpt)-[0-9a-f]{16,}\b')
MAXV = 8  # 每个 (kind, 路径) 保留的不同取值对上限；复核时可调大
TOP = ('fixed_cut', 'catalog', 'counts', 'scope', 'omissions', 'order_version')


def load(path):
    d = json.load(open(path))
    return d['projection'] if 'projection' in d else d


def names_of(rows):
    names = {}
    for r in rows:
        rec = r['record']
        if r['family'] in ('objects', 'withdrawn_objects') and rec.get('object_id'):
            names[rec['object_id']] = '<%s|%s|r%s|g%s|%s>' % (rec.get('kind'), json.dumps(rec.get('fact_key'), sort_keys=True),
                                                             rec.get('object_revision'), rec.get('first_known_generation'),
                                                             rec.get('lifecycle'))
    return names


def resolve(o, names):
    if isinstance(o, dict):
        return {k: resolve(v, names) for k, v in o.items()}
    if isinstance(o, list):
        out = [resolve(v, names) for v in o]
        if out and all(isinstance(v, str) for v in out):
            out = sorted(out)
        if out and all(isinstance(v, dict) and 'id' in v for v in out):
            out = {'id=' + str(v['id']): v for v in out}
        return out
    if isinstance(o, str):
        return ID.sub(lambda m: names.get(m.group(0), '<' + m.group(1) + '>'), o)
    return o


def flat(o, p=''):
    out = {}
    if isinstance(o, dict):
        if not o:
            out[p] = '{}'
        for k, v in o.items():
            out.update(flat(v, p + '.' + k))
    elif isinstance(o, list):
        if not o:
            out[p] = '[]'
        for i, v in enumerate(o):
            out.update(flat(v, f'{p}[{i}]'))
    else:
        out[p] = o
    return out


def kind(r):
    rec = r['record']
    return r['family'] + ':' + str(rec.get('kind', rec.get('relation_type', '')))


def ident(r, rec):
    if r['family'] == 'relations':
        return [rec.get('subject'), rec.get('relation_type'), rec.get('object')]
    if 'fact_key' in rec:
        return [rec['fact_key'], rec.get('object_revision'), rec.get('first_known_generation')]
    if r['family'] == 'raw_history':
        return r.get('key')
    if r['family'] == 'witnesses':
        return [rec.get('object_id'), rec.get('slot'), rec.get('merged_source_index')]
    return [rec.get('kind'), rec.get('source_coords'), rec.get('window_start'), rec.get('window_end'), rec.get('object_revision')]


FK = re.compile(r'\|"(?:[^"\\]|\\.)*"\|')


def relaxed(key):
    """第二轮配对键：去掉 fact_key（含引用名里的 fact_key），用于 fact_key 本身改名（如槽位 TB-02-D→TB-02-W）的同一对象。"""
    k = json.loads(key)
    if isinstance(k, list) and len(k) == 3 and not (isinstance(k[0], str) and k[0].startswith('<')):
        k = k[1:]
    if isinstance(k, list):
        k = [FK.sub('|*|', v) if isinstance(v, str) else v for v in k]
    return json.dumps(k, sort_keys=True, ensure_ascii=False)


class Diff:
    def __init__(self):
        self.count = collections.Counter()
        self.values = collections.defaultdict(list)

    def add(self, scope, fa, fb):
        for f in sorted(set(fa) | set(fb)):
            a, b = fa.get(f, '<absent>'), fb.get(f, '<absent>')
            if a != b:
                key = (scope, re.sub(r'\[\d+\]', '[]', f))
                self.count[key] += 1
                if [a, b] not in self.values[key] and len(self.values[key]) < MAXV:
                    self.values[key].append([a, b])

    def dump(self):
        out = collections.defaultdict(dict)
        for (scope, path), c in sorted(self.count.items()):
            out[scope][path] = {'count': c, 'distinct_old_new': self.values[(scope, path)]}
        return dict(out)


def run(old, new):
    on, nn = names_of(old['rows']), names_of(new['rows'])
    top = Diff()
    for k in TOP:
        top.add(k, flat(resolve(old.get(k), on)), flat(resolve(new.get(k), nn)))
    groups = {}
    for side, proj, names in (('old', old, on), ('new', new, nn)):
        for r in proj['rows']:
            rec = resolve(r['record'], names)
            g = groups.setdefault(kind(r), {'old': collections.Counter(), 'new': collections.Counter(), 'rec': {}})
            s = json.dumps(rec, sort_keys=True, ensure_ascii=False)
            g[side][s] += 1
            g['rec'][s] = (r, rec)
    rows, fields = {}, Diff()
    for k in sorted(groups):
        g = groups[k]
        a, b = g['old'] - g['new'], g['new'] - g['old']
        entry = {'old': sum(g['old'].values()), 'new': sum(g['new'].values()),
                 'identical': sum((g['old'] & g['new']).values()), 'old_only': sum(a.values()), 'new_only': sum(b.values())}
        if a or b:
            ka, kb = collections.defaultdict(list), collections.defaultdict(list)
            for src, dst in ((a, ka), (b, kb)):
                for s in sorted(src.elements()):
                    r, rec = g['rec'][s]
                    dst[json.dumps(ident(r, rec), sort_keys=True, ensure_ascii=False)].append(rec)
            paired = relaxed_pairs = 0
            ra, rb = collections.defaultdict(list), collections.defaultdict(list)
            for key in sorted(set(ka) | set(kb)):
                la, lb = ka.get(key, []), kb.get(key, [])
                for x, y in zip(la, lb):
                    fields.add(k, flat(x), flat(y)); paired += 1
                for extra, dst in ((la[len(lb):], ra), (lb[len(la):], rb)):
                    dst[relaxed(key)].extend((key, x) for x in extra)
            for key in sorted(set(ra) | set(rb)):
                la, lb = ra.get(key, []), rb.get(key, [])
                for (_, x), (_, y) in zip(la, lb):
                    fields.add(k, flat(x), flat(y)); relaxed_pairs += 1
                entry.setdefault('unpaired_old', []).extend(json.loads(i) for i, _ in la[len(lb):])
                entry.setdefault('unpaired_new', []).extend(json.loads(i) for i, _ in lb[len(la):])
            entry = {x: v for x, v in entry.items() if v != []}
            entry['paired'] = paired
            entry['paired_relaxed_without_fact_key'] = relaxed_pairs
        rows[k] = entry
    return {'schema': 'semdiff/2', 'top_level_diffs': top.dump(), 'rows_by_kind': rows, 'row_field_diffs': fields.dump()}


if __name__ == '__main__':
    print(json.dumps(run(load(sys.argv[1]), load(sys.argv[2])), ensure_ascii=False, indent=1, sort_keys=True))
