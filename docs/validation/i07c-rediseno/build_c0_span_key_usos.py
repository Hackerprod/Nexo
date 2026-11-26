"""Freeze the 240 span_key sites in main 31d91db for C0 review.

The line is a locator only.  Auditors match function + expression + ordinal.
"""

import hashlib
import json
import re
import subprocess
from collections import Counter, defaultdict
from pathlib import Path


BASE = "31d91dbebd31c35b3cd09095c2d8e65b96c80548"
SOURCE = "src/check/types.rs"
OUT = Path(__file__).with_name("c0-span-key-usos.json")


def classify(line, context, function, number):
    """Return destination, use, decision rule for an individual source site."""
    if number == 710:
        return ("span de salida", "definición del conversor", "La función solo extrae límites de un Span; se conserva para salida y comparación de spans.")
    if function.startswith(("reduced_quota_", "i07c_survivor2_")) and "primary_span" in line:
        return ("span de salida", "aserción de diagnóstico", "Compara los bytes del span primario emitido, sin identificar un hecho semántico.")
    if function == "assert_child":
        return ("ResultStatus", "prueba de dependencia por ocurrencia", "La prueba accede al estado de la expresión hija/padre; la clave pasa a OccurrenceId del ResultStatus.")
    if function == "occurrence":
        if "Constructor" in line or "PatternConstructor" in line:
            return ("ConstructorDuty", "clave de deber de constructor", "La variante de obligación identifica el constructor dueño, no el intervalo fuente.")
        if "Join" in line:
            return ("JoinRecord", "clave de unión pendiente", "La unión se identifica por la expresión dueña y el índice de continuación.")
        return ("Pending", "clave de obligación pendiente", "El dominio/proyección pendiente se identifica por OccurrenceId y clase de obligación.")
    if "with_independent_fact" in context or "child_fact" in context or "begin_visit" in context or "DriveStart::" in line:
        return ("FactId", "sitio de visita transitoria", "El span aportaba el sitio al hecho de visita; FactId queda transitorio y transporta el OccurrenceId del AST.")
    if function in ("pending_operation_guard", "pending_operation_type"):
        return ("Pending", "búsqueda de operación pendiente", "Compara la ocurrencia del operando con la del deber nominal/binario/guardado, nunca sus bytes.")
    if function == "resolve_pending_projections":
        return ("Pending", "sitio de proyección pendiente", "El resultado de proyección pertenece a su obligación y a su ocurrencia AST.")
    if function in ("record_unresolved_sites", "expression_variable") or "expr_vars" in line or "expression_types" in context:
        return ("OccurrenceId", "tipo o variable de expresión", "El tipo/variable se indexa por la ocurrencia AST, incluso si comparte span.")
    if function == "capture_expression_dependencies":
        return ("ResultStatus", "fuentes nominales de expresión", "El estado/dependencias se registra por ocurrencia de la expresión.")
    if function == "suppress_result_dependents":
        return ("Pending", "supresión de dependiente nominal", "La dependencia del resultado se enlaza a la obligación nominal dueña; los spans del error quedan para salida.")
    if function == "function" and 7600 <= number <= 8200:
        if number in (7797, 7808, 7817, 7832):
            return ("ConstructorDuty", "causalidad de inferencia de constructor", "La causa E0103 se atribuye al deber de constructor; el span se usa solo al serializarla.")
        return ("Pending", "cadena de obligaciones o resultados fallidos", "La clave sigue la obligación y su dependencia causal por ID, no la posición fuente.")
    if function == "check_with_partial_limit" and number in (15609, 15610):
        return ("Pending", "sitio de obligación tras cuota", "El deber pendiente conserva su OccurrenceId aunque el análisis agote presupuesto.")
    if function == "check_with_partial_limit" and number == 15432:
        return ("Completion", "profundidad tardía", "La comprobación tardía pertenece a la completación del origen, con span solo para diagnóstico.")
    if function in ("finish_statement_at_site", "finish_let") and "conditional_result" in context:
        return ("ResultStatus", "resultado condicional", "El resultado condicional es estado de la ocurrencia de valor.")
    if "conditional_result" in context or "set_conditional_result" in context:
        return ("ResultStatus", "resultado condicional", "El resultado condicional pertenece al estado de la ocurrencia.")
    if "occurrences" in context or "OccurrenceFlag::" in context or ".mark(" in context:
        return ("ResultStatus", "bandera o estado de ocurrencia", "Los bits y dependencias de resultado se indexan por OccurrenceId dentro de ResultStatus.")
    if function == "normal_witness_span":
        return ("ResultStatus", "estado de testigo normal", "La bandera de divergencia pertenece al resultado de la expresión.")
    if function in ("finish_statement_at_site", "finish_let", "branch_accept") and ".get(" in context:
        return ("OccurrenceId", "lectura de tipo de expresión", "El tipo calculado se busca por ocurrencia AST.")
    if function == "enter_binary":
        return ("FactId", "visita del operador", "El hecho transitorio recibe el OccurrenceId del operador.")
    if function == "propagate_group":
        return ("ResultStatus", "propagación de grupo", "Los estados del valor interior se copian al estado de la ocurrencia agrupada.")
    if function == "check_against_context_one":
        return ("ReadContext", "lectura contextual de expresión", "La comparación con el esperado usa el contexto inmutable de entrada y el OccurrenceId de la expresión.")
    if function in ("finish_deferred_binary_right", "finish_binary_right", "array_accept", "array_finish", "expr_inner", "resume_non_match", "if_advance", "if_after_first", "branch", "comparison_domain_error", "finish_union_call", "assert_eq_accept", "check_intrinsic", "option_none"):
        return ("ConsumerId", "restricción de consumidor", "El operador, array, unión o llamada dueña fija ConsumerId; su ranura identifica el operando.")
    return ("OccurrenceId", "sitio de expresión", "La ocurrencia AST sustituye la clave por intervalo; el span permanece como atributo de salida.")


