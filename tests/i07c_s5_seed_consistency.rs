//! F0-0002 §§4.1.2.2-4.1.2.4, 4.1.2.7, 4.7 y addendum `F0-0002-lagunas.md`
//! («Ancla con componente pendiente y asignacion posterior», D-REF): una capa
//! de forma planificada o una lectura adelantada solo sobreviven si el
//! veredicto de una pasada las confirma; una asignacion rechazada nunca retipa
//! lecturas anteriores. Esperados derivados de esas clausulas, con los spans
//! comprobados por bytes contra la fuente.
use super::i07c_normative_literals::{accepted, invoke};

const PROFILE: &str = "f0-0002-b-v2";
const CASES: &str = include_str!("../docs/validation/i07c-twin/casos.json");

#[derive(Clone, Copy)]
enum Expected {
    /// No `expected_type` relation.
    Unrelated,
    /// `expected_type` relation at this span with this source token.
    At((usize, usize), &'static str),
    /// The ADR does not fix the origin of this relation: only its presence is not checked.
    Unchecked,
}

#[derive(Clone, Copy)]
struct D {
    code: &'static str,
    subcause: Option<&'static str>,
    span: (usize, usize),
    token: &'static str,
    expected_type: &'static str,
    found_type: &'static str,
    origin: Expected,
}

const fn mismatch(
    span: (usize, usize),
    token: &'static str,
    expected_type: &'static str,
    found_type: &'static str,
    origin: Expected,
) -> D {
    D {
        code: "NX-E0102",
        subcause: Some("type_mismatch"),
        span,
        token,
        expected_type,
        found_type,
        origin,
    }
}

const fn join(
    span: (usize, usize),
    token: &'static str,
    expected_type: &'static str,
    found_type: &'static str,
    origin: Expected,
) -> D {
    D {
        code: "NX-E1303",
        subcause: None,
        span,
        token,
        expected_type,
        found_type,
        origin,
    }
}

fn related_span(diagnostic: &str, role: &str) -> Option<(usize, usize)> {
    let marker = format!("\"role\":\"{role}\",\"span\":{{\"file\":");
    let tail = diagnostic.split(&marker).nth(1)?;
    let number = |key: &str| -> usize {
        let rest = tail.split(key).nth(1).unwrap();
        rest.chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .parse()
            .unwrap()
    };
    Some((number("\"start\":"), number("\"end\":")))
}

fn diagnostics_of(json: &str) -> Vec<&str> {
    json.split("{\"schema_version\":1,\"diagnostic_namespace\"")
        .skip(1)
        .collect()
}

/// `check` and `run` reject with exactly these diagnostics, in this order.
fn rejects_with(source: &str, expected: &[D]) {
    assert!(source.is_ascii() && !source.ends_with('\n'), "{source}");
    for verb in ["check", "run"] {
        let (exit, json, stderr) = invoke(verb, source, PROFILE);
        assert_eq!(exit, 1, "{verb}: {source}: {json}: {stderr}");
        assert!(stderr.is_empty(), "{verb}: {stderr}");
        assert!(
            json.contains("\"outcome\":{\"kind\":\"rejected\"}"),
            "{json}"
        );
        for field in ["stdout", "events", "observations"] {
            assert!(
                json.contains(&format!("\"{field}\":[]")),
                "{source}: {json}"
            );
        }
        let actual = diagnostics_of(&json);
        assert_eq!(actual.len(), expected.len(), "{verb}: {source}: {json}");
        for (diagnostic, wanted) in actual.iter().zip(expected) {
            let (start, end) = wanted.span;
            assert!(start < end && end <= source.len(), "{source}");
            assert_eq!(&source[start..end], wanted.token, "{source}");
            let rule = match wanted.subcause {
                Some(sub) => format!("{}/{sub}", wanted.code),
                None => wanted.code.to_owned(),
            };
            for fragment in [
                format!("\"code\":\"{}\"", wanted.code),
                format!(
                    "\"subcause\":{}",
                    wanted
                        .subcause
                        .map_or("null".to_owned(), |s| format!("\"{s}\""))
                ),
                format!("\"rule\":\"{rule}\""),
                format!("\"explanation_id\":\"{rule}\""),
                "\"stage\":\"types\"".to_owned(),
                "\"operation\":\"types.check\"".to_owned(),
                "\"cause_chain\":[]".to_owned(),
                "\"repair\":[]".to_owned(),
                "\"feature_id\":null".to_owned(),
                "\"unexpected_token\":null".to_owned(),
                format!(
                    "\"data\":{{\"expected_type\":\"{}\",\"found_type\":\"{}\"}}",
                    wanted.expected_type, wanted.found_type
                ),
            ] {
                assert!(
                    diagnostic.contains(&fragment),
                    "{verb}: {source}: missing {fragment}: {json}"
                );
            }
            let primary = diagnostic
                .split("\"primary_span\":")
                .nth(1)
                .unwrap()
                .split(",\"related_spans\":")
                .next()
                .unwrap();
            assert!(
                primary.contains(&format!("\"start\":{start},\"end\":{end}")),
                "{verb}: {source}: {json}"
            );
            assert_eq!(
                related_span(diagnostic, "found_type"),
                Some(wanted.span),
                "{verb}: {source}: {json}"
            );
            match wanted.origin {
                Expected::Unrelated => {
                    assert_eq!(
                        related_span(diagnostic, "expected_type"),
                        None,
                        "{verb}: {source}: {json}"
                    );
                }
                Expected::Unchecked => {}
                Expected::At(origin, token) => {
                    assert_eq!(&source[origin.0..origin.1], token, "{source}");
                    assert_eq!(
                        related_span(diagnostic, "expected_type"),
                        Some(origin),
                        "{verb}: {source}: {json}"
                    );
                }
            }
            if !matches!(wanted.origin, Expected::Unchecked) {
                assert_eq!(
                    diagnostic.matches("\"role\":").count(),
                    1 + usize::from(matches!(wanted.origin, Expected::At(..))),
                    "{verb}: {source}: {json}"
                );
            }
        }
    }
}

fn finish(prefix: &str, tail: &str) -> String {
    format!("{prefix}{tail}")
}

fn corpus_source(id: &str) -> &'static str {
    let id_marker = format!("\"id\": \"{id}\"");
    let entry = CASES.split(&id_marker).nth(1).expect("case present");
    let reduced = entry
        .split("\"reduced\": \"")
        .nth(1)
        .expect("reduced source")
        .split('"')
        .next()
        .unwrap();
    assert!(
        !reduced.contains('\\') && reduced.is_ascii(),
        "{id}: escaped source"
    );
    reduced
}

