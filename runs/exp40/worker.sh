#!/bin/bash
# take jobs from runs/exp40/jobs.txt one at a time (a job is claimed by creating its directory); start none after 75 min
cd /home/claude/evolvehd; t0=$(date +%s)
while IFS=$'\t' read -r name cmd; do
  [ $(( $(date +%s) - t0 )) -gt 4500 ] && { echo "stopping: time"; exit 0; }
  mkdir runs/exp40/jobs/$name 2>/dev/null || continue
  echo "$(date +%H:%M) start $name"; eval "$cmd" > runs/exp40/jobs/$name/out 2>&1; touch runs/exp40/jobs/$name/done; echo "$(date +%H:%M) done $name"
done < runs/exp40/jobs.txt
echo "no jobs left"
