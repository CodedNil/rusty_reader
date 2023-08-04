# use PowerShell instead of sh:
set shell := ["powershell.exe", "-c"]

alias b := build
alias s := serve
alias bs := build-serve

build:
    dx build --features web

serve:
    cargo run --features ssr

build-serve: build serve