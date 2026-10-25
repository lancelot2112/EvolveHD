#!/bin/bash
# Full qa2, seeds 0-2: k90 + judge + act path against k90 + judge (mood detector, question drop)
cd /home/claude/evolvehd
J="--sit-bnd-unit --sit-bnd-unit-len"
(for s in 0 1 2; do SEED=$s T=x48ka_s$s G="$J --sit-act" bash runs/exp48/a.sh full; done) &
for s in 0 1 2; do SEED=$s T=x48kb_s$s G="$J" bash runs/exp48/a.sh full; done
wait
