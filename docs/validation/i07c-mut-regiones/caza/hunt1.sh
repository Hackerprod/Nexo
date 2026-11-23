# usage: hunt1.sh <cut> <id>   (run under heavy-lock)
. /c/nexo-work/val/mut-regiones/env.sh; export CARGO_BUILD_JOBS=2
V=/c/nexo-work/val/mut-regiones
R=/c/projects/Nexo-i07c-f
D=$R/docs/validation/i07c-mut-regiones
HT=C:/nexo-work/target-mut-regiones-hunt
cd $R
[ -f $V/exes/base.exe ] || python -B $D/hunt_build.py $V/tree-hunt $D/population-$1.json BASE $V/exes/base.exe $HT
python -B $D/hunt_build.py $V/tree-hunt $D/population-$1.json $2 $V/exes/$1-$2.exe $HT | tee -a $V/hunt-build.log
[ -f $V/exes/$1-$2.exe ] || exit 3
for c in corpus-small corpus-s5b1-small; do
  [ $c = corpus-s5b1-small ] && [ $1 != s5b1 ] && continue
  python -B docs/validation/i07c-s6cor2/diff_run.py $V/$c $V/hunt-$1-$2-$c.jsonl 0 100000 3 all $V/exes/base.exe $V/exes/$1-$2.exe
  python - <<PY
import json
n=d=0
for l in open("C:/nexo-work/val/mut-regiones/hunt-$1-$2-$c.jsonl"):
    r=json.loads(l); n+=1
    if r["texts"]: d+=1; print("DIFF",r["name"],list(r["texts"])[:2])
print("HUNT $1 $2 $c sources",n,"with-difference",d)
PY
done
