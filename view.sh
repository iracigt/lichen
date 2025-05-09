#!/bin/bash

PREFIX=${PREFIX-"."} # directory where the files are located
cargo run --release -- align $@ | while read -r ROW ; do
    DATA=$(echo $ROW | tr ':-' '  ' | tr -s ' ')

    SCORE=$(echo $DATA | cut -d' ' -f1)

    FILE1=$(echo $DATA | cut -d' ' -f2)
    START1=$(echo $DATA | cut -d' ' -f3)
    END1=$(echo $DATA | cut -d' ' -f5)

    FILE2=$(echo $DATA | cut -d' ' -f7)
    START2=$(echo $DATA | cut -d' ' -f8)
    END2=$(echo $DATA | cut -d' ' -f10)

    echo Match with score $SCORE:
    echo
    echo $FILE1:$START1-$END1
    echo "--------------------"
    echo
    sed -n "${START1},${END1}p" $PREFIX/$FILE1
    echo

    echo $FILE2:$START2-$END2
    echo "--------------------"
    echo
    sed -n "${START2},${END2}p" $PREFIX/$FILE2
    echo
    echo
done