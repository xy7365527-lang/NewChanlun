"""phi_L: Dependency trees -> typed directed simplicial complex.

Deterministic rules, zero neural networks. The core mapping.
"""

from __future__ import annotations

import sys
from dataclasses import dataclass

from engine import Graph, Vertex, Edge, EdgeType, VertexStatus, compute_beta_1
from nlp_preprocess import preprocess, SentenceTree, Token


# ---------------------------------------------------------------------------
# Verb -> EdgeType classification (deterministic lookup)
# ---------------------------------------------------------------------------

_DEPENDENCY_VERBS = frozenset({
    "requires", "require", "depends", "depend", "needs", "need",
    "caused", "causes", "cause", "follows", "follow",
})

_DEPENDENCY_REVERSE_VERBS = frozenset({
    "produces", "produce", "creates", "create",
    "generates", "generate", "exhibits", "exhibit",
    "leads", "lead", "comes", "come",
})

_NEGATION_VERBS = frozenset({
    "contradicts", "contradict", "incompatible",
})

_IDENTITY_VERBS = frozenset({
    "is", "means", "mean", "defined", "refers",
})

_REFERENCE_VERBS = frozenset({
    "similar", "related", "associated",
})


def _classify_verb(verb_lemma: str) -> tuple[EdgeType, bool]:
    """Return (edge_type, reverse).

    reverse=True means the dependency direction is target->source
    (e.g. "produces" means source depends on target conceptually,
     but we model it as target -> source for DEPENDENCY).
    """
    low = verb_lemma.lower()
    if low in _DEPENDENCY_VERBS:
        return EdgeType.DEPENDENCY, False
    if low in _DEPENDENCY_REVERSE_VERBS:
        return EdgeType.DEPENDENCY, True
    if low in _NEGATION_VERBS:
        return EdgeType.NEGATION, False
    if low in _IDENTITY_VERBS:
        return EdgeType.DEPENDENCY, False
    if low in _REFERENCE_VERBS:
        return EdgeType.REFERENCE, False
    return EdgeType.REFERENCE, False


# ---------------------------------------------------------------------------
# Negation cues
# ---------------------------------------------------------------------------

_NEG_WORDS = frozenset({"not", "never", "cannot", "no", "n't"})
_CONTRAST_CONJ = frozenset({"however", "but", "although", "though", "yet"})


# ---------------------------------------------------------------------------
# Vertex extraction
# ---------------------------------------------------------------------------

_NOUN_DEPS = frozenset({"nsubj", "nsubjpass", "dobj", "pobj", "nmod", "attr", "conj"})


def _normalize(text: str) -> str:
    """Lowercase, strip leading determiners/articles."""
    low = text.lower().strip()
    for prefix in ("all ", "the ", "a ", "an ", "some ", "any ", "every "):
        if low.startswith(prefix):
            low = low[len(prefix):]
    return low.strip()


@dataclass(frozen=True, slots=True)
class _RawVertex:
    label: str          # normalized text
    original: str       # original noun chunk text
    sent_idx: int
    token_idx: int      # root token index within sentence


def extract_vertices(
    trees: list[SentenceTree],
) -> tuple[list[Vertex], dict[str, str]]:
    """Extract concept vertices from dependency trees.

    Returns:
        vertices: list of unique Vertex objects
        token_to_vertex: mapping of (sent_idx, token_idx) -> vertex_id
    """
    raw: list[_RawVertex] = []
    # Track which tokens are covered by noun chunks
    chunk_coverage: dict[tuple[int, int], _RawVertex] = {}

    for si, tree in enumerate(trees):
        for chunk in tree.noun_chunks:
            root_tok = tree.tokens[chunk.root_idx]
            if root_tok.dep in _NOUN_DEPS or root_tok.dep == "ROOT":
                label = _normalize(chunk.text)
                rv = _RawVertex(label=label, original=chunk.text,
                                sent_idx=si, token_idx=chunk.root_idx)
                raw.append(rv)
                for ti in range(chunk.start, chunk.end):
                    chunk_coverage[(si, ti)] = rv

    # Deduplicate by normalized label (string exact match, case-insensitive)
    label_to_id: dict[str, str] = {}
    vertices: list[Vertex] = []
    vid_counter = 0

    for rv in raw:
        if rv.label not in label_to_id:
            vid = f"v{vid_counter}"
            vid_counter += 1
            label_to_id[rv.label] = vid
            vertices.append(Vertex(id=vid, content=rv.label))

    # Build token->vertex mapping (by root token of each chunk)
    token_to_vertex: dict[str, str] = {}
    for rv in raw:
        key = f"{rv.sent_idx}:{rv.token_idx}"
        token_to_vertex[key] = label_to_id[rv.label]

    return vertices, token_to_vertex


# ---------------------------------------------------------------------------
# Edge extraction
# ---------------------------------------------------------------------------

