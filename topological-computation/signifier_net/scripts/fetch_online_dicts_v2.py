"""
Fetch and process online dictionary data for S_net (v2).

Sources:
1. WordNet (English): synonyms, antonyms, hypernyms, hyponyms via NLTK
2. Open Multilingual Wordnet (French): French lemmas for WordNet synsets
3. Wiktionary API (en/de/zh/fr): definitions, etymology
4. CNRTL (French): definitions, synonyms, etymology

Produces:
- dictionaries/thesaurus_en_wordnet.jsonl
- dictionaries/thesaurus_fr_wordnet.jsonl
- dictionaries/wiktionary_en.jsonl
- dictionaries/wiktionary_de.jsonl
- dictionaries/wiktionary_zh.jsonl
- dictionaries/wiktionary_fr.jsonl
- dictionaries/dict_fr_cnrtl.jsonl
"""

import glob
import html
import json
import os
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

BASE_DIR = Path(__file__).resolve().parent.parent
DICT_DIR = BASE_DIR / "dictionaries"

# Rate limiting for API calls
WIKTIONARY_DELAY = 1.0  # seconds between requests
CNRTL_DELAY = 1.5       # seconds between requests
BATCH_SIZE = 20          # flush to file every N entries


def build_full_whitelist() -> set[str]:
    """Build whitelist of all unique terms from existing dictionaries."""
    terms = set()
    for f in glob.glob(str(DICT_DIR / "*.jsonl")):
        for line in open(f, encoding="utf-8"):
            line = line.strip()
            if not line:
                continue
            try:
                d = json.loads(line)
            except json.JSONDecodeError:
                continue
            for key in ["term", "term_a", "term_b", "signifier_id"]:
                if key in d and d[key] and isinstance(d[key], str):
                    terms.add(d[key])
    return terms


def classify_terms(terms: set[str]) -> dict[str, set[str]]:
    """Classify terms by language heuristic."""
    en_terms = set()
    zh_terms = set()
    de_terms = set()
    fr_terms = set()

    for t in terms:
        has_cjk = any('\u4e00' <= c <= '\u9fff' for c in t)
        has_umlaut = any(c in t for c in 'äöüÄÖÜß')

        if has_cjk:
            zh_terms.add(t)
        if has_umlaut:
            de_terms.add(t)

        # Simple ASCII words go to EN
        # German/French philosophy terms (often capitalized, no umlauts) also try EN
        if t.isascii() or (not has_cjk):
            en_terms.add(t)

        # French accented characters
        if any(c in t for c in 'àâéèêëïîôùûüÿçæœÀÂÉÈÊËÏÎÔÙÛÜŸÇÆŒ'):
            fr_terms.add(t)

    return {"en": en_terms, "zh": zh_terms, "de": de_terms, "fr": fr_terms}


def extract_simple_term(raw_term: str) -> str:
    """Extract a clean, searchable term from compound/multilingual entries.

    e.g. 'Aufhebung' from 'Aufhebung'
         'sublation' from 'sublation/Aufhebung/扬弃'
         '扬弃' from '扬弃'
    """
    # If it contains slashes, split and return parts
    if "/" in raw_term:
        return raw_term.split("/")[0].strip()
    return raw_term.strip()


def append_jsonl(path: Path, entries: list[dict]):
    """Append entries to a JSONL file."""
    with open(path, "a", encoding="utf-8") as f:
        for entry in entries:
            f.write(json.dumps(entry, ensure_ascii=False) + "\n")


def init_file(path: Path):
    """Initialize (truncate) output file."""
    with open(path, "w", encoding="utf-8") as f:
        pass  # create empty


# ═══════════════════════════════════════════════════════════════════════
# 1. WordNet (English)
# ═══════════════════════════════════════════════════════════════════════

