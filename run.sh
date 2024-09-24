#!/bin/bash

directory_name=${PWD##*/}
tmp_path="/mnt/c/Windows/temp/$directory_name"

mkdir -p $tmp_path

rsync . $tmp_path -r --exclude-from=.gitignore

cd $tmp_path

if [ $@ ] && [ $@ == "build" ]; then
    powershell.exe -Command "cargo build";
elif [ $@ ] && [ $@ == "build-r" ]; then
    powershell.exe -Command "cargo build --release";
else 
    powershell.exe -Command "cargo run $@";
fi