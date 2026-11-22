"""String literal grammar of K section 2.1 and its NX-E1401 diagnostics (class MODELO).

The analyser works on Python ``str`` positions and converts to UTF-8 byte offsets with
``utf8ref`` lengths.  Expressions inside a Hole are only *recognised* (operand/operator
alternation, nested strings, balanced blocks); their semantics belong to the model.
Outcomes that K does not turn into a fixed diagnostic are returned as ``other`` and are
not compared (LAGUNAS: NX-E0002 inside a Hole, raw control characters, escapes).
"""

from __future__ import annotations

from dataclasses import dataclass, field

from . import utf8ref

ESCAPES = {'"': '"', "\\": "\\", "n": "\n", "r": "\r", "t": "\t"}
KEYWORD_OPERAND = ("if", "then", "else", "and", "or", "not")
OPERATORS = ("==", "!=", "<=", ">=", "+", "-", "*", "/", "%", "<", ">")


@dataclass
class LitDiag:
    subcause: str
    primary: tuple          # char span
    token: tuple            # (kind, text, char span)
    delimiter: tuple        # char span of the outer opening quote


@dataclass
class Result:
    start: int              # index of the opening quote
    end: int                # index after the closing quote, or the resume index on error
    diag: LitDiag | None = None
    other: str | None = None
    parts: list = field(default_factory=list)   # ("text", str) | ("hole", (s, e))
    holes: int = 0


def byte_map(src: str) -> list[int]:
    out = [0]
    for ch in src:
        out.append(out[-1] + len(utf8ref.text(ch)))
    return out


def to_bytes(span, bmap):
    return (bmap[span[0]], bmap[span[1]])


def _is_ident_start(c):
    return c.isascii() and (c.isalpha() or c == "_")


def _is_ident(c):
    return c.isascii() and (c.isalnum() or c == "_")


def _complete_nested(src: str, q: int) -> bool:
    """Is there an unescaped closing quote before CR/LF/EOF after the quote at q?"""
    j = q + 1
    n = len(src)
    while j < n:
        c = src[j]
        if c in "\r\n":
            return False
        if c == "\\":
            j += 2
            continue
        if c == '"':
            return True
        j += 1
    return False


def _skip_string(src: str, q: int) -> int:
    j = q + 1
    n = len(src)
    while j < n:
        if src[j] == "\\":
            j += 2
            continue
        if src[j] == '"':
            return j + 1
        j += 1
    return n


def recover(src: str, i: int, depth: int) -> int:
    """Discard up to and including the outer closing quote (K 2.1, recovery)."""
    n = len(src)
    j = i
    while j < n:
        c = src[j]
        if depth == 0:
            if c == "\\":
                j += 2
                continue
            if c == '"':
                return j + 1
            if c == "{":
                if src[j + 1:j + 2] == "{":
                    j += 2
                    continue
                depth = 1
            elif c == "}" and src[j + 1:j + 2] == "}":
                j += 2
                continue
        else:
            if c == '"':
                j = _skip_string(src, j)
                continue
            if c == "/" and src[j + 1:j + 2] == "/":
                j += 2
                continue
            if c == "{":
                depth += 1
            elif c == "}":
                depth -= 1
        j += 1
    return n


def analyze(src: str, q: int) -> Result:
    """Analyse the literal whose opening quote is ``src[q]``."""
    n = len(src)
    res = Result(start=q, end=q)
    delim = (q, q + 1)
    text_buf: list[str] = []

    def flush():
        if text_buf:
            res.parts.append(("text", "".join(text_buf)))
            text_buf.clear()

    i = q + 1
    while True:
        if i >= n:
            res.other = "unterminated_string"
            res.end = n
            return res
        c = src[i]
        if c == '"':
            flush()
            res.end = i + 1
            return res
        if c in "\r\n":
            res.other = "raw_control_in_text"
            res.end = n
            return res
        if c == "\\":
            nxt = src[i + 1:i + 2]
            if nxt != "" and nxt in ESCAPES:
                text_buf.append(ESCAPES[nxt])
                i += 2
                continue
            res.other = "invalid_escape"
            res.end = recover(src, i + 2, 0)
            return res
        if c == "{":
            if src[i + 1:i + 2] == "{":
                text_buf.append("{")
                i += 2
                continue
            flush()
            failed = _hole(src, i, delim, res)
            if failed:
                return res
            i = res.end
            continue
        if c == "}":
            if src[i + 1:i + 2] == "}":
                text_buf.append("}")
                i += 2
                continue
            res.diag = LitDiag("unescaped_closing_brace", (i, i + 1), ("symbol", "}", (i, i + 1)), delim)
            res.end = recover(src, i + 1, 0)
            return res
        text_buf.append(c)
        i += 1