def fetch_wordnet_english(terms: set[str]) -> tuple[int, list[str]]:
    """Fetch English WordNet data for given terms."""
    from nltk.corpus import wordnet as wn

    out_path = DICT_DIR / "thesaurus_en_wordnet.jsonl"
    init_file(out_path)

    total = 0
    errors = []
    batch = []

    # Normalize terms for WordNet lookup (underscores instead of spaces)
    lookup_map = {}
    for t in sorted(terms):
        clean = extract_simple_term(t)
        # WordNet uses underscores
        wn_key = clean.replace(" ", "_").replace("-", "_").lower()
        if wn_key and wn_key not in lookup_map:
            lookup_map[wn_key] = clean

    print(f"  Querying {len(lookup_map)} unique terms...")

    for wn_key, original in sorted(lookup_map.items()):
        try:
            synsets = wn.synsets(wn_key)
            if not synsets:
                continue

            all_synonyms = set()
            all_antonyms = set()
            all_hypernyms = set()
            all_hyponyms = set()
            definitions = []

            for ss in synsets:
                definitions.append(ss.definition())
                for lemma in ss.lemmas():
                    name = lemma.name().replace("_", " ")
                    if name.lower() != original.lower():
                        all_synonyms.add(name)
                    for ant in lemma.antonyms():
                        all_antonyms.add(ant.name().replace("_", " "))
                for h in ss.hypernyms():
                    for lem in h.lemmas():
                        all_hypernyms.add(lem.name().replace("_", " "))
                for h in ss.hyponyms()[:10]:  # limit hyponyms
                    for lem in h.lemmas():
                        all_hyponyms.add(lem.name().replace("_", " "))

            entry = {
                "term": original,
                "lang": "en",
                "synonyms": sorted(all_synonyms),
                "antonyms": sorted(all_antonyms),
                "hypernyms": sorted(all_hypernyms),
                "hyponyms": sorted(all_hyponyms),
                "definitions": definitions[:5],
                "source": "WordNet 3.1"
            }
            batch.append(entry)
            total += 1

            if len(batch) >= BATCH_SIZE:
                append_jsonl(out_path, batch)
                batch = []

        except Exception as e:
            errors.append(f"WordNet {original}: {e}")

    if batch:
        append_jsonl(out_path, batch)

    print(f"  Written {total} entries to {out_path}")
    return total, errors


# ═══════════════════════════════════════════════════════════════════════
# 2. Open Multilingual WordNet (French)
# ═══════════════════════════════════════════════════════════════════════

def fetch_wordnet_french(terms: set[str]) -> tuple[int, list[str]]:
    """Fetch French WordNet lemmas via OMW."""
    from nltk.corpus import wordnet as wn

    out_path = DICT_DIR / "thesaurus_fr_wordnet.jsonl"
    init_file(out_path)

    total = 0
    errors = []
    batch = []

    lookup_map = {}
    for t in sorted(terms):
        clean = extract_simple_term(t)
        wn_key = clean.replace(" ", "_").replace("-", "_").lower()
        if wn_key and wn_key not in lookup_map:
            lookup_map[wn_key] = clean

    print(f"  Querying {len(lookup_map)} unique terms for French lemmas...")

    for wn_key, original in sorted(lookup_map.items()):
        try:
            synsets = wn.synsets(wn_key)
            if not synsets:
                continue

            fr_synonyms = set()
            for ss in synsets:
                try:
                    fr_lemmas = ss.lemma_names('fra')
                    for fl in fr_lemmas:
                        fr_synonyms.add(fl.replace("_", " "))
                except Exception:
                    pass

            if not fr_synonyms:
                continue

            entry = {
                "term": original,
                "lang": "fr",
                "fr_lemmas": sorted(fr_synonyms),
                "source": "Open Multilingual Wordnet 1.4"
            }
            batch.append(entry)
            total += 1

            if len(batch) >= BATCH_SIZE:
                append_jsonl(out_path, batch)
                batch = []

        except Exception as e:
            errors.append(f"OMW-FR {original}: {e}")

    if batch:
        append_jsonl(out_path, batch)

    print(f"  Written {total} entries to {out_path}")
    return total, errors


# ═══════════════════════════════════════════════════════════════════════
# 3. Wiktionary API
# ═══════════════════════════════════════════════════════════════════════