// ---------------------------------------------------------------- repro A

const A_PREFIX: &str =
    "fn Main(){let aar=[[1]];var r=[];let a=(if true then r else (if true then aar else ";

#[test]
fn pending_anchor_with_later_incompatible_assignment_reports_both_independent_conflicts() {
    // Addendum, ejemplo valido: `aar` fija el elemento de `r` y la asignacion concuerda.
    accepted(&finish(A_PREFIX, "r));r=[[3]];}"), PROFILE);
    // Inválido (96 B): E1303 en `[r]` y E0102 en el `3` de `r=[3]`, sin E0103.
    let source = finish(A_PREFIX, "[r]));r=[3];}");
    assert_eq!(source.len(), 96);
    rejects_with(
        &source,
        &[
            join(
                (83, 86),
                "[r]",
                "Array<Array<I64>>",
                "Array<Array<Array<I64>>>",
                Expected::At((53, 54), "r"),
            ),
            mismatch(
                (92, 93),
                "3",
                "Array<I64>",
                "I64",
                Expected::At((74, 77), "aar"),
            ),
        ],
    );
}

#[test]
fn compatible_later_assignment_leaves_only_the_union_conflict() {
    // Cambia solo `r=[3]` por `r=[[3]]`: queda la E1303 de `[r]`.
    rejects_with(
        &finish(A_PREFIX, "[r]));r=[[3]];}"),
        &[join(
            (83, 86),
            "[r]",
            "Array<Array<I64>>",
            "Array<Array<Array<I64>>>",
            Expected::At((53, 54), "r"),
        )],
    );
    // Sin asignacion tardia (90 B): solo la E1303, sin E0103.
    let source = finish(A_PREFIX, "[r]));}");
    assert_eq!(source.len(), 90);
    rejects_with(
        &source,
        &[join(
            (83, 86),
            "[r]",
            "Array<Array<I64>>",
            "Array<Array<Array<I64>>>",
            Expected::At((53, 54), "r"),
        )],
    );
}

