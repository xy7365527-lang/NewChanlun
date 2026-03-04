#!/usr/bin/env python3
"""Block topology cold-read analysis for v121-swarm."""
import json
from collections import defaultdict, Counter

# Build reverse ID mapping (hash -> short id)
with open('.chanlun/block-topology/meta.json', 'r', encoding='utf-8') as f:
    meta = json.load(f)
rev_map = {v: k for k, v in meta['id_mapping'].items()}
fwd_map = meta['id_mapping']  # short_id -> hash

def resolve_node(val):
    """Resolve a node reference to a hash. Could be a hash or a short id."""
    if val in rev_map:
        return val  # already a hash
    if val in fwd_map:
        return fwd_map[val]  # short id -> hash
    return val  # unknown, keep as-is

def get_relation_type(r):
    """Extract relation type from various schema formats."""
    if 'relation' in r:
        return r['relation']
    if 'type' in r:
        return r['type']
    return 'unknown'

def get_from_to(r):
    """Extract source and target from various schema formats."""
    src = r.get('from') or r.get('source')
    tgt = r.get('to') or r.get('target')
    return resolve_node(str(src)), resolve_node(str(tgt))

# Parse all relations
relations = []
with open('.chanlun/block-topology/relations.jsonl', 'r', encoding='utf-8') as f:
    for line in f:
        line = line.strip()
        if line:
            relations.append(json.loads(line))

print(f'Total blocks: {meta["block_count"]}')
print(f'Total relations: {len(relations)}')

# Count relation types
rel_types = Counter(get_relation_type(r) for r in relations)
print(f'\n=== Relation types ===')
for rt, cnt in rel_types.most_common():
    print(f'  {rt}: {cnt}')

# Schema variants
schema_variants = Counter()
for r in relations:
    schema_variants[frozenset(r.keys())] += 1
print(f'\n=== Schema variants ===')
for ks, cnt in schema_variants.most_common():
    print(f'  {sorted(ks)}: {cnt}')

# Build adjacency structures
out_degree = defaultdict(int)
in_degree = defaultdict(int)
out_by_type = defaultdict(lambda: defaultdict(int))
in_by_type = defaultdict(lambda: defaultdict(int))

# Track all nodes from id_mapping
all_nodes = set(meta['id_mapping'].values())

# Track edges for duplicate detection
edge_set = defaultdict(list)

for r in relations:
    src, tgt = get_from_to(r)
    rel = get_relation_type(r)
    out_degree[src] += 1
    in_degree[tgt] += 1
    out_by_type[src][rel] += 1
    in_by_type[tgt][rel] += 1
    edge_set[(src, tgt)].append(rel)

# Find multi-edges (same src, same tgt, multiple relations)
multi_edges = {k: v for k, v in edge_set.items() if len(v) > 1}
print(f'\n=== Multi-edges (same source->target, multiple edge types) ===')
print(f'Count: {len(multi_edges)}')
for (src, tgt), rels in sorted(multi_edges.items(), key=lambda x: -len(x[1]))[:25]:
    s = rev_map.get(src, src[:12])
    t = rev_map.get(tgt, tgt[:12])
    print(f'  {s} -> {t}: {rels}')

# Same-type duplicate edges
same_type_dupes = {}
for k, v in edge_set.items():
    if len(v) != len(set(v)):
        same_type_dupes[k] = v
print(f'\n=== Same-type duplicate edges (exact duplicates) ===')
print(f'Count: {len(same_type_dupes)}')
for (src, tgt), rels in sorted(same_type_dupes.items(), key=lambda x: -len(x[1]))[:15]:
    s = rev_map.get(src, src[:12])
    t = rev_map.get(tgt, tgt[:12])
    print(f'  {s} -> {t}: {rels}')

# Hub nodes: top out-degree
print(f'\n=== Top 25 out-degree (hub sources) ===')
for h, d in sorted(out_degree.items(), key=lambda x: -x[1])[:25]:
    sid = rev_map.get(h, h[:12])
    types = dict(out_by_type[h])
    print(f'  {sid}: out={d}  {types}')

# Hub nodes: top in-degree
print(f'\n=== Top 25 in-degree (hub targets) ===')
for h, d in sorted(in_degree.items(), key=lambda x: -x[1])[:25]:
    sid = rev_map.get(h, h[:12])
    types = dict(in_by_type[h])
    print(f'  {sid}: in={d}  {types}')

# Isolated nodes
nodes_with_edges = set(out_degree.keys()) | set(in_degree.keys())
isolated = all_nodes - nodes_with_edges
print(f'\n=== Isolated nodes (no edges at all) ===')
print(f'Count: {len(isolated)}')
for h in sorted(isolated, key=lambda x: rev_map.get(x, x)):
    sid = rev_map.get(h, h[:12])
    print(f'  {sid}')

# Leaf nodes (in-degree > 0, out-degree = 0)
leaves = [h for h in all_nodes if in_degree.get(h, 0) > 0 and out_degree.get(h, 0) == 0]
print(f'\n=== Leaf nodes (incoming edges only, no outgoing) ===')
print(f'Count: {len(leaves)}')

# Root nodes (out-degree > 0, in-degree = 0)
roots = [h for h in all_nodes if out_degree.get(h, 0) > 0 and in_degree.get(h, 0) == 0]
print(f'\n=== Root nodes (outgoing edges only, no incoming) ===')
print(f'Count: {len(roots)}')
for h in sorted(roots, key=lambda x: rev_map.get(x, x)):
    sid = rev_map.get(h, h[:12])
    print(f'  {sid} (out={out_degree[h]})')

# Self-loops
self_loops = []
for r in relations:
    src, tgt = get_from_to(r)
    if src == tgt:
        self_loops.append((src, get_relation_type(r)))
