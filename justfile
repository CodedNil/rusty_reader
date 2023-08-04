alias b := build
alias r := run
alias rd := rundebug

build:
    tsc src/main.ts --outDir assets
    cargo build

run:
    tsc src/main.ts --outDir assets
    cargo run

rundebug:
    @$env:RUST_LOG="debug"; cargo run