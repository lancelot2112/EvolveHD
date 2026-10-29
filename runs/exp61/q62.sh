#!/bin/bash
# Exp 62 queue: stride act (s), regime sense (r) and both (sr) on the test stream alone (Z), after primitives (P)
# and after English (C2); and the formats stream with the position sense, base and both. Binary sit-62a.
cd /home/claude/evolvehd
export B=runs/exp36/sit-62a
fm() { t=$1; shift; EVOLVEHD_PROGRESS=100000 $B stack --data runs/exp60/fmt2.bin --levels 1 --words --run-window 200000 --conjoin-words --long-trials 5 --grow-pools --pool-budget 524288 --readout3 --screen-mix --fresh-extensions --floor --prune-guard --map-two 127 --copy-act --file-starts runs/exp60/fmt2.starts "$@" --out-dir runs/exp61/fmt_$t > runs/exp61/fmt_$t.out 2>&1; }
export -f fm
printf '%s\n' "Z s" "Z r" "P sr" "C2 sr" "P s" "P r" "C2 s" "C2 r" "F base" "F sr" | xargs -P 2 -I{} bash -c '
set -- {}; s=$1; v=$2
case $v in s) X="--stride-act";; r) X="--regime 128,16";; sr) X="--stride-act --regime 128,16";; base) X="";; esac
if [ $s = F ]; then fm $v $X; else TAG=_$v bash runs/exp61/run.sh $s $X; fi'
