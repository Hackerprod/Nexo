#!/usr/bin/env python3
"""Coordina trabajo pesado entre carriles mediante creación exclusiva en C:."""

import argparse
import contextlib
import ctypes
import json
import os
from pathlib import Path
import secrets
import subprocess
import sys
from datetime import datetime, timezone
import time


DEFAULT_LOCK = 'C:/nexo-work/heavy.lock'
RECLAIM_MIN_AGE = 600
RECLAIM_LOG = 'heavy-lock-reclaim.log'


def lock_path():
    return Path(os.environ.get('NEXO_HEAVY_LOCK_PATH') or DEFAULT_LOCK)


def describe(data):
    """Añade pid_exists, age_seconds y reclaimable a los datos de un cerrojo."""
    pid = data.get('pid')
    if isinstance(pid, int) and pid > 0:
        try:
            if os.name == 'nt':
                kernel = ctypes.windll.kernel32
                handle = kernel.OpenProcess(0x1000, False, pid)
                if not handle:
                    data['pid_exists'] = False
                else:
                    exit_code = ctypes.c_ulong()
                    queried = kernel.GetExitCodeProcess(handle, ctypes.byref(exit_code))
                    kernel.CloseHandle(handle)
                    data['pid_exists'] = exit_code.value == 259 if queried else None
            else:
                os.kill(pid, 0)
                data['pid_exists'] = True
        except ProcessLookupError:
            data['pid_exists'] = False
        except (PermissionError, OSError):
            data['pid_exists'] = None
    try:
        data['age_seconds'] = max(0, round(time.time() - datetime.fromisoformat(data['time_utc']).timestamp()))
    except (KeyError, ValueError, TypeError):
        data['age_seconds'] = None
    # Solo con proceso dueño verificado muerto y edad conocida suficiente.
    age = data['age_seconds']
    data['reclaimable'] = (data.get('pid_exists') is False and isinstance(age, int)
                           and age >= RECLAIM_MIN_AGE)
    return data


def owner():
    try:
        return describe(json.loads(lock_path().read_text(encoding='utf-8')))
    except FileNotFoundError:
        return None
    except (OSError, json.JSONDecodeError, AttributeError):
        return {'error': 'cerrojo ilegible; coordinar con el otro carril'}


@contextlib.contextmanager
def reclaim_mutex(path):
    """Bloqueo del SO no bloqueante sobre `<cerrojo>.reclaim`; None si otro lo tiene.

    El SO lo libera al morir el proceso, así que el mutex nunca queda huérfano.
    """
    descriptor = os.open(path.with_name(path.name + '.reclaim'), os.O_RDWR | os.O_CREAT, 0o600)
    taken = False
    try:
        try:
            if os.name == 'nt':
                import msvcrt
                os.lseek(descriptor, 0, os.SEEK_SET)
                msvcrt.locking(descriptor, msvcrt.LK_NBLCK, 1)
            else:
                import fcntl
                fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
            taken = True
        except OSError:
            pass
        yield taken
    finally:
        if taken:
            try:
                if os.name == 'nt':
                    import msvcrt
                    os.lseek(descriptor, 0, os.SEEK_SET)
                    msvcrt.locking(descriptor, msvcrt.LK_UNLCK, 1)
            except OSError:
                pass
        os.close(descriptor)


def create_lock(path, data):
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, 'w', encoding='utf-8') as stream:
        json.dump(data, stream, ensure_ascii=False)
        stream.write('\n')


def busy():
    return RuntimeError('trabajo pesado ocupado: ' + json.dumps(owner(), ensure_ascii=False))


