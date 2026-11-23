//! F0-0002 lagunas, "Primario de una expresion bajo agrupaciones": every diagnostic whose subject
//! is an expression reports the source span of the inner expression once the outer `Expr::Group`s
//! are removed; the group node keeps its own span in the AST. Expected spans are transcribed from
//! the adopted rule and its addendum (byte offsets over the ASCII sources below); each row pairs
//! the grouped source with the bare control and a valid repair.
use super::i07c_normative_literals::{accepted, invoke};

const J: &str = "f0-0003-b-v1";
const F5: &str = "f0-0005-b-v1";

struct Case {
    family: &'static str,
    name: &'static str,
    profile: &'static str,
    source: &'static str,
    code: &'static str,
    subcause: Option<&'static str>,
    stage: &'static str,
    operation: &'static str,
    feature: Option<&'static str>,
    data: &'static [&'static str],
    primary: (usize, usize),
    token: &'static str,
    related: &'static [(&'static str, usize, usize)],
    cause: &'static [(&'static str, usize, usize)],
    control: &'static str,
    control_primary: (usize, usize),
    repair: &'static str,
}

const CASES: &[Case] = &[
    Case {
        family: "for_iteration",
        name: "for_single",
        profile: J,
        source: "fn Main(){for x in ([1]){}}",
        code: "NX-E0006",
        subcause: None,
        stage: "types",
        operation: "types.check",
        feature: Some("for_iteration"),
        data: &["\"pending_scope\":\"later_adr\""],
        primary: (20, 23),
        token: "[1]",
        related: &[],
        cause: &[],
        control: "fn Main(){for x in [1]{}}",
        control_primary: (19, 22),
        repair: "fn Main(){let x=([1]);}",
    },
    Case {
        family: "for_iteration",
        name: "for_double",
        profile: J,
        source: "fn Main(){for x in (([1])){}}",
        code: "NX-E0006",
        subcause: None,
        stage: "types",
        operation: "types.check",
        feature: Some("for_iteration"),
        data: &["\"pending_scope\":\"later_adr\""],
        primary: (21, 24),
        token: "[1]",
        related: &[],
        cause: &[],
        control: "fn Main(){for x in [1]{}}",
        control_primary: (19, 22),
        repair: "fn Main(){let x=(([1]));}",
    },
    Case {
        family: "call_bind",
        name: "non_callable_name",
        profile: J,
        source: "fn Main(){let x=1;(x)(2);}",
        code: "NX-E0111",
        subcause: Some("non_callable_target"),
        stage: "arguments",
        operation: "call.bind",
        feature: None,
        data: &[],
        primary: (19, 20),
        token: "x",
        related: &[("declaration", 14, 15)],
        cause: &[],
        control: "fn Main(){let x=1;x(2);}",
        control_primary: (18, 19),
        repair: "fn F(x:view I64){}fn Main(){(F)(1);}",
    },
    Case {
        family: "call_bind",
        name: "non_callable_name_double",
        profile: J,
        source: "fn Main(){let x=1;((x))(2);}",
        code: "NX-E0111",
        subcause: Some("non_callable_target"),
        stage: "arguments",
        operation: "call.bind",
        feature: None,
        data: &[],
        primary: (20, 21),
        token: "x",
        related: &[("declaration", 14, 15)],
        cause: &[],
        control: "fn Main(){let x=1;x(2);}",
        control_primary: (18, 19),
        repair: "fn F(x:view I64){}fn Main(){((F))(1);}",
    },
    Case {
        family: "call_bind",
        name: "non_callable_none",
        profile: J,
        source: "fn Main(){(None)(2);}",
        code: "NX-E0111",
        subcause: Some("non_callable_target"),
        stage: "arguments",
        operation: "call.bind",
        feature: None,
        data: &[],
        primary: (11, 15),
        token: "None",
        related: &[],
        cause: &[],
        control: "fn Main(){None(2);}",
        control_primary: (10, 14),
        repair: "fn F(x:view I64){}fn Main(){(F)(1);}",
    },
    Case {
        family: "call_bind",
        name: "missing_argument_function",
        profile: J,
        source: "fn F(x:view I64){}fn Main(){(F)();}",
        code: "NX-E0111",
        subcause: Some("missing_argument"),
        stage: "arguments",
        operation: "call.bind",
        feature: None,
        data: &[],
        primary: (29, 30),
        token: "F",
        related: &[("declaration", 5, 6)],
        cause: &[],
        control: "fn F(x:view I64){}fn Main(){F();}",
        control_primary: (28, 29),
        repair: "fn F(x:view I64){}fn Main(){(F)(1);}",
    },
    Case {
        family: "call_bind",
        name: "positional_after_named",
        profile: J,
        source: "fn F(x:view I64,y:view I64){}fn Main(){F(x:1,(2));}",
        code: "NX-E0111",
        subcause: Some("positional_after_named"),
        stage: "arguments",
        operation: "call.bind",
        feature: None,
        data: &[],
        primary: (46, 47),
        token: "2",
        related: &[("context", 41, 42)],
        cause: &[],
        control: "fn F(x:view I64,y:view I64){}fn Main(){F(x:1,2);}",
        control_primary: (45, 46),
        repair: "fn F(x:view I64,y:view I64){}fn Main(){F(x:1,y:(2));}",
    },
    Case {
        family: "call_bind",
        name: "argument_mode_mismatch",
        profile: J,
        source: "fn F(x:edit I64){}fn Main(){F((1));}",
        code: "NX-E0111",
        subcause: Some("argument_mode_mismatch"),
        stage: "arguments",
        operation: "call.bind",
        feature: None,
        data: &[],
        primary: (31, 32),
        token: "1",
        related: &[("declaration", 5, 6)],
        cause: &[],
        control: "fn F(x:edit I64){}fn Main(){F(1);}",
        control_primary: (30, 31),
        repair: "fn F(x:edit I64){}fn Main(){var x=1;F(edit x);}",
    },
    Case {
        family: "call_bind",
        name: "excess_positional",
        profile: J,
        source: "fn F(x:view I64){}fn Main(){F(1,(2));}",
        code: "NX-E0111",
        subcause: Some("excess_positional_argument"),
        stage: "arguments",
        operation: "call.bind",
        feature: None,
        data: &[],
        primary: (33, 34),
        token: "2",
        related: &[],
        cause: &[],
        control: "fn F(x:view I64){}fn Main(){F(1,2);}",
        control_primary: (32, 33),
        repair: "fn F(x:view I64){}fn Main(){F(1);}",
    },
    Case {
        family: "call_bind",
        name: "excess_positional_double",
        profile: J,
        source: "fn F(x:view I64){}fn Main(){F(1,((2)));}",
        code: "NX-E0111",
        subcause: Some("excess_positional_argument"),
        stage: "arguments",
        operation: "call.bind",
        feature: None,
        data: &[],
        primary: (34, 35),
        token: "2",
        related: &[],
        cause: &[],
        control: "fn F(x:view I64){}fn Main(){F(1,2);}",
        control_primary: (32, 33),
        repair: "fn F(x:view I64){}fn Main(){F(1);}",
    },
    Case {
        family: "call_bind",
        name: "mode_mismatch_prefix_value",
        profile: J,
        source: "fn Main(){var a=[1];Drop(edit (a));}",
        code: "NX-E0111",
        subcause: Some("argument_mode_mismatch"),
        stage: "arguments",
        operation: "call.bind",
        feature: None,
        data: &[],
        primary: (31, 32),
        token: "a",
        related: &[],
        cause: &[],
        control: "fn Main(){var a=[1];Drop(edit a);}",
        control_primary: (30, 31),
        repair: "fn Main(){var a=[1];Drop(a);}",
    },
    Case {
        family: "call_bind",
        name: "mode_mismatch_prefix_projection",
        profile: J,
        source: "fn Main(){var a=[1];Print(edit (a[0]));}",
        code: "NX-E0111",
        subcause: Some("argument_mode_mismatch"),
        stage: "arguments",
        operation: "call.bind",
        feature: None,
        data: &[],
        primary: (32, 36),
        token: "a[0]",
        related: &[],
        cause: &[],
        control: "fn Main(){var a=[1];Print(edit a[0]);}",
        control_primary: (31, 35),
        repair: "fn Main(){var a=[1];Print(a[0]);}",
    },
    Case {
        family: "call_bind",
        name: "closure_missing_argument",
        profile: F5,
        source: "fn Main(){let f=fn (x:view I64){};(f)();}",
        code: "NX-E0111",
        subcause: Some("missing_argument"),
        stage: "arguments",
        operation: "call.bind",
        feature: None,
        data: &[],
        primary: (35, 36),
        token: "f",
        related: &[("declaration", 20, 21)],
        cause: &[],
        control: "fn Main(){let f=fn (x:view I64){};f();}",
        control_primary: (34, 35),
        repair: "fn Main(){let f=fn (x:view I64){};(f)(1);}",
    },
    Case {
        family: "call_bind",
        name: "closure_missing_argument_double",
        profile: F5,
        source: "fn Main(){let f=fn (x:view I64){};((f))();}",
        code: "NX-E0111",
        subcause: Some("missing_argument"),
        stage: "arguments",
        operation: "call.bind",
        feature: None,
        data: &[],
        primary: (36, 37),
        token: "f",
        related: &[("declaration", 20, 21)],
        cause: &[],
        control: "fn Main(){let f=fn (x:view I64){};f();}",
        control_primary: (34, 35),
        repair: "fn Main(){let f=fn (x:view I64){};((f))(1);}",
    },
    Case {
        family: "call_bind",
        name: "partial_transfer_drop",
        profile: J,
        source: "fn Main(){let x=[1];Drop((x[0]));}",
        code: "NX-E0006",
        subcause: None,
        stage: "arguments",
        operation: "call.bind",
        feature: Some("partial_transfer"),
        data: &["\"pending_scope\":\"later_adr\""],
        primary: (26, 30),
        token: "x[0]",
        related: &[("context", 20, 24)],
        cause: &[],
        control: "fn Main(){let x=[1];Drop(x[0]);}",
        control_primary: (25, 29),
        repair: "fn Main(){let x=[1];Drop(x);}",
    },
    Case {
        family: "call_bind",
        name: "partial_transfer_drop_double",
        profile: J,
        source: "fn Main(){let x=[1];Drop((((x[0]))));}",
        code: "NX-E0006",
        subcause: None,
        stage: "arguments",
        operation: "call.bind",
        feature: Some("partial_transfer"),
        data: &["\"pending_scope\":\"later_adr\""],
        primary: (28, 32),
        token: "x[0]",
        related: &[("context", 20, 24)],
        cause: &[],
        control: "fn Main(){let x=[1];Drop(x[0]);}",
        control_primary: (25, 29),
        repair: "fn Main(){let x=[1];Drop(x);}",
    },
    Case {
        family: "parse_barrier",
        name: "partial_transfer_move",
        profile: J,
        source: "fn Main(){let a=[1];move (a[0]);}",
        code: "NX-E0006",
        subcause: None,
        stage: "parse",
        operation: "syntax.parse",
        feature: Some("partial_transfer"),
        data: &["\"pending_scope\":\"later_adr\""],
        primary: (26, 30),
        token: "a[0]",
        related: &[("context", 20, 24)],
        cause: &[],
        control: "fn Main(){let a=[1];move a[0];}",
        control_primary: (25, 29),
        repair: "fn Main(){let a=Some(1);Drop((a));}",
    },
    Case {
        family: "parse_barrier",
        name: "partial_transfer_match_move",
        profile: J,
        source: "fn Main(){let a=[1];match move (a[0]){_=>{}}}",
        code: "NX-E0006",
        subcause: None,
        stage: "parse",
        operation: "syntax.parse",
        feature: Some("partial_transfer"),
        data: &["\"pending_scope\":\"later_adr\""],
        primary: (32, 36),
        token: "a[0]",
        related: &[("context", 26, 30)],
        cause: &[],
        control: "fn Main(){let a=[1];match move a[0]{_=>{}}}",
        control_primary: (31, 35),
        repair: "fn Main(){let a=Some(1);match move (a){_=>{}}}",
    },
    Case {
        family: "ownership",
        name: "immutable_match_edit",
        profile: J,
        source: "fn Main(){let a=Some(1);match edit (a){_=>{}}}",
        code: "NX-E0105",
        subcause: None,
        stage: "ownership",
        operation: "place.write",
        feature: None,
        data: &[],
        primary: (36, 37),
        token: "a",
        related: &[("declaration", 14, 15)],
        cause: &[],
        control: "fn Main(){let a=Some(1);match edit a{_=>{}}}",
        control_primary: (35, 36),
        repair: "fn Main(){var a=Some(1);match edit (a){_=>{}}}",
    },
    Case {
        family: "ownership",
        name: "consumed_drop",
        profile: J,
        source: "fn Main(){let a=Some(1);Drop(a);Drop((a));}",
        code: "NX-E0202",
        subcause: Some("definitely_consumed"),
        stage: "ownership",
        operation: "ownership.use",
        feature: None,
        data: &[],
        primary: (38, 39),
        token: "a",
        related: &[("declaration", 14, 15), ("consumption", 24, 31)],
        cause: &[("value.drop", 24, 31)],
        control: "fn Main(){let a=Some(1);Drop(a);Drop(a);}",
        control_primary: (37, 38),
        repair: "fn Main(){let a=Some(1);Drop((a));}",
    },
    Case {
        family: "ownership",
        name: "consumed_move",
        profile: J,
        source: "fn Main(){let a=Some(1);Drop(a);let b=move (a);}",
        code: "NX-E0202",
        subcause: Some("definitely_consumed"),
        stage: "ownership",
        operation: "ownership.use",
        feature: None,
        data: &[],
        primary: (44, 45),
        token: "a",
        related: &[("declaration", 14, 15), ("consumption", 24, 31)],
        cause: &[("value.drop", 24, 31)],
        control: "fn Main(){let a=Some(1);Drop(a);let b=move a;}",
        control_primary: (43, 44),
        repair: "fn Main(){let a=Some(1);let b=move (a);}",
    },
    Case {
        family: "ownership",
        name: "consumed_match",
        profile: J,
        source: "fn Main(){let a=Some(1);Drop(a);match (a){_=>{}}}",
        code: "NX-E0202",
        subcause: Some("definitely_consumed"),
        stage: "ownership",
        operation: "ownership.use",
        feature: None,
        data: &[],
        primary: (39, 40),
        token: "a",
        related: &[("declaration", 14, 15), ("consumption", 24, 31)],
        cause: &[("value.drop", 24, 31)],
        control: "fn Main(){let a=Some(1);Drop(a);match a{_=>{}}}",
        control_primary: (38, 39),
        repair: "fn Main(){let a=Some(1);match (a){_=>{}}}",
    },
    Case {
        family: "ownership",
        name: "borrow_conflict_match_arm",
        profile: F5,
        source: "fn Main(){var a=Some(1);match edit a{Some(x)=>{Drop((a));},None=>{}}}",
        code: "NX-E1105",
        subcause: Some("active_borrow_conflict"),
        stage: "ownership",
        operation: "ownership.access",
        feature: None,
        data: &[
            "\"active_permission\":\"inout\"",
            "\"id\":\"binding:14\"",
            "\"id\":\"loan:35:0\"",
            "\"id\":\"binding:14\"",
            "\"requested_access\":\"drop\"",
        ],
        primary: (53, 54),
        token: "a",
        related: &[("declaration", 14, 15), ("origin", 35, 36)],
        cause: &[("borrow.inout", 35, 36)],
        control: "fn Main(){var a=Some(1);match edit a{Some(x)=>{Drop(a);},None=>{}}}",
        control_primary: (52, 53),
        repair: "fn Main(){var a=Some(1);match edit a{Some(x)=>{Print(1);},None=>{}}}",
    },
    Case {
        family: "ownership",
        name: "escaping_value_block",
        profile: F5,
        source: "fn Main(){let f=if true then {let x=1;value:(fn [view x]()->I64{return x;});}else{return;};Drop(f);}",
        code: "NX-E1104",
        subcause: None,
        stage: "ownership",
        operation: "ownership.check",
        feature: None,
        data: &[],
        primary: (45, 74),
        token: "fn [view x]()->I64{return x;}",
        related: &[
            ("declaration", 34, 35),
            ("origin", 49, 55),
            ("exit", 76, 77),
        ],
        cause: &[("borrow.read", 49, 55)],
        control: "fn Main(){let f=if true then {let x=1;value:fn [view x]()->I64{return x;};}else{return;};Drop(f);}",
        control_primary: (44, 73),
        repair: "fn Main(){let f=if true then {let x=1;value:(fn [copy x]()->I64{return x;});}else{return;};Drop(f);}",
    },
    Case {
        family: "ownership",
        name: "closure_receiver_mode",
        profile: F5,
        source: "fn Main(){var x=1;var f=fn [edit x](){x=2;};(f)();}",
        code: "NX-E1107",
        subcause: None,
        stage: "ownership",
        operation: "ownership.check",
        feature: None,
        data: &["\"requested_mode\":\"read\"", "\"required_mode\":\"inout\""],
        primary: (45, 46),
        token: "f",
        related: &[("declaration", 22, 23)],
        cause: &[],
        control: "fn Main(){var x=1;var f=fn [edit x](){x=2;};f();}",
        control_primary: (44, 45),
        repair: "fn Main(){var x=1;var f=fn [edit x](){x=2;};edit (f)();}",
    },
    Case {
        family: "ownership",
        name: "borrow_conflict_argument",
        profile: F5,
        source: "fn F(x:edit I64,y:view I64){}fn Main(){var a=1;F(edit a,(a));}",
        code: "NX-E1105",
        subcause: Some("active_borrow_conflict"),
        stage: "ownership",
        operation: "ownership.access",
        feature: None,
        data: &[
            "\"active_permission\":\"inout\"",
            "\"id\":\"binding:43\"",
            "\"id\":\"loan:49:0\"",
            "\"id\":\"binding:43\"",
            "\"requested_access\":\"read\"",
        ],
        primary: (57, 58),
        token: "a",
        related: &[("declaration", 43, 44), ("origin", 49, 55)],
        cause: &[("borrow.inout", 49, 55)],
        control: "fn F(x:edit I64,y:view I64){}fn Main(){var a=1;F(edit a,a);}",
        control_primary: (56, 57),
        repair: "fn F(x:edit I64,y:view I64){}fn Main(){var a=1;F(edit a,1);}",
    },
    Case {
        family: "ownership",
        name: "borrow_conflict_argument_projection",
        profile: F5,
        source: "fn F(x:edit I64,y:view I64){}fn Main(){var a=[1];F(edit a[0],(a[0]));}",
        code: "NX-E1105",
        subcause: Some("active_borrow_conflict"),
        stage: "ownership",
        operation: "ownership.access",
        feature: None,
        data: &[
            "\"active_permission\":\"inout\"",
            "\"id\":\"binding:43\"",
            "\"id\":\"loan:51:0\"",
            "\"id\":\"binding:43\"",
            "\"requested_access\":\"read\"",
        ],
        primary: (62, 66),
        token: "a[0]",
        related: &[("declaration", 43, 44), ("origin", 51, 60)],
        cause: &[("borrow.inout", 51, 60)],
        control: "fn F(x:edit I64,y:view I64){}fn Main(){var a=[1];F(edit a[0],a[0]);}",
        control_primary: (61, 65),
        repair: "fn F(x:edit I64,y:view I64){}fn Main(){var a=[1];F(edit a[0],1);}",
    },
    Case {
        family: "lower_barrier",
        name: "compound_assign_name",
        profile: F5,
        source: "fn Main(){var a=1;(a)+=2;}",
        code: "NX-E0006",
        subcause: None,
        stage: "lower",
        operation: "ir.lower",
        feature: Some("implementation_limit"),
        data: &["\"pending_scope\":\"L08\""],
        primary: (19, 20),
        token: "a",
        related: &[],
        cause: &[],
        control: "fn Main(){var a=1;a+=2;}",
        control_primary: (18, 19),
        repair: "fn Main(){var a=1;(a)=2;}",
    },
    Case {
        family: "lower_barrier",
        name: "compound_assign_projection",
        profile: F5,
        source: "fn Main(){var a=[1];(a[0])+=2;}",
        code: "NX-E0006",
        subcause: None,
        stage: "lower",
        operation: "ir.lower",
        feature: Some("implementation_limit"),
        data: &["\"pending_scope\":\"L08\""],
        primary: (21, 25),
        token: "a[0]",
        related: &[],
        cause: &[],
        control: "fn Main(){var a=[1];a[0]+=2;}",
        control_primary: (20, 24),
        repair: "fn Main(){var a=[1];(a[0])=2;}",
    },
    Case {
        family: "effects",
        name: "intrinsic_call",
        profile: J,
        source: "fn Main() with no_io{(Print)(1);}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "intrinsic.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (22, 27),
        token: "Print",
        related: &[("contract", 15, 20)],
        cause: &[("intrinsic.call", 22, 27)],
        control: "fn Main() with no_io{Print(1);}",
        control_primary: (21, 26),
        repair: "fn Main(){(Print)(1);}",
    },
    Case {
        family: "effects",
        name: "intrinsic_call_double",
        profile: J,
        source: "fn Main() with no_io{((Print))(1);}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "intrinsic.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (23, 28),
        token: "Print",
        related: &[("contract", 15, 20)],
        cause: &[("intrinsic.call", 23, 28)],
        control: "fn Main() with no_io{Print(1);}",
        control_primary: (21, 26),
        repair: "fn Main(){((Print))(1);}",
    },
    Case {
        family: "effects",
        name: "function_call",
        profile: J,
        source: "fn F(){Print(1);}fn Main() with no_io{(F)();}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "function.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (39, 40),
        token: "F",
        related: &[("contract", 32, 37)],
        cause: &[("function.call", 39, 40), ("intrinsic.call", 7, 12)],
        control: "fn F(){Print(1);}fn Main() with no_io{F();}",
        control_primary: (38, 39),
        repair: "fn F(){Print(1);}fn Main(){(F)();}",
    },
    Case {
        family: "effects",
        name: "function_call_double",
        profile: J,
        source: "fn F(){Print(1);}fn Main() with no_io{((F))();}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "function.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (40, 41),
        token: "F",
        related: &[("contract", 32, 37)],
        cause: &[("function.call", 40, 41), ("intrinsic.call", 7, 12)],
        control: "fn F(){Print(1);}fn Main() with no_io{F();}",
        control_primary: (38, 39),
        repair: "fn F(){Print(1);}fn Main(){((F))();}",
    },
    Case {
        family: "effects",
        name: "closure_call",
        profile: J,
        source: "fn G(){Print(1);}fn Main() with no_io{let f=fn (){G();};(f)();}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (57, 58),
        token: "f",
        related: &[("contract", 32, 37)],
        cause: &[
            ("closure.call", 57, 58),
            ("function.call", 50, 51),
            ("intrinsic.call", 7, 12),
        ],
        control: "fn G(){Print(1);}fn Main() with no_io{let f=fn (){G();};f();}",
        control_primary: (56, 57),
        repair: "fn G(){Print(1);}fn Main(){let f=fn (){G();};(f)();}",
    },
    Case {
        family: "effects",
        name: "closure_call_double",
        profile: J,
        source: "fn G(){Print(1);}fn Main() with no_io{let f=fn (){G();};((f))();}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (58, 59),
        token: "f",
        related: &[("contract", 32, 37)],
        cause: &[
            ("closure.call", 58, 59),
            ("function.call", 50, 51),
            ("intrinsic.call", 7, 12),
        ],
        control: "fn G(){Print(1);}fn Main() with no_io{let f=fn (){G();};f();}",
        control_primary: (56, 57),
        repair: "fn G(){Print(1);}fn Main(){let f=fn (){G();};((f))();}",
    },
    Case {
        family: "effects",
        name: "guard_intrinsic_call",
        profile: F5,
        source: "fn Main(){match true{true if (MockUnknown)()==()=>{},_=>{}}}",
        code: "NX-E1305",
        subcause: Some("guard_effect"),
        stage: "effects",
        operation: "effects.check",
        feature: None,
        data: &[
            "\"effect_evidence\":\"unknown_effect\"",
            "\"effect_model\":\"f0-0005-sim-v1\"",
            "\"required_effect\":\"alloc\"",
        ],
        primary: (30, 41),
        token: "MockUnknown",
        related: &[],
        cause: &[("intrinsic.call", 30, 41)],
        control: "fn Main(){match true{true if MockUnknown()==()=>{},_=>{}}}",
        control_primary: (29, 40),
        repair: "fn Main(){match true{true if (true)=>{},_=>{}}}",
    },
    Case {
        family: "effects",
        name: "guard_function_call",
        profile: F5,
        source: "fn G()->Bool{let u=MockUnknown();return true;}fn Main(){match true{true if (G)()=>{},_=>{}}}",
        code: "NX-E1305",
        subcause: Some("guard_effect"),
        stage: "effects",
        operation: "effects.check",
        feature: None,
        data: &[
            "\"effect_evidence\":\"unknown_effect\"",
            "\"effect_model\":\"f0-0005-sim-v1\"",
            "\"required_effect\":\"alloc\"",
        ],
        primary: (76, 77),
        token: "G",
        related: &[],
        cause: &[("function.call", 76, 77), ("intrinsic.call", 19, 30)],
        control: "fn G()->Bool{let u=MockUnknown();return true;}fn Main(){match true{true if G()=>{},_=>{}}}",
        control_primary: (75, 76),
        repair: "fn G()->Bool{return true;}fn Main(){match true{true if (G)()=>{},_=>{}}}",
    },
    Case {
        family: "ownership",
        name: "permission_escalation_drop",
        profile: F5,
        source: "fn F(a:view I64){Drop((a));}fn Main(){F(1);}",
        code: "NX-E1105",
        subcause: Some("permission_escalation"),
        stage: "ownership",
        operation: "ownership.access",
        feature: None,
        data: &[
            "\"active_permission\":\"read\"",
            "\"id\":\"parameter:5\"",
            "\"id\":\"parameter:5\"",
            "\"requested_access\":\"drop\"",
        ],
        primary: (23, 24),
        token: "a",
        related: &[("declaration", 5, 6)],
        cause: &[("borrow.read", 5, 6)],
        control: "fn F(a:view I64){Drop(a);}fn Main(){F(1);}",
        control_primary: (22, 23),
        repair: "fn F(a:move I64){Drop((a));}fn Main(){F(1);}",
    },
    Case {
        family: "ownership",
        name: "permission_escalation_match_edit",
        profile: F5,
        source: "fn F(a:view I64){match edit (a){_=>{}}}fn Main(){F(1);}",
        code: "NX-E1105",
        subcause: Some("permission_escalation"),
        stage: "ownership",
        operation: "ownership.access",
        feature: None,
        data: &[
            "\"active_permission\":\"read\"",
            "\"id\":\"parameter:5\"",
            "\"id\":\"parameter:5\"",
            "\"requested_access\":\"borrow_inout\"",
        ],
        primary: (29, 30),
        token: "a",
        related: &[("declaration", 5, 6)],
        cause: &[("borrow.read", 5, 6)],
        control: "fn F(a:view I64){match edit a{_=>{}}}fn Main(){F(1);}",
        control_primary: (28, 29),
        repair: "fn F(a:edit I64){match edit (a){_=>{}}}fn Main(){var b=1;F(edit b);}",
    },
    Case {
        family: "ownership",
        name: "consumed_closure_receiver",
        profile: F5,
        source: "fn Main(){let f=fn (){};Drop(f);(f)();}",
        code: "NX-E0202",
        subcause: Some("definitely_consumed"),
        stage: "ownership",
        operation: "ownership.use",
        feature: None,
        data: &[],
        primary: (33, 34),
        token: "f",
        related: &[("declaration", 14, 15), ("consumption", 24, 31)],
        cause: &[("value.drop", 24, 31)],
        control: "fn Main(){let f=fn (){};Drop(f);f();}",
        control_primary: (32, 33),
        repair: "fn Main(){let f=fn (){};(f)();}",
    },
    Case {
        family: "ownership",
        name: "consumed_closure_receiver_double",
        profile: F5,
        source: "fn Main(){let f=fn (){};Drop(f);((f))();}",
        code: "NX-E0202",
        subcause: Some("definitely_consumed"),
        stage: "ownership",
        operation: "ownership.use",
        feature: None,
        data: &[],
        primary: (34, 35),
        token: "f",
        related: &[("declaration", 14, 15), ("consumption", 24, 31)],
        cause: &[("value.drop", 24, 31)],
        control: "fn Main(){let f=fn (){};Drop(f);f();}",
        control_primary: (32, 33),
        repair: "fn Main(){let f=fn (){};((f))();}",
    },
    Case {
        family: "call_bind",
        name: "closure_argument_mode_mismatch",
        profile: F5,
        source: "fn Main(){let f=fn (x:view I64){};let a=1;(f)(edit (a));}",
        code: "NX-E0111",
        subcause: Some("argument_mode_mismatch"),
        stage: "arguments",
        operation: "call.bind",
        feature: None,
        data: &[],
        primary: (52, 53),
        token: "a",
        related: &[("declaration", 20, 21)],
        cause: &[],
        control: "fn Main(){let f=fn (x:view I64){};let a=1;f(edit a);}",
        control_primary: (49, 50),
        repair: "fn Main(){let f=fn (x:view I64){};let a=1;(f)(a);}",
    },
    Case {
        family: "ownership",
        name: "consumed_match_projection",
        profile: J,
        source: "fn Main(){let a=[1];Drop(a);match (a[0]){_=>{}}}",
        code: "NX-E0202",
        subcause: Some("definitely_consumed"),
        stage: "ownership",
        operation: "ownership.use",
        feature: None,
        data: &[],
        primary: (35, 39),
        token: "a[0]",
        related: &[("declaration", 14, 15), ("consumption", 20, 27)],
        cause: &[("value.drop", 20, 27)],
        control: "fn Main(){let a=[1];Drop(a);match a[0]{_=>{}}}",
        control_primary: (34, 38),
        repair: "fn Main(){let a=[1];match (a[0]){_=>{}}}",
    },
    Case {
        family: "ownership",
        name: "immutable_match_edit_projection",
        profile: F5,
        source: "fn Main(){let a=[1];match edit (a[0]){_=>{}}}",
        code: "NX-E0105",
        subcause: None,
        stage: "ownership",
        operation: "place.write",
        feature: None,
        data: &[],
        primary: (32, 36),
        token: "a[0]",
        related: &[("declaration", 14, 15)],
        cause: &[],
        control: "fn Main(){let a=[1];match edit a[0]{_=>{}}}",
        control_primary: (31, 35),
        repair: "fn Main(){var a=[1];match edit (a[0]){_=>{}}}",
    },
    Case {
        family: "ownership",
        name: "borrow_conflict_match_projection",
        profile: F5,
        source: "fn Main(){var a=[1];match edit a[0]{_=>{match (a[0]){_=>{}}}}}",
        code: "NX-E1105",
        subcause: Some("active_borrow_conflict"),
        stage: "ownership",
        operation: "ownership.access",
        feature: None,
        data: &[
            "\"active_permission\":\"inout\"",
            "\"id\":\"binding:14\"",
            "\"id\":\"loan:31:0\"",
            "\"id\":\"binding:14\"",
            "\"requested_access\":\"read\"",
        ],
        primary: (47, 51),
        token: "a[0]",
        related: &[("declaration", 14, 15), ("origin", 31, 35)],
        cause: &[("borrow.inout", 31, 35)],
        control: "fn Main(){var a=[1];match edit a[0]{_=>{match a[0]{_=>{}}}}}",
        control_primary: (46, 50),
        repair: "fn Main(){var a=[1];match edit a[0]{_=>{Print(1);}}}",
    },
    Case {
        family: "ownership",
        name: "borrow_conflict_match_read",
        profile: F5,
        source: "fn Main(){var a=Some(1);match edit a{Some(x)=>{match (a){_=>{}}},None=>{}}}",
        code: "NX-E1105",
        subcause: Some("active_borrow_conflict"),
        stage: "ownership",
        operation: "ownership.access",
        feature: None,
        data: &[
            "\"active_permission\":\"inout\"",
            "\"id\":\"binding:14\"",
            "\"id\":\"loan:35:0\"",
            "\"id\":\"binding:14\"",
            "\"requested_access\":\"read\"",
        ],
        primary: (54, 55),
        token: "a",
        related: &[("declaration", 14, 15), ("origin", 35, 36)],
        cause: &[("borrow.inout", 35, 36)],
        control: "fn Main(){var a=Some(1);match edit a{Some(x)=>{match a{_=>{}}},None=>{}}}",
        control_primary: (53, 54),
        repair: "fn Main(){var a=Some(1);match edit a{Some(x)=>{Print(1);},None=>{}}}",
    },
    Case {
        family: "ownership",
        name: "borrow_conflict_match_move",
        profile: F5,
        source: "fn Main(){var a=Some(1);match edit a{Some(x)=>{match move (a){_=>{}}},None=>{}}}",
        code: "NX-E1105",
        subcause: Some("active_borrow_conflict"),
        stage: "ownership",
        operation: "ownership.access",
        feature: None,
        data: &[
            "\"active_permission\":\"inout\"",
            "\"id\":\"binding:14\"",
            "\"id\":\"loan:35:0\"",
            "\"id\":\"binding:14\"",
            "\"requested_access\":\"consume\"",
        ],
        primary: (59, 60),
        token: "a",
        related: &[("declaration", 14, 15), ("origin", 35, 36)],
        cause: &[("borrow.inout", 35, 36)],
        control: "fn Main(){var a=Some(1);match edit a{Some(x)=>{match move a{_=>{}}},None=>{}}}",
        control_primary: (58, 59),
        repair: "fn Main(){var a=Some(1);match edit a{Some(x)=>{Print(1);},None=>{}}}",
    },
    Case {
        family: "effects",
        name: "dropped_temporary_of_grouped_call",
        profile: J,
        source: "fn Main() with no_io{(MockOpen)(1);}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "intrinsic.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (22, 30),
        token: "MockOpen",
        related: &[("contract", 15, 20)],
        cause: &[("intrinsic.call", 22, 30)],
        control: "fn Main() with no_io{MockOpen(1);}",
        control_primary: (21, 29),
        repair: "fn Main(){(MockOpen)(1);}",
    },
    Case {
        family: "effects",
        name: "dropped_temporary_of_grouped_call_double",
        profile: J,
        source: "fn Main() with no_io{((MockOpen))(1);}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "intrinsic.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (23, 31),
        token: "MockOpen",
        related: &[("contract", 15, 20)],
        cause: &[("intrinsic.call", 23, 31)],
        control: "fn Main() with no_io{MockOpen(1);}",
        control_primary: (21, 29),
        repair: "fn Main(){((MockOpen))(1);}",
    },
    Case {
        family: "effects",
        name: "temporary_argument_of_grouped_call",
        profile: J,
        source: "fn F(r:view MockResource){}fn Main() with no_io{F((MockOpen)(1));}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "intrinsic.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (51, 59),
        token: "MockOpen",
        related: &[("contract", 42, 47)],
        cause: &[("intrinsic.call", 51, 59)],
        control: "fn F(r:view MockResource){}fn Main() with no_io{F(MockOpen(1));}",
        control_primary: (50, 58),
        repair: "fn F(r:view MockResource){}fn Main(){F((MockOpen)(1));}",
    },
    Case {
        family: "effects",
        name: "temporary_argument_of_grouped_call_double",
        profile: J,
        source: "fn F(r:view MockResource){}fn Main() with no_io{F(((MockOpen))(1));}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "intrinsic.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (52, 60),
        token: "MockOpen",
        related: &[("contract", 42, 47)],
        cause: &[("intrinsic.call", 52, 60)],
        control: "fn F(r:view MockResource){}fn Main() with no_io{F(MockOpen(1));}",
        control_primary: (50, 58),
        repair: "fn F(r:view MockResource){}fn Main(){F(((MockOpen))(1));}",
    },
    Case {
        family: "effects",
        name: "closure_temporary_of_grouped_call",
        profile: J,
        source: "fn Main() with no_io{let f=fn (){(MockOpen)(1);};f();}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (49, 50),
        token: "f",
        related: &[("contract", 15, 20)],
        cause: &[("closure.call", 49, 50), ("intrinsic.call", 34, 42)],
        control: "fn Main() with no_io{let f=fn (){MockOpen(1);};f();}",
        control_primary: (47, 48),
        repair: "fn Main(){let f=fn (){(MockOpen)(1);};f();}",
    },
    Case {
        family: "effects",
        name: "closure_temporary_of_grouped_call_double",
        profile: J,
        source: "fn Main() with no_io{let f=fn (){((MockOpen))(1);};((f))();}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (53, 54),
        token: "f",
        related: &[("contract", 15, 20)],
        cause: &[("closure.call", 53, 54), ("intrinsic.call", 35, 43)],
        control: "fn Main() with no_io{let f=fn (){MockOpen(1);};f();}",
        control_primary: (47, 48),
        repair: "fn Main(){let f=fn (){((MockOpen))(1);};((f))();}",
    },
    Case {
        family: "ownership",
        name: "borrow_conflict_move_group",
        profile: J,
        source: "fn Main(){var a=Some(1);match edit a{Some(x)=>{let b=move (a);},None=>{}}}",
        code: "NX-E1105",
        subcause: Some("active_borrow_conflict"),
        stage: "ownership",
        operation: "ownership.access",
        feature: None,
        data: &[
            "\"active_permission\":\"inout\"",
            "\"id\":\"binding:14\"",
            "\"id\":\"loan:35:0\"",
            "\"id\":\"binding:14\"",
            "\"requested_access\":\"consume\"",
        ],
        primary: (59, 60),
        token: "a",
        related: &[("declaration", 14, 15), ("origin", 35, 36)],
        cause: &[("borrow.inout", 35, 36)],
        control: "fn Main(){var a=Some(1);match edit a{Some(x)=>{let b=move a;},None=>{}}}",
        control_primary: (58, 59),
        repair: "fn Main(){var a=Some(1);let b=move (a);}",
    },
    Case {
        family: "ownership",
        name: "borrow_conflict_move_group_double",
        profile: J,
        source: "fn Main(){var a=Some(1);match edit a{Some(x)=>{let b=move ((a));},None=>{}}}",
        code: "NX-E1105",
        subcause: Some("active_borrow_conflict"),
        stage: "ownership",
        operation: "ownership.access",
        feature: None,
        data: &[
            "\"active_permission\":\"inout\"",
            "\"id\":\"binding:14\"",
            "\"id\":\"loan:35:0\"",
            "\"id\":\"binding:14\"",
            "\"requested_access\":\"consume\"",
        ],
        primary: (60, 61),
        token: "a",
        related: &[("declaration", 14, 15), ("origin", 35, 36)],
        cause: &[("borrow.inout", 35, 36)],
        control: "fn Main(){var a=Some(1);match edit a{Some(x)=>{let b=move a;},None=>{}}}",
        control_primary: (58, 59),
        repair: "fn Main(){var a=Some(1);let b=move ((a));}",
    },
    Case {
        family: "call_bind",
        name: "intrinsic_edit_argument_push",
        profile: F5,
        source: "fn Main(){var a=[1];Push((a),2);}",
        code: "NX-E0111",
        subcause: Some("argument_mode_mismatch"),
        stage: "arguments",
        operation: "call.bind",
        feature: None,
        data: &[],
        primary: (26, 27),
        token: "a",
        related: &[],
        cause: &[],
        control: "fn Main(){var a=[1];Push(a,2);}",
        control_primary: (25, 26),
        repair: "",
    },
    Case {
        family: "call_bind",
        name: "intrinsic_edit_argument_pop",
        profile: F5,
        source: "fn Main(){var a=[1];let z=Pop(((a)));}",
        code: "NX-E0111",
        subcause: Some("argument_mode_mismatch"),
        stage: "arguments",
        operation: "call.bind",
        feature: None,
        data: &[],
        primary: (32, 33),
        token: "a",
        related: &[],
        cause: &[],
        control: "fn Main(){var a=[1];let z=Pop(a);}",
        control_primary: (30, 31),
        repair: "",
    },
    Case {
        family: "call_bind",
        name: "intrinsic_edit_argument_next",
        profile: F5,
        source: "gen fn G()->I64{yield 1;}fn Main(){var g=G();Iterator.Next((g));}",
        code: "NX-E0111",
        subcause: Some("argument_mode_mismatch"),
        stage: "arguments",
        operation: "call.bind",
        feature: None,
        data: &[],
        primary: (60, 61),
        token: "g",
        related: &[],
        cause: &[],
        control: "gen fn G()->I64{yield 1;}fn Main(){var g=G();Iterator.Next(g);}",
        control_primary: (59, 60),
        repair: "gen fn G()->I64{yield 1;}fn Main(){var g=G();Iterator.Next(edit g);}",
    },
    Case {
        family: "effects",
        name: "head_no_io_name_1_stmt",
        profile: J,
        source: "fn Main() with no_io{let f=fn (){Print(1);};(f)();}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (45, 46),
        token: "f",
        related: &[("contract", 15, 20)],
        cause: &[("closure.call", 45, 46), ("intrinsic.call", 33, 38)],
        control: "fn Main() with no_io{let f=fn (){Print(1);};f();}",
        control_primary: (44, 45),
        repair: "fn Main(){let f=fn (){Print(1);};(f)();}",
    },
    Case {
        family: "effects",
        name: "head_no_io_name_1_guard",
        profile: J,
        source: "fn Main() with no_io{let f=fn ()->Bool{Print(1); return true;};match true{true if (f)()=>{},_=>{}}}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (83, 84),
        token: "f",
        related: &[("contract", 15, 20)],
        cause: &[("closure.call", 83, 84), ("intrinsic.call", 39, 44)],
        control: "fn Main() with no_io{let f=fn ()->Bool{Print(1); return true;};match true{true if f()=>{},_=>{}}}",
        control_primary: (82, 83),
        repair: "fn Main(){let f=fn ()->Bool{Print(1); return true;};match true{true if (f)()=>{},_=>{}}}",
    },
    Case {
        family: "effects",
        name: "head_no_io_name_2_stmt",
        profile: J,
        source: "fn Main() with no_io{let f=fn (){Print(1);};((f))();}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (46, 47),
        token: "f",
        related: &[("contract", 15, 20)],
        cause: &[("closure.call", 46, 47), ("intrinsic.call", 33, 38)],
        control: "fn Main() with no_io{let f=fn (){Print(1);};f();}",
        control_primary: (44, 45),
        repair: "fn Main(){let f=fn (){Print(1);};((f))();}",
    },
    Case {
        family: "effects",
        name: "head_no_io_name_2_guard",
        profile: J,
        source: "fn Main() with no_io{let f=fn ()->Bool{Print(1); return true;};match true{true if ((f))()=>{},_=>{}}}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (84, 85),
        token: "f",
        related: &[("contract", 15, 20)],
        cause: &[("closure.call", 84, 85), ("intrinsic.call", 39, 44)],
        control: "fn Main() with no_io{let f=fn ()->Bool{Print(1); return true;};match true{true if f()=>{},_=>{}}}",
        control_primary: (82, 83),
        repair: "fn Main(){let f=fn ()->Bool{Print(1); return true;};match true{true if ((f))()=>{},_=>{}}}",
    },
    Case {
        family: "effects",
        name: "head_no_io_literal_1_stmt",
        profile: J,
        source: "fn Main() with no_io{(fn (){Print(1);})();}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (22, 24),
        token: "fn",
        related: &[("contract", 15, 20)],
        cause: &[("closure.call", 22, 24), ("intrinsic.call", 28, 33)],
        control: "fn Main() with no_io{fn (){Print(1);}();}",
        control_primary: (21, 23),
        repair: "fn Main(){(fn (){Print(1);})();}",
    },
    Case {
        family: "effects",
        name: "head_no_io_literal_1_guard",
        profile: J,
        source: "fn Main() with no_io{match true{true if (fn ()->Bool{Print(1); return true;})()=>{},_=>{}}}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (41, 43),
        token: "fn",
        related: &[("contract", 15, 20)],
        cause: &[("closure.call", 41, 43), ("intrinsic.call", 53, 58)],
        control: "fn Main() with no_io{match true{true if fn ()->Bool{Print(1); return true;}()=>{},_=>{}}}",
        control_primary: (40, 42),
        repair: "fn Main(){match true{true if (fn ()->Bool{Print(1); return true;})()=>{},_=>{}}}",
    },
    Case {
        family: "effects",
        name: "head_no_io_literal_2_stmt",
        profile: J,
        source: "fn Main() with no_io{((fn (){Print(1);}))();}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (23, 25),
        token: "fn",
        related: &[("contract", 15, 20)],
        cause: &[("closure.call", 23, 25), ("intrinsic.call", 29, 34)],
        control: "fn Main() with no_io{fn (){Print(1);}();}",
        control_primary: (21, 23),
        repair: "fn Main(){((fn (){Print(1);}))();}",
    },
    Case {
        family: "effects",
        name: "head_no_io_literal_2_guard",
        profile: J,
        source: "fn Main() with no_io{match true{true if ((fn ()->Bool{Print(1); return true;}))()=>{},_=>{}}}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (42, 44),
        token: "fn",
        related: &[("contract", 15, 20)],
        cause: &[("closure.call", 42, 44), ("intrinsic.call", 54, 59)],
        control: "fn Main() with no_io{match true{true if fn ()->Bool{Print(1); return true;}()=>{},_=>{}}}",
        control_primary: (40, 42),
        repair: "fn Main(){match true{true if ((fn ()->Bool{Print(1); return true;}))()=>{},_=>{}}}",
    },
    Case {
        family: "effects",
        name: "head_no_io_if_1_stmt",
        profile: J,
        source: "fn Main() with no_io{let f=fn (){Print(1);};(if true then f else f)();}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (45, 47),
        token: "if",
        related: &[("contract", 15, 20)],
        cause: &[("closure.call", 45, 47), ("intrinsic.call", 33, 38)],
        control: "",
        control_primary: (0, 0),
        repair: "fn Main(){let f=fn (){Print(1);};(if true then f else f)();}",
    },
    Case {
        family: "effects",
        name: "head_no_io_if_1_guard",
        profile: J,
        source: "fn Main() with no_io{let f=fn ()->Bool{Print(1); return true;};match true{true if (if true then f else f)()=>{},_=>{}}}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (83, 85),
        token: "if",
        related: &[("contract", 15, 20)],
        cause: &[("closure.call", 83, 85), ("intrinsic.call", 39, 44)],
        control: "",
        control_primary: (0, 0),
        repair: "fn Main(){let f=fn ()->Bool{Print(1); return true;};match true{true if (if true then f else f)()=>{},_=>{}}}",
    },
    Case {
        family: "effects",
        name: "head_no_io_if_2_stmt",
        profile: J,
        source: "fn Main() with no_io{let f=fn (){Print(1);};((if true then f else f))();}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (46, 48),
        token: "if",
        related: &[("contract", 15, 20)],
        cause: &[("closure.call", 46, 48), ("intrinsic.call", 33, 38)],
        control: "",
        control_primary: (0, 0),
        repair: "fn Main(){let f=fn (){Print(1);};((if true then f else f))();}",
    },
    Case {
        family: "effects",
        name: "head_no_io_if_2_guard",
        profile: J,
        source: "fn Main() with no_io{let f=fn ()->Bool{Print(1); return true;};match true{true if ((if true then f else f))()=>{},_=>{}}}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (84, 86),
        token: "if",
        related: &[("contract", 15, 20)],
        cause: &[("closure.call", 84, 86), ("intrinsic.call", 39, 44)],
        control: "",
        control_primary: (0, 0),
        repair: "fn Main(){let f=fn ()->Bool{Print(1); return true;};match true{true if ((if true then f else f))()=>{},_=>{}}}",
    },
    Case {
        family: "effects",
        name: "head_no_io_move_1_stmt",
        profile: J,
        source: "fn Main() with no_io{let f=fn (){Print(1);};(move f)();}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (45, 49),
        token: "move",
        related: &[("contract", 15, 20)],
        cause: &[("closure.call", 45, 49), ("intrinsic.call", 33, 38)],
        control: "",
        control_primary: (0, 0),
        repair: "fn Main(){let f=fn (){Print(1);};(move f)();}",
    },
    Case {
        family: "effects",
        name: "head_no_io_move_2_stmt",
        profile: J,
        source: "fn Main() with no_io{let f=fn (){Print(1);};((move f))();}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (46, 50),
        token: "move",
        related: &[("contract", 15, 20)],
        cause: &[("closure.call", 46, 50), ("intrinsic.call", 33, 38)],
        control: "",
        control_primary: (0, 0),
        repair: "fn Main(){let f=fn (){Print(1);};((move f))();}",
    },
    Case {
        family: "effects",
        name: "head_no_alloc_name_1_stmt",
        profile: J,
        source: "fn Main() with no_alloc{let f=fn (){let a=[1];};(f)();}",
        code: "NX-E0412",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"alloc\"",
        ],
        primary: (49, 50),
        token: "f",
        related: &[("contract", 15, 23)],
        cause: &[("closure.call", 49, 50), ("value.build_array", 42, 45)],
        control: "fn Main() with no_alloc{let f=fn (){let a=[1];};f();}",
        control_primary: (48, 49),
        repair: "fn Main(){let f=fn (){let a=[1];};(f)();}",
    },
    Case {
        family: "effects",
        name: "head_no_alloc_name_2_stmt",
        profile: J,
        source: "fn Main() with no_alloc{let f=fn (){let a=[1];};((f))();}",
        code: "NX-E0412",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"alloc\"",
        ],
        primary: (50, 51),
        token: "f",
        related: &[("contract", 15, 23)],
        cause: &[("closure.call", 50, 51), ("value.build_array", 42, 45)],
        control: "fn Main() with no_alloc{let f=fn (){let a=[1];};f();}",
        control_primary: (48, 49),
        repair: "fn Main(){let f=fn (){let a=[1];};((f))();}",
    },
    Case {
        family: "effects",
        name: "head_no_alloc_literal_1_stmt",
        profile: J,
        source: "fn Main() with no_alloc{(fn (){let a=[1];})();}",
        code: "NX-E0412",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"alloc\"",
        ],
        primary: (25, 27),
        token: "fn",
        related: &[("contract", 15, 23)],
        cause: &[("closure.call", 25, 27), ("value.build_array", 37, 40)],
        control: "fn Main() with no_alloc{fn (){let a=[1];}();}",
        control_primary: (24, 26),
        repair: "fn Main(){(fn (){let a=[1];})();}",
    },
    Case {
        family: "effects",
        name: "head_no_alloc_literal_2_stmt",
        profile: J,
        source: "fn Main() with no_alloc{((fn (){let a=[1];}))();}",
        code: "NX-E0412",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"alloc\"",
        ],
        primary: (26, 28),
        token: "fn",
        related: &[("contract", 15, 23)],
        cause: &[("closure.call", 26, 28), ("value.build_array", 38, 41)],
        control: "fn Main() with no_alloc{fn (){let a=[1];}();}",
        control_primary: (24, 26),
        repair: "fn Main(){((fn (){let a=[1];}))();}",
    },
    Case {
        family: "effects",
        name: "head_no_alloc_if_1_stmt",
        profile: J,
        source: "fn Main() with no_alloc{let f=fn (){let a=[1];};(if true then f else f)();}",
        code: "NX-E0412",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"alloc\"",
        ],
        primary: (49, 51),
        token: "if",
        related: &[("contract", 15, 23)],
        cause: &[("closure.call", 49, 51), ("value.build_array", 42, 45)],
        control: "",
        control_primary: (0, 0),
        repair: "fn Main(){let f=fn (){let a=[1];};(if true then f else f)();}",
    },
    Case {
        family: "effects",
        name: "head_no_alloc_if_2_stmt",
        profile: J,
        source: "fn Main() with no_alloc{let f=fn (){let a=[1];};((if true then f else f))();}",
        code: "NX-E0412",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"alloc\"",
        ],
        primary: (50, 52),
        token: "if",
        related: &[("contract", 15, 23)],
        cause: &[("closure.call", 50, 52), ("value.build_array", 42, 45)],
        control: "",
        control_primary: (0, 0),
        repair: "fn Main(){let f=fn (){let a=[1];};((if true then f else f))();}",
    },
    Case {
        family: "effects",
        name: "head_no_alloc_move_1_stmt",
        profile: J,
        source: "fn Main() with no_alloc{let f=fn (){let a=[1];};(move f)();}",
        code: "NX-E0412",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"alloc\"",
        ],
        primary: (49, 53),
        token: "move",
        related: &[("contract", 15, 23)],
        cause: &[("closure.call", 49, 53), ("value.build_array", 42, 45)],
        control: "",
        control_primary: (0, 0),
        repair: "fn Main(){let f=fn (){let a=[1];};(move f)();}",
    },
    Case {
        family: "effects",
        name: "head_no_alloc_move_2_stmt",
        profile: J,
        source: "fn Main() with no_alloc{let f=fn (){let a=[1];};((move f))();}",
        code: "NX-E0412",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"alloc\"",
        ],
        primary: (50, 54),
        token: "move",
        related: &[("contract", 15, 23)],
        cause: &[("closure.call", 50, 54), ("value.build_array", 42, 45)],
        control: "",
        control_primary: (0, 0),
        repair: "fn Main(){let f=fn (){let a=[1];};((move f))();}",
    },
    Case {
        family: "effects",
        name: "head_no_block_name_1_stmt",
        profile: J,
        source: "fn Main() with no_block{let f=fn (){let r=MockBlockingOpen(1);};(f)();}",
        code: "NX-E0413",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"block\"",
        ],
        primary: (65, 66),
        token: "f",
        related: &[("contract", 15, 23), ("origin", 40, 41), ("exit", 62, 63)],
        cause: &[
            ("closure.call", 65, 66),
            ("value.drop", 40, 41),
            ("resource.drop", 0, 0),
        ],
        control: "fn Main() with no_block{let f=fn (){let r=MockBlockingOpen(1);};f();}",
        control_primary: (64, 65),
        repair: "fn Main(){let f=fn (){let r=MockBlockingOpen(1);};(f)();}",
    },
    Case {
        family: "effects",
        name: "head_no_block_name_2_stmt",
        profile: J,
        source: "fn Main() with no_block{let f=fn (){let r=MockBlockingOpen(1);};((f))();}",
        code: "NX-E0413",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"block\"",
        ],
        primary: (66, 67),
        token: "f",
        related: &[("contract", 15, 23), ("origin", 40, 41), ("exit", 62, 63)],
        cause: &[
            ("closure.call", 66, 67),
            ("value.drop", 40, 41),
            ("resource.drop", 0, 0),
        ],
        control: "fn Main() with no_block{let f=fn (){let r=MockBlockingOpen(1);};f();}",
        control_primary: (64, 65),
        repair: "fn Main(){let f=fn (){let r=MockBlockingOpen(1);};((f))();}",
    },
    Case {
        family: "effects",
        name: "head_no_block_literal_1_stmt",
        profile: J,
        source: "fn Main() with no_block{(fn (){let r=MockBlockingOpen(1);})();}",
        code: "NX-E0413",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"block\"",
        ],
        primary: (25, 27),
        token: "fn",
        related: &[("contract", 15, 23), ("origin", 35, 36), ("exit", 57, 58)],
        cause: &[
            ("closure.call", 25, 27),
            ("value.drop", 35, 36),
            ("resource.drop", 0, 0),
        ],
        control: "fn Main() with no_block{fn (){let r=MockBlockingOpen(1);}();}",
        control_primary: (24, 26),
        repair: "fn Main(){(fn (){let r=MockBlockingOpen(1);})();}",
    },
    Case {
        family: "effects",
        name: "head_no_block_literal_2_stmt",
        profile: J,
        source: "fn Main() with no_block{((fn (){let r=MockBlockingOpen(1);}))();}",
        code: "NX-E0413",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"block\"",
        ],
        primary: (26, 28),
        token: "fn",
        related: &[("contract", 15, 23), ("origin", 36, 37), ("exit", 58, 59)],
        cause: &[
            ("closure.call", 26, 28),
            ("value.drop", 36, 37),
            ("resource.drop", 0, 0),
        ],
        control: "fn Main() with no_block{fn (){let r=MockBlockingOpen(1);}();}",
        control_primary: (24, 26),
        repair: "fn Main(){((fn (){let r=MockBlockingOpen(1);}))();}",
    },
    Case {
        family: "effects",
        name: "head_no_block_if_1_stmt",
        profile: J,
        source: "fn Main() with no_block{let f=fn (){let r=MockBlockingOpen(1);};(if true then f else f)();}",
        code: "NX-E0413",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"block\"",
        ],
        primary: (65, 67),
        token: "if",
        related: &[("contract", 15, 23), ("origin", 40, 41), ("exit", 62, 63)],
        cause: &[
            ("closure.call", 65, 67),
            ("value.drop", 40, 41),
            ("resource.drop", 0, 0),
        ],
        control: "",
        control_primary: (0, 0),
        repair: "fn Main(){let f=fn (){let r=MockBlockingOpen(1);};(if true then f else f)();}",
    },
    Case {
        family: "effects",
        name: "head_no_block_if_2_stmt",
        profile: J,
        source: "fn Main() with no_block{let f=fn (){let r=MockBlockingOpen(1);};((if true then f else f))();}",
        code: "NX-E0413",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"block\"",
        ],
        primary: (66, 68),
        token: "if",
        related: &[("contract", 15, 23), ("origin", 40, 41), ("exit", 62, 63)],
        cause: &[
            ("closure.call", 66, 68),
            ("value.drop", 40, 41),
            ("resource.drop", 0, 0),
        ],
        control: "",
        control_primary: (0, 0),
        repair: "fn Main(){let f=fn (){let r=MockBlockingOpen(1);};((if true then f else f))();}",
    },
    Case {
        family: "effects",
        name: "head_no_block_move_1_stmt",
        profile: J,
        source: "fn Main() with no_block{let f=fn (){let r=MockBlockingOpen(1);};(move f)();}",
        code: "NX-E0413",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"block\"",
        ],
        primary: (65, 69),
        token: "move",
        related: &[("contract", 15, 23), ("origin", 40, 41), ("exit", 62, 63)],
        cause: &[
            ("closure.call", 65, 69),
            ("value.drop", 40, 41),
            ("resource.drop", 0, 0),
        ],
        control: "",
        control_primary: (0, 0),
        repair: "fn Main(){let f=fn (){let r=MockBlockingOpen(1);};(move f)();}",
    },
    Case {
        family: "effects",
        name: "head_no_block_move_2_stmt",
        profile: J,
        source: "fn Main() with no_block{let f=fn (){let r=MockBlockingOpen(1);};((move f))();}",
        code: "NX-E0413",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"block\"",
        ],
        primary: (66, 70),
        token: "move",
        related: &[("contract", 15, 23), ("origin", 40, 41), ("exit", 62, 63)],
        cause: &[
            ("closure.call", 66, 70),
            ("value.drop", 40, 41),
            ("resource.drop", 0, 0),
        ],
        control: "",
        control_primary: (0, 0),
        repair: "fn Main(){let f=fn (){let r=MockBlockingOpen(1);};((move f))();}",
    },
    Case {
        family: "effects",
        name: "head_no_suspend_name_1_stmt",
        profile: J,
        source: "fn Main() with no_suspend{let f=fn (){MockUnknown();};(f)();}",
        code: "NX-E0414",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"unknown_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"suspend\"",
        ],
        primary: (55, 56),
        token: "f",
        related: &[("contract", 15, 25)],
        cause: &[("closure.call", 55, 56), ("intrinsic.call", 38, 49)],
        control: "fn Main() with no_suspend{let f=fn (){MockUnknown();};f();}",
        control_primary: (54, 55),
        repair: "fn Main(){let f=fn (){MockUnknown();};(f)();}",
    },
    Case {
        family: "effects",
        name: "head_no_suspend_name_1_guard",
        profile: J,
        source: "fn Main() with no_suspend{let f=fn ()->Bool{MockUnknown(); return true;};match true{true if (f)()=>{},_=>{}}}",
        code: "NX-E0414",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"unknown_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"suspend\"",
        ],
        primary: (93, 94),
        token: "f",
        related: &[("contract", 15, 25)],
        cause: &[("closure.call", 93, 94), ("intrinsic.call", 44, 55)],
        control: "fn Main() with no_suspend{let f=fn ()->Bool{MockUnknown(); return true;};match true{true if f()=>{},_=>{}}}",
        control_primary: (92, 93),
        repair: "fn Main(){let f=fn ()->Bool{MockUnknown(); return true;};match true{true if (f)()=>{},_=>{}}}",
    },
    Case {
        family: "effects",
        name: "head_no_suspend_name_2_stmt",
        profile: J,
        source: "fn Main() with no_suspend{let f=fn (){MockUnknown();};((f))();}",
        code: "NX-E0414",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"unknown_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"suspend\"",
        ],
        primary: (56, 57),
        token: "f",
        related: &[("contract", 15, 25)],
        cause: &[("closure.call", 56, 57), ("intrinsic.call", 38, 49)],
        control: "fn Main() with no_suspend{let f=fn (){MockUnknown();};f();}",
        control_primary: (54, 55),
        repair: "fn Main(){let f=fn (){MockUnknown();};((f))();}",
    },
    Case {
        family: "effects",
        name: "head_no_suspend_name_2_guard",
        profile: J,
        source: "fn Main() with no_suspend{let f=fn ()->Bool{MockUnknown(); return true;};match true{true if ((f))()=>{},_=>{}}}",
        code: "NX-E0414",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"unknown_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"suspend\"",
        ],
        primary: (94, 95),
        token: "f",
        related: &[("contract", 15, 25)],
        cause: &[("closure.call", 94, 95), ("intrinsic.call", 44, 55)],
        control: "fn Main() with no_suspend{let f=fn ()->Bool{MockUnknown(); return true;};match true{true if f()=>{},_=>{}}}",
        control_primary: (92, 93),
        repair: "fn Main(){let f=fn ()->Bool{MockUnknown(); return true;};match true{true if ((f))()=>{},_=>{}}}",
    },
    Case {
        family: "effects",
        name: "head_no_suspend_literal_1_stmt",
        profile: J,
        source: "fn Main() with no_suspend{(fn (){MockUnknown();})();}",
        code: "NX-E0414",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"unknown_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"suspend\"",
        ],
        primary: (27, 29),
        token: "fn",
        related: &[("contract", 15, 25)],
        cause: &[("closure.call", 27, 29), ("intrinsic.call", 33, 44)],
        control: "fn Main() with no_suspend{fn (){MockUnknown();}();}",
        control_primary: (26, 28),
        repair: "fn Main(){(fn (){MockUnknown();})();}",
    },
    Case {
        family: "effects",
        name: "head_no_suspend_literal_1_guard",
        profile: J,
        source: "fn Main() with no_suspend{match true{true if (fn ()->Bool{MockUnknown(); return true;})()=>{},_=>{}}}",
        code: "NX-E0414",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"unknown_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"suspend\"",
        ],
        primary: (46, 48),
        token: "fn",
        related: &[("contract", 15, 25)],
        cause: &[("closure.call", 46, 48), ("intrinsic.call", 58, 69)],
        control: "fn Main() with no_suspend{match true{true if fn ()->Bool{MockUnknown(); return true;}()=>{},_=>{}}}",
        control_primary: (45, 47),
        repair: "fn Main(){match true{true if (fn ()->Bool{MockUnknown(); return true;})()=>{},_=>{}}}",
    },
    Case {
        family: "effects",
        name: "head_no_suspend_literal_2_stmt",
        profile: J,
        source: "fn Main() with no_suspend{((fn (){MockUnknown();}))();}",
        code: "NX-E0414",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"unknown_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"suspend\"",
        ],
        primary: (28, 30),
        token: "fn",
        related: &[("contract", 15, 25)],
        cause: &[("closure.call", 28, 30), ("intrinsic.call", 34, 45)],
        control: "fn Main() with no_suspend{fn (){MockUnknown();}();}",
        control_primary: (26, 28),
        repair: "fn Main(){((fn (){MockUnknown();}))();}",
    },
    Case {
        family: "effects",
        name: "head_no_suspend_literal_2_guard",
        profile: J,
        source: "fn Main() with no_suspend{match true{true if ((fn ()->Bool{MockUnknown(); return true;}))()=>{},_=>{}}}",
        code: "NX-E0414",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"unknown_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"suspend\"",
        ],
        primary: (47, 49),
        token: "fn",
        related: &[("contract", 15, 25)],
        cause: &[("closure.call", 47, 49), ("intrinsic.call", 59, 70)],
        control: "fn Main() with no_suspend{match true{true if fn ()->Bool{MockUnknown(); return true;}()=>{},_=>{}}}",
        control_primary: (45, 47),
        repair: "fn Main(){match true{true if ((fn ()->Bool{MockUnknown(); return true;}))()=>{},_=>{}}}",
    },
    Case {
        family: "effects",
        name: "head_no_suspend_if_1_stmt",
        profile: J,
        source: "fn Main() with no_suspend{let f=fn (){MockUnknown();};(if true then f else f)();}",
        code: "NX-E0414",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"unknown_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"suspend\"",
        ],
        primary: (55, 57),
        token: "if",
        related: &[("contract", 15, 25)],
        cause: &[("closure.call", 55, 57), ("intrinsic.call", 38, 49)],
        control: "",
        control_primary: (0, 0),
        repair: "fn Main(){let f=fn (){MockUnknown();};(if true then f else f)();}",
    },
    Case {
        family: "effects",
        name: "head_no_suspend_if_1_guard",
        profile: J,
        source: "fn Main() with no_suspend{let f=fn ()->Bool{MockUnknown(); return true;};match true{true if (if true then f else f)()=>{},_=>{}}}",
        code: "NX-E0414",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"unknown_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"suspend\"",
        ],
        primary: (93, 95),
        token: "if",
        related: &[("contract", 15, 25)],
        cause: &[("closure.call", 93, 95), ("intrinsic.call", 44, 55)],
        control: "",
        control_primary: (0, 0),
        repair: "fn Main(){let f=fn ()->Bool{MockUnknown(); return true;};match true{true if (if true then f else f)()=>{},_=>{}}}",
    },
    Case {
        family: "effects",
        name: "head_no_suspend_if_2_stmt",
        profile: J,
        source: "fn Main() with no_suspend{let f=fn (){MockUnknown();};((if true then f else f))();}",
        code: "NX-E0414",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"unknown_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"suspend\"",
        ],
        primary: (56, 58),
        token: "if",
        related: &[("contract", 15, 25)],
        cause: &[("closure.call", 56, 58), ("intrinsic.call", 38, 49)],
        control: "",
        control_primary: (0, 0),
        repair: "fn Main(){let f=fn (){MockUnknown();};((if true then f else f))();}",
    },
    Case {
        family: "effects",
        name: "head_no_suspend_if_2_guard",
        profile: J,
        source: "fn Main() with no_suspend{let f=fn ()->Bool{MockUnknown(); return true;};match true{true if ((if true then f else f))()=>{},_=>{}}}",
        code: "NX-E0414",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"unknown_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"suspend\"",
        ],
        primary: (94, 96),
        token: "if",
        related: &[("contract", 15, 25)],
        cause: &[("closure.call", 94, 96), ("intrinsic.call", 44, 55)],
        control: "",
        control_primary: (0, 0),
        repair: "fn Main(){let f=fn ()->Bool{MockUnknown(); return true;};match true{true if ((if true then f else f))()=>{},_=>{}}}",
    },
    Case {
        family: "effects",
        name: "head_no_suspend_move_1_stmt",
        profile: J,
        source: "fn Main() with no_suspend{let f=fn (){MockUnknown();};(move f)();}",
        code: "NX-E0414",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"unknown_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"suspend\"",
        ],
        primary: (55, 59),
        token: "move",
        related: &[("contract", 15, 25)],
        cause: &[("closure.call", 55, 59), ("intrinsic.call", 38, 49)],
        control: "",
        control_primary: (0, 0),
        repair: "fn Main(){let f=fn (){MockUnknown();};(move f)();}",
    },
    Case {
        family: "effects",
        name: "head_no_suspend_move_2_stmt",
        profile: J,
        source: "fn Main() with no_suspend{let f=fn (){MockUnknown();};((move f))();}",
        code: "NX-E0414",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"unknown_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"suspend\"",
        ],
        primary: (56, 60),
        token: "move",
        related: &[("contract", 15, 25)],
        cause: &[("closure.call", 56, 60), ("intrinsic.call", 38, 49)],
        control: "",
        control_primary: (0, 0),
        repair: "fn Main(){let f=fn (){MockUnknown();};((move f))();}",
    },
    Case {
        family: "effects",
        name: "head_nested_literal",
        profile: J,
        source: "fn Main() with no_io{(fn (){(fn (){Print(1);})();})();}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (22, 24),
        token: "fn",
        related: &[("contract", 15, 20)],
        cause: &[
            ("closure.call", 22, 24),
            ("closure.call", 29, 31),
            ("intrinsic.call", 35, 40),
        ],
        control: "fn Main() with no_io{fn (){fn (){Print(1);}();}();}",
        control_primary: (21, 23),
        repair: "fn Main(){(fn (){(fn (){Print(1);})();})();}",
    },
    Case {
        family: "effects",
        name: "head_literal_calls_name",
        profile: J,
        source: "fn Main() with no_io{let f=fn (){Print(1);};(fn (){f();})();}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (45, 47),
        token: "fn",
        related: &[("contract", 15, 20)],
        cause: &[
            ("closure.call", 45, 47),
            ("closure.call", 51, 52),
            ("intrinsic.call", 33, 38),
        ],
        control: "fn Main() with no_io{let f=fn (){Print(1);};fn (){f();}();}",
        control_primary: (44, 46),
        repair: "fn Main(){let f=fn (){Print(1);};(fn (){f();})();}",
    },
    Case {
        family: "effects",
        name: "head_trivia_before_name",
        profile: J,
        source: "fn Main() with no_io{let f=fn (){Print(1);};(  f)();}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "closure.call",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (47, 48),
        token: "f",
        related: &[("contract", 15, 20)],
        cause: &[("closure.call", 47, 48), ("intrinsic.call", 33, 38)],
        control: "fn Main() with no_io{let f=fn (){Print(1);};f();}",
        control_primary: (44, 45),
        repair: "fn Main(){let f=fn (){Print(1);};(  f)();}",
    },
    Case {
        family: "effects",
        name: "temporary_function_result_grouped",
        profile: J,
        source: "fn G()->MockResource{return MockOpen(1);}fn F(r:view MockResource){}fn Main() with no_io{F((G)());}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "value.drop",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (92, 93),
        token: "G",
        related: &[("contract", 83, 88)],
        cause: &[("value.drop", 91, 96)],
        control: "fn G()->MockResource{return MockOpen(1);}fn F(r:view MockResource){}fn Main() with no_io{F(G());}",
        control_primary: (91, 92),
        repair: "fn G()->MockResource{return MockOpen(1);}fn F(r:view MockResource){}fn Main(){F((G)());}",
    },
    Case {
        family: "effects",
        name: "temporary_function_result_grouped_double",
        profile: J,
        source: "fn G()->MockResource{return MockOpen(1);}fn F(r:view MockResource){}fn Main() with no_io{F(((G))());}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "value.drop",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (93, 94),
        token: "G",
        related: &[("contract", 83, 88)],
        cause: &[("value.drop", 91, 98)],
        control: "fn G()->MockResource{return MockOpen(1);}fn F(r:view MockResource){}fn Main() with no_io{F(G());}",
        control_primary: (91, 92),
        repair: "fn G()->MockResource{return MockOpen(1);}fn F(r:view MockResource){}fn Main(){F(((G))());}",
    },
    Case {
        family: "effects",
        name: "statement_temporary_of_grouped_call",
        profile: J,
        source: "fn G()->MockResource{return MockOpen(1);}fn Main() with no_io{(G)();}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "value.drop",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (63, 64),
        token: "G",
        related: &[("contract", 56, 61)],
        cause: &[("value.drop", 62, 67)],
        control: "fn G()->MockResource{return MockOpen(1);}fn Main() with no_io{G();}",
        control_primary: (62, 63),
        repair: "fn G()->MockResource{return MockOpen(1);}fn Main(){(G)();}",
    },
    Case {
        family: "effects",
        name: "statement_temporary_of_grouped_call_double",
        profile: J,
        source: "fn G()->MockResource{return MockOpen(1);}fn Main() with no_io{((G))();}",
        code: "NX-E0415",
        subcause: None,
        stage: "effects",
        operation: "value.drop",
        feature: None,
        data: &[
            "\"effect_evidence\":\"known_effect\"",
            "\"effect_model\":\"f0-0003-sim-v1\"",
            "\"required_effect\":\"io\"",
        ],
        primary: (64, 65),
        token: "G",
        related: &[("contract", 56, 61)],
        cause: &[("value.drop", 62, 69)],
        control: "fn G()->MockResource{return MockOpen(1);}fn Main() with no_io{G();}",
        control_primary: (62, 63),
        repair: "fn G()->MockResource{return MockOpen(1);}fn Main(){((G))();}",
    },
    Case {
        family: "ownership",
        name: "permission_escalation_move_group",
        profile: J,
        source: "fn G(y:view Option<I64>){let z=move (y);}fn Main(){}",
        code: "NX-E1105",
        subcause: Some("permission_escalation"),
        stage: "ownership",
        operation: "ownership.access",
        feature: None,
        data: &[
            "\"active_permission\":\"read\"",
            "\"id\":\"parameter:5\"",
            "\"id\":\"parameter:5\"",
            "\"requested_access\":\"consume\"",
        ],
        primary: (37, 38),
        token: "y",
        related: &[("declaration", 5, 6)],
        cause: &[("borrow.read", 5, 6)],
        control: "fn G(y:view Option<I64>){let z=move y;}fn Main(){}",
        control_primary: (36, 37),
        repair: "fn G(y:move Option<I64>){let z=move (y);}fn Main(){}",
    },
    Case {
        family: "ownership",
        name: "permission_escalation_move_group_double",
        profile: J,
        source: "fn G(y:view Option<I64>){let z=move ((y));}fn Main(){}",
        code: "NX-E1105",
        subcause: Some("permission_escalation"),
        stage: "ownership",
        operation: "ownership.access",
        feature: None,
        data: &[
            "\"active_permission\":\"read\"",
            "\"id\":\"parameter:5\"",
            "\"id\":\"parameter:5\"",
            "\"requested_access\":\"consume\"",
        ],
        primary: (38, 39),
        token: "y",
        related: &[("declaration", 5, 6)],
        cause: &[("borrow.read", 5, 6)],
        control: "fn G(y:view Option<I64>){let z=move y;}fn Main(){}",
        control_primary: (36, 37),
        repair: "fn G(y:move Option<I64>){let z=move ((y));}fn Main(){}",
    },
    Case {
        family: "for_iteration",
        name: "dedupe_for_empty_array",
        profile: J,
        source: "fn Main(){for x in ([]){}}",
        code: "NX-E0006",
        subcause: None,
        stage: "types",
        operation: "types.check",
        feature: Some("for_iteration"),
        data: &["\"pending_scope\":\"later_adr\""],
        primary: (20, 22),
        token: "[]",
        related: &[],
        cause: &[],
        control: "fn Main(){for x in []{}}",
        control_primary: (19, 21),
        repair: "",
    },
    Case {
        family: "for_iteration",
        name: "dedupe_for_empty_array_double",
        profile: J,
        source: "fn Main(){for x in (([])){}}",
        code: "NX-E0006",
        subcause: None,
        stage: "types",
        operation: "types.check",
        feature: Some("for_iteration"),
        data: &["\"pending_scope\":\"later_adr\""],
        primary: (21, 23),
        token: "[]",
        related: &[],
        cause: &[],
        control: "fn Main(){for x in []{}}",
        control_primary: (19, 21),
        repair: "",
    },
    Case {
        family: "for_iteration",
        name: "dedupe_for_empty_array_space",
        profile: J,
        source: "fn Main(){for x in ( [] ){}}",
        code: "NX-E0006",
        subcause: None,
        stage: "types",
        operation: "types.check",
        feature: Some("for_iteration"),
        data: &["\"pending_scope\":\"later_adr\""],
        primary: (21, 23),
        token: "[]",
        related: &[],
        cause: &[],
        control: "fn Main(){for x in []{}}",
        control_primary: (19, 21),
        repair: "",
    },
    Case {
        family: "for_iteration",
        name: "dedupe_for_empty_array_comment",
        profile: J,
        source: "fn Main(){for x in (// c
[]){}}",
        code: "NX-E0006",
        subcause: None,
        stage: "types",
        operation: "types.check",
        feature: Some("for_iteration"),
        data: &["\"pending_scope\":\"later_adr\""],
        primary: (25, 27),
        token: "[]",
        related: &[],
        cause: &[],
        control: "fn Main(){for x in []{}}",
        control_primary: (19, 21),
        repair: "",
    },
    Case {
        family: "call_bind",
        name: "dedupe_call_target_empty_array",
        profile: J,
        source: "fn Main(){([])();}",
        code: "NX-E0111",
        subcause: Some("non_callable_target"),
        stage: "arguments",
        operation: "call.bind",
        feature: None,
        data: &[],
        primary: (11, 13),
        token: "[]",
        related: &[],
        cause: &[],
        control: "fn Main(){[]();}",
        control_primary: (10, 12),
        repair: "",
    },
    Case {
        family: "call_bind",
        name: "dedupe_call_target_empty_array_double",
        profile: J,
        source: "fn Main(){(([]))();}",
        code: "NX-E0111",
        subcause: Some("non_callable_target"),
        stage: "arguments",
        operation: "call.bind",
        feature: None,
        data: &[],
        primary: (12, 14),
        token: "[]",
        related: &[],
        cause: &[],
        control: "fn Main(){[]();}",
        control_primary: (10, 12),
        repair: "",
    },
    Case {
        family: "call_bind",
        name: "dedupe_call_target_empty_array_comment",
        profile: J,
        source: "fn Main(){(// c
[])();}",
        code: "NX-E0111",
        subcause: Some("non_callable_target"),
        stage: "arguments",
        operation: "call.bind",
        feature: None,
        data: &[],
        primary: (16, 18),
        token: "[]",
        related: &[],
        cause: &[],
        control: "fn Main(){[]();}",
        control_primary: (10, 12),
        repair: "",
    },
    Case {
        family: "call_bind",
        name: "dedupe_call_target_empty_array_f5",
        profile: F5,
        source: "fn Main(){([])();}",
        code: "NX-E0111",
        subcause: Some("non_callable_target"),
        stage: "arguments",
        operation: "call.bind",
        feature: None,
        data: &[],
        primary: (11, 13),
        token: "[]",
        related: &[],
        cause: &[],
        control: "fn Main(){[]();}",
        control_primary: (10, 12),
        repair: "",
    },
    // S6
    Case {
        family: "guard_mutation",
        name: "guard_mutation_single_group",
        profile: J,
        source: "fn Main(){var x=1;match true{true if (if true then {(x)=2;value:true;} else {value:false;}) =>{},_=>{}}}",
        code: "NX-E1305",
        subcause: Some("guard_mutation"),
        stage: "ownership",
        operation: "ownership.check",
        feature: None,
        data: &[],
        primary: (53, 54),
        token: "x",
        related: &[],
        cause: &[],
        control: "fn Main(){var x=1;match true{true if (if true then {x=2;value:true;} else {value:false;}) =>{},_=>{}}}",
        control_primary: (52, 53),
        repair: "fn Main(){var x=1;match true{true if (if true then {value:true;} else {value:false;}) =>{},_=>{}}}",
    },
    Case {
        family: "guard_mutation",
        name: "guard_mutation_double_group",
        profile: J,
        source: "fn Main(){var x=1;match true{true if (if true then {((x))=2;value:true;} else {value:false;}) =>{},_=>{}}}",
        code: "NX-E1305",
        subcause: Some("guard_mutation"),
        stage: "ownership",
        operation: "ownership.check",
        feature: None,
        data: &[],
        primary: (54, 55),
        token: "x",
        related: &[],
        cause: &[],
        control: "fn Main(){var x=1;match true{true if (if true then {x=2;value:true;} else {value:false;}) =>{},_=>{}}}",
        control_primary: (52, 53),
        repair: "fn Main(){var x=1;match true{true if (if true then {value:true;} else {value:false;}) =>{},_=>{}}}",
    },
    Case {
        family: "guard_mutation",
        name: "guard_mutation_group_with_trivia",
        profile: J,
        source: "fn Main(){var x=1;match true{true if (if true then {( x )=2;value:true;} else {value:false;}) =>{},_=>{}}}",
        code: "NX-E1305",
        subcause: Some("guard_mutation"),
        stage: "ownership",
        operation: "ownership.check",
        feature: None,
        data: &[],
        primary: (54, 55),
        token: "x",
        related: &[],
        cause: &[],
        control: "fn Main(){var x=1;match true{true if (if true then {x=2;value:true;} else {value:false;}) =>{},_=>{}}}",
        control_primary: (52, 53),
        repair: "fn Main(){var x=1;match true{true if (if true then {value:true;} else {value:false;}) =>{},_=>{}}}",
    },
    // S7
    Case {
        family: "guard_transfer",
        name: "guard_transfer_single_group",
        profile: J,
        source: "fn Main(){let a=Some(1);match true{true if (if true then {(Drop)(a);value:true;} else {value:false;}) =>{},_=>{}}}",
        code: "NX-E1305",
        subcause: Some("guard_transfer"),
        stage: "ownership",
        operation: "ownership.check",
        feature: None,
        data: &[],
        primary: (59, 63),
        token: "Drop",
        related: &[],
        cause: &[],
        control: "fn Main(){let a=Some(1);match true{true if (if true then {Drop(a);value:true;} else {value:false;}) =>{},_=>{}}}",
        control_primary: (58, 62),
        repair: "fn Main(){let a=Some(1);match true{true if (if true then {value:true;} else {value:false;}) =>{},_=>{}}}",
    },
    Case {
        family: "guard_transfer",
        name: "guard_transfer_double_group",
        profile: J,
        source: "fn Main(){let a=Some(1);match true{true if (if true then {((Drop))(a);value:true;} else {value:false;}) =>{},_=>{}}}",
        code: "NX-E1305",
        subcause: Some("guard_transfer"),
        stage: "ownership",
        operation: "ownership.check",
        feature: None,
        data: &[],
        primary: (60, 64),
        token: "Drop",
        related: &[],
        cause: &[],
        control: "fn Main(){let a=Some(1);match true{true if (if true then {Drop(a);value:true;} else {value:false;}) =>{},_=>{}}}",
        control_primary: (58, 62),
        repair: "fn Main(){let a=Some(1);match true{true if (if true then {value:true;} else {value:false;}) =>{},_=>{}}}",
    },
    Case {
        family: "guard_transfer",
        name: "guard_transfer_group_with_trivia",
        profile: J,
        source: "fn Main(){let a=Some(1);match true{true if (if true then {( Drop )(a);value:true;} else {value:false;}) =>{},_=>{}}}",
        code: "NX-E1305",
        subcause: Some("guard_transfer"),
        stage: "ownership",
        operation: "ownership.check",
        feature: None,
        data: &[],
        primary: (60, 64),
        token: "Drop",
        related: &[],
        cause: &[],
        control: "fn Main(){let a=Some(1);match true{true if (if true then {Drop(a);value:true;} else {value:false;}) =>{},_=>{}}}",
        control_primary: (58, 62),
        repair: "fn Main(){let a=Some(1);match true{true if (if true then {value:true;} else {value:false;}) =>{},_=>{}}}",
    },
];

