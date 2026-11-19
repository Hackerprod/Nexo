#!/usr/bin/env sh
set -eu
repo=$(git rev-parse --show-toplevel)
python -B - "$repo" "$@" <<'PY'
"""Puerta completa reproducible de un commit; el verde se publica al final."""
import hashlib
import io
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import time

repo = Path(sys.argv[1]).resolve()
if len(sys.argv) not in (3, 4) or (len(sys.argv) == 4 and sys.argv[3] != '--verify'):
    raise SystemExit('uso: sh scripts/gate.sh <commit> [--verify]')
revision = sys.argv[2]


def git(*args):
    return subprocess.check_output(['git', *args], cwd=repo)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def blob_equivalent(data, expected):
    return sha(data) == expected or sha(data.replace(b'\r\n', b'\n')) == expected


commit = git('rev-parse', '--verify', revision + '^{commit}').decode().strip()
tree = git('rev-parse', commit + '^{tree}').decode().strip()
source_hashes = {}
for name in ('scripts/validate.sh', 'scripts/validate-fast.sh',
             'scripts/check-nextest-selection.py', '.config/nextest.toml',
             'scripts/gate.sh'):
    source_hashes[name] = sha(git('show', commit + ':' + name))
try:
    main_validate_sha256 = sha(subprocess.check_output(
        ['git', 'show', 'origin/main:scripts/validate.sh'], cwd=repo,
        stderr=subprocess.DEVNULL))
except subprocess.CalledProcessError:
    main_validate_sha256 = None
env = os.environ.copy()
env['CARGO_BUILD_JOBS'] = '6'
env['NO_COLOR'] = '1'
relevant = {key: sha(value.encode()) for key, value in sorted(env.items())
            if key.startswith(('NEXO_', 'J16_', 'NEXTEST_'))}
rustc = subprocess.check_output(['rustc', '--version'], text=True).strip()
try:
    nextest = subprocess.check_output(['cargo', 'nextest', '--version'],
                                      text=True, stderr=subprocess.DEVNULL).splitlines()[0]
except (FileNotFoundError, subprocess.CalledProcessError):
    nextest = None
runner = 'cargo-test' if nextest is None or env.get('NEXO_VALIDATE_RUNNER') == 'cargo-test' else 'nextest'
identity = {'commit': commit, 'tree': tree, 'source_sha256': source_hashes,
            'rustc': rustc, 'nextest': nextest, 'environment_sha256': relevant,
            'cargo_build_jobs': 6, 'runner': runner,
            'main_validate_sha256': main_validate_sha256}
root = Path('C:/nexo-work/gates') / tree
root.mkdir(parents=True, exist_ok=True)
identity_tag = sha(json.dumps(identity, sort_keys=True).encode())[:16]
record_file = root / (commit + '-' + identity_tag + '.json')
log_file = root / (commit + '-' + identity_tag + '.log')

if len(sys.argv) == 4:
    if not record_file.exists() or not log_file.exists():
        raise SystemExit('registro ausente para esta identidad de árbol, herramientas y entorno')
    record = json.loads(record_file.read_text(encoding='utf-8'))
    if record.get('status') != 'GREEN' or record.get('identity') != identity:
        raise SystemExit('registro ausente o identidad distinta')
    if sha(log_file.read_bytes()) != record.get('log_sha256'):
        raise SystemExit('SHA-256 del log distinto')
    if any(step.get('exit') != 0 for step in record.get('steps', {}).values()):
        raise SystemExit('registro con paso fallido')
    if set(record.get('counts', {})) != {'debug', 'release'}:
        raise SystemExit('recuentos por perfil ausentes')
    for counts in record['counts'].values():
        if (counts['discovered'] != counts['runnable'] + counts['ignored']
                or counts['executed'] != counts['runnable']
                or counts['passed'] != counts['executed']):
            raise SystemExit('recuentos incoherentes')
    if record.get('rd', {}).get('matched') != record.get('rd', {}).get('total'):
        raise SystemExit('RD incompleto')
    if not record.get('objective'):
        raise SystemExit('objetivo ausente')
    print(f'registro GREEN verificado: {commit} {tree}')
    raise SystemExit(0)

work = Path('C:/nexo-work/tmp')
work.mkdir(parents=True, exist_ok=True)
copy = Path(tempfile.mkdtemp(prefix='nx-gate-', dir=work))
target = copy.parent / (copy.name + '-target')
target.mkdir()
env['CARGO_TARGET_DIR'] = str(target)
run_log = copy.parent / (copy.name + '.log')
record = {'status': 'FAIL', 'identity': identity, 'steps': {}, 'counts': {}}
started = time.monotonic()


def run(label, command):
    with run_log.open('ab') as stream:
        stream.write(('\nCOMMAND ' + label + ': ' + ' '.join(command) + '\n').encode())
        stream.flush()
        step_start = time.monotonic()
        done = subprocess.run(command, cwd=copy, env=env, stdout=stream,
                              stderr=subprocess.STDOUT, check=False)
    record['steps'][label] = {'exit': done.returncode,
                              'seconds': round(time.monotonic() - step_start, 3)}
    if done.returncode:
        raise RuntimeError(f'{label} terminó con exit {done.returncode}')


