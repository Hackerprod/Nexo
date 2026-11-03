use nexo::check::check_proposed;
use nexo::diagnostic::DataValue;

type Cause = (
    String,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
);

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut x = self.0;
        x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        x ^ (x >> 31)
    }

    fn pick(&mut self, max: u64) -> u64 {
        self.next() % max
    }
}

fn expression(rng: &mut Rng, depth: usize, number: usize) -> String {
    if depth == 0 || rng.pick(5) == 0 {
        return match rng.pick(5) {
            0 => "ar".into(),
            1 => "p".into(),
            2 => "true".into(),
            3 => "ar[0]".into(),
            _ => number.to_string(),
        };
    }
    let next = depth - 1;
    match rng.pick(8) {
        0 => format!(
            "[{},{}]",
            expression(rng, next, number),
            expression(rng, next, number)
        ),
        1 => format!("ar[{}]", expression(rng, next, number)),
        2 => format!("Len({})", expression(rng, next, number)),
        3 => format!(
            "(if p then {} else {})",
            expression(rng, next, number),
            expression(rng, next, number)
        ),
        other => {
            let op = ["+", "-", "<", "=="][((other - 4) % 4) as usize];
            format!(
                "({} {op} {})",
                expression(rng, next, number),
                expression(rng, next, number)
            )
        }
    }
}

fn rhs_causes(source: &str, rhs_start: usize) -> Vec<Cause> {
    let errors = check_proposed("metamorphic.n", source, false)
        .err()
        .unwrap_or_default();
    errors
        .into_iter()
        .filter_map(|error| {
            error.validate("metamorphic.n", source).unwrap();
            let site = error.primary_span.as_ref()?;
            ((site.start as usize) >= rhs_start).then(|| {
                let field = |name| match error.data.get(name) {
                    Some(DataValue::String(value)) => Some(value.clone()),
                    _ => None,
                };
                (
                    error.code.clone(),
                    error.subcause.clone(),
                    source[site.start as usize..site.end as usize].to_owned(),
                    field("expected_type"),
                    field("found_type"),
                )
            })
        })
        .collect()
}

fn all_causes(source: &str) -> Vec<Cause> {
    rhs_causes(source, 0)
}

fn late_assignment_conflicts_with_prefix(source: &str) -> bool {
    // §4.1.2.7: if an accepted prefix fixed a different element type,
    // the deferred program blames the later assignment while the concrete
    // twin blames the earlier incompatible use.
    let assignment_rhs = source.rfind("r=").unwrap() + 2;
    check_proposed("metamorphic.n", source, false)
        .err()
        .unwrap_or_default()
        .iter()
        .any(|error| {
            error.code == "NX-E0102"
                && error
                    .primary_span
                    .as_ref()
                    .is_some_and(|span| span.start as usize >= assignment_rhs)
        })
}

fn expression_twin(rng: &mut Rng, depth: usize) -> String {
    let mut value = match rng.pick(7) {
        0 => "(r < r)".to_string(),
        1 => "(r == r)".to_string(),
        2 => "(r + x)".to_string(),
        3 => "(x + r)".to_string(),
        4 => "r[0]".to_string(),
        5 => "Len(r)".to_string(),
        _ => "r[(r < r)]".to_string(),
    };
    for _ in 0..depth {
        value = match rng.pick(9) {
            0 => format!("[{value},true]"),
            1 => format!("[true,{value}]"),
            2 => format!("(if p then {value} else x)"),
            3 => format!("(if p then x else {value})"),
            4 => format!("(if p then [ar] else {value})"),
            5 => format!("(if p then {value} else ar[x])"),
            6 => format!("({value} + x)"),
            7 => format!("(x < {value})"),
            _ => format!("({value} == [ar])"),
        };
    }
    value
}

#[test]
#[ignore = "50,000-program metamorphic campaign; run explicitly in debug and release"]
fn poisoned_assignment_and_independent_let_have_identical_rhs_causes() {
    // F0-0002 §4.1.2.5: a poisoned destination does not remove any C from
    // the independent RHS. Seed 21031 is distinct from independent seed 16401.
    let mut rng = Rng(21031);
    let mut seen = std::collections::HashSet::new();
    for number in 1..=50_000 {
        let rhs = format!(
            "(if p then {} else {number})",
            expression(&mut rng, 3, number)
        );
        assert!(seen.insert(rhs.clone()), "duplicate RHS {number}");
        let prefix = "fn Main(){let ar=[1];let p=true;var t=true+1;";
        let assigned = format!("{prefix}t={rhs};}}");
        let independent = format!("{prefix}let z={rhs};}}");
        let left = rhs_causes(&assigned, prefix.len() + 2);
        let right = rhs_causes(&independent, prefix.len() + 6);
        assert_eq!(left, right, "case {number}: {rhs}");
    }
}

#[test]
#[ignore = "campaña de gemelos de 300.000 fuentes; ejecutar por perfil"]
fn later_array_assignment_and_concrete_initializer_have_identical_causes() {
    // F0-0002 §§4.1.2.2–7. Seeds 21032/21033 are independent of
    // 16801–16803. The number changes a live I64 binding in every source.
    let mut differences = Vec::new();
    let mut incompatible_prefixes = 0_u64;
    for (seed, count) in [(21032, 150_000), (21033, 150_000)] {
        let mut rng = Rng(seed);
        for number in 1..=count {
            let depth = 1 + rng.pick(3) as usize;
            let expression = expression_twin(&mut rng, depth);
            let array = if rng.pick(2) == 0 { "[3]" } else { "[[true]]" };
            let prefix = format!("fn Main(){{let p=true;let x={number};let ar=[1];");
            let deferred = format!("{prefix}var r=[];let a={expression};r={array};}}");
            let concrete = format!("{prefix}let r={array};let a={expression};}}");
            let actual = all_causes(&deferred);
            let twin = all_causes(&concrete);
            if actual != twin {
                if late_assignment_conflicts_with_prefix(&deferred) {
                    incompatible_prefixes += 1;
                    continue;
                }
                differences.push((seed, number, array, expression, actual, twin));
                if differences.len() == 20 {
                    panic!("first 20 expression-twin differences: {differences:#?}");
                }
            }
        }
    }
    eprintln!("§4.1.2.7 incompatible prefixes: {incompatible_prefixes}");
    assert!(differences.is_empty(), "{differences:#?}");
}
