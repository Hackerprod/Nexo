"""Diagnostic constructor for contract C (K section 10.1).  Class MODELO: no I/O.

The 21 comparable fields are always present; ``message`` is informative and only
N13 fixes it exactly.  Defaults follow K 10.1 (common contract C); stage/operation
come from K section 9 / 10 or A2 13.12 and may be overridden per row.
"""

from __future__ import annotations

NAMESPACE = "Nexo-proposed-v1"
SIM_NAMESPACE = "F0-0002-sim"
PROFILE = "f0-0004-b-v1"

FIELDS = (
    "schema_version", "diagnostic_namespace", "profile_id", "code", "subcause", "rule",
    "explanation_id", "severity", "stage", "operation", "primary_span", "related_spans",
    "cause_chain", "feature_id", "unexpected_token", "expected_tokens",
    "expected_tokens_kind", "parser_context", "recognized_prefix", "repair", "data",
)

# (stage, operation) by code; K section 9 and A2 13.12.  Per-row overrides are explicit.
STAGE_OP = {
    "NX-E0001": ("lex", "syntax.lex"),
    "NX-E0002": ("parse", "syntax.parse"),
    "NX-E0003": ("types", "analysis.limit"),
    "NX-E0006": ("types", "types.check"),
    "NX-E0101": ("names", "names.resolve"),
    "NX-E0102": ("types", "types.check"),
    "NX-E0104": ("names", "names.declare"),
    "NX-E0105": ("ownership", "place.write"),
    "NX-E0111": ("arguments", "call.bind"),
    "NX-E0201": ("ownership", "value.copy"),
    "NX-E0202": ("ownership", "ownership.use"),
    "NX-E0412": ("effects", "value.build_string"),
    "NX-E1105": ("ownership", "ownership.access"),
    "NX-E1107": ("ownership", "ownership.check"),
    "NX-E1304": ("types", "types.check"),
    "NX-E1308": ("names", "names.resolve"),
    "NX-E1309": ("names", "names.resolve"),
    "NX-E1310": ("names", "names.resolve"),
    "NX-E1315": ("names", "names.resolve"),
    "NX-E1322": ("names", "names.declare"),
    "NX-E1323": ("types", "types.check"),
    "NX-E1325": ("types", "types.check"),
    "NX-E1401": ("parse", "syntax.parse"),
    "NX-E1402": ("types", "types.check"),
    "NX-E1403": ("types", "types.check"),
    "NX-E1404": ("types", "types.check"),
    "R0002": ("runtime", "runtime.limit"),
}

BARRIER_MESSAGE = "Capacidad pendiente de implementación en {k}."

_AUTO = object()


def span(file: str, start: int, end: int) -> dict:
    return {"file": file, "start": start, "end": end}


def make(code: str, subcause=None, *, file: str, primary, related=(), data=None,
         stage=None, operation=None, profile_id=PROFILE, namespace=None,
         feature_id=None, unexpected_token=None, expected_tokens=(),
         expected_tokens_kind=None, parser_context=None, recognized_prefix=None,
         cause_chain=(), repair=(), rule=_AUTO, explanation_id=_AUTO) -> dict:
    """Build the comparable diagnostic.  ``primary`` is (start, end) in UTF-8 bytes.

    ``related`` items are (role, (start, end)); ``cause_chain`` items are
    (kind, operation, (start, end), entity_dict).  ``unexpected_token`` is
    (kind, text, (start, end)); ``recognized_prefix`` is (start, end).
    """
    if code not in STAGE_OP:
        raise KeyError(f"no stage/operation known for {code}")
    d_stage, d_op = STAGE_OP[code]
    ident = code if subcause is None else f"{code}/{subcause}"
    if namespace is None:
        namespace = SIM_NAMESPACE if code.startswith("R") else NAMESPACE
    if rule is _AUTO:
        rule = ident
    if explanation_id is _AUTO:
        explanation_id = None if code.startswith("R") else ident
    rel = sorted(((r[2] if len(r) > 2 else file, r[1][0], r[1][1], r[0]) for r in related))
    diag = {
        "schema_version": 1,
        "diagnostic_namespace": namespace,
        "profile_id": profile_id,
        "code": code,
        "subcause": subcause,
        "rule": rule,
        "explanation_id": explanation_id,
        "severity": "error",
        "stage": stage or d_stage,
        "operation": operation or d_op,
        "primary_span": span(file, primary[0], primary[1]),
        "related_spans": [{"role": r[3], "span": span(r[0], r[1], r[2])} for r in rel],
        "cause_chain": [
            {"kind": k, "operation": op, "span": span(file, sp[0], sp[1]),
             "entity": entity, "evidence": "verified"}
            for (k, op, sp, entity) in cause_chain],
        "feature_id": feature_id,
        "unexpected_token": (None if unexpected_token is None else
                             {"kind": unexpected_token[0], "text": unexpected_token[1],
                              "span": span(file, unexpected_token[2][0], unexpected_token[2][1])}),
        "expected_tokens": list(expected_tokens),
        "expected_tokens_kind": expected_tokens_kind,
        "parser_context": parser_context,
        "recognized_prefix": (None if recognized_prefix is None else
                              span(file, recognized_prefix[0], recognized_prefix[1])),
        "repair": list(repair),
        "data": {} if data is None else dict(data),
    }
    return diag


def barrier(pending_scope: str, *, file: str, primary, feature_id="implementation_limit") -> dict:
    """NX-E0006/implementation_limit for a pending K capability (K section 9.1)."""
    diag = make("NX-E0006", file=file, primary=primary, feature_id=feature_id,
                data={"pending_scope": pending_scope})
    diag["message"] = BARRIER_MESSAGE.format(k=pending_scope)
    return diag


def check_shape(diag: dict) -> list[str]:
    """Structural problems of a diagnostic (empty list when well formed)."""
    problems = [f"missing field {name}" for name in FIELDS if name not in diag]
    extra = [name for name in diag if name not in FIELDS and name != "message"]
    problems += [f"unexpected field {name}" for name in extra]
    return problems
