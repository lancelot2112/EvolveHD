#!/bin/bash
# Exp 53: the merge stack with the learning schedule (seed 6: full rate for a feature's first 64 updates, then 1 in 2,
# then 1 in 4 for ever) and nothing frozen. Full qa2, long BABILong, and the small sets, seeds 0-2.
cd /home/claude/evolvehd
J="--sit-bnd-unit --sit-bnd-unit-len --sit-act --sit-rel-merge --sit-learn-sched 6,2"
export FREEZE=999999999
(for s in 0 1 2; do SEED=$s T=x53_s$s G="$J" bash runs/exp53/a.sh full; done) &
for s in 0 1 2; do SEED=$s T=x53_s$s G="$J" bash runs/exp53/a.sh long; done
for s in 0 1 2; do SEED=$s T=x53_s$s G="$J" bash runs/exp53/a.sh qs story pilot; done
wait
