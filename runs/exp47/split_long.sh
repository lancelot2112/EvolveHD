#!/bin/bash
# Which part costs long BABILong over seeds: k90 + act path (seeds 0-2) and union without the act path (seeds 1-2)
cd /home/claude/evolvehd
J="--sit-bnd-unit --sit-bnd-unit-len"
while pgrep -f "R_bl_long_x47base_s2" > /dev/null || [ ! -s runs/exp38/R_bl_long_x47base_s2.score ]; do sleep 30; done
(for s in 0 1 2; do SEED=$s B=runs/exp36/sit-46a T=x47kact_s$s G="$J --sit-act" bash runs/exp46/a.sh long; done) &
while [ ! -s runs/exp46/R_qa2_x46un_s2.score ]; do sleep 30; done
for s in 1 2; do SEED=$s B=runs/exp36/sit-46a T=x47un_s$s G="--sit-bnd-union $J" bash runs/exp46/a.sh long; done
wait
