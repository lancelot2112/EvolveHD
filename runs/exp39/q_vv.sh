#!/bin/bash
# read then ask on a three-level book: train on book A (ld1m_v), read book B by the index pass, answer at its end
cd /home/claude/evolvehd; F=$(cat runs/exp37/F.flags); export B=runs/exp36/sit-39g
sleep 30; while ps -eo args | grep -q "[R]_bl_pilot_w"; do sleep 20; done
T=1004771; A="--sit-answer-only $T --sit-freeze-from $T"
bash runs/exp37/rld.sh runs/exp39/ld2m_vv vv_base $F --sit-nest --sit-scope --sit-epoch $A &
bash runs/exp37/rld.sh runs/exp39/ld2m_vv vv_lev $F --sit-nest --sit-scope --sit-epoch --sit-levels $A
wait
for n in vv_base vv_lev; do echo "== $n"; grep -E "found  by fact|wall" runs/exp37/R_$n.score; python3 scripts/exp39/score_levels.py runs/exp39/ld2m_vv runs/exp37/R_$n 0; done