#[test]
fn later_incompatible_assignment_alone_reports_only_its_own_element() {
    // Cambia solo `[r]` por `r` frente al caso valido: queda el E0102 del `3`.
    rejects_with(
        &finish(A_PREFIX, "r));r=[3];}"),
        &[mismatch(
            (90, 91),
            "3",
            "Array<I64>",
            "I64",
            Expected::At((74, 77), "aar"),
        )],
    );
}

#[test]
fn union_leaf_order_decides_which_leaf_is_reported() {
    let source = "fn Main(){let aar=[[1]];var r=[];let a=(if true then r else (if true then [r] else aar));r=[3];}";
    assert_eq!(source.len(), 96);
    rejects_with(
        source,
        &[
            join(
                (74, 77),
                "[r]",
                "Array<Array<I64>>",
                "Array<Array<Array<I64>>>",
                Expected::At((53, 54), "r"),
            ),
            mismatch(
                (92, 93),
                "3",
                "Array<I64>",
                "I64",
                Expected::At((83, 86), "aar"),
            ),
        ],
    );
}

#[test]
fn concrete_twin_is_a_contrast_and_not_the_oracle_of_the_deferred_program() {
    // §4.1.2.7: con `var r=[3]` el prefijo ya fija `Array<I64>`; el diferido no es ese caso.
    let twin = "fn Main(){let aar=[[1]];var r=[3];let a=(if true then r else (if true then aar else [r]));}";
    assert_eq!(twin.len(), 91);
    rejects_with(
        twin,
        &[join(
            (75, 78),
            "aar",
            "Array<I64>",
            "Array<Array<I64>>",
            Expected::At((54, 55), "r"),
        )],
    );
}

// ---------------------------------------------------------------- repro B

const B_PREFIX: &str =
    "fn Main(){let p=true;var r=[];var q=[];let a=(if p then p else q);let c=[q,[r]];";

#[test]
fn assignment_order_does_not_change_the_reported_conflicts() {
    let join_q = join(
        (63, 64),
        "q",
        "Bool",
        "Array<Array<I64>>",
        Expected::At((56, 57), "p"),
    );
    // `q=[3]` antes de `r=[3]` y al reves: el mismo conjunto, el `3` de `q` en su sitio.
    rejects_with(
        &finish(B_PREFIX, "q=[3];r=[3];}"),
        &[
            join_q,
            mismatch((83, 84), "3", "Array<I64>", "I64", Expected::Unchecked),
        ],
    );
    rejects_with(
        &finish(B_PREFIX, "r=[3];q=[3];}"),
        &[
            join_q,
            mismatch((89, 90), "3", "Array<I64>", "I64", Expected::Unchecked),
        ],
    );
    // Positivo: `q=[[3]]` es compatible con `c=[q,[r]]`; solo queda la union `a`.
    rejects_with(&finish(B_PREFIX, "q=[[3]];r=[3];}"), &[join_q]);
}

#[test]
fn array_constructor_elements_stay_unresolved_only_when_no_compatible_equality_fixes_them() {
    accepted("fn Main(){var r=[];r=[3];}", PROFILE);
    // `α=Array<α>` no tiene solucion finita (§4.1.2.2): un unico E0103 en el `[]`.
    let (exit, json, stderr) = invoke("check", "fn Main(){var r=[];r=[r];}", PROFILE);
    assert_eq!(exit, 1, "{json}: {stderr}");
    let all = diagnostics_of(&json);
    assert_eq!(all.len(), 1, "{json}");
    assert!(
        all[0].contains("\"code\":\"NX-E0103\"")
            && all[0].contains("\"primary_span\":{\"file\":")
            && all[0].contains("\"start\":16,\"end\":18")
            && all[0].contains("\"data\":{}")
            && all[0].contains("\"related_spans\":[]"),
        "{json}"
    );
}

// ---------------------------------------------------------------- F06, M2, D-REF

