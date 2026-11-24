cd /c/nexo-work/val/mut-regiones
V=C:/nexo-work/val/mut-regiones
for p in s6cor-M00380 s6cor-M00216 s6cor-M00164 s5b1-M00062 s5b1-M00156 s5b1-M00043 s5b1-M00380 s5b1-M00216 s5b1-M00044 s5b1-M00143 s5b1-M00022 s5b1-M00273; do
  for d in targeted tg3 corpus-small; do
    echo "$p $d $(python -B tgrun.py $d $V/exes/dbg-base.exe $V/exes/dbg-$p.exe 3 2>&1 | tail -1)" >> tgall.log
  done
done
echo done > tgall.done