def main():
    raw = subprocess.check_output(["git", "show", f"{BASE}:{SOURCE}"])
    lines = raw.decode("utf-8").splitlines()
    fn_re = re.compile(r"^\s*(?:(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?)?fn\s+([A-Za-z_][A-Za-z_0-9]*)")
    function = "(módulo)"
    ordinals = defaultdict(int)
    rows = []
    for number, raw_line in enumerate(lines, 1):
        match = fn_re.match(raw_line)
        if match:
            function = match.group(1)
        if "span_key" not in raw_line:
            continue
        expression = raw_line.strip()
        context = " ".join(line.strip() for line in lines[max(0, number - 4):min(len(lines), number + 2)])
        domain, use, rule = classify(expression, context, function, number)
        ordinals[(function, expression)] += 1
        rows.append({
            "archivo": SOURCE,
            "funcion": function,
            "expresion": expression,
            "ordinal_misma_expresion_en_funcion": ordinals[(function, expression)],
            "linea_base_solo_localizador": number,
            "uso": use,
            "dominio_destino": domain,
            "regla_decision": rule,
        })
    assert len(rows) == 240, len(rows)
    counts = Counter(row["dominio_destino"] for row in rows)
    document = {
        "base": BASE,
        "archivo": SOURCE,
        "sha256_blob": hashlib.sha256(raw).hexdigest(),
        "criterio_g3": "Comparar cada llamada por (archivo, función, expresión Rust normalizada, ordinal de esa expresión en la función), sin depender del número de línea. Cada uso semántico migra al dominio indicado; solo las filas 'span de salida' conservan los bytes como clave de comparación o serialización. Ningún cambio de línea por refactor crea o elimina una fila.",
        "total": len(rows),
        "por_dominio": dict(sorted(counts.items())),
        "usos": rows,
    }
    OUT.write_text(json.dumps(document, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