def parse_wikitext_definitions(wikitext: str, lang_section: str) -> dict:
    """Extract definitions and etymology from Wiktionary wikitext."""
    result = {"definitions": [], "etymology": ""}

    # Find the language section
    # Sections are delimited by ==Language==
    lang_pattern = re.compile(r"^==" + re.escape(lang_section) + r"==\s*$", re.MULTILINE)
    match = lang_pattern.search(wikitext)
    if not match:
        return result

    # Extract text from this language section until next ==Language==
    start = match.end()
    next_lang = re.search(r"^==[^=]", wikitext[start:], re.MULTILINE)
    section_text = wikitext[start:start + next_lang.start()] if next_lang else wikitext[start:]

    # Extract etymology
    etym_match = re.search(r"===Etymology(?:\s+\d+)?===\s*\n(.*?)(?=\n===|\Z)", section_text, re.DOTALL)
    if etym_match:
        etym_text = etym_match.group(1).strip()
        # Clean wiki markup
        etym_text = re.sub(r"\{\{[^}]*\}\}", "", etym_text)
        etym_text = re.sub(r"\[\[([^\]|]*\|)?([^\]]*)\]\]", r"\2", etym_text)
        etym_text = re.sub(r"<[^>]+>", "", etym_text)
        etym_text = etym_text.strip()
        if etym_text:
            result["etymology"] = etym_text[:500]  # cap length

    # Extract definitions (lines starting with # but not ##)
    for line in section_text.splitlines():
        line = line.strip()
        if line.startswith("# ") and not line.startswith("##"):
            defn = line[2:].strip()
            # Clean wiki markup
            defn = re.sub(r"\{\{[^}]*\}\}", "", defn)
            defn = re.sub(r"\[\[([^\]|]*\|)?([^\]]*)\]\]", r"\2", defn)
            defn = re.sub(r"<[^>]+>", "", defn)
            defn = re.sub(r"'''?", "", defn)
            defn = defn.strip()
            if defn and len(defn) > 3:
                result["definitions"].append(defn)

    return result


def fetch_wiktionary_term(term: str, lang_edition: str = "en") -> dict | None:
    """Fetch a single term from Wiktionary API.

    lang_edition: which Wiktionary edition (en, de, zh, fr)
    """
    # Map edition to the language section name in wikitext
    lang_section_map = {
        "en": "English",
        "de": "German",
        "zh": "Chinese",
        "fr": "French",
    }

    encoded_term = urllib.parse.quote(term, safe="")
    base_url = f"https://{lang_edition}.wiktionary.org/w/api.php"
    params = {
        "action": "parse",
        "page": term,
        "prop": "wikitext",
        "format": "json",
        "redirects": "1",
    }
    url = base_url + "?" + urllib.parse.urlencode(params)

    req = urllib.request.Request(url, headers={
        "User-Agent": "S_net-dictionary-fetcher/1.0 (research project; contact: none)",
        "Accept": "application/json",
    })

    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            data = json.loads(resp.read().decode("utf-8"))
    except urllib.error.HTTPError as e:
        if e.code == 404:
            return None
        raise
    except urllib.error.URLError:
        return None

    if "error" in data:
        return None

    wikitext = data.get("parse", {}).get("wikitext", {}).get("*", "")
    if not wikitext:
        return None

    # For the English Wiktionary, we look for sections matching the target language
    # For non-English editions, we look for the primary language
    if lang_edition == "en":
        # English Wiktionary has sections for each language
        # We try multiple relevant sections
        results = {}
        for lang_code, section_name in lang_section_map.items():
            parsed = parse_wikitext_definitions(wikitext, section_name)
            if parsed["definitions"] or parsed["etymology"]:
                results[lang_code] = parsed
        return results if results else None
    else:
        # Non-English editions: the wikitext structure differs
        # Just extract top-level definitions
        defs = []
        etym = ""
        for line in wikitext.splitlines():
            line = line.strip()
            if line.startswith("# ") and not line.startswith("##"):
                defn = line[2:].strip()
                defn = re.sub(r"\{\{[^}]*\}\}", "", defn)
                defn = re.sub(r"\[\[([^\]|]*\|)?([^\]]*)\]\]", r"\2", defn)
                defn = re.sub(r"<[^>]+>", "", defn)
                defn = re.sub(r"'''?", "", defn)
                defn = defn.strip()
                if defn and len(defn) > 3:
                    defs.append(defn)

        etym_match = re.search(r"===\s*(?:Étymologie|Etymologie|Herkunft|Etymology|词源)\s*===\s*\n(.*?)(?=\n===|\Z)", wikitext, re.DOTALL)
        if etym_match:
            etym_text = etym_match.group(1).strip()
            etym_text = re.sub(r"\{\{[^}]*\}\}", "", etym_text)
            etym_text = re.sub(r"\[\[([^\]|]*\|)?([^\]]*)\]\]", r"\2", etym_text)
            etym_text = re.sub(r"<[^>]+>", "", etym_text)
            etym = etym_text.strip()[:500]

        if defs or etym:
            return {"definitions": defs, "etymology": etym}
        return None