def _find_subject_object(
    tree: SentenceTree,
) -> list[tuple[int, int, int]]:
    """Find (subject_token_idx, verb_token_idx, object_token_idx) triples.

    verb_token_idx points to the content verb for classification. In "cannot
    produce" patterns, the aux/modal is ROOT but the xcomp carries the semantic
    verb — we return the xcomp index so _classify_verb sees "produce".
    """
    triples: list[tuple[int, int, int]] = []

    for tok in tree.tokens:
        if tok.pos == "VERB" or tok.dep == "ROOT":
            verb_idx = tok.idx
            subj_idx: int | None = None
            obj_idx: int | None = None
            xcomp_idx: int | None = None

            for child in tree.tokens:
                if child.head_idx == verb_idx:
                    if child.dep in ("nsubj", "nsubjpass"):
                        subj_idx = child.idx
                    elif child.dep in ("dobj", "attr", "oprd"):
                        obj_idx = child.idx
                    elif child.dep == "prep" and obj_idx is None:
                        # Only use pobj as object when no direct object found
                        for grandchild in tree.tokens:
                            if grandchild.head_idx == child.idx and grandchild.dep == "pobj":
                                obj_idx = grandchild.idx
                                break
                    elif child.dep == "xcomp" and child.pos == "VERB":
                        xcomp_idx = child.idx

            # If ROOT is a modal/aux with xcomp, collect object from xcomp children too
            if obj_idx is None and xcomp_idx is not None:
                for child in tree.tokens:
                    if child.head_idx == xcomp_idx:
                        if child.dep in ("dobj", "attr", "oprd"):
                            obj_idx = child.idx
                        elif child.dep == "prep":
                            for grandchild in tree.tokens:
                                if grandchild.head_idx == child.idx and grandchild.dep == "pobj":
                                    obj_idx = grandchild.idx
                                    break

            if subj_idx is not None and obj_idx is not None:
                # Use xcomp as the semantic verb when ROOT is a modal
                effective_verb = xcomp_idx if xcomp_idx is not None else verb_idx
                triples.append((subj_idx, effective_verb, obj_idx))

    return triples


def _token_vertex(
    sent_idx: int,
    tok_idx: int,
    token_to_vertex: dict[str, str],
    tree: SentenceTree,
) -> str | None:
    """Resolve a token index to a vertex id, climbing to head if needed."""
    key = f"{sent_idx}:{tok_idx}"
    if key in token_to_vertex:
        return token_to_vertex[key]
    # Try the head token (for cases where the dep child isn't the chunk root)
    tok = tree.tokens[tok_idx]
    head_key = f"{sent_idx}:{tok.head_idx}"
    return token_to_vertex.get(head_key)


def _has_negation_child(tree: SentenceTree, verb_idx: int) -> bool:
    """Check if verb has a negation modifier (not, never, cannot, n't).

    Also checks the verb's head — for xcomp patterns like "cannot produce",
    the negation ("cannot") is on the modal ROOT, not on "produce" itself.
    """
    for tok in tree.tokens:
        if tok.head_idx == verb_idx and tok.dep == "neg":
            return True
        if tok.head_idx == verb_idx and tok.word.lower() in _NEG_WORDS:
            return True
    # Check if the verb is an xcomp whose head is a negation-bearing modal
    verb_tok = tree.tokens[verb_idx]
    if verb_tok.dep == "xcomp":
        head_idx = verb_tok.head_idx
        head_tok = tree.tokens[head_idx]
        if head_tok.word.lower() == "cannot":
            return True
        # Check head's children for negation
        for tok in tree.tokens:
            if tok.head_idx == head_idx and tok.dep == "neg":
                return True
            if tok.head_idx == head_idx and tok.word.lower() in _NEG_WORDS:
                return True
    return False


def extract_edges(
    trees: list[SentenceTree],
    token_to_vertex: dict[str, str],
) -> list[Edge]:
    """Extract typed edges from dependency trees."""
    edges: list[Edge] = []
    seen: set[tuple[str, str, str]] = set()

    for si, tree in enumerate(trees):
        triples = _find_subject_object(tree)

        for subj_idx, verb_idx, obj_idx in triples:
            src_vid = _token_vertex(si, subj_idx, token_to_vertex, tree)
            tgt_vid = _token_vertex(si, obj_idx, token_to_vertex, tree)
            if src_vid is None or tgt_vid is None:
                continue
            if src_vid == tgt_vid:
                continue

            verb_word = tree.tokens[verb_idx].word.lower()

            # Check for negation modifier on verb
            if _has_negation_child(tree, verb_idx):
                edge_type = EdgeType.NEGATION
                edge_key = (src_vid, tgt_vid, edge_type.value)
                if edge_key not in seen:
                    seen.add(edge_key)
                    edges.append(Edge(source=src_vid, target=tgt_vid, edge_type=edge_type))
                continue

            edge_type, reverse = _classify_verb(verb_word)
            if reverse:
                src_vid, tgt_vid = tgt_vid, src_vid

            edge_key = (src_vid, tgt_vid, edge_type.value)
            if edge_key not in seen:
                seen.add(edge_key)
                edges.append(Edge(source=src_vid, target=tgt_vid, edge_type=edge_type))

    return edges