#[test]
fn pending_projection_anchor_with_later_assignment_matches_d_ref() {
    let prefix = "fn Main(){let ar=[1];var r=[];let a0=(if true then r[1] else (if false then ";
    // Positivo: `ar` en ambas hojas fija el elemento y `r=[[3]]` concuerda; la
    // condicion `false` evita leer `r[1]` en la ejecucion.
    accepted(
        "fn Main(){let ar=[1];var r=[];let a0=(if false then r[1] else (if false then ar else ar));r=[[3]];}",
        PROFILE,
    );
    // Hoja `r` recursiva frente a `r[1]`: una sola E1303 y `ar` no recibe otra.
    rejects_with(
        &finish(prefix, "r else ar));r=[[3]];}"),
        &[join(
            (76, 77),
            "r",
            "Array<I64>",
            "Array<Array<I64>>",
            Expected::At((51, 55), "r[1]"),
        )],
    );
    // F06 mínimo: ademas la asignacion `r=[3]` incompatible con `α=Array<I64>`.
    rejects_with(
        &finish(prefix, "r else ar));r=[3];}"),
        &[
            join(
                (76, 77),
                "r",
                "Array<I64>",
                "Array<Array<I64>>",
                Expected::At((51, 55), "r[1]"),
            ),
            mismatch(
                (91, 92),
                "3",
                "Array<I64>",
                "I64",
                Expected::At((83, 85), "ar"),
            ),
        ],
    );
}

#[test]
fn d_ref_valid_and_invalid_programs() {
    accepted(
        "fn Main(){let aar=[[1]];var r=[];let a=(if false then r[0] else aar);r=[[[3]]];}",
        PROFILE,
    );
    let source = "fn Main(){let aar=[[1]];var r=[];let a=(if true then r[0] else (if true then r else aar));r=aar;}";
    assert_eq!(source.len(), 97);
    rejects_with(
        source,
        &[
            join(
                (77, 78),
                "r",
                "Array<Array<I64>>",
                "Array<Array<Array<I64>>>",
                Expected::At((53, 57), "r[0]"),
            ),
            mismatch(
                (92, 95),
                "aar",
                "Array<Array<Array<I64>>>",
                "Array<Array<I64>>",
                Expected::At((84, 87), "aar"),
            ),
        ],
    );
}

// ---------------------------------------------------------------- O01, O03, O07, O08

const ARRAY3: &str = "Array<Array<Array<I64>>>";

#[test]
fn deferred_union_with_assignment_inside_array_and_condition_o03() {
    let source = corpus_source("O03");
    rejects_with(
        source,
        &[
            join(
                (212, 215),
                "[r]",
                "Array<Array<I64>>",
                ARRAY3,
                Expected::At((185, 186), "r"),
            ),
            mismatch((277, 278), "q", "Bool", "Array<I64>", Expected::Unrelated),
            mismatch(
                (358, 359),
                "x",
                "Array<I64>",
                "I64",
                Expected::At((203, 206), "aar"),
            ),
        ],
    );
}

#[test]
fn deferred_union_with_numeric_comparison_and_assignment_o07() {
    let source = corpus_source("O07");
    rejects_with(
        source,
        &[
            join(
                (149, 152),
                "[r]",
                "Array<Array<I64>>",
                ARRAY3,
                Expected::At((119, 120), "r"),
            ),
            mismatch(
                (159, 160),
                "3",
                "Array<I64>",
                "I64",
                Expected::At((140, 143), "aar"),
            ),
        ],
    );
}

#[test]
fn deferred_union_after_invalid_condition_and_unindexable_leaf_o01() {
    let source = corpus_source("O01");
    let (exit, json, _) = invoke("check", source, PROFILE);
    assert_eq!(exit, 1, "{json}");
    let all = diagnostics_of(&json);
    assert_eq!(all.len(), 4, "{json}");
    // La segunda es `NX-E0102/not_indexable` (`false[x]`): dominio array, hallado Bool.
    assert!(
        all[1].contains("\"subcause\":\"not_indexable\"")
            && all[1]
                .contains("\"data\":{\"expected_type\":\"domain:array\",\"found_type\":\"Bool\"}")
            && all[1].contains("\"start\":139,\"end\":144")
            && related_span(all[1], "expected_type").is_none(),
        "{json}"
    );
    assert_eq!(&source[139..144], "false");
    for fragment in [
        "\"code\":\"NX-E0102\"",
        "\"data\":{\"expected_type\":\"Bool\",\"found_type\":\"Array<Array<Array<I64>>>\"}",
        "\"start\":93,\"end\":99",
    ] {
        assert!(all[0].contains(fragment), "{fragment}: {json}");
    }
    assert_eq!(&source[93..99], "[r][0]");
    assert!(related_span(all[0], "expected_type").is_none());
    for (index, span, token, expected_type, found_type, origin) in [
        (2, (191, 192), "r", "Array<Array<I64>>", ARRAY3, (166, 170)),
        (
            3,
            (209, 212),
            "aar",
            ARRAY3,
            "Array<Array<I64>>",
            (198, 203),
        ),
    ] {
        assert_eq!(&source[span.0..span.1], token);
        assert_eq!(related_span(all[index], "found_type"), Some(span), "{json}");
        assert_eq!(
            related_span(all[index], "expected_type"),
            Some(origin),
            "{json}"
        );
        assert!(
            all[index].contains(&format!(
                "\"data\":{{\"expected_type\":\"{expected_type}\",\"found_type\":\"{found_type}\"}}"
            )),
            "{json}"
        );
    }
    assert!(all[2].contains("\"code\":\"NX-E1303\"") && all[3].contains("\"code\":\"NX-E0102\""));
    assert!(!json.contains("NX-E0103"), "{json}");
}