def reclaim_and_create(path, data, before_delete=None):
    """Reclama un huérfano y crea el cerrojo, todo bajo el mutex de reclamo.

    Toda la secuencia leer, verificar, borrar y crear está serializada: quien no
    obtiene el mutex devuelve ocupado sin tocar nada, y quien lo obtiene relee el
    cerrojo dentro del mutex, de modo que nunca borra un cerrojo vivo y nuevo.
    before_delete es un gancho de prueba.
    """
    with reclaim_mutex(path) as taken:
        if not taken:
            raise busy()
        seen = owner()
        if not seen or not seen.get('reclaimable') or not isinstance(seen.get('token'), str):
            raise busy()
        if before_delete is not None:
            before_delete()
        again = owner()
        if not again or again.get('token') != seen['token'] or not again.get('reclaimable'):
            raise busy()
        record = (f"{datetime.now(timezone.utc).isoformat()} reclama sesion={data['session']} "
                  f"pid={os.getpid()} huerfano: sesion={seen.get('session')} pid={seen.get('pid')} "
                  f"token={seen['token']} edad_s={seen['age_seconds']} pid_exists=False\n")
        # En Windows el borrado falla mientras otro proceso lee el archivo: se reintenta
        # y la línea de registro se escribe solo si el borrado ocurrió de verdad.
        deadline = time.time() + 3
        while True:
            try:
                path.unlink()
                break
            except FileNotFoundError:
                raise busy()
            except OSError:
                if time.time() > deadline:
                    raise busy()
                time.sleep(0.02)
        try:
            with open(path.parent / RECLAIM_LOG, 'a', encoding='utf-8') as log:
                log.write(record)
        except OSError as error:
            print(f'no se pudo registrar el reclamo: {error}', file=sys.stderr)
        try:
            create_lock(path, data)
        except OSError:
            raise busy()


def acquire(session, before_delete=None):
    path = lock_path()
    path.parent.mkdir(parents=True, exist_ok=True)
    token = secrets.token_hex(16)
    data = {'pid': os.getpid(), 'time_utc': datetime.now(timezone.utc).isoformat(),
            'session': session, 'token': token}
    try:
        create_lock(path, data)
    except (FileExistsError, PermissionError):
        reclaim_and_create(path, data, before_delete)
    return token


def release(token):
    current = owner()
    if not current or current.get('token') != token or current.get('pid') != os.getpid():
        raise RuntimeError('no se libera un cerrojo de otro dueño')
    lock_path().unlink()


def concurrent_rounds(here, dead_pid, rounds=200, procs=4, batch=25):
    """Rondas de `procs` reclamantes simultáneos sobre un huérfano; rondas en lotes paralelos."""
    import tempfile
    hold = 'import time; time.sleep(2)'
    with tempfile.TemporaryDirectory() as directory:
        for start in range(0, rounds, batch):
            running = []
            for number in range(start, min(start + batch, rounds)):
                path = Path(directory) / f'r{number}' / 'heavy.lock'
                path.parent.mkdir()
                when = datetime.fromtimestamp(time.time() - 900, timezone.utc).isoformat()
                path.write_text(json.dumps({'pid': dead_pid, 'time_utc': when, 'session': 'huerfano',
                                            'token': 'viejo'}) + '\n', encoding='utf-8')
                env = dict(os.environ, NEXO_HEAVY_LOCK_PATH=str(path))
                children = [subprocess.Popen([sys.executable, '-B', here, 'run', '--session',
                                              f'c{index}', '--', sys.executable, '-c', hold],
                                             env=env, stderr=subprocess.PIPE, text=True)
                            for index in range(procs)]
                running.append((path, children))
            for path, children in running:
                results = []
                for child in children:
                    _, error = child.communicate()
                    results.append((child.returncode, error))
                codes = sorted(code for code, _ in results)
                assert codes == [0] + [75] * (procs - 1), (path, results)
                assert not path.exists(), (path, 'el ganador no liberó el cerrojo')
                lines = (path.parent / RECLAIM_LOG).read_text(encoding='utf-8').splitlines()
                assert len(lines) == 1 and 'token=viejo' in lines[0], (path, lines)


