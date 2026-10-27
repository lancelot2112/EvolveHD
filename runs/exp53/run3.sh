#!/bin/bash
# Exp 53, final stack: nothing frozen, the route learner at full rate for ever, the hysteresis act gate (opens by
# majority, closes only by contrast), relation merging. Full qa2 and long BABILong, seeds 0-2.
cd /home/claude/evolvehd
export FREEZE=999999999 B=runs/exp36/sit-53c
J="--sit-bnd-unit --sit-bnd-unit-len --sit-act --sit-act-hyst --sit-rel-merge"
(for s in 0 1 2; do SEED=$s T=x53h_s$s G="$J" bash runs/exp53/a.sh full; done) &
for s in 0 1 2; do SEED=$s T=x53h_s$s G="$J" bash runs/exp53/a.sh long; done
for s in 0 1 2; do SEED=$s T=x53h_s$s G="$J" bash runs/exp53/a.sh qs story; done
wait
