"""Chinese NLP preprocessing: text -> dependency trees.

Pure Python, no jieba/pkuseg dependency. Uses punctuation-based sentence splitting,
stopword filtering, and pattern-based concept extraction for Chinese text
(primarily 缠论 original texts).
"""

from __future__ import annotations

import re
from dataclasses import dataclass

from nlp_preprocess import Token, NounChunk, SentenceTree


# ---------------------------------------------------------------------------
# Chinese stopwords (function words / particles)
# ---------------------------------------------------------------------------

_ZH_STOPWORDS = frozenset({
    "的", "了", "是", "在", "和", "与", "或", "但", "而", "也",
    "又", "不", "没", "将", "把", "被", "让", "给", "着", "过",
    "得", "地", "都", "就", "才", "要", "会", "能", "可", "以",
    "这", "那", "有", "对", "从", "到", "为", "所", "其", "之",
    "于", "则", "若", "如", "因", "由", "等", "中", "上", "下",
    "来", "去", "做", "它", "他", "她", "们", "我", "你",
    "终", "已", "曾", "正", "再", "还", "很", "最", "更",
    "任何", "每", "各", "某", "此", "该",
    # Common verbs that act as structural connectors (not concepts)
    "包含", "属于", "构成", "形成", "产生", "存在", "表现",
    "决定", "导致", "依赖", "需要", "意味", "说明", "表示",
})

# ---------------------------------------------------------------------------
# Negation cues
# ---------------------------------------------------------------------------

_ZH_NEGATION_WORDS = frozenset({
    "不", "非", "没有", "无法", "不能", "不是", "未", "无",
    "没", "莫", "勿", "别", "不会", "不可", "不可能",
})

# ---------------------------------------------------------------------------
# Contrastive conjunctions
# ---------------------------------------------------------------------------

_ZH_CONTRAST_CONJ = frozenset({
    "但是", "然而", "但", "却", "不过", "虽然", "可是",
    "尽管", "即使", "虽", "反而",
})

# ---------------------------------------------------------------------------
# Sentence splitting
# ---------------------------------------------------------------------------

_ZH_SENT_DELIMITERS = re.compile(r'[。！？；\n]+')


def _split_sentences(text: str) -> list[str]:
    """Split Chinese text into sentences by punctuation."""
    parts = _ZH_SENT_DELIMITERS.split(text.strip())
    return [s.strip() for s in parts if s.strip()]


# ---------------------------------------------------------------------------
# Character classification
# ---------------------------------------------------------------------------

def _is_cjk(char: str) -> bool:
    """Check if a character is CJK unified ideograph."""
    cp = ord(char)
    return (
        (0x4E00 <= cp <= 0x9FFF)
        or (0x3400 <= cp <= 0x4DBF)
        or (0x20000 <= cp <= 0x2A6DF)
        or (0x2A700 <= cp <= 0x2B73F)
        or (0x2B740 <= cp <= 0x2B81F)
        or (0xF900 <= cp <= 0xFAFF)
    )


def _is_zh_punct(char: str) -> bool:
    """Check if character is Chinese or ASCII punctuation."""
    return char in "，。！？；：、（）【】《》""''「」『』…—·,.!?;:()[]<>\"\'-"


# ---------------------------------------------------------------------------
# Concept extraction
# ---------------------------------------------------------------------------

def _extract_concept_fragments(sentence: str) -> list[str]:
    """Extract concept candidate fragments from a Chinese sentence.

    Strategy:
    1. Remove punctuation
    2. Split on stopwords to get continuous content-word runs
    3. From each run, extract 2-6 character fragments as concept candidates
    4. Also keep the full run if 2-6 chars
    """
    # Remove punctuation
    cleaned = ""
    for ch in sentence:
        if _is_zh_punct(ch):
            cleaned += " "
        else:
            cleaned += ch

    # Split into character runs (sequences of CJK characters)
    runs: list[str] = []
    current_run = ""
    for ch in cleaned:
        if _is_cjk(ch):
            current_run += ch
        else:
            if current_run:
                runs.append(current_run)
                current_run = ""
            # Non-CJK non-punct: could be space or letter, skip
    if current_run:
        runs.append(current_run)

    # Split each run on stopwords to get content-word fragments
    fragments: list[str] = []
    for run in runs:
        _split_on_stopwords(run, fragments)

    return fragments


