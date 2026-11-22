"""Temporal barrier of capabilities K02..K10 (class MODELO; sources: K 9.1 and 12 only).

``CONSTRUCT_K`` maps each construction kind to the capability that closes it.  A
construction whose K is not in the manifest receives NX-E0006/implementation_limit with
``pending_scope`` equal to that K; ties on the same position break by K order.  Syntax
errors precede the barrier (K 9.1, "Bajo perfiles K"); that ordering is applied by the
callers, which run ``literal`` first.
"""

from __future__ import annotations

from dataclasses import dataclass

from . import diagnostics

ALL_K = ("K01", "K02", "K03", "K04", "K05", "K06", "K07", "K08", "K09", "K10")

# K 12: dependencies of each capability (K 12 table, column "Dependencias").
DEPENDS = {
    "K01": (),
    "K02": ("K01",),
    "K03": ("K02",),
    "K04": ("K03",),
    "K05": ("K04",),
    "K06": ("K05",),
    "K07": ("K05",),
    "K08": ("K05",),
    "K09": ("K06", "K08"),
    "K10": ("K06", "K07", "K09"),
}

# Plan 2.1: phase -> (must contain, may additionally contain).
PHASES = {
    "G0": (("K01",), ()),
    "G1": (("K01", "K02"), ()),
    "G2": (("K01", "K02", "K03"), ()),
    "G3": (("K01", "K02", "K03", "K04"), ()),
    "G4": (("K01", "K02", "K03", "K04", "K05"), ()),
    "G5": (("K01", "K02", "K03", "K04", "K05", "K06"), ("K07", "K08")),
    "G6": (("K01", "K02", "K03", "K04", "K05", "K07"), ("K06", "K08")),
    "G7": (("K01", "K02", "K03", "K04", "K05", "K08"), ("K06", "K07")),
    "G8": (("K01", "K02", "K03", "K04", "K05", "K06", "K08", "K09"), ("K07",)),
    "G9": (ALL_K, ()),
    "G10": (ALL_K, ()),
}

# K 9.1 table and K 12 rows (plan 2.3).  Kinds are the oracle's own construction names.
CONSTRUCT_K = {
    # K02: literal String without Hole, String as type/owner/copy, ==, Print, ByteLen
    "string_literal": "K02", "string_type": "K02", "string_copy": "K02",
    "string_eq": "K02", "string_print": "K02", "string_bytelen": "K02",
    # K03: rest of the text operations, qualified std.*, != and orders, Array<String>
    "string_is_empty": "K03", "string_scalar_len": "K03", "string_concat": "K03",
    "string_slice_bytes": "K03", "string_scalars": "K03", "std_text_from_i64": "K03",
    "std_parse_i64": "K03", "string_ne": "K03", "string_order": "K03",
    "array_string": "K03", "for_string": "K03",
    # K04: nominal declarations, fields, constructors, Array/for nominal
    "struct_decl": "K04", "enum_decl": "K04", "field_access": "K04",
    "nominal_constructor": "K04", "array_nominal": "K04", "for_nominal": "K04",
    # K05: resource inside a nominal
    "resource_in_nominal": "K05",
    # K06: extend, use extension, methods
    "extend_decl": "K06", "use_extension": "K06", "method_call": "K06",
    # K07: nominal patterns
    "nominal_pattern": "K07",
    # K08: derive, nominal ==/!=, transitive capacity, literal with Hole
    "derive_attr": "K08", "nominal_eq": "K08", "transitive_capacity": "K08",
    "string_hole": "K08",
    # K09: pub, ordinary use, alias, export
    "pub_decl": "K09", "use_import": "K09", "use_alias": "K09", "pub_use": "K09",
    # K10: std.console.*
    "console_call": "K10",
}

# Constructions whose K is ambiguous in K (plan 2.3 / LAGUNAS LF-02): excluded from class B.
AMBIGUOUS_KINDS = {
    "print_nominal": ("K04", "K08"),
    "eq_nominal_bare": ("K04", "K08"),
    "array_resource": ("K04", "K05"),
    "readline_with_use": ("K09", "K10"),
}


class ManifestError(Exception):
    """Impossible manifest: an arness error (exit 2), never a product verdict."""


@dataclass(frozen=True)
class Construct:
    kind: str
    span: tuple        # UTF-8 byte interval


def validate_manifest(phase: str, complete) -> tuple:
    if phase not in PHASES:
        raise ManifestError(f"unknown phase {phase}")
    declared = tuple(complete)
    if len(set(declared)) != len(declared):
        raise ManifestError("duplicate capability in --complete")
    for k in declared:
        if k not in ALL_K:
            raise ManifestError(f"unknown capability {k}")
        for dep in DEPENDS[k]:
            if dep not in declared:
                raise ManifestError(f"{k} requires {dep}")
    must, may = PHASES[phase]
    for k in must:
        if k not in declared:
            raise ManifestError(f"{phase} requires {k}")
    for k in declared:
        if k not in must and k not in may:
            raise ManifestError(f"{phase} cannot include {k}")
    return tuple(sorted(declared))


def literal_constructs(span, holes: int) -> list:
    """A String literal is K02; one with at least one Hole is additionally K08."""
    out = [Construct("string_literal", span)]
    if holes > 0:
        out.append(Construct("string_hole", span))
    return out


def pending(constructs, closed) -> list:
    out = []
    for c in constructs:
        k = CONSTRUCT_K[c.kind]
        if k not in closed:
            out.append((c.span[0], int(k[1:]), k, c))
    out.sort(key=lambda item: (item[0], item[1]))
    return out


def select(constructs, closed):
    """First pending construction in source order, ties by K order; None if none."""
    found = pending(constructs, closed)
    return found[0][3] if found else None


def distinct_pending_scopes(constructs, closed) -> set:
    return {item[2] for item in pending(constructs, closed)}


def class_b_eligible(constructs, closed, ambiguous=()) -> bool:
    """Class B sources hold exactly one pending K and no ambiguous construction."""
    if ambiguous:
        return False
    return len(distinct_pending_scopes(constructs, closed)) == 1


def barrier_diagnostic(constructs, closed, file: str):
    """NX-E0006/implementation_limit for the selected construction, or None."""
    chosen = select(constructs, closed)
    if chosen is None:
        return None
    return diagnostics.barrier(CONSTRUCT_K[chosen.kind], file=file, primary=chosen.span)


def compares_primary(kind: str) -> bool:
    """Q6: the primary is compared only for a String literal barrier (K 9.1, N13)."""
    return kind in ("string_literal", "string_hole")
