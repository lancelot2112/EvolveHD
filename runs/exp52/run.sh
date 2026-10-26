#!/bin/bash
# Exp 52: the merge stack (exp 50) on full qa2, seeds 0-2: router learning after the freeze at 1 word in 4 and 1 in 16,
# and fully on for seed 1 (seeds 0 and 2 fully on are in exp 51).
cd /home/claude/evolvehd
J="--sit-bnd-unit --sit-bnd-unit-len --sit-act --sit-rel-merge"
(for s in 0 1 2; do SEED=$s T=x52r4_s$s G="$J --sit-slow-after 4" bash runs/exp52/a.sh full; done; SEED=1 FREEZE=99999999 T=x52live_s1 G="$J" bash runs/exp52/a.sh full) &
for s in 0 1 2; do SEED=$s T=x52r16_s$s G="$J --sit-slow-after 16" bash runs/exp52/a.sh full; done
wait
