"""Reference UTF-8 codec written from the byte-level definition (K section 3 and 12.2).

It never calls ``bytes.decode`` or ``str.encode``: ``selftest`` contrasts it with them.
Strings are plain ``bytes``; scalars are ints.  All comparisons use unsigned bytes in an
explicit loop.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class Utf8Error(Exception):
    kind: str
    offset: int


def encode_scalar(cp: int) -> bytes:
    if cp < 0 or cp > 0x10FFFF or 0xD800 <= cp <= 0xDFFF:
        raise Utf8Error("invalid_scalar", cp)
    if cp < 0x80:
        return bytes([cp])
    if cp < 0x800:
        return bytes([0xC0 | (cp >> 6), 0x80 | (cp & 0x3F)])
    if cp < 0x10000:
        return bytes([0xE0 | (cp >> 12), 0x80 | ((cp >> 6) & 0x3F), 0x80 | (cp & 0x3F)])
    return bytes([0xF0 | (cp >> 18), 0x80 | ((cp >> 12) & 0x3F),
                  0x80 | ((cp >> 6) & 0x3F), 0x80 | (cp & 0x3F)])


def encode(scalars) -> bytes:
    out = bytearray()
    for cp in scalars:
        out += encode_scalar(cp)
    return bytes(out)


def decode_one(data: bytes, i: int) -> tuple[int, int]:
    """Return (scalar, byte length) at offset ``i`` or raise Utf8Error."""
    n = len(data)
    b0 = data[i]
    if b0 < 0x80:
        return b0, 1
    if 0x80 <= b0 <= 0xBF:
        raise Utf8Error("isolated_continuation", i)
    if b0 in (0xC0, 0xC1) or b0 > 0xF4:
        raise Utf8Error("invalid_lead", i)
    if b0 < 0xE0:
        need, cp, low = 2, b0 & 0x1F, 0x80
    elif b0 < 0xF0:
        need, cp, low = 3, b0 & 0x0F, 0x800
    else:
        need, cp, low = 4, b0 & 0x07, 0x10000
    if i + need > n:
        raise Utf8Error("truncated", i)
    for k in range(1, need):
        b = data[i + k]
        if b & 0xC0 != 0x80:
            raise Utf8Error("truncated", i)
        cp = (cp << 6) | (b & 0x3F)
    if cp < low:
        raise Utf8Error("overlong", i)
    if 0xD800 <= cp <= 0xDFFF:
        raise Utf8Error("surrogate", i)
    if cp > 0x10FFFF:
        raise Utf8Error("out_of_range", i)
    return cp, need


def decode(data: bytes) -> list[int]:
    out: list[int] = []
    i = 0
    while i < len(data):
        cp, width = decode_one(data, i)
        out.append(cp)
        i += width
    return out


def is_valid(data: bytes) -> bool:
    try:
        decode(data)
    except Utf8Error:
        return False
    return True


def scalar_len(data: bytes) -> int:
    return len(decode(data))


def is_boundary(data: bytes, i: int) -> bool:
    """True when ``0 <= i <= len`` and ``i`` starts a scalar (or is the end)."""
    if i < 0 or i > len(data):
        return False
    if i == len(data):
        return True
    return data[i] & 0xC0 != 0x80


def scalar_offsets(data: bytes) -> list[int]:
    """Byte offset of every scalar start, plus the final length."""
    out = []
    i = 0
    while i < len(data):
        out.append(i)
        _, width = decode_one(data, i)
        i += width
    out.append(len(data))
    return out


def compare(a: bytes, b: bytes) -> int:
    """Lexicographic order over unsigned bytes, then length (K section 3)."""
    n = len(a) if len(a) < len(b) else len(b)
    for i in range(n):
        x = a[i]
        y = b[i]
        if x != y:
            return -1 if x < y else 1
    if len(a) == len(b):
        return 0
    return -1 if len(a) < len(b) else 1


def text(s: str) -> bytes:
    """Bytes of a Python str through the reference encoder (surrogates are an error)."""
    return encode([ord(c) for c in s])


def from_bytes(data: bytes) -> str:
    return "".join(chr(cp) for cp in decode(data))
