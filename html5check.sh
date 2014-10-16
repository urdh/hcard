#!/bin/bash
for FILE in {public,errors}/*.html; do
  echo ./html5check.py $FILE
  [[ -z `./html5check.py -g $FILE` ]] || exit 1
done
