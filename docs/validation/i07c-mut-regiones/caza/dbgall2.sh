. /c/nexo-work/val/mut-regiones/env.sh
cd /c/nexo-work/val/mut-regiones
export CARGO_BUILD_JOBS=3
for p in s6cor:M00275 s6cor:M00268 s5b1:M00170 s5b1:M00134; do
  c=${p%%:*}; id=${p##*:}
  [ -f exes/dbg-$c-$id.exe ] || python -B dbg_build.py $c $id exes/dbg-$c-$id.exe
done
V=C:/nexo-work/val/mut-regiones
for p in s6cor-M00275 s6cor-M00268 s5b1-M00170 s5b1-M00134; do
  for d in targeted tg3 tg4 tg5 corpus-small; do
    echo "$p $d $(python -B tgrun.py $d $V/exes/dbg-base.exe $V/exes/dbg-$p.exe 3 2>&1 | tail -1)" >> tgall2.log
  done
done
echo done > dbgall2.done
