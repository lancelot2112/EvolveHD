#!/bin/bash
# Exp 61: none (Z), text (C) and primitives (P) before the test stream, with the default prune rule and with
# --prune-harmful; binary sit-61b (position nodes never collected).
cd /home/claude/evolvehd
export B=runs/exp36/sit-61b
printf '%s\n' "Z|" "C|" "P|" "Z|h" "C|h" "P|h" | xargs -P 2 -I{} bash -c 'IFS="|" read s h <<< "{}"; if [ "$h" = h ]; then TAG=_b_h bash runs/exp61/run.sh $s --prune-harmful; else TAG=_b bash runs/exp61/run.sh $s; fi'
