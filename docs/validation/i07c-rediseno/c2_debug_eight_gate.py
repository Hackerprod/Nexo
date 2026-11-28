#!/usr/bin/env python3
"""Run eight entire frozen corpora in debug with the frozen C2 classifier."""

import argparse
import collections
import hashlib
import json
from pathlib import Path

import candidate_gate as gate


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--main-exe', type=Path, required=True)
    parser.add_argument('--candidate-exe', type=Path, required=True)
    parser.add_argument('--case', type=Path, required=True)
    parser.add_argument('--previous', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    definitions = json.loads((gate.HERE / 'r7-frozen-batteries.json').read_text(encoding='utf-8'))['corpora'][:8]
    assert len(definitions) == 8
    prior = json.loads(args.previous.read_text(encoding='utf-8'))
    assert prior['complete'] and prior['commit'] == 'C1'
    assert prior['main_sha256'] == gate.sha(args.main_exe)
    assert prior['oracle_sha256'] == gate.sha(gate.HERE.parents[2] / 'scripts/i07c_rd_oracle.py')
    hard_rows = json.loads((gate.HERE / 'r14-main-hard.json').read_text(encoding='utf-8'))['rows']
    report = {
        'schema': 'i07c-c2-debug-eight-v1',
        'complete_selected_corpora': True,
        'selected_ids': [entry['id'] for entry in definitions],
        'main_sha256': gate.sha(args.main_exe),
        'candidate_sha256': gate.sha(args.candidate_exe),
        'oracle_sha256': gate.sha(gate.HERE.parents[2] / 'scripts/i07c_rd_oracle.py'),
        'classifier_sha256': gate.sha(Path(gate.__file__)),
        'runner_sha256': gate.sha(Path(__file__)),
        'previous_sha256': gate.sha(args.previous),
        'corpora': {}, 'class_by_row': {}, 'failures': [],
    }
    cache = {}
    for entry in definitions:
        ident = entry['id']
        corpus = gate.HERE / gate.FILE[ident.split('@')[0]]
        assert gate.sha(corpus) == entry['sha256'], ident
        rows = json.loads(corpus.read_text(encoding='utf-8'))
        if isinstance(rows, dict):
            rows = rows['progs']
        assert len(rows) == entry['rows'], ident
        counts = collections.Counter()
        for index, item in enumerate(rows):
            src = gate.source(item)
            key = (entry['profile'], src)
            if key not in cache:
                main = gate.execute(args.main_exe, src, entry['profile'], args.case)
                candidate = gate.execute(args.candidate_exe, src, entry['profile'], args.case)
                cache[key] = gate.classify(main, candidate, gate.oracle.derive(src))
            label = cache[key]
            row_id = f'{ident}:{index}'
            if row_id in hard_rows:
                expected = hard_rows[row_id]
                assert gate.sha(corpus) == expected['corpus_sha256']
                assert hashlib.sha256(src.encode('ascii')).hexdigest() == expected['source_sha256']
                main = gate.execute(args.main_exe, src, entry['profile'], args.case)
                candidate = gate.execute(args.candidate_exe, src, entry['profile'], args.case)
                label = 'H' if candidate == main and main['outcome'] == {'kind': 'driver_error', 'reason': 'internal_error'} else 'H-divergent'
            elif label.startswith('H'):
                report['failures'].append(f'{row_id}=unexpected-main-hard')
            report['class_by_row'][row_id] = label
            counts[label] += 1
            if label in ('A', 'A-unmodeled', 'D', 'H-divergent'):
                report['failures'].append(f'{row_id}={label}')
            if prior['class_by_row'].get(row_id) == 'B' and label == 'C':
                report['failures'].append(f'{row_id}=oracle-to-main')
        report['corpora'][ident] = {'rows': len(rows), 'profile': entry['profile'],
                                    'sha256': entry['sha256'], 'counts': dict(sorted(counts.items()))}
        print(ident, report['corpora'][ident]['counts'], flush=True)
    report['total_rows'] = sum(entry['rows'] for entry in report['corpora'].values())
    args.output.write_text(json.dumps(report, indent=2, ensure_ascii=True) + '\n', encoding='utf-8')
    print('debug eight', report['total_rows'], 'rows', len(report['failures']), 'failures')
    if report['failures']:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