fn number_after(text: &str, key: &str) -> usize {
    let at = text
        .find(key)
        .unwrap_or_else(|| panic!("missing {key} in {text}"))
        + key.len();
    text[at..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap()
}

fn span_of(text: &str) -> (usize, usize) {
    (
        number_after(text, "\"start\":"),
        number_after(text, "\"end\":"),
    )
}

fn section<'a>(json: &'a str, from: &str, to: &str) -> &'a str {
    let start = json.find(from).unwrap_or_else(|| panic!("missing {from}")) + from.len();
    let end = start
        + json[start..]
            .find(to)
            .unwrap_or_else(|| panic!("missing {to}"));
    &json[start..end]
}

fn related(json: &str) -> Vec<(String, usize, usize)> {
    let body = section(json, "\"related_spans\":[", "],\"cause_chain\"");
    body.split("{\"role\":\"")
        .skip(1)
        .map(|part| {
            let (start, end) = span_of(part);
            (part[..part.find('"').unwrap()].to_owned(), start, end)
        })
        .collect()
}

fn cause(json: &str) -> Vec<(String, usize, usize)> {
    let body = section(json, "\"cause_chain\":[", "],\"feature_id\"");
    body.split("\"operation\":\"")
        .skip(1)
        .map(|part| {
            let operation = &part[..part.find('"').unwrap()];
            // A step without a span (a resource drop) is recorded as (0, 0).
            let (start, end) = if part[operation.len()..].starts_with("\",\"span\":null") {
                (0, 0)
            } else {
                span_of(part)
            };
            (operation.to_owned(), start, end)
        })
        .collect()
}