def inventory(profile):
    command = ['cargo', 'nextest', 'list', '--locked', '--offline', '--profile',
               'default', '--message-format', 'json']
    if profile == 'release':
        command.append('--release')
    start = time.monotonic()
    done = subprocess.run(command, cwd=copy, env=env, capture_output=True, check=False)
    record['steps']['list-' + profile] = {'exit': done.returncode,
                                         'seconds': round(time.monotonic() - start, 3)}
    with run_log.open('ab') as stream:
        stream.write(('\nCOMMAND list-' + profile + ': ' + ' '.join(command) + '\n').encode())
        stream.write(done.stderr)
    if done.returncode:
        raise RuntimeError(f'nextest list {profile} falló')
    data = json.loads(done.stdout)
    tests = [item for suite in data['rust-suites'].values()
             for item in suite['testcases'].values()]
    if len(tests) != data['test-count']:
        raise RuntimeError('inventario nextest truncado')
    ignored = sum(bool(item['ignored']) for item in tests)
    return {'discovered': len(tests), 'ignored': ignored,
            'runnable': len(tests) - ignored}


def cargo_inventory(profile):
    counts = []
    for suffix in (['--', '--list'], ['--', '--list', '--ignored']):
        command = ['cargo', 'test', '--locked', '--offline']
        if profile == 'release':
            command.append('--release')
        command.extend(suffix)
        start = time.monotonic()
        done = subprocess.run(command, cwd=copy, env=env, capture_output=True, check=False)
        record['steps']['cargo-list-' + profile + '-' + str(len(counts))] = {
            'exit': done.returncode, 'seconds': round(time.monotonic() - start, 3)}
        with run_log.open('ab') as stream:
            stream.write(('\nCOMMAND cargo-list-' + profile + ': ' + ' '.join(command) + '\n').encode())
            stream.write(done.stderr)
        if done.returncode:
            raise RuntimeError(f'cargo list {profile} falló')
        counts.append(sum(line.endswith(': test') for line in done.stdout.decode('utf-8', 'replace').splitlines()))
    return {'discovered': counts[0], 'ignored': counts[1],
            'runnable': counts[0] - counts[1]}


try:
    archive = git('archive', commit)
    record['archive_sha256'] = sha(archive)
    with tarfile.open(fileobj=io.BytesIO(archive)) as bundle:
        bundle.extractall(copy, filter='data')
    for name, expected in source_hashes.items():
        if not blob_equivalent((copy / name).read_bytes(), expected):
            raise RuntimeError(f'copia distinta: {name}')
    run('validate', ['sh', 'scripts/validate.sh'])
    run('objetivo', ['python', '-B', 'scripts/objetivo.py'])
    run('rd', ['python', '-B', 'scripts/i07c_rd_conformidad.py', '--verify',
               str(target / 'debug' / 'nexo.exe')])
    log_text = run_log.read_text(encoding='utf-8', errors='replace')
    objective = re.search(r'Programas que ejecutan:\s*(\d+)/(\d+)', log_text)
    rd = re.search(r'RD normative:\s*(\d+)/(\d+) check/run pairs', log_text)
    if objective is None or rd is None:
        raise RuntimeError('resumen de objetivo o RD ausente')
    record['objective'] = {'executing': int(objective[1]), 'total': int(objective[2])}
    record['rd'] = {'matched': int(rd[1]), 'total': int(rd[2])}
    if record['rd']['matched'] != record['rd']['total']:
        raise RuntimeError('RD incompleto')
    if runner == 'nextest':
        summaries = re.findall(r'Summary\s+\[[^\]]+\]\s+(\d+) tests? run: (\d+) passed[^\n]*?(\d+) skipped', log_text)
        if len(summaries) != 2:
            raise RuntimeError(f'resumen nextest ausente o truncado: {len(summaries)}')
        for profile, summary in zip(('debug', 'release'), summaries):
            counts = inventory(profile)
            ran, passed, skipped = map(int, summary)
            if (ran, passed, skipped) != (counts['runnable'], counts['runnable'], counts['ignored']):
                raise RuntimeError(f'recuento distinto en {profile}: {summary} != {counts}')
            record['counts'][profile] = {**counts, 'executed': ran, 'passed': passed}
    else:
        boundary = log_text.find('Finished `release` profile')
        if boundary < 0:
            raise RuntimeError('resumen cargo test release ausente')
        for profile, section in (('debug', log_text[:boundary]), ('release', log_text[boundary:])):
            summaries = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', section)
            if not summaries:
                raise RuntimeError(f'resumen cargo test {profile} ausente')
            ran = sum(int(row[0]) for row in summaries)
            failed = sum(int(row[1]) for row in summaries)
            ignored = sum(int(row[2]) for row in summaries)
            counts = cargo_inventory(profile)
            if failed or (ran, ignored) != (counts['runnable'], counts['ignored']):
                raise RuntimeError(f'recuento distinto en {profile}: {(ran, failed, ignored)} != {counts}')
            record['counts'][profile] = {**counts, 'executed': ran, 'passed': ran}
    record['status'] = 'GREEN'
except Exception as error:
    record['error'] = str(error)
finally:
    record['seconds'] = round(time.monotonic() - started, 3)
    record['log_sha256'] = sha(run_log.read_bytes()) if run_log.exists() else None
    os.replace(run_log, log_file) if run_log.exists() else None
    temporary = record_file.with_suffix('.json.tmp')
    temporary.write_text(json.dumps(record, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    os.replace(temporary, record_file)
    if copy.resolve().is_relative_to(work.resolve()):
        shutil.rmtree(copy)
    if target.resolve().is_relative_to(work.resolve()):
        shutil.rmtree(target)

print(f"{record['status']}: {commit} {tree}; registro {record_file}")
if record['status'] != 'GREEN':
    raise SystemExit(record.get('error', 'puerta fallida'))
PY
