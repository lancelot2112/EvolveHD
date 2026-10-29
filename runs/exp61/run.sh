#!/bin/bash
# Exp 61: pretraining on primitives (P) or plain text of the same length (C), then experiment 57's byte stream.
# usage: run.sh TAG [extra flags]   (TAG P or C; the stream runs/exp61/TAG.bin)
cd /home/claude/evolvehd
S=$1; shift
EVOLVEHD_PROGRESS=200000 ${B:-runs/exp36/sit-61a} stack --data runs/exp61/$S.bin --levels 1 --words --run-window 200000 --conjoin-words --long-trials 5 --grow-pools --pool-budget 524288 --readout3 --screen-mix --fresh-extensions --floor --prune-guard --map-two 127 --copy-act --file-starts runs/exp61/$S.starts "$@" --out-dir runs/exp61/brain_$S${TAG:-} > runs/exp61/brain_$S${TAG:-}.out 2>&1