def fetch_wiktionary_all(classified_terms: dict[str, set[str]]) -> tuple[dict[str, int], list[str]]:
    """Fetch Wiktionary data for all languages.

    Strategy: Use English Wiktionary as primary source (it has entries for
    English, German, French, Chinese terms). Supplement with language-specific
    editions for terms not found.
    """
    counts = {}
    errors = []

    # Collect all unique simple terms across all languages
    all_terms = set()
    for lang, terms in classified_terms.items():
        for t in terms:
            all_terms.add(extract_simple_term(t))

    # Remove very long terms and sentences (likely definitions, not terms)
    all_terms = {t for t in all_terms if len(t) < 80 and t.count(" ") < 6}

    print(f"  Total unique terms to query: {len(all_terms)}")

    # Initialize output files
    out_files = {
        "en": DICT_DIR / "wiktionary_en.jsonl",
        "de": DICT_DIR / "wiktionary_de.jsonl",
        "zh": DICT_DIR / "wiktionary_zh.jsonl",
        "fr": DICT_DIR / "wiktionary_fr.jsonl",
    }
    for path in out_files.values():
        init_file(path)

    # Batches per language
    batches: dict[str, list[dict]] = {lang: [] for lang in out_files}
    lang_counts = {lang: 0 for lang in out_files}

    # Query English Wiktionary for all terms (it's the most comprehensive)
    queried = 0
    for term in sorted(all_terms):
        queried += 1
        if queried % 50 == 0:
            print(f"    Progress: {queried}/{len(all_terms)} terms queried...")

        try:
            result = fetch_wiktionary_term(term, "en")
            if result is None:
                time.sleep(WIKTIONARY_DELAY)
                continue

            # result is a dict of {lang_code: {definitions, etymology}}
            if isinstance(result, dict) and "definitions" in result:
                # Non-English edition result (shouldn't happen for "en" edition, but handle)
                pass
            elif isinstance(result, dict):
                for lang_code, data in result.items():
                    if lang_code not in out_files:
                        continue
                    if not data.get("definitions") and not data.get("etymology"):
                        continue
                    entry = {
                        "term": term,
                        "lang": lang_code,
                        "definitions": data.get("definitions", []),
                        "etymology": data.get("etymology", ""),
                        "source": "Wiktionary (en)"
                    }
                    batches[lang_code].append(entry)
                    lang_counts[lang_code] += 1

                    if len(batches[lang_code]) >= BATCH_SIZE:
                        append_jsonl(out_files[lang_code], batches[lang_code])
                        batches[lang_code] = []

        except Exception as e:
            errors.append(f"Wiktionary {term}: {e}")

        time.sleep(WIKTIONARY_DELAY)

    # Flush remaining batches
    for lang_code in out_files:
        if batches[lang_code]:
            append_jsonl(out_files[lang_code], batches[lang_code])

    for lang_code, count in lang_counts.items():
        print(f"  Written {count} entries to {out_files[lang_code]}")
        counts[lang_code] = count

    return counts, errors


# ═══════════════════════════════════════════════════════════════════════
# 4. CNRTL (French)
# ═══════════════════════════════════════════════════════════════════════

