#!/usr/bin/env python3
"""Coordina trabajo pesado entre carriles mediante creación exclusiva en C:."""

import argparse
import json
import os
from pathlib import Path
import secrets
import subprocess
import sys
from datetime import datetime, timezone


LOCK = Path('C:/nexo-work/heavy.lock')


def owner():
    try:
        return json.loads(LOCK.read_text(encoding='utf-8'))
    except FileNotFoundError:
        return None
    except (OSError, json.JSONDecodeError):
        return {'error': 'cerrojo ilegible; coordinar con el otro carril'}


def acquire(session):
    LOCK.parent.mkdir(parents=True, exist_ok=True)
    token = secrets.token_hex(16)
    data = {'pid': os.getpid(), 'time_utc': datetime.now(timezone.utc).isoformat(),
            'session': session, 'token': token}
    try:
        descriptor = os.open(LOCK, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    except FileExistsError:
        raise RuntimeError('trabajo pesado ocupado: ' + json.dumps(owner(), ensure_ascii=False))
    with os.fdopen(descriptor, 'w', encoding='utf-8') as stream:
        json.dump(data, stream, ensure_ascii=False)
        stream.write('\n')
    return token


def release(token):
    current = owner()
    if not current or current.get('token') != token or current.get('pid') != os.getpid():
        raise RuntimeError('no se libera un cerrojo de otro dueño')
    LOCK.unlink()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='action', required=True)
    sub.add_parser('status')
    run = sub.add_parser('run')
    run.add_argument('--session', required=True)
    run.add_argument('command', nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.action == 'status':
        print(json.dumps(owner(), ensure_ascii=False))
        return
    command = args.command[1:] if args.command and args.command[0] == '--' else args.command
    if not command:
        parser.error('falta comando tras --')
    try:
        token = acquire(args.session)
    except RuntimeError as error:
        print(error, file=sys.stderr)
        raise SystemExit(75)
    try:
        env = os.environ.copy()
        env['CARGO_BUILD_JOBS'] = '6'
        env['NEXO_HEAVY_LOCK_HELD'] = '1'
        process = subprocess.Popen(command, env=env)
        try:
            code = process.wait()
        except KeyboardInterrupt:
            if os.name == 'nt':
                subprocess.run(['taskkill', '/PID', str(process.pid), '/T', '/F'],
                               capture_output=True, check=False)
            else:
                process.terminate()
            process.wait()
            raise
        raise SystemExit(code)
    finally:
        release(token)


if __name__ == '__main__':
    main()