def selftest():
    import tempfile
    here = os.path.abspath(__file__)
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / 'heavy.lock'
        log = path.parent / RECLAIM_LOG
        previous = os.environ.get('NEXO_HEAVY_LOCK_PATH')
        os.environ['NEXO_HEAVY_LOCK_PATH'] = str(path)

        def plant(pid, age, token='viejo'):
            when = datetime.fromtimestamp(time.time() - age, timezone.utc).isoformat()
            path.write_text(json.dumps({'pid': pid, 'time_utc': when, 'session': 'prueba',
                                        'token': token}) + '\n', encoding='utf-8')

        def clean():
            for item in (path, log):
                if item.exists():
                    item.unlink()

        def refused(**kwargs):
            try:
                acquire('nueva', **kwargs)
            except RuntimeError:
                return True
            return False

        def cli(*args):
            return subprocess.run([sys.executable, '-B', here, *args], capture_output=True, text=True)

        try:
            probe = subprocess.Popen([sys.executable, '-c', 'pass'])
            probe.wait()
            dead_pid = probe.pid
            assert describe({'pid': dead_pid})['pid_exists'] is False, 'pid muerto no verificado'
            # 1. pid muerto y edad > 600 s: se reclama y queda registrado.
            plant(dead_pid, 700)
            assert owner()['reclaimable'] is True
            token = acquire('nueva')
            assert json.loads(path.read_text())['token'] == token
            text = log.read_text(encoding='utf-8')
            assert ('sesion=prueba' in text and f'pid={dead_pid}' in text and 'token=viejo' in text
                    and 'edad_s=' in text and 'reclama sesion=nueva' in text), text
            release(token)
            clean()
            # 2. pid muerto pero edad < 600 s: no se reclama.
            plant(dead_pid, 100)
            assert owner()['reclaimable'] is False
            assert refused(), 'reclamó un cerrojo joven'
            assert not log.exists() and json.loads(path.read_text())['token'] == 'viejo'
            clean()
            # 3. pid vivo (el propio) aunque viejo: no se reclama.
            plant(os.getpid(), 5000)
            assert owner()['pid_exists'] is True and owner()['reclaimable'] is False
            assert refused(), 'reclamó un cerrojo con dueño vivo'
            assert not log.exists()
            clean()
            # 3b. dueño desconocido (sin pid_exists): no se reclama.
            data = describe({'pid': 0, 'time_utc': '2000-01-01T00:00:00+00:00'})
            assert data.get('pid_exists') is None and data['reclaimable'] is False
            # 4. token cambiado entre lectura y borrado: no se reclama.
            plant(dead_pid, 700)
            assert refused(before_delete=lambda: plant(dead_pid, 700, token='otro')), \
                'reclamó con el token cambiado'
            assert json.loads(path.read_text())['token'] == 'otro' and not log.exists()
            clean()
            # 4b. reclamo concurrente: un ganador, el resto ocupado, una línea de registro.
            concurrent_rounds(here, dead_pid)
            # 5. flujo normal: run propaga el exit del hijo y libera; ocupado da 75.
            result = cli('run', '--session', 'st', '--', sys.executable, '-c', 'raise SystemExit(7)')
            assert result.returncode == 7 and not path.exists(), (result.returncode, result.stderr)
            result = cli('run', '--session', 'st', '--', sys.executable, '-c', 'pass')
            assert result.returncode == 0 and not path.exists()
            plant(os.getpid(), 5)
            result = cli('run', '--session', 'st', '--', sys.executable, '-c', 'pass')
            assert result.returncode == 75, result.returncode
            status = json.loads(cli('status').stdout)
            assert status['reclaimable'] is False and status['pid_exists'] is True
            clean()
            assert cli('status').stdout.strip() == 'null'
        finally:
            if previous is None:
                os.environ.pop('NEXO_HEAVY_LOCK_PATH', None)
            else:
                os.environ['NEXO_HEAVY_LOCK_PATH'] = previous
    print('heavy-lock selftest: reclamo huerfano, no reclamo (joven, vivo, desconocido, token), '
          'run/release y 200 rondas concurrentes ok')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='action', required=True)
    sub.add_parser('status')
    sub.add_parser('selftest')
    run = sub.add_parser('run')
    run.add_argument('--session', required=True)
    run.add_argument('command', nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.action == 'selftest':
        selftest()
        return
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
    code = 1
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
    finally:
        try:
            release(token)
        except (OSError, RuntimeError) as error:
            print(f'no se pudo liberar el cerrojo: {error}', file=sys.stderr)
            if code == 0:
                code = 1
    raise SystemExit(code)


if __name__ == '__main__':
    main()