def _split_on_stopwords(run: str, out: list[str]) -> None:
    """Split a CJK character run on stopwords, keep 2-6 char fragments."""
    # First handle multi-char stopwords by replacing them with separator
    processed = run
    multi_char_stops = sorted(
        [s for s in _ZH_STOPWORDS if len(s) > 1],
        key=len, reverse=True,
    )
    for stop in multi_char_stops:
        processed = processed.replace(stop, "\x00")

    # Then split on single-char stopwords
    current = ""
    for ch in processed:
        if ch == "\x00" or ch in _ZH_STOPWORDS:
            if len(current) >= 2:
                out.append(current)
            current = ""
        else:
            current += ch
    if len(current) >= 2:
        out.append(current)


def _detect_negation_in_sentence(sentence: str) -> list[tuple[int, str]]:
    """Find negation positions in sentence, return (char_position, negation_word)."""
    results: list[tuple[int, str]] = []
    # Check multi-char negations first (longest match)
    for neg in sorted(_ZH_NEGATION_WORDS, key=len, reverse=True):
        start = 0
        while True:
            idx = sentence.find(neg, start)
            if idx < 0:
                break
            # Avoid double-counting overlapping matches
            results.append((idx, neg))
            start = idx + len(neg)
    # Deduplicate by position
    results.sort(key=lambda x: x[0])
    deduped: list[tuple[int, str]] = []
    covered = set()
    for pos, word in results:
        positions = set(range(pos, pos + len(word)))
        if not positions & covered:
            deduped.append((pos, word))
            covered |= positions
    return deduped


def _detect_contrast(sentence: str) -> list[int]:
    """Find contrastive conjunction positions in sentence."""
    positions: list[int] = []
    for conj in sorted(_ZH_CONTRAST_CONJ, key=len, reverse=True):
        idx = sentence.find(conj)
        if idx >= 0:
            positions.append(idx)
    return sorted(positions)


# ---------------------------------------------------------------------------
# Build SentenceTree from Chinese sentence
# ---------------------------------------------------------------------------

def _build_sentence_tree(sentence: str, sent_idx: int) -> SentenceTree:
    """Build a SentenceTree from a Chinese sentence.

    Since we don't have a real dependency parser for Chinese, we simulate
    the structure that phi_L.py expects:
    - Tokens with pos/dep labels
    - NounChunks pointing to concept vertices
    - Negation and contrast information encoded in dep labels
    """
    concepts = _extract_concept_fragments(sentence)
    negations = _detect_negation_in_sentence(sentence)
    contrasts = _detect_contrast(sentence)

    if not concepts:
        return SentenceTree(tokens=[], noun_chunks=[], text=sentence)

    # Build tokens: each concept becomes a NOUN token, with a synthetic verb between them
    tokens: list[Token] = []
    chunks: list[NounChunk] = []
    tok_idx = 0

    # Determine if there's a contrast conjunction — splits sentence into two clauses
    has_contrast = len(contrasts) > 0
    contrast_pos = contrasts[0] if has_contrast else -1

    # Determine negation context: which concepts follow a negation word
    neg_positions = {pos for pos, _ in negations}

    # Place concepts as noun tokens, with implicit dependency/negation verbs between them
    concept_indices: list[int] = []
    for concept in concepts:
        concept_pos = sentence.find(concept)

        # Check if this concept is preceded by a negation
        is_negated = False
        for neg_pos, neg_word in negations:
            if neg_pos < concept_pos <= neg_pos + len(neg_word) + 2:
                is_negated = True
                break

        # Add concept as NOUN token
        tokens.append(Token(
            word=concept,
            pos="NOUN",
            dep="",  # assigned below
            head_idx=0,  # assigned below
            idx=tok_idx,
        ))
        concept_indices.append(tok_idx)
        tok_idx += 1

    if not concept_indices:
        return SentenceTree(tokens=[], noun_chunks=[], text=sentence)

    # Assign deps: first concept = nsubj, rest = dobj/pobj
    # If contrast detected, concepts after contrast point get separate verb context
    # Insert synthetic verb token(s)

    rebuilt_tokens: list[Token] = []
    rebuilt_chunks: list[NounChunk] = []

    if len(concepts) == 1:
        # Single concept — just a subject
        rebuilt_tokens.append(Token(
            word=concepts[0], pos="NOUN", dep="nsubj",
            head_idx=0, idx=0,
        ))
        rebuilt_chunks.append(NounChunk(
            text=concepts[0], root_idx=0, start=0, end=1,
        ))
        return SentenceTree(tokens=rebuilt_tokens, noun_chunks=rebuilt_chunks, text=sentence)

    # Multi-concept sentence: build [subj] [verb] [obj] structure
    # Determine if negation applies between concepts
    first_concept_pos = sentence.find(concepts[0])
    neg_between: list[bool] = []
    for i in range(len(concepts) - 1):
        c1_pos = sentence.find(concepts[i])
        c2_pos = sentence.find(concepts[i + 1])
        has_neg = any(c1_pos < np < c2_pos for np, _ in negations)
        neg_between.append(has_neg)

    # Build structure: concept[0] --nsubj--> verb, concept[1..] --dobj--> verb
    # For each pair of adjacent concepts, create a synthetic relationship
    tidx = 0

    # Subject (first concept)
    subj_idx = tidx
    rebuilt_tokens.append(Token(
        word=concepts[0], pos="NOUN", dep="nsubj",
        head_idx=tidx + 1,  # points to verb
        idx=tidx,
    ))
    rebuilt_chunks.append(NounChunk(
        text=concepts[0], root_idx=tidx, start=tidx, end=tidx + 1,
    ))
    tidx += 1

    # For each subsequent concept, add verb + object
    for i, concept in enumerate(concepts[1:], start=0):
        # Determine verb type based on negation and contrast
        concept_pos_in_sent = sentence.find(concept)
        is_after_contrast = has_contrast and concept_pos_in_sent > contrast_pos
        has_neg = i < len(neg_between) and neg_between[i]

        if has_neg or is_after_contrast:
            verb_word = "contradicts"
            verb_dep = "ROOT" if i == 0 else "conj"
        else:
            verb_word = "requires"
            verb_dep = "ROOT" if i == 0 else "conj"

        verb_idx = tidx
        rebuilt_tokens.append(Token(
            word=verb_word, pos="VERB", dep=verb_dep,
            head_idx=subj_idx + 1 if verb_dep == "conj" else verb_idx,
            idx=tidx,
        ))
        tidx += 1

        # Object
        obj_idx = tidx
        rebuilt_tokens.append(Token(
            word=concept, pos="NOUN", dep="dobj",
            head_idx=verb_idx,
            idx=tidx,
        ))
        rebuilt_chunks.append(NounChunk(
            text=concept, root_idx=tidx, start=tidx, end=tidx + 1,
        ))
        tidx += 1

    # Fix nsubj head to point to first verb
    first_verb_idx = 1
    rebuilt_tokens[0] = Token(
        word=rebuilt_tokens[0].word, pos="NOUN", dep="nsubj",
        head_idx=first_verb_idx,
        idx=0,
    )

    return SentenceTree(tokens=rebuilt_tokens, noun_chunks=rebuilt_chunks, text=sentence)


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

