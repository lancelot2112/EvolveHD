#!/bin/bash
# Seeds 1 and 2 of the union proposer with the act path (exp 46) on full qa2 and long BABILong (seed 0 is in exp 46/45)
cd /home/claude/evolvehd
J="--sit-bnd-unit --sit-bnd-unit-len"
for s in 1 2; do
  SEED=$s B=runs/exp36/sit-46a T=x46un_s$s G="--sit-bnd-union $J --sit-act" bash runs/exp46/a.sh long full
done
