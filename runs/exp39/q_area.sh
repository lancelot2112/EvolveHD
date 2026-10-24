#!/bin/bash
# BABILong pilot: leader kinds against kinds from an area of matchers (no anti-Hebbian, contested-bit anti-Hebbian)
cd /home/claude/evolvehd
while ps -eo args | grep -q "[s]it-39a assoc"; do sleep 20; done
(BIN=runs/exp36/sit-39d TAG=_lead bash runs/exp38/bl_split.sh bl_pilot > /dev/null &)
BIN=runs/exp36/sit-39d TAG=_ak EXTRA="--sit-area-kinds" bash runs/exp38/bl_split.sh bl_pilot > /dev/null
while ps -eo args | grep -q "[R]_bl_pilot_lead"; do sleep 10; done
BIN=runs/exp36/sit-39d TAG=_ak2 EXTRA="--sit-area-kinds --sit-area-anti 2" bash runs/exp38/bl_split.sh bl_pilot > /dev/null
for t in _lead _ak _ak2; do echo "== $t"; cat runs/exp38/R_bl_pilot$t.score; grep "words," runs/exp38/R_bl_pilot$t.log; done
