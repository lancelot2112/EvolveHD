#!/bin/bash
# Exp 61, clean reruns with sit-61b: the English control without the test books (C2), and the formats stream with
# and without the position sense.
cd /home/claude/evolvehd
export B=runs/exp36/sit-61b
fm() { EVOLVEHD_PROGRESS=100000 $B stack --data runs/exp60/fmt2.bin --levels 1 --words --run-window 200000 --conjoin-words --long-trials 5 --grow-pools --pool-budget 524288 --readout3 --screen-mix --fresh-extensions --floor --prune-guard --map-two 127 --copy-act "$@"; }
(TAG=_b bash runs/exp61/run.sh C2) &
(fm --out-dir runs/exp61/fmt_nopos > runs/exp61/fmt_nopos.out 2>&1; fm --file-starts runs/exp60/fmt2.starts --out-dir runs/exp61/fmt_pos > runs/exp61/fmt_pos.out 2>&1) &
wait