#[test]
fn deferred_union_with_four_independent_conditions_o08() {
    let source = corpus_source("O08");
    rejects_with(
        source,
        &[
            mismatch((93, 94), "r", "Bool", ARRAY3, Expected::Unrelated),
            mismatch((100, 101), "r", "Bool", ARRAY3, Expected::Unrelated),
            mismatch((107, 109), "ar", "Bool", "Array<I64>", Expected::Unrelated),
            mismatch((130, 131), "0", "Bool", "I64", Expected::Unrelated),
            join(
                (137, 138),
                "r",
                "Array<Array<I64>>",
                ARRAY3,
                Expected::At((116, 120), "r[1]"),
            ),
            mismatch(
                (155, 158),
                "aar",
                ARRAY3,
                "Array<Array<I64>>",
                Expected::At((144, 147), "aar"),
            ),
        ],
    );
}

// ---------------------------------------------------------------- corpus 26+1

fn spurious_constructor_errors(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    for verb in ["check", "run"] {
        let (exit, json, stderr) = invoke(verb, source, PROFILE);
        assert!(exit == 0 || exit == 1, "{verb}: {source}: {stderr}");
        assert!(stderr.is_empty(), "{verb}: {stderr}");
        for diagnostic in diagnostics_of(&json) {
            if diagnostic.contains("\"code\":\"NX-E0103\"") {
                let (start, end) = related_primary(diagnostic);
                found.push(format!("{verb}:{start}-{end}"));
            }
        }
    }
    found
}

fn related_primary(diagnostic: &str) -> (usize, usize) {
    let tail = diagnostic
        .split("\"primary_span\":{\"file\":")
        .nth(1)
        .unwrap();
    let number = |key: &str| -> usize {
        tail.split(key)
            .nth(1)
            .unwrap()
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .parse()
            .unwrap()
    };
    (number("\"start\":"), number("\"end\":"))
}

const CASE_IDS: [&str; 27] = [
    "O01", "O02", "O03", "O04", "O05", "O06", "O07", "O08", "F01", "F02", "F03", "F04", "F05",
    "F06", "F07", "F08", "F09", "F10", "F11", "F12", "F13", "F14", "F15", "F16", "F17", "F18",
    "F19",
];

#[test]
fn twin_corpus_has_no_spurious_unresolved_constructor() {
    for id in CASE_IDS {
        let source = corpus_source(id);
        let errors = spurious_constructor_errors(source);
        match id {
            // F08: el elemento del constructor sigue genuinamente irresuelto al cierre.
            "F08" => assert_eq!(errors, ["check:108-149", "run:108-149"], "{id}"),
            // F10: cota conocida (residuo FAM-E, igual que antes del corte): 2 E0103 por verbo.
            "F10" => assert_eq!(errors.len(), 4, "{id}: {errors:?}"),
            _ => assert!(errors.is_empty(), "{id}: {errors:?}: {source}"),
        }
    }
}

fn trailing_assignments(source: &str) -> Option<(String, Vec<String>)> {
    let body = source.strip_prefix("fn Main(){")?.strip_suffix('}')?;
    let mut depth = 0i32;
    let mut ends = Vec::new();
    for (index, byte) in body.bytes().enumerate() {
        match byte {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b';' if depth == 0 => ends.push(index + 1),
            _ => {}
        }
    }
    let mut statements = Vec::new();
    let mut end = body.len();
    for &cut in ends.iter().rev() {
        if cut != end {
            continue;
        }
        let start = ends
            .iter()
            .copied()
            .filter(|&other| other < cut)
            .max()
            .unwrap_or(0);
        let statement = &body[start..cut];
        let is_assignment = statement.split_once('=').is_some_and(|(name, rest)| {
            !name.is_empty()
                && name
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
                && !rest.starts_with('=')
        });
        if !is_assignment || statements.len() == 3 {
            break;
        }
        statements.push(statement.to_owned());
        end = start;
    }
    if statements.len() < 2 {
        return None;
    }
    statements.reverse();
    Some((format!("fn Main(){{{}", &body[..end]), statements))
}

