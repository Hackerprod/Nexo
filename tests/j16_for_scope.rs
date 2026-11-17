use nexo::diagnostic::{DataValue, Operation, Span, Stage};
use nexo::ir::{CompileError, ReducedProfile, compile_scalar_with_profile};

const FILE: &str = "for-scope.n";
const PROFILE: &str = "f0-0003-b-v1";

#[test]
fn generator_for_binding_ends_with_its_body_scope() {
    let profile = ReducedProfile::from_id(PROFILE).unwrap();
    let inside = "gen fn Gn()->I64{yield 1;}fn Main(){for v in Gn(){let q=v;}}";
    compile_scalar_with_profile(FILE, inside, profile, true).unwrap();

    let outside = "gen fn Gn()->I64{yield 1;}fn Main(){for v in Gn(){}let q=v;}";
    let error = match compile_scalar_with_profile(FILE, outside, profile, true) {
        Err(CompileError::Rejected(mut errors)) if errors.len() == 1 => errors.pop().unwrap(),
        other => panic!("expected one name error: {other:?}"),
    };
    error.validate(FILE, outside).unwrap();
    assert_eq!(error.code, "NX-E0101");
    assert_eq!(error.subcause, None);
    assert_eq!(error.stage, Stage::Names);
    assert_eq!(error.operation, Operation::NamesResolve);
    assert_eq!(
        error.primary_span,
        Some(Span {
            file: FILE.into(),
            start: 57,
            end: 58,
        })
    );
    assert_eq!(error.data.len(), 1);
    assert_eq!(
        error.data.get("use_kind"),
        Some(&DataValue::String("value".into()))
    );
}
