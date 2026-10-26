#!/bin/bash
# Exp 53, second part: the contrast act gate (--sit-act-contrast) with the schedule and nothing frozen, on long
# BABILong (seeds 0-2), then the small sets (seeds 1-2), then full qa2 (seeds 0-2).
cd /home/claude/evolvehd
export FREEZE=999999999 B=runs/exp36/sit-53b
J="--sit-bnd-unit --sit-bnd-unit-len --sit-act --sit-act-contrast --sit-rel-merge --sit-learn-sched 6,2"
for s in 0 1 2; do SEED=$s T=x53c_s$s G="$J" bash runs/exp53/a.sh long; done
for s in 1 2; do SEED=$s T=x53c_s$s G="$J" bash runs/exp53/a.sh pilot qs story; done
for s in 0 1 2; do SEED=$s T=x53c_s$s G="$J" bash runs/exp53/a.sh full; done