fn one_diagnostic(verb: &str, source: &str, profile: &str) -> String {
    let (exit, json, stderr) = invoke(verb, source, profile);
    assert_eq!(exit, 1, "{verb}: {source}: {json}: {stderr}");
    assert!(stderr.is_empty(), "{verb}: {source}: {stderr}");
    for fragment in [
        "\"outcome\":{\"kind\":\"rejected\"}",
        "\"stdout\":[]",
        "\"events\":[]",
        "\"observations\":[]",
    ] {
        assert!(json.contains(fragment), "{verb}: {source}: {json}");
    }
    assert_eq!(
        json.matches("{\"schema_version\":1,\"diagnostic_namespace\"")
            .count(),
        1,
        "{verb}: {source}: {json}"
    );
    json
}

fn primary(json: &str) -> (usize, usize) {
    span_of(section(json, "\"primary_span\":{", "}"))
}

fn rule_fragments(json: &str, case: &Case, with_data: bool) {
    let rule = format!(
        "{}{}",
        case.code,
        case.subcause.map(|s| format!("/{s}")).unwrap_or_default()
    );
    for fragment in [
        format!("\"code\":\"{}\"", case.code),
        format!(
            "\"subcause\":{}",
            case.subcause
                .map(|s| format!("\"{s}\""))
                .unwrap_or_else(|| "null".into())
        ),
        format!("\"rule\":\"{rule}\""),
        format!("\"explanation_id\":\"{rule}\""),
        format!("\"stage\":\"{}\"", case.stage),
        format!(
            "\"feature_id\":{}",
            case.feature
                .map(|s| format!("\"{s}\""))
                .unwrap_or_else(|| "null".into())
        ),
    ] {
        assert!(
            json.contains(&fragment),
            "{}: {fragment}: {json}",
            case.name
        );
    }
    // The control only has to be the same rule; its witness may differ.
    if with_data {
        let operation = format!("\"operation\":\"{}\"", case.operation);
        assert!(
            json.contains(&operation),
            "{}: {operation}: {json}",
            case.name
        );
    }
    for fragment in case.data.iter().filter(|_| with_data) {
        assert!(json.contains(fragment), "{}: {fragment}: {json}", case.name);
    }
}

