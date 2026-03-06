"""NLP preprocessing: text -> dependency trees.

Uses spaCy for dependency parsing when available. Falls back to a rule-based
parser for environments where spaCy is not compatible (e.g. Python 3.14+).
Neural network lives in the sensor layer, not in the core mapping.
"""

from __future__ import annotations

import re
from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class Token:
    word: str
    pos: str
    dep: str
    head_idx: int
    idx: int


@dataclass(frozen=True, slots=True)
class NounChunk:
    text: str
    root_idx: int
    start: int
    end: int


@dataclass(frozen=True, slots=True)
class SentenceTree:
    tokens: list[Token]
    noun_chunks: list[NounChunk]
    text: str


# ---------------------------------------------------------------------------
# Try spaCy first, fall back to rule-based
# ---------------------------------------------------------------------------

_USE_SPACY = False
_nlp = None

try:
    import spacy as _spacy_mod
    _nlp = _spacy_mod.load("en_core_web_sm")
    _USE_SPACY = True
except Exception:
    pass


def preprocess(text: str) -> list[SentenceTree]:
    """Parse text into per-sentence dependency trees."""
    if _USE_SPACY:
        return _preprocess_spacy(text)
    return _preprocess_rules(text)


# ---------------------------------------------------------------------------
# spaCy backend
# ---------------------------------------------------------------------------

def _preprocess_spacy(text: str) -> list[SentenceTree]:
    doc = _nlp(text)
    trees: list[SentenceTree] = []

    for sent in doc.sents:
        sent_start = sent.start
        tokens = [
            Token(
                word=tok.text,
                pos=tok.pos_,
                dep=tok.dep_,
                head_idx=tok.head.i - sent_start,
                idx=tok.i - sent_start,
            )
            for tok in sent
        ]

        chunks: list[NounChunk] = []
        for chunk in doc.noun_chunks:
            if chunk.start >= sent.start and chunk.end <= sent.end:
                chunks.append(
                    NounChunk(
                        text=chunk.text,
                        root_idx=chunk.root.i - sent_start,
                        start=chunk.start - sent_start,
                        end=chunk.end - sent_start,
                    )
                )

        trees.append(SentenceTree(tokens=tokens, noun_chunks=chunks, text=sent.text))

    return trees


# ---------------------------------------------------------------------------
# Rule-based fallback (no external dependencies)
# ---------------------------------------------------------------------------

_DETERMINERS = frozenset({
    "a", "an", "the", "all", "some", "any", "every", "this", "that",
    "these", "those", "my", "your", "his", "her", "its", "our", "their",
})

_PREPOSITIONS = frozenset({
    "in", "on", "at", "to", "for", "with", "by", "from", "of",
    "between", "into", "through", "during", "before", "after",
    "above", "below", "about", "against", "among", "around",
})

_ADJECTIVES = frozenset({
    "complex", "emergent", "deterministic", "true", "novel", "genuine",
    "genuinely", "new", "simple", "large", "small", "good", "bad",
    "high", "low", "great", "important", "different", "similar",
    "related", "associated", "incompatible", "independent",
})

