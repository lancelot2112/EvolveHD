#!/bin/bash
# Exp 54: the route learner by surprise at answers (--sit-learn-surprise 6,2), nothing frozen, hysteresis act gate,
# merging. Full qa2 and long BABILong, then the small sets, seeds 0-2. Compare exp 53's full-rate never-stop stack.
cd /home/claude/evolvehd
export FREEZE=999999999 B=runs/exp36/sit-54b
J="--sit-bnd-unit --sit-bnd-unit-len --sit-act --sit-act-hyst --sit-rel-merge --sit-learn-surprise 6,2"
(for s in 0 1 2; do SEED=$s T=x54b_s$s G="$J" bash runs/exp54/a.sh full; done) &
for s in 0 1 2; do SEED=$s T=x54b_s$s G="$J" bash runs/exp54/a.sh long; done
for s in 0 1 2; do SEED=$s T=x54b_s$s G="$J" bash runs/exp54/a.sh qs story; done
wait
