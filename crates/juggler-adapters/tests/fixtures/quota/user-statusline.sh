#!/bin/sh
# The user's own statusline (chaining fixture): reads stdin, prints a marker line.
data=$(cat)
printf 'USER-MARKER %s' "$data"
