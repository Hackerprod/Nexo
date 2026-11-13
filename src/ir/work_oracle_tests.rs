//! Independent source-tree generator and cost oracle for basic work rows.
use crate::{interp, ir};

const VERSION: &str = "i10b-owned-tree-v2";
const NORMAL_COUNT: usize = 3_000;
const DEFAULT_SEED: u64 = 0x0083_1106_1106_c0de;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NodeKind {
    Integer,
    Array,
    Add,
}

#[derive(Clone, Debug)]
enum Expr {
    Integer(u8),
    Array(Vec<Expr>),
    Add(Box<Expr>, Box<Expr>),
}

#[derive(Clone, Debug)]
struct RecordedSpan {
    kind: NodeKind,
    start: usize,
    end: usize,
}

#[derive(Default)]
struct Printer {
    text: String,
    spans: Vec<RecordedSpan>,
}

impl Printer {
    fn expression(&mut self, expression: &Expr) {
        let start = self.text.len();
        let kind = match expression {
            Expr::Integer(value) => {
                self.text.push_str(&value.to_string());
                NodeKind::Integer
            }
            Expr::Array(items) => {
                self.text.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        self.text.push(',');
                    }
                    self.expression(item);
                }
                self.text.push(']');
                NodeKind::Array
            }
            Expr::Add(left, right) => {
                self.text.push('(');
                self.expression(left);
                self.text.push('+');
                self.expression(right);
                self.text.push(')');
                NodeKind::Add
            }
        };
        self.spans.push(RecordedSpan {
            kind,
            start,
            end: self.text.len(),
        });
    }
}

fn work(expression: &Expr) -> u64 {
    match expression {
        Expr::Integer(_) => 2,
        Expr::Array(items) => 2 + items.iter().map(work).sum::<u64>(),
        Expr::Add(left, right) => 2 + work(left) + work(right),
    }
}

fn make_expression(state: &mut u64, depth: usize, family: usize) -> Expr {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    match family % 3 {
        0 => Expr::Integer((*state % 31) as u8),
        1 if depth < 3 => {
            let len = (*state % 3 + 1) as usize;
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                *state ^= *state << 13;
                *state ^= *state >> 7;
                *state ^= *state << 17;
                items.push(Expr::Integer((*state % 31) as u8));
            }
            Expr::Array(items)
        }
        _ if depth < 3 => {
            let left = Expr::Integer((*state % 17) as u8);
            *state ^= *state << 13;
            *state ^= *state >> 7;
            *state ^= *state << 17;
            let right = Expr::Integer((*state % 17) as u8);
            Expr::Add(Box::new(left), Box::new(right))
        }
        _ => Expr::Integer((*state % 31) as u8),
    }
}

// This is the only adapter from the independently generated source to production.
fn observe(source: &str) -> Result<(interp::Outcome, u64, usize), ir::CompileError> {
    let program = ir::compile_scalar("owned_work_oracle.n", source)?;
    let execution = interp::run_scalar(&program, Default::default());
    Ok((
        execution.outcome,
        execution.work_consumed,
        execution.diagnostics.len(),
    ))
}

const CATALOG_ROWS: &[(&str, &str, u64)] = &[
    ("entry", "fn Main(){}", 2),
    ("final_unit", "fn Main(){}", 2),
    (
        "while_condition",
        "fn Main(){var i=0;while i<1{i=i+1;}}",
        28,
    ),
    (
        "inout_prefix",
        "fn E(x:edit I64){}fn Main(){var x=1;E(edit x);}",
        11,
    ),
    (
        "read_selector",
        "fn Main(){var a=[3,4];var v=7;var w=a[1];}",
        18,
    ),
    ("read_name", "fn Main(){var a=[3,4];var v=7;var w=v;}", 15),
    (
        "place_selector",
        "fn E(x:edit I64){}fn Main(){var a=[1];E(edit a[0]);}",
        16,
    ),
    ("return_statement", "fn Main(){return;}", 3),
    (
        "value_statement_explicit",
        "fn Main(){let y=(if true then {value:1;}else{value:2;});}",
        9,
    ),
    ("expression_statement", "fn Main(){Print(1);}", 10),
    ("if_statement", "fn Main(){var x=0;if x<3{}else{}}", 13),
    ("block", "fn Main(){{}}", 3),
    ("assignment", "fn Main(){var a=[3,4];var v=7;a[1]=v;}", 18),
    ("binding", "fn Main(){var b=7;}", 5),
    (
        "if_expression_implicit",
        "fn Main(){let y=(if true then 1 else 2);}",
        9,
    ),
    // The nested index row is checked below as a delta transcribed from §11.6.
];

