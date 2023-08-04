# use PowerShell instead of sh:
set shell := ["powershell.exe", "-c"]

alias b := build
alias r := run
alias rd := rundebug

build:
    cargo build

run:
    cargo run

rundebug:
    @$env:RUST_LOG="debug"; cargo run