def _hole(src: str, h: int, delim: tuple, res: Result) -> bool:
    """Recognise the Hole opened at ``h``.  True when ``res`` carries an error or an
    unclassified outcome; otherwise ``res.end`` is the index after the closing brace."""
    n = len(src)
    i = h + 1
    depth = 0                 # blocks opened inside the Hole
    expect_operand = True
    any_token = False
    res.holes += 1

    def fail(sub, primary, tok, at, in_depth):
        res.diag = LitDiag(sub, primary, tok, delim)
        res.end = recover(src, at, in_depth)
        return True

    def other(tag, at, in_depth):
        res.other = tag
        res.end = recover(src, at, in_depth)
        return True

    while True:
        while i < n and src[i] in " \t":
            i += 1
        if i >= n:
            res.diag = LitDiag("unterminated_interpolation", (h, h + 1), ("symbol", "{", (h, h + 1)), delim)
            res.end = n
            return True
        c = src[i]
        if c in "\r\n":
            width = 2 if src[i:i + 2] == "\r\n" else 1
            return fail("raw_line_break_in_interpolation", (i, i + width),
                        ("invalid", src[i:i + width], (i, i + width)), i + width, 1 + depth)
        if c == "/" and src[i + 1:i + 2] == "/":
            return fail("comment_in_interpolation", (i, i + 2), ("symbol", "//", (i, i + 2)),
                        i + 2, 1 + depth)
        if c == "}":
            if depth > 0:
                depth -= 1
                i += 1
                expect_operand = False
                continue
            if not any_token:
                return fail("empty_interpolation", (h, i + 1), ("symbol", "}", (i, i + 1)), i + 1, 0)
            if expect_operand:
                return other("expr_error", i + 1, 0)
            res.parts.append(("hole", (h + 1, i)))
            res.end = i + 1
            return False
        if c == '"':
            if expect_operand:
                if not _complete_nested(src, i):
                    return fail("incomplete_interpolation_expression", (i, i + 1),
                                ("symbol", '"', (i, i + 1)), i + 1, 1 + depth)
                inner = analyze(src, i)
                res.holes += inner.holes
                if inner.diag is not None or inner.other is not None:
                    res.diag = inner.diag
                    res.other = inner.other
                    res.end = inner.end
                    return True
                i = inner.end
                expect_operand = False
                any_token = True
                continue
            return fail("unterminated_interpolation", (h, h + 1), ("symbol", "{", (h, h + 1)),
                        i, 1 + depth)
        any_token = True
        if c.isascii() and c.isdigit():
            if not expect_operand:
                return other("expr_error", i, 1 + depth)
            while i < n and src[i].isascii() and src[i].isdigit():
                i += 1
            expect_operand = False
            continue
        if _is_ident_start(c):
            j = i
            while j < n and _is_ident(src[j]):
                j += 1
            word = src[i:j]
            i = j
            if word in KEYWORD_OPERAND:
                expect_operand = True
            else:
                if not expect_operand:
                    return other("expr_error", i, 1 + depth)
                expect_operand = False
            continue
        if c in ".([,":
            expect_operand = True
            i += 1
            continue
        if c in ")]":
            expect_operand = False
            i += 1
            continue
        if c == "{":
            depth += 1
            expect_operand = True
            i += 1
            continue
        matched = None
        for op in OPERATORS:
            if src.startswith(op, i):
                matched = op
                break
        if matched is not None:
            if expect_operand and matched != "-":
                return other("expr_error", i, 1 + depth)
            expect_operand = True
            i += len(matched)
            continue
        return other("expr_error", i + 1, 1 + depth)


def find_literals(src: str) -> list[Result]:
    """All string literals of a source, in order (outside line comments)."""
    out = []
    i = 0
    n = len(src)
    while i < n:
        c = src[i]
        if c == '"':
            r = analyze(src, i)
            out.append(r)
            i = max(r.end, i + 1)
            continue
        if c == "/" and src[i + 1:i + 2] == "/":
            while i < n and src[i] not in "\r\n":
                i += 1
            continue
        i += 1
    return out


def first_diagnostic(src: str):
    """First literal diagnostic of a source as a dict in byte offsets, else None.

    Returns ``{"other": tag}`` for outcomes that K leaves open (not compared).
    """
    bmap = byte_map(src)
    for r in find_literals(src):
        if r.diag is not None:
            d = r.diag
            return {
                "subcause": d.subcause,
                "primary": to_bytes(d.primary, bmap),
                "token": (d.token[0], d.token[1], to_bytes(d.token[2], bmap)),
                "delimiter": to_bytes(d.delimiter, bmap),
                "resume": bmap[min(r.end, len(src))],
            }
        if r.other is not None:
            return {"other": r.other}
    return None


def decode_text(parts) -> str:
    return "".join(p[1] for p in parts if p[0] == "text")