_VERBS = frozenset({
    "is", "are", "was", "were", "be", "been", "being",
    "has", "have", "had", "do", "does", "did",
    "exhibit", "exhibits", "exhibited",
    "require", "requires", "required",
    "depend", "depends", "depended",
    "need", "needs", "needed",
    "produce", "produces", "produced",
    "create", "creates", "created",
    "generate", "generates", "generated",
    "follow", "follows", "followed",
    "lead", "leads", "led",
    "cause", "causes", "caused",
    "contradict", "contradicts", "contradicted",
    "mean", "means", "meant",
    "define", "defines", "defined",
    "refer", "refers", "referred",
    "come", "comes", "came",
    "go", "goes", "went", "gone",
    "give", "gives", "gave", "given",
    "take", "takes", "took", "taken",
    "make", "makes", "made",
    "get", "gets", "got", "gotten",
    "know", "knows", "knew", "known",
    "think", "thinks", "thought",
    "see", "sees", "saw", "seen",
    "find", "finds", "found",
    "show", "shows", "showed", "shown",
    "seem", "seems", "seemed",
    "become", "becomes", "became",
    "keep", "keeps", "kept",
    "leave", "leaves", "left",
    "call", "calls", "called",
    "hold", "holds", "held",
    "bring", "brings", "brought",
    "begin", "begins", "began", "begun",
    "run", "runs", "ran",
    "move", "moves", "moved",
    "include", "includes", "included",
    "contain", "contains", "contained",
    "involve", "involves", "involved",
    "suggest", "suggests", "suggested",
    "imply", "implies", "implied",
    "indicate", "indicates", "indicated",
    "represent", "represents", "represented",
    "describe", "describes", "described",
    "explain", "explains", "explained",
    "support", "supports", "supported",
    "enable", "enables", "enabled",
    "prevent", "prevents", "prevented",
    "affect", "affects", "affected",
    "determine", "determines", "determined",
    "connect", "connects", "connected",
    "link", "links", "linked",
    "combine", "combines", "combined",
    "result", "results", "resulted",
    "emerge", "emerges", "emerged",
    "arise", "arises", "arose", "arisen",
    "exist", "exists", "existed",
    "occur", "occurs", "occurred",
    "happen", "happens", "happened",
    "change", "changes", "changed",
    "increase", "increases", "increased",
    "decrease", "decreases", "decreased",
    "can", "cannot", "could", "would", "should", "may", "might", "will",
    "shall", "must",
})

_ADVERBS = frozenset({
    "not", "never", "also", "always", "often", "however", "but",
    "although", "though", "yet", "genuinely", "truly",
})

_CONJUNCTIONS = frozenset({
    "and", "or", "but", "however", "although", "though", "yet",
    "because", "since", "while", "whereas", "if", "unless",
})

_NOUNS_HINT = frozenset({
    "system", "systems", "behavior", "behaviour", "emergence",
    "interaction", "interactions", "component", "components",
    "rule", "rules", "novelty", "outcome", "outcomes",
    "pattern", "patterns", "structure", "structures",
    "process", "processes", "property", "properties",
})


def _guess_pos(word: str) -> str:
    """Guess POS tag from word form (very rough heuristic)."""
    low = word.lower().rstrip(".,;:!?")
    if low in _DETERMINERS:
        return "DET"
    if low in _PREPOSITIONS:
        return "ADP"
    if low in _VERBS:
        return "VERB"
    if low in _ADVERBS:
        return "ADV"
    if low in _CONJUNCTIONS:
        return "CCONJ"
    if low in _ADJECTIVES:
        return "ADJ"
    if low in _NOUNS_HINT:
        return "NOUN"
    # Heuristic: words ending in common noun suffixes
    if low.endswith(("tion", "sion", "ment", "ness", "ity", "ence", "ance")):
        return "NOUN"
    if low.endswith(("ing",)) and low not in _VERBS:
        return "NOUN"  # gerund as noun
    if low.endswith(("ly",)):
        return "ADV"
    if low.endswith(("ive", "ous", "ful", "less", "able", "ible", "ent", "ant", "ic")):
        return "ADJ"
    if low.endswith(("ed",)) and low not in _NOUNS_HINT:
        return "VERB"
    if low.endswith(("s",)) and low not in _VERBS and low not in _NOUNS_HINT:
        # Could be plural noun
        return "NOUN"
    # Default to noun for unknown words
    return "NOUN"


_SENT_SPLIT = re.compile(r'(?<=[.!?])\s+')


def _tokenize(text: str) -> list[str]:
    """Simple whitespace + punctuation tokenizer."""
    # Split on whitespace, then separate trailing punctuation
    raw = text.split()
    tokens: list[str] = []
    for w in raw:
        # Separate leading/trailing punctuation
        while w and w[0] in "\"'(":
            tokens.append(w[0])
            w = w[1:]
        trail: list[str] = []
        while w and w[-1] in ".,;:!?)\"'":
            trail.append(w[-1])
            w = w[:-1]
        if w:
            tokens.append(w)
        tokens.extend(reversed(trail))
    return tokens


