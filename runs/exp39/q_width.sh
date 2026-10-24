#!/bin/bash
# BABILong pilot: code width x kind rule (leader, area without anti-Hebbian, contested, near-tie gated), two at a time
cd /home/claude/evolvehd
while ps -eo args | grep -q "[s]it-39e assoc"; do sleep 20; done
run() { BIN=runs/exp36/sit-39g TAG=$1 EXTRA="$2" bash runs/exp38/bl_split.sh bl_pilot > /dev/null; }
for w in 512 128 64; do
  run _w${w}_l "--sit-kind-bits $w" & run _w${w}_a0 "--sit-kind-bits $w --sit-area-kinds"; wait
  run _w${w}_a2 "--sit-kind-bits $w --sit-area-kinds --sit-area-anti 2" & run _w${w}_a3 "--sit-kind-bits $w --sit-area-kinds --sit-area-anti 3"; wait
done
for w in 512 128 64; do for v in l a0 a2 a3; do n=bl_pilot_w${w}_$v
  echo "$n | $(grep -E 'qa1 +(1k|0k|4k)' runs/exp38/R_$n.score | awk '{print $2, $3}' | tr '\n' ' ') | $(grep 'words,' runs/exp38/R_$n.log) | $(grep 'kind area' runs/exp38/R_$n.log | cut -c12-)"
  grep "^kinds" runs/exp38/R_$n.log | tr '|' '\n' | grep -E "(^| )kitchen( |$)" | awk '{print "   places kind: " NF " words: " $0}' | cut -c1-160
done; done