fn run_family(family: &str) {
    let mut seen = 0;
    for case in CASES.iter().filter(|case| case.family == family) {
        seen += 1;
        assert!(case.source.is_ascii() && case.control.is_ascii() && case.repair.is_ascii());
        // Only the intrinsic edit arguments (Push, Pop) and the unresolved `[]` rows have no accepted repair.
        assert!(
            !case.repair.is_empty()
                || (case.name.starts_with("intrinsic_edit_argument_p")
                    || case.name.starts_with("dedupe_"))
        );
        assert_eq!(
            &case.source[case.primary.0..case.primary.1],
            case.token,
            "{}: the expected span must slice the stated token",
            case.name
        );
        if !case.control.is_empty() {
            let bare = &case.control[case.control_primary.0..case.control_primary.1];
            assert!(
                !bare.starts_with('('),
                "{}: the control primary is the bare expression",
                case.name
            );
        }
        for verb in ["check", "run"] {
            let json = one_diagnostic(verb, case.source, case.profile);
            rule_fragments(&json, case, true);
            assert_eq!(
                primary(&json),
                case.primary,
                "{}: {verb}: {json}",
                case.name
            );
            let expected_related: Vec<_> = case
                .related
                .iter()
                .map(|(role, a, b)| (role.to_string(), *a, *b))
                .collect();
            assert_eq!(related(&json), expected_related, "{}: {verb}", case.name);
            let expected_cause: Vec<_> = case
                .cause
                .iter()
                .map(|(operation, a, b)| (operation.to_string(), *a, *b))
                .collect();
            if case.cause.is_empty() {
                assert!(json.contains("\"cause_chain\":[]"), "{}: {json}", case.name);
            } else {
                assert_eq!(cause(&json), expected_cause, "{}: {verb}", case.name);
            }
            if !case.control.is_empty() {
                let json = one_diagnostic(verb, case.control, case.profile);
                rule_fragments(&json, case, false);
                assert_eq!(
                    primary(&json),
                    case.control_primary,
                    "{}: control {verb}: {json}",
                    case.name
                );
            }
            if case.repair.is_empty() {
                continue;
            }
            let (exit, json, stderr) = invoke(verb, case.repair, case.profile);
            assert_eq!(exit, 0, "{}: repair {verb}: {json}: {stderr}", case.name);
            assert!(json.contains("\"diagnostics\":[]"), "{}: {json}", case.name);
            assert!(
                json.contains(if verb == "check" {
                    "\"outcome\":{\"kind\":\"checked\"}"
                } else {
                    "\"outcome\":{\"kind\":\"unit\"}"
                }),
                "{}: {json}",
                case.name
            );
        }
    }
    assert!(seen > 0, "no cases for {family}");
}