def fetch_cnrtl_term(term: str) -> dict | None:
    """Fetch definition from CNRTL for a French term."""
    encoded = urllib.parse.quote(term, safe="")
    url = f"https://www.cnrtl.fr/definition/{encoded}"

    req = urllib.request.Request(url, headers={
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) S_net-research/1.0",
        "Accept": "text/html",
        "Accept-Language": "fr-FR,fr;q=0.9",
    })

    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            raw_html = resp.read().decode("utf-8", errors="replace")
    except (urllib.error.HTTPError, urllib.error.URLError):
        return None

    # Check for error page
    if "Cette forme est introuvable" in raw_html or "Erreur" in raw_html[:2000]:
        return None

    # Extract definitions from CNRTL HTML
    # CNRTL puts definitions in <div class="tlf_cdefinition"> or similar
    definitions = []

    # Pattern for definition blocks
    def_patterns = [
        r'<div\s+class="tlf_cdefinition"[^>]*>(.*?)</div>',
        r'<div\s+class="tlf_cexemple"[^>]*>(.*?)</div>',
    ]

    for pattern in def_patterns[:1]:  # just definitions, not examples
        for match in re.finditer(pattern, raw_html, re.DOTALL):
            text = match.group(1)
            text = re.sub(r"<[^>]+>", "", text)
            text = html.unescape(text).strip()
            if text and len(text) > 5:
                definitions.append(text[:300])

    # Extract etymology section
    etymology = ""
    etym_match = re.search(r'<div\s+id="etym"[^>]*>(.*?)</div>', raw_html, re.DOTALL)
    if etym_match:
        etym_text = re.sub(r"<[^>]+>", "", etym_match.group(1))
        etymology = html.unescape(etym_text).strip()[:500]

    # Also try broader definition extraction if the specific class didn't match
    if not definitions:
        # Try extracting from contentbox
        content_match = re.search(r'<div\s+id="contentbox"[^>]*>(.*?)</div>\s*</div>', raw_html, re.DOTALL)
        if content_match:
            text = content_match.group(1)
            text = re.sub(r"<[^>]+>", " ", text)
            text = html.unescape(text).strip()
            text = re.sub(r"\s+", " ", text)
            if text and len(text) > 10:
                # Take first 500 chars as definition summary
                definitions.append(text[:500])

    # Filter out entries that are just error messages
    definitions = [d for d in definitions if "introuvable" not in d.lower() and "erreur" not in d.lower()]

    if definitions or etymology:
        return {"definitions": definitions[:10], "etymology": etymology}
    return None


def fetch_cnrtl(fr_terms: set[str]) -> tuple[int, list[str]]:
    """Fetch CNRTL definitions for French terms."""
    out_path = DICT_DIR / "dict_fr_cnrtl.jsonl"
    init_file(out_path)

    french_candidates = set()

    # Add terms from bilingual dictionaries that have French sides
    for pattern_name in ["bilingual_en_fr_*.jsonl", "bilingual_zh_fr_*.jsonl",
                         "bilingual_de_fr_*.jsonl", "bilingual_*_fr_*.jsonl"]:
        for f in glob.glob(str(DICT_DIR / pattern_name)):
            for line in open(f, encoding="utf-8"):
                line_s = line.strip()
                if not line_s:
                    continue
                try:
                    d = json.loads(line_s)
                    for key in ["term_a", "term_b"]:
                        lang_key = key.replace("term", "lang")
                        if d.get(lang_key) == "fr" and d.get(key):
                            t = d[key].strip()
                            if t and len(t) < 50 and not any('\u4e00' <= c <= '\u9fff' for c in t):
                                french_candidates.add(t)
                except json.JSONDecodeError:
                    pass

    # Add accented terms from fr_terms (but exclude German umlauts)
    german_chars = set('äöüÄÖÜß')
    for t in fr_terms:
        clean = extract_simple_term(t)
        if clean and len(clean) < 50:
            # Skip if it has German-specific characters
            if any(c in german_chars for c in clean):
                continue
            french_candidates.add(clean)

    # Also get French lemmas already found by OMW
    omw_fr = DICT_DIR / "thesaurus_fr_wordnet.jsonl"
    if omw_fr.exists():
        for line in open(omw_fr, encoding="utf-8"):
            line_s = line.strip()
            if not line_s:
                continue
            try:
                d = json.loads(line_s)
                for lemma in d.get("fr_lemmas", []):
                    if lemma and len(lemma) < 50:
                        french_candidates.add(lemma)
            except json.JSONDecodeError:
                pass

    print(f"  Querying {len(french_candidates)} French terms on CNRTL...")

    total = 0
    errors = []
    batch = []
    queried = 0

    for term in sorted(french_candidates):
        queried += 1
        if queried % 20 == 0:
            print(f"    Progress: {queried}/{len(french_candidates)} terms queried...")

        try:
            result = fetch_cnrtl_term(term)
            if result is None:
                time.sleep(CNRTL_DELAY)
                continue

            entry = {
                "term": term,
                "lang": "fr",
                "definitions": result.get("definitions", []),
                "etymology": result.get("etymology", ""),
                "source": "CNRTL"
            }
            batch.append(entry)
            total += 1

            if len(batch) >= BATCH_SIZE:
                append_jsonl(out_path, batch)
                batch = []

        except Exception as e:
            errors.append(f"CNRTL {term}: {e}")

        time.sleep(CNRTL_DELAY)

    if batch:
        append_jsonl(out_path, batch)

    print(f"  Written {total} entries to {out_path}")
    return total, errors


