import hashlib
import json
import subprocess
from pathlib import Path

root = Path(__file__).resolve().parents[1]
path = root / 'src/interp/machine.rs'
cases = (
    ('generator_header_charge', 'build_generator', '|| !self.charge_work(1, &ins.span)', '|| !self.charge_work(0, &ins.span)'),
    ('advance_entry_charge', 'resume_generator', 'if !self.charge_work(1, &ins.span)', 'if !self.charge_work(0, &ins.span)'),
    ('closure_header_charge', 'build_closure', 'if !self.charge_work(1, &ins.span) || !self.cell(&ins.span)', 'if !self.charge_work(0, &ins.span) || !self.cell(&ins.span)'),
)
records = []
for label, function, before, after in cases:
    original = path.read_bytes()
    text = original.decode('utf-8')
    start = text.index(f'    fn {function}(')
    end = text.index('    fn ', start + 1)
    body = text[start:end]
    assert body.count(before) == 1, label
    mutated = (text[:start] + body.replace(before, after, 1) + text[end:]).encode('utf-8')
    try:
        path.write_bytes(mutated)
        run = subprocess.run(['cargo', 'test', '--locked', '--offline', '--test', 'j14_accounting', 'published_new_operation_work_totals', '--', '--exact'], cwd=root, capture_output=True, text=True, timeout=120)
        status = 'killed' if run.returncode != 0 and 'FAILED' in run.stdout and 'error[' not in run.stderr else 'invalid_or_survived'
    finally:
        path.write_bytes(original)
    restored = path.read_bytes()
    assert restored == original
    records.append({'name': label, 'site': 'src/interp/machine.rs', 'function': function,
                    'before': before, 'after': after,
                    'fixture': 'tests/j14_accounting.rs::published_new_operation_work_totals',
                    'status': status, 'original_sha256': hashlib.sha256(original).hexdigest(),
                    'mutant_sha256': hashlib.sha256(mutated).hexdigest(),
                    'restored_sha256': hashlib.sha256(restored).hexdigest()})
    print(label, status, flush=True)
(root / 'docs/validation/F0-0003-J14-stack-mutations.json').write_text(json.dumps(records, indent=2) + '\n', encoding='utf-8')
raise SystemExit(any(record['status'] != 'killed' for record in records))
