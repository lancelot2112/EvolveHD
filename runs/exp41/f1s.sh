#!/bin/bash
# F1 alone (--sit-bnd-unit, no hand ban) on seeds 1 and 2 (small qa2, story set, pilot), then the full qa2 stream with
# chains, budget and the per-entity cap (compare R_qa2_dnef0, which banned "the" by hand)
cd /home/claude/evolvehd
B=runs/exp36/sit-41p
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096"
for sd in 1 2; do
  F=$(sed "s/--sit-seed 0/--sit-seed $sd/" runs/exp37/F.flags)
  (SIT_DEBUG_BND=1 $B assoc --sit --data runs/exp40/bl_qa2s.ids --vocab runs/exp40/bl_qa2s.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-path-forget 2 --sit-chain --sit-lt-budget 131072 --sit-bnd-unit --sit-freeze-from 102303 --out runs/exp41/R_qa2s_f1u_s$sd 2> runs/exp41/R_qa2s_f1u_s$sd.log; python3 scripts/exp38/score_babilong.py runs/exp40/bl_qa2s runs/exp41/R_qa2s_f1u_s$sd > runs/exp41/R_qa2s_f1u_s$sd.score) &
  B=$B bash runs/exp36/rq30.sh "f1u_story_s$sd $F $X --sit-bnd-unit"; wait
  SEED=$sd BIN=$B TAG=_f1u_s$sd EXTRA="--sit-bnd-unit" bash runs/exp38/bl_split.sh bl_pilot > /dev/null
done
F=$(cat runs/exp37/F.flags)
SIT_DEBUG_BND=1 SIT_TIMING=1 $B assoc --sit --data runs/exp40/bl_qa2.ids --vocab runs/exp40/bl_qa2.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-path-forget 2 --sit-chain --sit-lt-budget 131072 --sit-entity-fair --sit-bnd-unit --sit-freeze-from 220618 --out runs/exp41/R_qa2_f1uef 2> runs/exp41/R_qa2_f1uef.log; python3 scripts/exp38/score_babilong.py runs/exp40/bl_qa2 runs/exp41/R_qa2_f1uef > runs/exp41/R_qa2_f1uef.score
s() { grep -E "words,|binding stops|boundary unit" $1 | cut -c1-200; grep "^UNIT" $1 | head -6 | tr '\n' ';'; echo; }
for sd in 1 2; do echo "== seed $sd qa2s"; cat runs/exp41/R_qa2s_f1u_s$sd.score; s runs/exp41/R_qa2s_f1u_s$sd.log; echo "== story"; tail -1 runs/exp36/R_f1u_story_s$sd.score; s runs/exp36/R_f1u_story_s$sd.log; echo "== pilot"; cat runs/exp38/R_bl_pilot_f1u_s$sd.score; s runs/exp38/R_bl_pilot_f1u_s$sd.log; done
echo "== full qa2 f1u + cap"; cat runs/exp41/R_qa2_f1uef.score; s runs/exp41/R_qa2_f1uef.log; grep "cost per" runs/exp41/R_qa2_f1uef.log | cut -c1-120
