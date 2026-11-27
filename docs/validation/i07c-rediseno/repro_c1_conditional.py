#!/usr/bin/env python3
"""Reproduce the C1 conditional counterexample from the frozen directed corpus."""

import argparse
import hashlib
import json
from pathlib import Path

from audit_families import source
from candidate_gate import execute, project_main
from verify_corpora import FILE


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--main-exe', type=Path, required=True)
    parser.add_argument('--candidate-exe', type=Path, required=True)
    parser.add_argument('--case', type=Path, required=True)
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    corpus = json.loads((here / FILE['dirigido5']).read_text(encoding='utf-8'))
    rows = corpus['progs'] if isinstance(corpus, dict) else corpus
    program = source(rows[2069])
    digest = hashlib.sha256(program.encode('ascii')).hexdigest()
    assert digest == '46a9d45f31f56be5314629233935cc704da00179b8f3a9c67a35a835174bd6ea'
    main_result = execute(args.main_exe, program, 'f0-0002-b-v2', args.case)
    candidate_result = execute(args.candidate_exe, program, 'f0-0002-b-v2', args.case)
    def primaries(result):
        return [(d[0], d[2]) for d in project_main(result)]
    expected_main = [('NX-E0103', [73, 75]), ('NX-E0103', [135, 136])]
    expected_ablation = [('NX-E0103', [73, 75]), ('NX-E0103', [129, 132])]
    assert primaries(main_result) == expected_main
    print('source_sha256', digest)
    print('main', primaries(main_result))
    print('candidate', primaries(candidate_result))
    print('full_json_equal', main_result == candidate_result)
    if primaries(candidate_result) == expected_ablation:
        print('C1 unwitnessed-constructor ablation counterexample reproduced')
    else:
        print('candidate differs from measured ablation; inspect full JSON')
    if main_result == candidate_result:
        raise SystemExit('counterexample absent for this candidate')


if __name__ == '__main__':
    main()
