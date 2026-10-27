#!/bin/bash
# Exp 55: the default never-stop stack on the generalization probe streams. usage: p.sh SET PROBE SEED
# SET qs (small qa2) or st (story set); PROBE base (unchanged stream) or a probe name in runs/exp55/data
cd /home/claude/evolvehd
B=${B:-runs/exp36/sit-55a}; S=$1; P=$2; SEED=${3:-0}
F=$(sed "s/--sit-seed 0/--sit-seed $SEED/" runs/exp37/F.flags)
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096"
G="--sit-bnd-unit --sit-bnd-unit-len --sit-act --sit-act-hyst --sit-rel-merge"
if [ $P = base ]; then D=$([ $S = qs ] && echo runs/exp40/bl_qa2s || echo runs/exp31c/small/syn300); else D=runs/exp55/data/${S}_$P; fi
O=runs/exp55/R_${S}_${P}_s$SEED
if [ $S = qs ]; then
  SIT_DEBUG_BND=1 $B assoc --sit --data $D.ids --vocab $D.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-path-forget 2 --sit-chain --sit-lt-budget 131072 $G --sit-freeze-from 999999999 --out $O 2> $O.log
else
  SIT_DEBUG_BND=1 $B assoc --sit --data $D.ids --vocab $D.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F $X $G --out $O 2> $O.log
fi
python3 scripts/exp55/score_probe.py $D $O runs/exp55/data/${S}_names.info.json > $O.score
