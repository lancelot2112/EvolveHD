#!/bin/bash
# usage: grid.sh "TAG|EXTRA" ... with SETS, PROBES, SEEDS env
cd /home/claude/evolvehd
jobs=(); for cfg in "$@"; do for s in ${SEEDS:-0}; do for S in ${SETS:-qs st}; do for P in ${PROBES:-base names verb names_verb all}; do jobs+=("$cfg|$S|$P|$s"); done; done; done; done
printf '%s\n' "${jobs[@]}" | xargs -P 2 -I{} bash -c 'IFS="|" read t e S P s <<< "{}"; TAG=$t EXTRA="$e" bash runs/exp56/p.sh $S $P $s'
