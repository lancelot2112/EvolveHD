#!/bin/bash
# Baseline for the seed check: exp 42's best stack (k90 + length-weighted judge, mood detector, question drop) with the
# budget and cap on long BABILong, seeds 0-2
cd /home/claude/evolvehd
for s in 0 1 2; do SEED=$s B=runs/exp36/sit-46a T=x47base_s$s G="--sit-bnd-unit --sit-bnd-unit-len" bash runs/exp46/a.sh long; done
