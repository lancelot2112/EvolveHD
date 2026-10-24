#!/bin/bash
# chains for every level against today's pairs: the three-level book, and the 200k book for no loss
cd /home/claude/evolvehd; F=$(cat runs/exp37/F.flags); export B=runs/exp36/sit-39e
(bash runs/exp37/rld.sh runs/exp37/ld200k c2_base $F --sit-nest --sit-scope --sit-epoch; bash runs/exp37/rld.sh runs/exp37/ld200k c2_lev $F --sit-nest --sit-scope --sit-epoch --sit-levels) &
bash runs/exp37/rld.sh runs/exp38/ld1m_v v_lev2 $F --sit-nest --sit-scope --sit-epoch --sit-levels
wait
python3 scripts/exp39/score_levels.py runs/exp38/ld1m_v runs/exp37/R_v_lev2
for n in c2_base c2_lev v_lev2; do echo "== $n"; grep -E "found  by fact|home   by fact|owner  by fact|where-person|who-has|where-object|wall" runs/exp37/R_$n.score; done