#[test]
fn array_iterable_barrier_reports_the_inner_iterable() {
    run_family("for_iteration");
}

#[test]
fn call_binding_diagnostics_report_the_inner_target_and_argument() {
    run_family("call_bind");
}

#[test]
fn parser_partial_transfer_barrier_reports_the_inner_place() {
    run_family("parse_barrier");
}

#[test]
fn ownership_diagnostics_report_the_inner_place_or_value() {
    run_family("ownership");
}

#[test]
fn lowering_capability_barrier_reports_the_inner_place() {
    run_family("lower_barrier");
}

#[test]
fn effect_diagnostics_report_the_target_name_of_a_grouped_call() {
    run_family("effects");
}

// Anchors that are not the call target expression keep their own site: the
// scheduled `Next` token of a written `Iterator.Next` call (F0-0003 section
// 5.2) and the `for` keyword of the synthetic resume of a generator loop.
#[test]
fn generator_resume_anchors_ignore_the_group_around_the_target() {
    const GEN: &str = "gen fn G()->I64{Print(1);yield 1;}";
    for (body, token_start) in [
        ("var g=G();Iterator.Next(edit g);", 74),
        ("var g=G();(Iterator.Next)(edit g);", 75),
        ("for x in G(){}", 55),
        ("for x in (G()){}", 55),
        ("for x in ((G())){}", 55),
    ] {
        let source = format!("{GEN}fn Main() with no_io{{{body}}}");
        let token = if body.starts_with("for") {
            "for"
        } else {
            "Next"
        };
        assert_eq!(
            &source[token_start..token_start + token.len()],
            token,
            "{source}"
        );
        for verb in ["check", "run"] {
            let json = one_diagnostic(verb, &source, F5);
            for fragment in ["\"code\":\"NX-E0415\"", "\"stage\":\"effects\""] {
                assert!(json.contains(fragment), "{source}: {json}");
            }
            assert_eq!(
                primary(&json),
                (token_start, token_start + token.len()),
                "{source}: {verb}"
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Quota ticks under groups (F0-0002 lagunas, "Ticks, cuotas y guardas bajo agrupaciones"): the
// R0001 primary of a tick whose origin is an expression is the inner expression, never the group.
// A quota failure is a run-time fact: `check` accepts every source below. Expected spans are
// derived from the source (the position of the expression inside its template) and from the
// control without a group, never from the binary under test: the grouped source must give the
// control diagnostic with its spans moved by the wrapper characters.
// ---------------------------------------------------------------------------------------------

/// Outer wrappers around the expression under test: none (the control), one group, two groups and
/// trivia inside one group.
const WRAPS: [(&str, &str); 4] = [("", ""), ("(", ")"), ("((", "))"), ("( ", " )")];

struct QuotaSite {
    /// `@P` is the pad of `{}` blocks and `@E` the wrapped expression.
    template: &'static str,
    /// The same program with a bounded loop: it is accepted whatever the wrapper.
    positive: &'static str,
    inner: &'static str,
    /// Pad for which the rejected tick of the control is the tick of this site.
    pad: usize,
}

/// A site whose group-free tick origin was already the inner expression before the group removal.
const LET_INITIALIZER: QuotaSite = QuotaSite {
    template: "fn Main(){@Pvar n=0;while true{let y=@E;}}",
    positive: "fn Main(){@Pvar n=0;while n<3{n=n+1;let y=@E;}}",
    inner: "n",
    pad: 0,
};

/// The source with `pad` empty blocks before the loop, and the start of the inner expression.
fn quota_source(site: &QuotaSite, pad: usize, wrap: (&str, &str)) -> (String, usize) {
    quota_template(site.template, site.inner, pad, wrap)
}

fn quota_template(template: &str, inner: &str, pad: usize, wrap: (&str, &str)) -> (String, usize) {
    let padded = template.replace("@P", &"{}".repeat(pad));
    let at = padded
        .find("@E")
        .expect("the template places an expression");
    let source = padded.replacen("@E", &format!("{}{}{}", wrap.0, inner, wrap.1), 1);
    (source, at + wrap.0.len())
}

/// The diagnostic without the numbers of its spans and without the file name.
fn without_spans(json: &str) -> String {
    let mut out = String::new();
    let mut rest = json;
    while let Some((at, key)) = ["\"start\":", "\"end\":", "\"file\":\""]
        .iter()
        .filter_map(|key| rest.find(key).map(|at| (at, *key)))
        .min()
    {
        let after = at + key.len();
        out.push_str(&rest[..after]);
        let skip = if key.ends_with('"') {
            rest[after..].find('"').unwrap()
        } else {
            rest[after..]
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(rest.len() - after)
        };
        out.push('#');
        rest = &rest[after + skip..];
    }
    out.push_str(rest);
    out
}

/// Moves a span of the bare control to the same program with the wrapper around `interior`: the
/// interior itself moves inside the wrapper, a span that contains it keeps its start and grows by
/// the wrapper, and any other span moves by the wrapper characters that precede each end.
fn moved_span(
    span: (usize, usize),
    interior: (usize, usize),
    wrap: (&str, &str),
) -> (usize, usize) {
    let (open, close) = (wrap.0.len(), wrap.1.len());
    if span == interior {
        return (span.0 + open, span.1 + open);
    }
    if span.0 <= interior.0 && span.1 >= interior.1 {
        return (span.0, span.1 + open + close);
    }
    let start = if span.0 < interior.0 {
        span.0
    } else if span.0 < interior.1 {
        span.0 + open
    } else {
        span.0 + open + close
    };
    let end = if span.1 <= interior.0 {
        span.1
    } else if span.1 <= interior.1 {
        span.1 + open
    } else {
        span.1 + open + close
    };
    (start, end)
}

#[test]
fn quota_span_helpers_move_spans_across_the_wrapper() {
    let site = QuotaSite {
        template: "ab@P@Ecd",
        positive: "",
        inner: "xy",
        pad: 0,
    };
    let (bare, bare_at) = quota_source(&site, 1, WRAPS[0]);
    assert_eq!((bare.as_str(), bare_at), ("ab{}xycd", 4));
    let (grouped, at) = quota_source(&site, 1, WRAPS[2]);
    assert_eq!((grouped.as_str(), at), ("ab{}((xy))cd", 6));
    let interior = (4, 6);
    for (span, wrap, moved) in [
        ((0, 2), WRAPS[2], (0, 2)),
        ((4, 6), WRAPS[1], (5, 7)),
        ((3, 7), WRAPS[3], (3, 11)),
        ((6, 8), WRAPS[2], (10, 12)),
        ((4, 9), WRAPS[1], (4, 11)),
        ((2, 6), WRAPS[1], (2, 8)),
    ] {
        assert_eq!(moved_span(span, interior, wrap), moved);
    }
    assert_eq!(
        without_spans("{\"file\":\"a.n\",\"start\":12,\"end\":7,\"k\":3}"),
        "{\"file\":\"#\",\"start\":#,\"end\":#,\"k\":3}"
    );
}

/// One pad of one site: the control and the wrapped variants give the same R0001 (`run`), with the
/// primary of the control moved across the wrapper; `fixed` also requires, for the pad whose
/// rejected tick is the tick of the site, that the control primary is the inner expression.
/// Returns whether the rejected tick of the control is the tick of the site.
fn quota_row(site: &QuotaSite, pad: usize, fixed: bool) -> bool {
    let (bare, bare_at) = quota_source(site, pad, WRAPS[0]);
    let (exit, control, stderr) = invoke("run", &bare, J);
    assert!(stderr.is_empty(), "{bare}: {stderr}");
    assert_eq!(exit, 1, "{bare}: {control}");
    for fragment in [
        "\"outcome\":{\"kind\":\"simulator_fault\"}",
        "\"code\":\"R0001\"",
        "\"operation\":\"runtime.limit\"",
    ] {
        assert!(control.contains(fragment), "{bare}: {control}");
    }
    assert_eq!(control.matches("\"code\":").count(), 1, "{bare}: {control}");
    let control_primary = primary(&control);
    let interior = (bare_at, bare_at + site.inner.len());
    if fixed {
        assert_eq!(
            control_primary, interior,
            "{bare}: the site tick is rejected"
        );
    }
    for wrap in &WRAPS[1..] {
        let (source, at) = quota_source(site, pad, *wrap);
        let (exit_g, json, stderr_g) = invoke("run", &source, J);
        assert!(stderr_g.is_empty(), "{source}: {stderr_g}");
        assert_eq!(exit_g, exit, "{source}: {json}");
        assert_eq!(without_spans(&json), without_spans(&control), "{source}");
        assert_eq!(
            primary(&json),
            moved_span(control_primary, interior, *wrap),
            "{source}: {json}"
        );
        if fixed {
            assert_eq!(primary(&json), (at, at + site.inner.len()), "{source}");
        }
        let (exit_c, checked, stderr_c) = invoke("check", &source, J);
        assert!(stderr_c.is_empty(), "{source}: {stderr_c}");
        assert_eq!(exit_c, 0, "{source}: {checked}");
        assert!(
            checked.contains("\"outcome\":{\"kind\":\"checked\"}")
                && checked.contains("\"diagnostics\":[]"),
            "{source}: {checked}"
        );
    }
    control_primary == interior
}

/// Every pad (each tick of the iteration is rejected in turn), the site tick among them, and the
/// bounded program accepted under every wrapper.
fn quota_sweep(site: &QuotaSite) {
    let mut at_the_site = 0;
    for pad in 0..=11 {
        at_the_site += usize::from(quota_row(site, pad, pad == site.pad));
    }
    assert!(
        at_the_site > 0,
        "no pad rejects the tick of {}",
        site.template
    );
    for wrap in WRAPS {
        accepted(&quota_template(site.positive, site.inner, 0, wrap).0, J);
    }
}

#[test]
fn let_initializer_tick_keeps_the_inner_primary_under_groups() {
    quota_sweep(&LET_INITIALIZER);
}

// S1: the tick of an expression statement has the whole expression (without terminator) as origin
// (F0-0002 section 11, "Origen de los ticks"); under groups it is the inner expression.
const S1_STATEMENT: QuotaSite = QuotaSite {
    template: "fn Main(){@Pvar n=0;while true{@E;}}",
    positive: "fn Main(){@Pvar n=0;while n<3{n=n+1;@E;}}",
    inner: "n+1",
    pad: 2,
};

fn s1_form(inner: &'static str) -> QuotaSite {
    QuotaSite {
        template: "fn Id(x:view I64)->I64{return x;}fn Main(){@Pvar n=0;while true{@E;}}",
        positive: "fn Id(x:view I64)->I64{return x;}fn Main(){@Pvar n=0;while n<3{n=n+1;@E;}}",
        inner,
        pad: 0,
    }
}

#[test]
fn expression_statement_tick_reports_the_inner_expression() {
    quota_sweep(&S1_STATEMENT);
}

#[test]
fn expression_statement_tick_reports_the_inner_call() {
    quota_sweep(&s1_form("Id(n)"));
}

#[test]
fn expression_statement_tick_reports_the_inner_assert_call() {
    quota_sweep(&s1_form("AssertEq(1,1)"));
}

#[test]
fn expression_statement_tick_reports_the_inner_closure() {
    quota_sweep(&s1_form("fn (){}"));
}

#[test]
fn expression_statement_tick_reports_the_inner_unit() {
    quota_sweep(&s1_form("()"));
}

#[test]
fn expression_statement_tick_reports_the_inner_array() {
    quota_sweep(&s1_form("[1]"));
}

// S2: the extra tick of a loop condition has the condition expression as origin (F0-0002 section
// 11, "Origen de los ticks"); under groups it is the inner condition.
const S2_CONDITION: QuotaSite = QuotaSite {
    template: "fn Main(){@Pvar n=0;while @E{}}",
    positive: "fn Main(){@Pvar n=0;while @E{n=n+1;}}",
    inner: "n<3",
    pad: 3,
};

#[test]
fn while_condition_tick_reports_the_inner_condition() {
    quota_sweep(&S2_CONDITION);
}

#[test]
fn while_condition_tick_reports_the_inner_comparison_of_a_large_bound() {
    // The witness of the quota: the loop never reaches its bound before the work quota.
    quota_row(
        &QuotaSite {
            template: "fn Main(){@Pvar n=0;while @E{}}",
            positive: "",
            inner: "n<9999999",
            pad: 3,
        },
        3,
        true,
    );
}

// S3: the tick of an assignment has the whole destination place as origin (F0-0002 section 11,
// "Origen de los ticks"); under groups it is the inner place.
const S3_PLACE: QuotaSite = QuotaSite {
    template: "fn Main(){@Pvar n=0;while true{@E=1;}}",
    positive: "fn Main(){@Pvar n=0;while n<3{@E=n+1;}}",
    inner: "n",
    pad: 2,
};

#[test]
fn assignment_tick_reports_the_inner_place() {
    quota_sweep(&S3_PLACE);
}

// S4: the tick of a branch operand without `value:` has the operand expression as origin (F0-0002
// section 11, "Origen de los ticks"); under groups it is the inner operand. The marker `value:`
// stays a token.
const S4_THEN: QuotaSite = QuotaSite {
    template: "fn Main(){@Pvar n=0;while true{let y=if true then @E else 2;}}",
    positive: "fn Main(){@Pvar n=0;while n<3{n=n+1;let y=if true then @E else 2;}}",
    inner: "n",
    pad: 8,
};

const S4_ELSE: QuotaSite = QuotaSite {
    template: "fn Main(){@Pvar n=0;while true{let y=if false then 2 else @E;}}",
    positive: "fn Main(){@Pvar n=0;while n<3{n=n+1;let y=if false then 2 else @E;}}",
    inner: "n",
    pad: 8,
};

const S4_EXPRESSION: QuotaSite = QuotaSite {
    template: "fn Main(){@Pvar n=0;while true{let y=if true then @E else 2;}}",
    positive: "fn Main(){@Pvar n=0;while n<3{n=n+1;let y=if true then @E else 2;}}",
    inner: "n+1",
    pad: 1,
};

#[test]
fn then_operand_tick_reports_the_inner_operand() {
    quota_sweep(&S4_THEN);
}

#[test]
fn else_operand_tick_reports_the_inner_operand() {
    quota_sweep(&S4_ELSE);
}

#[test]
fn branch_operand_tick_reports_the_inner_expression() {
    quota_sweep(&S4_EXPRESSION);
}

// S6: the place assigned inside a guard is reported without the outer groups (F0-0003 section 3.3,
// F0-0002 lagunas "Ticks, cuotas y guardas bajo agrupaciones").
#[test]
fn guard_mutation_reports_the_inner_place() {
    run_family("guard_mutation");
}

// S5: grouping does not add or remove chargeable nodes (F0-0002 section 11: "Los parentesis de
// agrupacion no crean nodos cobrables"; "Nombre del destino de una llamada directa": 1), so a call
// through a grouped target charges the ticks of the bare call. The relation is checked on the
// quota boundary of a loop: the largest bound the control completes is the one the grouped source
// completes, and one more fails with the same R0001 diagnostic. `@N` is the loop bound and `@E`
// the (wrapped) call target of the template.
fn count_source(template: &str, inner: &str, bound: usize, wrap: (&str, &str)) -> (String, usize) {
    quota_template(&template.replace("@N", &bound.to_string()), inner, 0, wrap)
}

/// The R0001 diagnostic when the source exceeds the work quota, `None` when it completes.
fn quota_outcome(template: &str, inner: &str, bound: usize, wrap: (&str, &str)) -> Option<String> {
    let (source, _) = count_source(template, inner, bound, wrap);
    let (exit, json, stderr) = invoke("run", &source, J);
    assert!(stderr.is_empty(), "{source}: {stderr}");
    if exit == 0 {
        assert!(json.contains("\"outcome\":{\"kind\":\"unit\"}"), "{source}");
        return None;
    }
    assert!(
        exit == 1
            && json.contains("\"code\":\"R0001\"")
            && json.contains("\"outcome\":{\"kind\":\"simulator_fault\"}"),
        "{source}: {json}"
    );
    Some(json)
}

/// The largest loop bound the control completes without exceeding the work quota.
fn quota_boundary(template: &str, inner: &str) -> usize {
    let (mut pass, mut fail) = (1, 2);
    while quota_outcome(template, inner, fail, WRAPS[0]).is_none() {
        (pass, fail) = (fail, fail * 2);
        assert!(fail < 100_000_000, "the quota is never exceeded");
    }
    while fail - pass > 1 {
        let middle = pass + (fail - pass) / 2;
        if quota_outcome(template, inner, middle, WRAPS[0]).is_none() {
            pass = middle;
        } else {
            fail = middle;
        }
    }
    pass
}

fn quota_count_row(template: &str, inner: &str) {
    let bound = quota_boundary(template, inner);
    assert!(bound > 1, "{template}: the control must iterate");
    let control = quota_outcome(template, inner, bound + 1, WRAPS[0])
        .unwrap_or_else(|| panic!("{template}: the control fails one iteration later"));
    let at = count_source(template, inner, bound + 1, WRAPS[0]).1;
    let interior = (at, at + inner.len());
    for wrap in &WRAPS[1..] {
        assert_eq!(
            quota_outcome(template, inner, bound, *wrap),
            None,
            "{template}: the bound {bound} completes under {wrap:?}"
        );
        let json = quota_outcome(template, inner, bound + 1, *wrap)
            .unwrap_or_else(|| panic!("{template}: {} must fail under {wrap:?}", bound + 1));
        assert_eq!(without_spans(&json), without_spans(&control), "{wrap:?}");
        assert_eq!(
            primary(&json),
            moved_span(primary(&control), interior, *wrap),
            "{template}: {wrap:?}"
        );
    }
}

#[test]
fn grouped_call_target_charges_the_ticks_of_the_bare_call() {
    quota_count_row(
        "fn Id(x:view I64)->I64{return x;}fn Main(){var n=0;while n<@N{n=n+1;@E(1);}}",
        "Id",
    );
}

#[test]
fn grouped_closure_call_charges_the_ticks_of_the_bare_call() {
    quota_count_row(
        "fn Main(){var n=0;let f=fn (){};while n<@N{n=n+1;@E();}}",
        "f",
    );
}

// S7: the callee `Drop` inside a guard is recognised and reported without the outer groups
// (F0-0003 section 3.3, F0-0002 lagunas "Ticks, cuotas y guardas bajo agrupaciones"): `(Drop)(a)`
// is rejected like `Drop(a)`.
#[test]
fn guard_transfer_reports_the_inner_callee() {
    run_family("guard_transfer");
}

#[test]
fn grouped_callee_other_than_drop_is_accepted_in_a_guard() {
    for (open, close) in WRAPS {
        accepted(
            &format!(
                "fn F(x:view I64){{}}fn Main(){{match true{{true if (if true then {{{open}F{close}(1);value:true;}} else {{value:false;}}) =>{{}},_=>{{}}}}}}"
            ),
            J,
        );
    }
}

// S6/S7 guard walker: every edge of the structure reached by the guard check is descended into, so a
// transfer or a mutation nested in any sub-expression, branch, loop, arm, argument or operand of a
// guard is reported once, at its own token (F0-0003 section 3.3). Each row places one `Drop(a)` (a
// transfer at the callee name) or one selection mode token behind a different edge; the expected
// primaries are the positions of the tokens in the source.
#[test]
fn guard_checks_reach_every_nested_edge() {
    // (body of the guard block, extra diagnostics: (subcause, anchor text, primary text))
    type Extra = (&'static str, &'static str, &'static str);
    const ROWS: &[(&str, &[Extra])] = &[
        ("let z=Drop(a);", &[]),
        ("var u=();u=Drop(a);", &[("guard_mutation", "u=Drop", "u")]),
        (
            "v[if true then {Drop(a);value:0;} else {value:1;}]=1;",
            &[(
                "guard_mutation",
                "v[if",
                "v[if true then {Drop(a);value:0;} else {value:1;}]",
            )],
        ),
        ("if true {Drop(a);}", &[]),
        (
            "if (if true then {Drop(a);value:true;} else {value:false;}) {}",
            &[],
        ),
        ("if false {} else {Drop(a);}", &[]),
        (
            "while (if true then {Drop(a);value:false;} else {value:false;}) {}",
            &[],
        ),
        ("while false {Drop(a);}", &[]),
        (
            "match (if true then {Drop(a);value:true;} else {value:false;}){true=>{},_=>{}}",
            &[],
        ),
        ("match true{true=>{Drop(a);},_=>{}}", &[]),
        (
            "match true{true if (if true then {Drop(a);value:true;} else {value:false;})=>{},_=>{}}",
            &[],
        ),
        ("{Drop(a);}", &[]),
        (
            "let q=[if true then {Drop(a);value:1;} else {value:2;}];",
            &[],
        ),
        (
            "let q=(if true then {Drop(a);value:1;} else {value:2;})+1;",
            &[],
        ),
        (
            "let q=1+(if true then {Drop(a);value:1;} else {value:2;});",
            &[],
        ),
        (
            "let q=0-(if true then {Drop(a);value:1;} else {value:2;});",
            &[],
        ),
        (
            "let q=(if true then {Drop(a);value:v;} else {value:v;})[0];",
            &[],
        ),
        (
            "let q=v[if true then {Drop(a);value:0;} else {value:1;}];",
            &[],
        ),
        ("let q=if true then 1 else {Drop(a);value:2;};", &[]),
        (
            "let q=if false then 1 else (if true then {Drop(a);value:2;} else {value:3;});",
            &[],
        ),
        (
            "let q=if (if true then {Drop(a);value:true;} else {value:false;}) then 1 else 2;",
            &[],
        ),
        (
            "Id((if true then {Drop(a);value:1;} else {value:2;}));",
            &[],
        ),
        (
            "let q=match true{true=>{Drop(a);value:1;},_=>{value:2;}};",
            &[],
        ),
    ];
    for (body, extra) in ROWS {
        let source = format!(
            "fn Id(x:view I64)->I64{{return x;}}fn Main(){{let a=Some(1);var x=1;var v=[1,2];match true{{true if (if true then {{{body} value:true;}} else {{value:false;}}) =>{{}},_=>{{}}}}}}"
        );
        let drop = source.find("Drop(a)").unwrap();
        let mut expected = vec![("guard_transfer", drop, drop + 4)];
        for (cause, anchor, primary_text) in *extra {
            let at = source.find(anchor).unwrap();
            assert!(source[at..].starts_with(primary_text), "{source}");
            expected.push((cause, at, at + primary_text.len()));
        }
        expected.sort_by_key(|row| row.1);
        for verb in ["check", "run"] {
            let (exit, json, stderr) = invoke(verb, &source, J);
            assert_eq!(exit, 1, "{verb}: {source}: {json}");
            assert!(stderr.is_empty(), "{source}: {stderr}");
            let found: Vec<(String, usize, usize)> = json
                .split("\"subcause\":\"")
                .skip(1)
                .map(|part| {
                    let cause = part[..part.find('"').unwrap()].to_owned();
                    let (start, end) = span_of(section(part, "\"primary_span\":{", "}"));
                    (cause, start, end)
                })
                .collect();
            let wanted: Vec<(String, usize, usize)> = expected
                .iter()
                .map(|(cause, a, b)| (cause.to_string(), *a, *b))
                .collect();
            assert_eq!(found, wanted, "{verb}: {source}");
        }
    }
    // The value of a `value:` statement is part of the guard as well.
    let source = "fn Main(){let a=Some(1);match true{true if (if true then {value:(if true then {Drop(a);value:true;} else {value:false;});} else {value:false;}) =>{},_=>{}}}";
    let drop = source.find("Drop").unwrap();
    for verb in ["check", "run"] {
        let json = one_diagnostic(verb, source, J);
        assert!(
            json.contains("\"subcause\":\"guard_transfer\""),
            "{source}: {json}"
        );
        assert_eq!(primary(&json), (drop, drop + 4), "{verb}: {source}");
    }
    // A selection that consumes or mutates a place inside a guard is reported at its mode token.
    for (body, cause) in [
        ("match move a{Some(q)=>{},_=>{}}", "guard_transfer"),
        ("match edit x{_=>{}}", "guard_mutation"),
    ] {
        let source = format!(
            "fn Main(){{let a=Some(1);var x=1;match true{{true if (if true then {{{body} value:true;}} else {{value:false;}}) =>{{}},_=>{{}}}}}}"
        );
        let token = source.find(&body[6..10]).unwrap();
        for verb in ["check", "run"] {
            let json = one_diagnostic(verb, &source, J);
            assert!(
                json.contains(&format!("\"subcause\":\"{cause}\"")),
                "{source}: {json}"
            );
            assert_eq!(primary(&json), (token, token + 4), "{verb}: {source}");
        }
    }
}

// S9: guard walker edges and one tick origin that the mutation pass of the touched regions
// exhibited with a program. Expected spans are byte positions of tokens in the source (F0-0003
// section 3.3: every transfer, mutation or control exit in a guard is reported at its own token;
// F0-0002 section 11: the tick of an expression statement has the whole expression as origin);
// each row is paired with the same program without the nested transfer, which has to report only
// what remains (or be accepted).
fn guard_diagnostics(verb: &str, source: &str) -> (i32, Vec<(String, usize, usize)>) {
    let (exit, json, stderr) = invoke(verb, source, J);
    assert!(stderr.is_empty(), "{verb}: {source}: {stderr}");
    let found = json
        .split("\"subcause\":\"")
        .skip(1)
        .map(|part| {
            let cause = part[..part.find('"').unwrap()].to_owned();
            let (start, end) = span_of(section(part, "\"primary_span\":{", "}"));
            (cause, start, end)
        })
        .collect();
    (exit, found)
}

fn at(source: &str, token: &str) -> usize {
    source
        .find(token)
        .unwrap_or_else(|| panic!("{token} in {source}"))
}

#[test]
fn guard_transfer_inside_the_callee_expression_of_a_call_is_reported() {
    let with = "fn Main(){let a=Some(1);let f=fn (){};match true{true if (if true then {(if true then {Drop(a);value:f;} else {value:f;})();value:true;} else {value:false;}) =>{},_=>{}}}";
    let without = with.replace("Drop(a);", "");
    let drop = at(with, "Drop");
    for verb in ["check", "run"] {
        let (exit, found) = guard_diagnostics(verb, with);
        assert_eq!(exit, 1, "{verb}");
        assert_eq!(
            found,
            [("guard_transfer".to_owned(), drop, drop + 4)],
            "{verb}"
        );
    }
    accepted(&without, J);
}

#[test]
fn guard_transfer_inside_a_return_value_is_reported_after_the_control_exit() {
    let with = "fn G()->I64{let a=Some(1);match true{true if (if true then {return if true then {Drop(a);value:1;} else {value:2;};} else {value:false;}) =>{},_=>{}} return 0;}fn Main(){G();}";
    let without = with.replace("Drop(a);", "");
    let (ret, drop) = (at(with, "return if"), at(with, "Drop"));
    for verb in ["check", "run"] {
        let (exit, found) = guard_diagnostics(verb, with);
        assert_eq!(exit, 1, "{verb}");
        assert_eq!(
            found,
            [
                ("guard_control".to_owned(), ret, ret + 6),
                ("guard_transfer".to_owned(), drop, drop + 4)
            ],
            "{verb}"
        );
        let (exit, found) = guard_diagnostics(verb, &without);
        assert_eq!(exit, 1, "{verb}");
        assert_eq!(
            found,
            [("guard_control".to_owned(), ret, ret + 6)],
            "{verb}"
        );
    }
}

#[test]
fn guard_transfer_inside_a_yield_value_is_reported_after_the_control_exit() {
    let with = "fn Main(){let a=Some(1);match true{true if (if true then {yield if true then {Drop(a);value:1;} else {value:2;};} else {value:false;}) =>{},_=>{}}}";
    let without = with.replace("Drop(a);", "");
    let (token, drop) = (at(with, "yield"), at(with, "Drop"));
    for verb in ["check", "run"] {
        let (exit, found) = guard_diagnostics(verb, with);
        assert_eq!(exit, 1, "{verb}");
        assert_eq!(
            found,
            [
                ("guard_control".to_owned(), token, token + 5),
                ("guard_transfer".to_owned(), drop, drop + 4)
            ],
            "{verb}"
        );
        let (exit, found) = guard_diagnostics(verb, &without);
        assert_eq!(exit, 1, "{verb}");
        assert_eq!(
            found,
            [("guard_control".to_owned(), token, token + 5)],
            "{verb}"
        );
    }
}

#[test]
fn guard_transfer_inside_the_operand_of_a_postfix_try_is_reported() {
    let with = "fn Main()->Result<(),I64>{let a=Some(1);match true{true if ((if true then {Drop(a);value:Ok(1);} else {value:Ok(2);})?)==1 =>{},_=>{}}return Ok(());}";
    let without = with.replace("Drop(a);", "");
    let (drop, question) = (at(with, "Drop"), at(with, "?"));
    for verb in ["check", "run"] {
        let (exit, found) = guard_diagnostics(verb, with);
        assert_eq!(exit, 1, "{verb}");
        assert_eq!(
            found,
            [
                ("guard_transfer".to_owned(), drop, drop + 4),
                ("guard_control".to_owned(), question, question + 1)
            ],
            "{verb}"
        );
        let (exit, found) = guard_diagnostics(verb, &without);
        assert_eq!(exit, 1, "{verb}");
        let q = at(&without, "?");
        assert_eq!(found, [("guard_control".to_owned(), q, q + 1)], "{verb}");
    }
}

#[test]
fn expression_statement_tick_of_an_index_read_is_the_whole_expression() {
    let source = "fn Main(){var a=[1,2,3];while true{a[0];}}";
    let (exit, json, stderr) = invoke("run", source, J);
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(exit, 1, "{json}");
    assert!(json.contains("\"code\":\"R0001\""), "{json}");
    let start = at(source, "a[0]");
    assert_eq!(primary(&json), (start, start + 4), "{json}");
    accepted("fn Main(){var a=[1,2,3];var n=0;while n<3{n=n+1;a[0];}}", J);
}