def preprocess_zh(text: str) -> list[SentenceTree]:
    """Chinese text preprocessing, returns SentenceTree list identical to English API."""
    sentences = _split_sentences(text)
    trees: list[SentenceTree] = []
    for si, sent in enumerate(sentences):
        tree = _build_sentence_tree(sent, si)
        if tree.tokens:
            trees.append(tree)
    return trees


# ---------------------------------------------------------------------------
# Language detection
# ---------------------------------------------------------------------------

def detect_language(text: str) -> str:
    """Detect if text is primarily Chinese or English.

    Returns 'zh' for Chinese, 'en' for English.
    Uses CJK character ratio as the discriminator.
    """
    if not text.strip():
        return "en"
    cjk_count = sum(1 for ch in text if _is_cjk(ch))
    total_alpha = sum(1 for ch in text if ch.isalpha() or _is_cjk(ch))
    if total_alpha == 0:
        return "en"
    return "zh" if cjk_count / total_alpha > 0.3 else "en"


# ---------------------------------------------------------------------------
# CLI test
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    test_text = "走势终完美。任何级别的任何走势类型终要完成。"
    print(f"Input: {test_text!r}\n")

    lang = detect_language(test_text)
    print(f"Detected language: {lang}\n")

    trees = preprocess_zh(test_text)
    for i, tree in enumerate(trees):
        print(f"Sentence {i}: {tree.text!r}")
        print(f"  Tokens ({len(tree.tokens)}):")
        for tok in tree.tokens:
            print(f"    [{tok.idx}] {tok.word!r} pos={tok.pos} dep={tok.dep} head={tok.head_idx}")
        print(f"  NounChunks ({len(tree.noun_chunks)}):")
        for chunk in tree.noun_chunks:
            print(f"    {chunk.text!r} root_idx={chunk.root_idx} [{chunk.start}:{chunk.end}]")
        print()