fn permutations(items: &[String]) -> Vec<Vec<String>> {
    if items.len() <= 1 {
        return vec![items.to_vec()];
    }
    let mut all = Vec::new();
    for index in 0..items.len() {
        let mut rest = items.to_vec();
        let head = rest.remove(index);
        for mut tail in permutations(&rest) {
            tail.insert(0, head.clone());
            all.push(tail);
        }
    }
    all
}

#[test]
fn twin_corpus_assignment_permutations_keep_constructors_resolved() {
    let mut permuted = 0;
    for id in CASE_IDS {
        if matches!(id, "F08" | "F10") {
            continue;
        }
        let Some((head, statements)) = trailing_assignments(corpus_source(id)) else {
            continue;
        };
        for order in permutations(&statements) {
            let source = format!("{head}{}}}", order.concat());
            let errors = spurious_constructor_errors(&source);
            assert!(errors.is_empty(), "{id}: {errors:?}: {source}");
            permuted += 1;
        }
    }
    assert!(permuted >= 20, "{permuted}");
}

// ---------------------------------------------------------------- lectura adelantada y capas retraidas

#[test]
fn retracted_seed_site_keeps_its_element_fixed_by_a_later_compatible_leaf() {
    // §4.1.2.2/§4.1.2.4: `aar` (union 2) fija el elemento de `r` a `Array<I64>`; el `1` de la
    // union 1 es incompatible con `r` ya completado (su E1303 usa los tipos completos) y `r=[r]`
    // es la asignacion incompatible (su `r` interior). Ningun constructor queda sin resolver.
    let source = "fn Main(){let x=1;let ar=[1];let aar=[[1]];var r=[];let a=((if true then r else 1)==ar);Print((if x then r else (if (x<1) then aar else [r])));r=[r];}";
    assert_eq!(source.len(), 150);
    rejects_with(
        source,
        &[
            join(
                (80, 81),
                "1",
                "Array<Array<I64>>",
                "I64",
                Expected::At((73, 74), "r"),
            ),
            mismatch((98, 99), "x", "Bool", "I64", Expected::Unrelated),
            join(
                (136, 139),
                "[r]",
                "Array<Array<I64>>",
                "Array<Array<Array<I64>>>",
                Expected::At((105, 106), "r"),
            ),
            mismatch(
                (146, 147),
                "r",
                "Array<I64>",
                "Array<Array<I64>>",
                Expected::At((127, 130), "aar"),
            ),
        ],
    );
    // Cambia solo `r=[r]` por `r=[[3]]`: desaparece el E0102 de la asignacion y quedan los otros tres.
    rejects_with(
        &source.replace("r=[r];", "r=[[3]];"),
        &[
            join(
                (80, 81),
                "1",
                "Array<Array<I64>>",
                "I64",
                Expected::At((73, 74), "r"),
            ),
            mismatch((98, 99), "x", "Bool", "I64", Expected::Unrelated),
            join(
                (136, 139),
                "[r]",
                "Array<Array<I64>>",
                "Array<Array<Array<I64>>>",
                Expected::At((105, 106), "r"),
            ),
        ],
    );
    // Positivo: unica union compatible, `r=[[3]]` concuerda con `aar`.
    accepted(
        "fn Main(){let aar=[[1]];var r=[];let a=(if true then r else aar);r=[[3]];}",
        PROFILE,
    );
}

// ---------------------------------------------------------------- proteccion del segundo mecanismo

#[test]
fn nested_union_with_pending_anchor_reports_a_single_union_conflict() {
    // `r=[true]` es compatible con `α`; la unica discrepancia de la union es la hoja `1`.
    let source =
        "fn Main(){let p=true;var r=[];let a0=(if p then r else (if p then 1 else 2));r=[true];}";
    rejects_with(
        source,
        &[join(
            (66, 67),
            "1",
            "Array<Bool>",
            "I64",
            Expected::At((48, 49), "r"),
        )],
    );
    accepted(
        "fn Main(){let p=true;var r=[];let a0=(if p then r else (if p then r else r));r=[true];}",
        PROFILE,
    );
}
