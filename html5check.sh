#!/bin/sh
for FILE in {public,errors}/*.html; do
  echo ./html5check.py $FILE
  ./html5check.py $FILE || exit 1
done
