#!/bin/bash
# Exp 50: k90 + judge + act path, with and without --sit-rel-merge, on full qa2 and long BABILong, seeds 0-2.
# Baselines without merging exist (full qa2: runs/exp48/R_qa2_x48ka_s*, long: runs/exp38/R_bl_long_x47kact_s*); seed 0
# of the long baseline is rerun on this binary as a check that flags off reproduce it.
cd /home/claude/evolvehd
J="--sit-bnd-unit --sit-bnd-unit-len --sit-act"
(for s in 0 1 2; do SEED=$s T=x50m_s$s G="$J --sit-rel-merge" bash runs/exp50/a.sh full; done) &
for s in 0 1 2; do SEED=$s T=x50m_s$s G="$J --sit-rel-merge" bash runs/exp50/a.sh long; done
SEED=0 T=x50base_s0 G="$J" bash runs/exp50/a.sh long
wait
