#!/usr/bin/env bash

SCRIPTPATH="$( cd "$(dirname "$0")" ; pwd -P )"
cd $SCRIPTPATH
cd ../../
cargo build --release && cd target/release
./server &
pid1=$!
./server &
pid2=$!
./server &
pid3=$!
./server &
pid4=$!

wait $pid1
exit_code1=$?
wait $pid2
exit_code2=$?
wait $pid3
exit_code3=$?
wait $pid4
exit_code4=$?

echo "Task 1 (PID $pid1) existed with code $exit_code1"
echo "Task 2 (PID $pid2) existed with code $exit_code2"
echo "Task 3 (PID $pid3) existed with code $exit_code3"
echo "Task 4 (PID $pid4) existed with code $exit_code4"
