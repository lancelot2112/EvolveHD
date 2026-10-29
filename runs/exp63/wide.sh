#!/bin/bash
# Exp 63 step 2: the stride act with whole-number steps (--stride-wide), test stream alone and after primitives,
# default seed (the act sits after the brain, so its runs keep the brain of brain_{Z,P}_s). Binary sit-63b.
cd /home/claude/evolvehd
export B=runs/exp36/sit-63b
printf '%s\n' Z P | xargs -P 2 -I{} bash -c 'TAG=_63w bash runs/exp61/run.sh {} --stride-act --stride-wide'
