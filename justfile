# use PowerShell instead of sh:
set shell := ["powershell.exe", "-c"]

build:
    dx build --features web

serve:
    cargo run --features ssr

backend:
    cargo run --features backend