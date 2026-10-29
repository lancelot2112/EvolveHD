#!/bin/bash
# Exp 63 step 1: steadier regimes. With the stride act: A = 4 regimes as a node; B = 16 regimes as a gate on edges
# (no node); C = 4 regimes as a gate. Seeds 0-2, test stream alone (Z) and after primitives (P). Binary sit-63a.
cd /home/claude/evolvehd
export B=runs/exp36/sit-63a
jobs=(); for v in A B C; do for st in Z P; do for d in 0 1 2; do [ -f runs/exp61/brain_${st}_63${v}_seed$d/report.txt ] || jobs+=("$st $v $d"); done; done; done
printf '%s\n' "${jobs[@]}" | xargs -P 2 -I{} bash -c '
set -- {}; s=$1; v=$2; d=$3
case $v in A) X="--regime 128,4";; B) X="--regime 128,16 --regime-no-node --regime-gate";; C) X="--regime 128,4 --regime-no-node --regime-gate";; esac
TAG=_63${v}_seed$d bash runs/exp61/run.sh $s --stride-act $X --seed $d'
