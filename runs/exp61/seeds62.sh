#!/bin/bash
# Exp 62: the regime node against the RNG confound: stride act with and without the regime node, seeds 1 and 2
# (seed 0 = the runs brain_{Z,P}_{s,sr}), on the test stream alone (Z) and after primitives (P). Binary sit-62b.
cd /home/claude/evolvehd
export B=runs/exp36/sit-62b
printf '%s\n' "Z s 1" "Z s 2" "Z sr 1" "Z sr 2" "P s 1" "P sr 1" "P s 2" "P sr 2" | xargs -P 2 -I{} bash -c '
set -- {}; s=$1; v=$2; d=$3
case $v in s) X="--stride-act";; sr) X="--stride-act --regime 128,16";; esac
TAG=_${v}_seed$d bash runs/exp61/run.sh $s $X --seed $d'
