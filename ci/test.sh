#!/bin/bash
x=1
cargo build
while [ $x -le 10 ]
do
  #cargo test -p tedge-mapper c8y::tests::mapper_tests 1>>mappertest.out 2>&1
  cargo test -p tedge-mapper 
 # cargo test -- --ignored 1>>mappertest.out 2>&1
  x=$(( $x + 1 ))
done
