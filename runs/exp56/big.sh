#!/bin/bash
# Exp 56: the instant-kinds stack on the big benchmarks (full qa2, long BABILong, pilot), three seeds, never frozen.
cd /home/claude/evolvehd
export FREEZE=999999999 B=runs/exp36/sit-56a
J="--sit-bnd-unit --sit-bnd-unit-len --sit-act --sit-act-hyst --sit-rel-merge --sit-kind-instant --sit-kind-instant-move"
(for s in 0 1 2; do SEED=$s T=x56k_s$s G="$J" bash runs/exp56/a.sh full; done) &
for s in 0 1 2; do SEED=$s T=x56k_s$s G="$J" bash runs/exp56/a.sh long pilot; done
wait