fn catalog_campaign() -> [usize; 16] {
    let mut coverage = [0usize; 16];
    for repeat in 0..50 {
        for (index, (name, source, expected)) in CATALOG_ROWS.iter().enumerate() {
            let (outcome, actual, diagnostics) = observe(source).unwrap();
            assert_eq!(outcome, interp::Outcome::Unit, "row={name}");
            assert_eq!(diagnostics, 0, "row={name}");
            assert_eq!(actual, *expected, "row={name}");
            coverage[index] += 1;
        }
        let nested_baseline = "fn Main(){let c=[[7]];}";
        let nested_access = "fn Main(){let c=[[7]];Print(c[0][0]);}";
        let baseline = observe(nested_baseline).unwrap().1;
        let actual = observe(nested_access).unwrap().1;
        assert_eq!(
            actual - baseline,
            13,
            "row=index_expression repeat={repeat}"
        );
        coverage[15] += 1;
    }

    // EmptyValue is retained as an unreachable syntax-path row: the parser rejects
    // a missing `value:` operand before lowering can charge it.
    assert!(matches!(
        observe("fn Main(){let y=(if true then {value:}else{value:});}"),
        Err(ir::CompileError::Rejected(_))
    ));

    let moved_index =
        "fn F(p:move Array<I64>)->I64{return p[0];}fn Main(){var a=[1];let n=F(move a);}";
    let (outcome, work, diagnostics) = observe(moved_index).unwrap();
    assert_eq!(outcome, interp::Outcome::Unit);
    assert_eq!(diagnostics, 0);
    assert!(work > 0);

    // Runtime rows have their own independent expected values: size mismatch
    // stops after the header/length pair, while a Result tag mismatch stops after
    // the wrapper/tag pair.
    for source in [
        "fn Main(){let x=[1]==[1,2];}",
        "fn Main(){let x=[1,2]==[1];}",
    ] {
        assert_eq!(observe(source).unwrap().1, 17, "{source}");
    }
    for source in [
        "fn Main(){AssertEq([1],[1,2]);}",
        "fn Main(){AssertEq([1,2],[1]);}",
    ] {
        let (outcome, actual, diagnostics) = observe(source).unwrap();
        assert_eq!(outcome, interp::Outcome::SimulatorFault, "{source}");
        assert_eq!(diagnostics, 1, "{source}");
        assert_eq!(actual, 17, "{source}");
    }
    let result_tag = "fn Main()->Result<(),I64>{let a:Result<I64,I64>=Ok(1);let b:Result<I64,I64>=Err(1);AssertEq(a,b);return Ok(());}";
    assert_eq!(observe(result_tag).unwrap().1, 23);
    coverage
}

fn campaign(count: usize, seed: u64) -> ([usize; 3], [u64; 3], [usize; 16]) {
    let mut state = seed;
    let mut coverage = [0usize; 3];
    let mut spans_seen = [0u64; 3];
    for index in 0..count {
        let expression = make_expression(&mut state, 0, index % 3);
        let mut printer = Printer::default();
        printer.text.push_str("fn Main(){let v=");
        printer.expression(&expression);
        printer.text.push_str(";}");
        assert!(!printer.spans.is_empty());
        for span in &printer.spans {
            assert!(span.start < span.end && span.end <= printer.text.len());
            let _printed_node = &printer.text[span.start..span.end];
            coverage[span.kind as usize] += 1;
        }
        let expected = 3 + work(&expression);
        let (outcome, actual, diagnostic_count) = observe(&printer.text).unwrap();
        assert_eq!(outcome, interp::Outcome::Unit, "seed={seed} index={index}");
        assert_eq!(diagnostic_count, 0, "seed={seed} index={index}");
        assert_eq!(
            actual, expected,
            "seed={seed} index={index} source={}",
            printer.text
        );
        spans_seen[0] += 1;
    }
    let catalog_coverage = catalog_campaign();
    (coverage, spans_seen, catalog_coverage)
}

#[test]
fn owned_source_tree_oracle_covers_catalog_work_rows() {
    let source = include_str!("work_oracle_tests.rs");
    let forbidden = [
        ["Work", "Rule"].concat(),
        ["Work", "Planner"].concat(),
        ["Work", "Charge"].concat(),
        ["charge", "_event"].concat(),
        ["ir::", "model"].concat(),
    ];
    let adapter = source
        .split("fn observe(")
        .nth(1)
        .unwrap()
        .split("\n}\n")
        .next()
        .unwrap();
    let outside_adapter = source.replace(adapter, "");
    for term in forbidden {
        assert!(!outside_adapter.contains(&term));
    }
    let (coverage, _, catalog_coverage) = campaign(NORMAL_COUNT, DEFAULT_SEED);
    assert!(
        coverage.iter().all(|count| *count >= 50),
        "coverage={coverage:?}"
    );
    assert!(catalog_coverage.iter().all(|count| *count >= 50));
}

#[test]
#[ignore = "campaign of independently generated source trees"]
fn generated_sources_match_owned_tree_cost_oracle() {
    let count = std::env::var("NEXO_I10B_CAMPAIGN_COUNT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(100_000);
    let seed = std::env::var("NEXO_I10B_CAMPAIGN_SEED")
        .ok()
        .and_then(|value| u64::from_str_radix(value.trim_start_matches("0x"), 16).ok())
        .unwrap_or(DEFAULT_SEED);
    let (coverage, _, catalog_coverage) = campaign(count, seed);
    assert!(
        coverage.iter().all(|value| *value >= 50),
        "coverage={coverage:?}"
    );
    assert!(catalog_coverage.iter().all(|value| *value >= 50));
    println!(
        "generator={VERSION} seed={seed} count={count} coverage={coverage:?} catalog_coverage={catalog_coverage:?}"
    );
}