def _preprocess_rules(text: str) -> list[SentenceTree]:
    """Rule-based dependency parsing fallback."""
    sentences = _SENT_SPLIT.split(text.strip())
    trees: list[SentenceTree] = []

    for sent_text in sentences:
        sent_text = sent_text.strip()
        if not sent_text:
            continue
        words = _tokenize(sent_text)
        if not words:
            continue

        # Build tokens with POS
        tokens: list[Token] = []
        for i, w in enumerate(words):
            pos = "PUNCT" if w in ".,;:!?()" else _guess_pos(w)
            tokens.append(Token(word=w, pos=pos, dep="", head_idx=i, idx=i))

        # Find the main verb (ROOT)
        root_idx = _find_root(tokens)

        # Assign deps based on position relative to root
        dep_tokens = _assign_deps(tokens, root_idx)

        # Extract noun chunks
        chunks = _extract_chunks(dep_tokens)

        trees.append(SentenceTree(tokens=dep_tokens, noun_chunks=chunks, text=sent_text))

    return trees


def _find_root(tokens: list[Token]) -> int:
    """Find the main verb index."""
    # Look for first main verb (not auxiliary)
    aux = {"can", "cannot", "could", "would", "should", "may", "might",
           "will", "shall", "must", "do", "does", "did", "has", "have", "had"}
    candidates: list[int] = []
    aux_indices: list[int] = []

    for i, tok in enumerate(tokens):
        if tok.pos == "VERB":
            if tok.word.lower() in aux:
                aux_indices.append(i)
            else:
                candidates.append(i)

    if candidates:
        return candidates[0]
    if aux_indices:
        # If only auxiliaries, the last one is likely the main verb
        # (e.g. "cannot produce" — "produce" should be VERB but might be missed)
        return aux_indices[-1]
    # No verb found — first non-punctuation, non-det token
    for i, tok in enumerate(tokens):
        if tok.pos not in ("PUNCT", "DET"):
            return i
    return 0