print(f'\n=== Self-loops ===')
print(f'Count: {len(self_loops)}')
for h, rel in self_loops:
    sid = rev_map.get(h, h[:12])
    print(f'  {sid}: {rel}')

# negates edges analysis
negates = []
for r in relations:
    if get_relation_type(r) == 'negates':
        src, tgt = get_from_to(r)
        negates.append((src, tgt))
print(f'\n=== negates edges ({len(negates)} total) ===')
negates_sources = Counter(rev_map.get(s, s[:12]) for s, _ in negates)
negates_targets = Counter(rev_map.get(t, t[:12]) for _, t in negates)
print('Top negates sources (nodes that negate others):')
for s, c in negates_sources.most_common(15):
    print(f'  {s}: {c}')
print('Top negates targets (nodes being negated):')
for t, c in negates_targets.most_common(15):
    print(f'  {t}: {c}')

# negates edge list (full)
print('\nFull negates edge list:')
for src, tgt in negates:
    s = rev_map.get(src, src[:12])
    t = rev_map.get(tgt, tgt[:12])
    print(f'  {s} --negates--> {t}')

# Reciprocal edges (A->B and B->A, potential 2-cycles, excluding self-loops)
forward_edges = set()
reciprocal = []
for r in relations:
    src, tgt = get_from_to(r)
    if src != tgt:
        pair = (src, tgt)
        reverse = (tgt, src)
        if reverse in forward_edges:
            s1 = rev_map.get(src, src[:12])
            s2 = rev_map.get(tgt, tgt[:12])
            fwd_types = edge_set.get(reverse, [])
            bwd_types = edge_set.get(pair, [])
            reciprocal.append((s1, s2, bwd_types, fwd_types))
        forward_edges.add(pair)

print(f'\n=== Reciprocal edges (A<->B, potential 2-cycles) ===')
print(f'Count: {len(reciprocal)}')
for s1, s2, t1, t2 in sorted(reciprocal, key=lambda x: (x[0], x[1]))[:40]:
    print(f'  {s1} <-> {s2}: fwd={t1}, rev={t2}')

# Degree distribution stats
all_out = [out_degree.get(h, 0) for h in all_nodes]
all_in = [in_degree.get(h, 0) for h in all_nodes]
print(f'\n=== Degree distribution stats ===')
print(f'Out-degree: min={min(all_out)}, max={max(all_out)}, mean={sum(all_out)/len(all_out):.2f}, median={sorted(all_out)[len(all_out)//2]}')
print(f'In-degree:  min={min(all_in)}, max={max(all_in)}, mean={sum(all_in)/len(all_in):.2f}, median={sorted(all_in)[len(all_in)//2]}')

# Out-degree histogram
out_hist = Counter(all_out)
print('\nOut-degree histogram:')
for deg in sorted(out_hist.keys()):
    bar = "#" * min(out_hist[deg], 60)
    print(f'  {deg:3d}: {bar} ({out_hist[deg]})')

# In-degree histogram
in_hist = Counter(all_in)
print('\nIn-degree histogram:')
for deg in sorted(in_hist.keys()):
    bar = "#" * min(in_hist[deg], 60)
    print(f'  {deg:3d}: {bar} ({in_hist[deg]})')

# Phantom nodes (in relations but not in id_mapping)
phantom_nodes = (set(out_degree.keys()) | set(in_degree.keys())) - all_nodes
print(f'\n=== Phantom nodes (in relations but not in id_mapping) ===')
print(f'Count: {len(phantom_nodes)}')
for h in sorted(phantom_nodes, key=lambda x: rev_map.get(x, x)):
    sid = rev_map.get(h, h[:16])
    # check degrees
    print(f'  {sid} (in={in_degree.get(h,0)}, out={out_degree.get(h,0)})')

# Total degree (in + out) ranking
total_degree = {}
for h in all_nodes | nodes_with_edges:
    total_degree[h] = in_degree.get(h, 0) + out_degree.get(h, 0)
print(f'\n=== Top 30 total degree ===')
for h, d in sorted(total_degree.items(), key=lambda x: -x[1])[:30]:
    sid = rev_map.get(h, h[:12])
    print(f'  {sid}: total={d} (in={in_degree.get(h,0)}, out={out_degree.get(h,0)})')

# Edge density
n = len(all_nodes)
possible_edges = n * (n - 1)  # directed graph
actual_unique_edges = len(edge_set)
print(f'\n=== Graph density ===')
print(f'Nodes: {n}')
print(f'Unique directed edges: {actual_unique_edges}')
print(f'Possible directed edges: {possible_edges}')
print(f'Density: {actual_unique_edges / possible_edges:.6f}')
print(f'Average edges per node: {len(relations) / n:.2f}')

# Connected components (treating as undirected)
adj_undirected = defaultdict(set)
for (src, tgt) in edge_set:
    adj_undirected[src].add(tgt)
    adj_undirected[tgt].add(src)

visited = set()
components = []
for node in all_nodes:
    if node not in visited:
        # BFS
        component = set()
        queue = [node]
        while queue:
            current = queue.pop(0)
            if current in visited:
                continue
            visited.add(current)
            component.add(current)
            for neighbor in adj_undirected.get(current, set()):
                if neighbor not in visited:
                    queue.append(neighbor)
        components.append(component)

components.sort(key=len, reverse=True)
print(f'\n=== Connected components (undirected) ===')
print(f'Number of components: {len(components)}')
for i, comp in enumerate(components):
    members = sorted([rev_map.get(h, h[:12]) for h in comp])
    if len(comp) <= 10:
        print(f'  Component {i+1} (size={len(comp)}): {members}')
    else:
        print(f'  Component {i+1} (size={len(comp)}): [{members[0]}..{members[-1]}]')
