#!/bin/bash
# Exp 63 step 1, last variant: 16 regimes as a node and as a gate, seeds 0-2, test alone (Z) and after primitives (P).
cd /home/claude/evolvehd
export B=runs/exp36/sit-63b
jobs=(); for st in P Z; do for d in 0 1 2; do [ -f runs/exp61/brain_${st}_63D_seed$d/report.txt ] || jobs+=("$st $d"); done; done
printf '%s\n' "${jobs[@]}" | xargs -P 2 -I{} bash -c 'set -- {}; TAG=_63D_seed$2 bash runs/exp61/run.sh $1 --stride-act --regime 128,16 --regime-gate --seed $2'