# ═══════════════════════════════════════════════════════════════════════
# Validation
# ═══════════════════════════════════════════════════════════════════════

def validate_jsonl(path: Path) -> tuple[int, int]:
    """Validate JSONL file. Returns (valid_lines, error_lines)."""
    if not path.exists():
        return 0, 0
    valid = 0
    invalid = 0
    for line in open(path, encoding="utf-8"):
        line = line.strip()
        if not line:
            continue
        try:
            json.loads(line)
            valid += 1
        except json.JSONDecodeError:
            invalid += 1
    return valid, invalid


# ═══════════════════════════════════════════════════════════════════════
# Main
# ═══════════════════════════════════════════════════════════════════════

def main():
    all_errors = []

    # Build whitelist
    print("=== Building term whitelist ===")
    whitelist = build_full_whitelist()
    print(f"Total unique terms: {len(whitelist)}")

    classified = classify_terms(whitelist)
    for lang, terms in classified.items():
        print(f"  {lang}: {len(terms)} terms")

    # ── 1. WordNet English ──
    print("\n=== WordNet (English) ===")
    try:
        wn_count, wn_errors = fetch_wordnet_english(classified["en"])
        all_errors.extend(wn_errors)
    except Exception as e:
        msg = f"WordNet English failed: {e}"
        print(f"  ERROR: {msg}")
        all_errors.append(msg)

    # ── 2. WordNet French (OMW) ──
    print("\n=== Open Multilingual WordNet (French) ===")
    try:
        fr_count, fr_errors = fetch_wordnet_french(classified["en"])  # Use EN terms, get FR lemmas
        all_errors.extend(fr_errors)
    except Exception as e:
        msg = f"OMW French failed: {e}"
        print(f"  ERROR: {msg}")
        all_errors.append(msg)

    # ── 3. Wiktionary (all languages) ──
    print("\n=== Wiktionary (en/de/zh/fr) ===")
    try:
        wikt_counts, wikt_errors = fetch_wiktionary_all(classified)
        all_errors.extend(wikt_errors)
    except Exception as e:
        msg = f"Wiktionary failed: {e}"
        print(f"  ERROR: {msg}")
        all_errors.append(msg)

    # ── 4. CNRTL (French) ──
    print("\n=== CNRTL (French) ===")
    try:
        cnrtl_count, cnrtl_errors = fetch_cnrtl(classified["fr"])
        all_errors.extend(cnrtl_errors)
    except Exception as e:
        msg = f"CNRTL failed: {e}"
        print(f"  ERROR: {msg}")
        all_errors.append(msg)

    # ── Validation ──
    print("\n=== Validation ===")
    output_files = [
        DICT_DIR / "thesaurus_en_wordnet.jsonl",
        DICT_DIR / "thesaurus_fr_wordnet.jsonl",
        DICT_DIR / "wiktionary_en.jsonl",
        DICT_DIR / "wiktionary_de.jsonl",
        DICT_DIR / "wiktionary_zh.jsonl",
        DICT_DIR / "wiktionary_fr.jsonl",
        DICT_DIR / "dict_fr_cnrtl.jsonl",
    ]
    for path in output_files:
        valid, invalid = validate_jsonl(path)
        status = "OK" if invalid == 0 else f"WARN: {invalid} invalid lines"
        print(f"  {path.name}: {valid} valid entries [{status}]")

    # ── Summary ──
    print("\n=== Summary ===")
    if all_errors:
        print(f"Errors ({len(all_errors)}):")
        for e in all_errors[:20]:
            print(f"  - {e}")
        if len(all_errors) > 20:
            print(f"  ... and {len(all_errors) - 20} more")
    else:
        print("All sources processed successfully.")

    return 1 if all_errors else 0


if __name__ == "__main__":
    sys.exit(main())
