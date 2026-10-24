#!/bin/bash
# The per-entity cap without the halving sketch on the full qa2 stream ("the" banned, chains, budget), compare R_qa2_dnef
cd /home/claude/evolvehd
F=$(cat runs/exp37/F.flags); B=runs/exp36/sit-41k
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096 --sit-path-forget 2"
S="--sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-chain --sit-lt-budget 131072"
SIT_BND_DENY=2 SIT_TIMING=1 $B assoc --sit --data runs/exp40/bl_qa2.ids --vocab runs/exp40/bl_qa2.txt $S --sit-entity-fair --sit-freeze-from 220618 --out runs/exp41/R_qa2_dnef0 2> runs/exp41/R_qa2_dnef0.log; python3 scripts/exp38/score_babilong.py runs/exp40/bl_qa2 runs/exp41/R_qa2_dnef0 > runs/exp41/R_qa2_dnef0.score
echo "== qa2 dnef0 (no halving sketch)"; cat runs/exp41/R_qa2_dnef0.score; grep -E "words,|cost per" runs/exp41/R_qa2_dnef0.log | cut -c1-130; grep -E "^TIMING|^HEADS" runs/exp41/R_qa2_dnef0.log | tail -2 | cut -c1-230