def extract_negations(
    trees: list[SentenceTree],
    token_to_vertex: dict[str, str],
) -> list[Edge]:
    """Extract NEGATION edges from contrast conjunctions (however, but, although).

    When two clauses are connected by a contrast conjunction, add a NEGATION
    edge between their subjects.
    """
    edges: list[Edge] = []
    seen: set[tuple[str, str]] = set()

    for si, tree in enumerate(trees):
        for tok in tree.tokens:
            if tok.word.lower() in _CONTRAST_CONJ and tok.dep in ("cc", "advmod", "mark"):
                # Find the two clauses linked by this conjunction
                head_idx = tok.head_idx
                head_tok = tree.tokens[head_idx]

                # Find subjects of the two linked verbs
                conj_subjects: list[str] = []

                # Subject of head verb
                for child in tree.tokens:
                    if child.head_idx == head_idx and child.dep in ("nsubj", "nsubjpass"):
                        vid = _token_vertex(si, child.idx, token_to_vertex, tree)
                        if vid:
                            conj_subjects.append(vid)
                        break

                # Find the other verb in the conjunction
                for child in tree.tokens:
                    if child.head_idx == head_idx and child.dep == "conj":
                        for gc in tree.tokens:
                            if gc.head_idx == child.idx and gc.dep in ("nsubj", "nsubjpass"):
                                vid = _token_vertex(si, gc.idx, token_to_vertex, tree)
                                if vid:
                                    conj_subjects.append(vid)
                                break
                        break

                if len(conj_subjects) >= 2:
                    s, t = conj_subjects[0], conj_subjects[1]
                    if s != t and (s, t) not in seen and (t, s) not in seen:
                        seen.add((s, t))
                        edges.append(Edge(source=s, target=t, edge_type=EdgeType.NEGATION))

    return edges


# ---------------------------------------------------------------------------
# Cross-sentence merging
# ---------------------------------------------------------------------------

def merge_cross_sentence(
    vertices: list[Vertex],
    edges: list[Edge],
) -> tuple[list[Vertex], list[Edge]]:
    """Merge vertices with identical normalized content across sentences.

    Already handled by extract_vertices (dedup by label), so this is a no-op
    in the current design. Kept as explicit pipeline stage for clarity.
    """
    return vertices, edges


# ---------------------------------------------------------------------------
# Full pipeline
# ---------------------------------------------------------------------------

def phi_L(text: str) -> Graph:
    """Complete pipeline: text -> typed directed simplicial complex."""
    trees = preprocess(text)
    vertices, token_to_vertex = extract_vertices(trees)
    edges = extract_edges(trees, token_to_vertex)
    neg_edges = extract_negations(trees, token_to_vertex)

    all_edges = edges + neg_edges
    vertices, all_edges = merge_cross_sentence(vertices, all_edges)

    # Deduplicate edges
    seen: set[tuple[str, str, str]] = set()
    deduped: list[Edge] = []
    for e in all_edges:
        key = (e.source, e.target, e.edge_type.value)
        if key not in seen:
            seen.add(key)
            deduped.append(e)

    # Build Graph
    g = Graph()
    for v in vertices:
        g = g.add_vertex(v)
    for e in deduped:
        # Only add edge if both endpoints exist
        if g.vertex(e.source) is not None and g.vertex(e.target) is not None:
            g = g.add_edge(e)

    return g


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def _format_graph(g: Graph) -> str:
    """Format graph for display."""
    lines: list[str] = []
    verts = g.vertices
    edges = g.edges

    lines.append(f"Vertices ({len(verts)}):")
    for vid, v in sorted(verts.items()):
        lines.append(f"  {vid}: {v.content!r}")

    lines.append(f"\nEdges ({len(edges)}):")
    for e in edges:
        src_content = verts[e.source].content if e.source in verts else e.source
        tgt_content = verts[e.target].content if e.target in verts else e.target
        lines.append(f"  {src_content!r} --[{e.edge_type.value}]--> {tgt_content!r}")

    beta = compute_beta_1(g)
    lines.append(f"\nbeta_1 = {beta}")

    return "\n".join(lines)


if __name__ == "__main__":
    test_sentences = [
        "All complex systems exhibit emergent behavior.",
        "Emergence requires interaction between components.",
        "Component interaction follows deterministic rules.",
        "Deterministic rules cannot produce true novelty.",
        "Complex systems produce genuinely novel outcomes.",
    ]

    if len(sys.argv) > 1:
        text = " ".join(sys.argv[1:])
    else:
        text = " ".join(test_sentences)

    print(f"Input: {text!r}\n")
    g = phi_L(text)
    print(_format_graph(g))
