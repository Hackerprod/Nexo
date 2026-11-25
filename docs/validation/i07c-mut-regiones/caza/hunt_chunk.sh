. /c/nexo-work/val/mut-regiones/env.sh
export CARGO_BUILD_JOBS=3
cd /c/nexo-work/val/mut-regiones
V=C:/nexo-work/val/mut-regiones
start=$(date +%s)
[ -f exes/dbg-base.exe ] || python -B dbg_build.py s6cor BASE exes/dbg-base.exe
for p in $(cat queue.txt); do
  c=${p%%:*}; id=${p##*:}
  [ -f hunt-done-$c-$id ] && continue
  [ $(( $(date +%s) - start )) -gt 840 ] && exit 0
  python -B dbg_build.py $c $id exes/dbg-$c-$id.exe > /dev/null 2>&1 || { echo "$c $id build-fail" >> hunt.log; touch hunt-done-$c-$id; continue; }
  for d in targeted tg3 tg4 tg5 corpus-small; do
    echo "$c-$id $d $(python -B tgrun.py $d $V/exes/dbg-base.exe $V/exes/dbg-$c-$id.exe 3 2>&1 | tail -1)" >> hunt.log
  done
  rm -f exes/dbg-$c-$id.exe
  touch hunt-done-$c-$id
done
echo ALL > hunt.done