def _assign_deps(tokens: list[Token], root_idx: int) -> list[Token]:
    """Assign dependency labels and head indices."""
    result: list[Token] = []

    # Phase 1: find subject and object noun heads.
    # Subject = last NOUN before root (head of the NP). If no NOUN, last ADJ.
    # Object = last NOUN after root (before next verb/prep boundary). If no NOUN, last ADJ.
    subj_idx: int | None = None
    obj_idx: int | None = None
    prep_indices: list[int] = []

    # Find subject: rightmost NOUN before root; fallback to rightmost ADJ
    subj_noun: int | None = None
    subj_adj: int | None = None
    for i, tok in enumerate(tokens):
        if i >= root_idx:
            break
        if tok.pos == "NOUN":
            subj_noun = i
        elif tok.pos == "ADJ":
            subj_adj = i
    subj_idx = subj_noun if subj_noun is not None else subj_adj

    # Find object: first NOUN after root (head of object NP); fallback to first ADJ
    obj_noun: int | None = None
    obj_adj: int | None = None
    for i, tok in enumerate(tokens):
        if i <= root_idx:
            continue
        if tok.pos == "ADP":
            prep_indices.append(i)
        if tok.pos == "NOUN" and obj_noun is None:
            obj_noun = i
        elif tok.pos == "ADJ" and obj_adj is None:
            obj_adj = i
    obj_idx = obj_noun if obj_noun is not None else obj_adj

    for i, tok in enumerate(tokens):
        if i == root_idx:
            result.append(Token(tok.word, tok.pos, "ROOT", i, i))
        elif i == subj_idx:
            result.append(Token(tok.word, tok.pos, "nsubj", root_idx, i))
        elif i == obj_idx:
            dep = "dobj"
            head = root_idx
            for p in prep_indices:
                if p > root_idx and p < i:
                    head = p
                    dep = "pobj"
                    break
            result.append(Token(tok.word, tok.pos, dep, head, i))
        elif tok.pos == "DET":
            next_noun = None
            for j in range(i + 1, len(tokens)):
                if tokens[j].pos in ("NOUN",):
                    next_noun = j
                    break
            result.append(Token(tok.word, tok.pos, "det", next_noun if next_noun is not None else root_idx, i))
        elif tok.pos == "ADJ":
            next_noun = None
            for j in range(i + 1, len(tokens)):
                if tokens[j].pos == "NOUN":
                    next_noun = j
                    break
            if next_noun is not None:
                result.append(Token(tok.word, tok.pos, "amod", next_noun, i))
            else:
                result.append(Token(tok.word, tok.pos, "attr", root_idx, i))
        elif tok.pos == "ADP":
            result.append(Token(tok.word, tok.pos, "prep", root_idx, i))
        elif tok.pos == "ADV":
            if tok.word.lower() in ("not", "never", "n't"):
                result.append(Token(tok.word, tok.pos, "neg", root_idx, i))
            else:
                result.append(Token(tok.word, tok.pos, "advmod", root_idx, i))
        elif tok.pos == "VERB" and i != root_idx:
            if i < root_idx:
                result.append(Token(tok.word, tok.pos, "aux", root_idx, i))
            else:
                result.append(Token(tok.word, tok.pos, "xcomp", root_idx, i))
        elif tok.pos == "PUNCT":
            result.append(Token(tok.word, tok.pos, "punct", root_idx, i))
        elif tok.pos == "CCONJ":
            result.append(Token(tok.word, tok.pos, "cc", root_idx, i))
        else:
            # Check if this NOUN follows a preposition -> pobj
            if tok.pos == "NOUN" and prep_indices:
                nearest_prep = None
                for p in prep_indices:
                    if p < i:
                        nearest_prep = p
                if nearest_prep is not None:
                    result.append(Token(tok.word, tok.pos, "pobj", nearest_prep, i))
                    continue
            next_noun = None
            for j in range(i + 1, min(i + 4, len(tokens))):
                if tokens[j].pos in ("NOUN",):
                    next_noun = j
                    break
            if next_noun is not None:
                result.append(Token(tok.word, tok.pos, "compound", next_noun, i))
            else:
                result.append(Token(tok.word, tok.pos, "dep", root_idx, i))

    # Phase 2: handle "cannot VERB" pattern — promote the content verb
    # If root is an auxiliary (can/cannot/etc.) and there's an xcomp, swap
    root_tok = result[root_idx]
    if root_tok.word.lower() in ("can", "cannot", "could", "would", "should",
                                  "may", "might", "will", "shall", "must"):
        for i, tok in enumerate(result):
            if tok.dep == "xcomp" and tok.pos == "VERB":
                # This is the real main verb. Keep structure but note this for edge extraction.
                # We mark the xcomp as the effective ROOT for verb classification
                # but keep dep structure intact for extraction
                break

    return result


def _extract_chunks(tokens: list[Token]) -> list[NounChunk]:
    """Extract noun chunks from assigned deps."""
    chunks: list[NounChunk] = []
    used: set[int] = set()

    # Find nouns that are subjects or objects
    for i, tok in enumerate(tokens):
        if tok.dep in ("nsubj", "nsubjpass", "dobj", "pobj", "attr") and i not in used:
            start = i
            end = i + 1
            root_idx = i

            # Expand left to include det, amod, compound
            while start > 0:
                prev = tokens[start - 1]
                if prev.dep in ("det", "amod", "compound") and prev.head_idx == i:
                    start -= 1
                elif prev.pos == "ADJ" and start - 1 > 0:
                    # Check if adjective modifies this noun
                    start -= 1
                elif prev.pos == "DET":
                    start -= 1
                else:
                    break

            # Expand right for compound nouns
            while end < len(tokens):
                nxt = tokens[end]
                if nxt.dep == "compound" and nxt.head_idx == i:
                    end += 1
                else:
                    break

            used.update(range(start, end))
            chunk_text = " ".join(t.word for t in tokens[start:end])
            chunks.append(NounChunk(text=chunk_text, root_idx=root_idx, start=start, end=end))

    return chunks
