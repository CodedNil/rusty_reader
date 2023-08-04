alias b := build
alias r := run
alias rd := rundebug

build:
    tsc assets/main.ts
    cargo build

run:
    tsc assets/main.ts
    cargo run

rundebug:
    @$env:RUST_LOG="debug"; cargo run