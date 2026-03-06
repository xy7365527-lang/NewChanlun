"""PDF to plain text extraction.

Uses PyMuPDF (fitz) if available, falls back to pdftotext CLI.
"""

from __future__ import annotations

import subprocess


def extract_text(pdf_path: str) -> str:
    """Extract text from PDF file.

    Tries PyMuPDF first, then pdftotext CLI.
    Returns empty string if neither is available.
    """
    text = _try_pymupdf(pdf_path)
    if text is not None:
        return text

    text = _try_pdftotext(pdf_path)
    if text is not None:
        return text

    return ""


def _try_pymupdf(pdf_path: str) -> str | None:
    """Try extracting with PyMuPDF (fitz). Returns None if not installed."""
    try:
        import fitz  # type: ignore[import-untyped]
    except ImportError:
        return None

    doc = fitz.open(pdf_path)
    parts: list[str] = []
    for page in doc:
        parts.append(page.get_text())
    doc.close()
    return "".join(parts)


def _try_pdftotext(pdf_path: str) -> str | None:
    """Try extracting with pdftotext CLI. Returns None if not available."""
    try:
        result = subprocess.run(
            ["pdftotext", pdf_path, "-"],
            capture_output=True,
            text=True,
            timeout=30,
        )
        if result.returncode == 0:
            return result.stdout
    except (FileNotFoundError, subprocess.TimeoutExpired):
        pass
    return None


def is_available() -> str:
    """Check which PDF extraction backend is available.

    Returns one of: 'pymupdf', 'pdftotext', 'none'.
    """
    try:
        import fitz  # type: ignore[import-untyped]  # noqa: F401
        return "pymupdf"
    except ImportError:
        pass

    try:
        result = subprocess.run(
            ["pdftotext", "-v"],
            capture_output=True,
            timeout=5,
        )
        if result.returncode == 0:
            return "pdftotext"
    except (FileNotFoundError, subprocess.TimeoutExpired):
        pass

    return "none"
