#!/usr/bin/env python3
"""Generate distinct one-line programs for causal C/D/V adjudication."""

import json
import os
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
DV = "(if true then {return;} else {return;})"
GENERATED_SEED = 23_017
GENERATED_COUNT = 50_000

# The original directed templates isolate C sites. The expanded differential
# families also contain consumers of failed expressions, so differential
# diagnostics from those families need §4.1.2.5 C/D/V adjudication.
TEMPLATES = [
    ("union_without_context", "fn Main(){let x=(if true then $N else true);}"),
    ("union_with_context", "fn Main(){let x:I64=(if true then true else $N);}"),
    ("array_element", "fn Main(){let a=[$N,true];}"),
    ("nested_array_element", "fn Main(){let a=[[$N],[true]];}"),
    ("add_right", "fn Main(){let a=$N+true;}"),
    ("subtract_left", "fn Main(){let a=true-$N;}"),
    ("less_right", "fn Main(){let a=$N<true;}"),
    ("less_left", "fn Main(){let a=true<$N;}"),
    ("equal_right", "fn Main(){let a=$N==true;}"),
    ("assert_equality", "fn Main(){AssertEq($N,true);}"),
    ("two_comparison_operands", "fn Main(){AssertEq($N<true,$N<true);}"),
    ("index_selector", "fn Main(){let ar=[$N];let x=ar[true];}"),
    ("len_domain", "fn Main(){let x=Len(true);let nonce=$N;}"),
    ("deferred_add", "fn Main(){var q=$DV;let a=q+true;q=$N;}"),
    ("deferred_add_and_assignment", "fn Main(){var q=$DV;let a=true+q;q=true;let nonce=$N;}"),
    ("deferred_selector_union", "fn Main(){var q=$DV;let a=q[(if true then $N else true)];q=[$N];}"),
    ("deferred_array_join", "fn Main(){var q=$DV;let a=(if true then q else [$N]);q=[true];}"),
    ("block_value_union", "fn Main(){let a=(if true then {value:[$N];} else {value:[true];});}"),
    ("array_external_context", "fn Main(){let x:Array<I64>=[true,false,$N];}"),
    ("resolved_empty_array", "fn Main(){var q=[];q=[$N];}"),
]
TARGETED = [
    ("comparison_leaves", "fn Main(){AssertEq(1<true,2<true);let nonce=$N;}"),
    ("branch_selector", "fn Main(){let v=(if true then [false] else [true]);let w=((if 18 then 29 else 1)+v[0]);let nonce=$N;}"),
    ("two_constructors", "fn Main(){var a=[];AssertEq([[]],a);AssertEq(a,true);var b=[];AssertEq(b,true);let nonce=$N;}"),
    ("constructor_and_join", "fn F1(){var v=[];let w=(if true then [5] else (if true then true else 3));let nonce=$N;}fn Main(){}"),
    ("assignment_constructor", "fn Main(){var q1=$DV;var q2=[[]];var q3=$DV;AssertEq([true],q1);q1=q2;let nonce=$N;}"),
    ("indexed_assignment", "fn Main(){var w8=[true];var w10=w8;var w11=$DV;w11[0]=[1,2];w8=[w10];AssertEq([w8],[w11,[[true]]]);let nonce=$N;}"),
    ("nested_constructor", "fn Main(){var q0=[];var q1=[[]];let z=(if true then q1 else q0);AssertEq([q0],[q0,true]);let nonce=$N;}"),
    ("array_equality_code", "fn Main(){var w=$DV;let a0=[w,[1]==w];let nonce=$N;}"),
    ("len_and_constructor", "fn Main(){var w=$DV;AssertEq((if true then Len(true) else [w]),[]);w=[[3]];let nonce=$N;}"),
    ("later_array_assignment", "fn Main(){let ar=[1];var q=$DV;Print([q[0],[ar]]);let a1:Array<I64>=q;q=[true];let nonce=$N;}"),
    ("failed_condition_keeps_constructor", "fn Main(){var q=[];q=(if [$N] then {value:$N;} else $N);}"),
    ("whole_deferred_array_operand", "fn Main(){let ar=[$N];let aar=[[$N]];var r=[];let a=[r,ar,aar];r=[$N];}"),
    ("nested_pending_projection_types", "fn Main(){let ar=[$N];var r=[];let a=ar==r[r[1]];r=[$N];}"),
    ("independent_numeric_rhs_after_array", "fn Main(){var r=[];let a=[1,r]+[$N];r=[$N];}"),
    ("less_than_with_contaminated_sibling", "fn Main(){var q=$DV;let y=(true+$N)<q;q=true;}"),
    ("less_than_in_pending_selector", "fn Main(){var q=$DV;let a=q[q<q];q=[$N];}"),
]


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: i07c_normative_corpus.py OUTPUT")
    path = Path(sys.argv[1])
    path.parent.mkdir(parents=True, exist_ok=True)
    count = 0
    expanded = json.loads((Path(__file__).with_name("i07c_normative_families.json")).read_text(encoding="utf-8"))
    assert len(expanded) == 74
    with path.open("w", encoding="utf-8", newline="\n") as output:
        for _, template in TEMPLATES + TARGETED:
            limit = 3_000 if (_, template) in TEMPLATES else 4_000
            for number in range(1, limit + 1):
                output.write(template.replace("$DV", DV).replace("$N", str(number)) + "\n")
                count += 1
        for index, family in enumerate(expanded, 1):
            source = family["source"]
            assert source.count("fn Main(){") == 1
            if "$N" not in source:
                source = source.replace("fn Main(){", f"fn Main(){{let census{index}=$N;", 1)
            for number in range(1, 4_501):
                output.write(source.replace("$DV", DV).replace("$N", str(number)) + "\n")
                count += 1
        target = Path(os.environ.get("CARGO_TARGET_DIR", Path(__file__).resolve().parents[1] / "target"))
        manifest = Path(__file__).with_name("i07c_corpus_generator") / "Cargo.toml"
        env = os.environ.copy()
        env["CARGO_TARGET_DIR"] = str(target)
        subprocess.run(["cargo", "build", "--offline", "--locked", "--manifest-path", str(manifest)],
                       check=True, env=env, stdout=subprocess.DEVNULL)
        binary = target / "debug" / ("i07c-corpus-generator.exe" if os.name == "nt" else "i07c-corpus-generator")
        generated = subprocess.run([str(binary), str(GENERATED_SEED), str(GENERATED_COUNT)],
                                   check=True, capture_output=True, text=True, encoding="utf-8").stdout
        lines = generated.splitlines()
        assert len(lines) == GENERATED_COUNT and all("\n" not in line for line in lines)
        output.write("\n".join(lines) + "\n")
        count += len(lines)
    assert count == 124_000 + 4_500 * len(expanded) + GENERATED_COUNT
    print(f"programs={count} templates={len(TEMPLATES) + len(TARGETED) + len(expanded)}")


if __name__ == "__main__":
    main()
