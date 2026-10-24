#!/bin/bash
# Derived per-entity cap (--sit-entity-fair: N >> bl(entities)) on the full qa2 stream (with "the" banned, chains,
# budget, halving sketch; compare R_qa2_dn), and on long BABILong qa1 with the budget (compare exp38 R_bl_long_bud17)
cd /home/claude/evolvehd
F=$(cat runs/exp37/F.flags); B=runs/exp36/sit-41i
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096 --sit-path-forget 2"
S="--sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-chain --sit-lt-budget 131072 --sit-sketch-decay"
(SIT_BND_DENY=2 SIT_TIMING=1 $B assoc --sit --data runs/exp40/bl_qa2.ids --vocab runs/exp40/bl_qa2.txt $S --sit-entity-fair --sit-freeze-from 220618 --out runs/exp41/R_qa2_dnef 2> runs/exp41/R_qa2_dnef.log; python3 scripts/exp38/score_babilong.py runs/exp40/bl_qa2 runs/exp41/R_qa2_dnef > runs/exp41/R_qa2_dnef.score) &
BIN=$B TAG=_bud17sdef EXTRA="--sit-lt-budget 131072 --sit-sketch-decay --sit-entity-fair" bash runs/exp38/bl_split.sh bl_long > /dev/null; wait
echo "== qa2 dnef"; cat runs/exp41/R_qa2_dnef.score; grep -E "words,|cost per" runs/exp41/R_qa2_dnef.log | cut -c1-130; grep -E "^TIMING|^HEADS" runs/exp41/R_qa2_dnef.log | tail -2 | cut -c1-230
echo "== bl_long bud17 + sketch decay + entity fair"; cat runs/exp38/R_bl_long_bud17sdef.score; grep -E "words,|cost per" runs/exp38/R_bl_long_bud17sdef.log | cut -c1-130; grep -E "^HEADS" runs/exp38/R_bl_long_bud17sdef.log | tail -1 | cut -c1-230
