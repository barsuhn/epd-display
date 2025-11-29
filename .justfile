releaseargs := "--release --no-default-features"
release2040 := releaseargs + " --features rp2040"
release2350 := releaseargs + " --features rp2350 --target thumbv8m.main-none-eabihf"

alias bdd := build-display-demo
alias rdd := run-display-demo
alias bdd2 := build-display-demo-2
alias rdd2 := run-display-demo-2
alias bwd := build-wifi-demo
alias rwd := run-wifi-demo
alias bwd2 := build-wifi-demo-2
alias rwd2 := run-wifi-demo-2
alias bpd := build-pico-display
alias rpd := run-pico-display
alias bpd2 := build-pico-display-2
alias rpd2 := run-pico-display-2

default:
    just --list

build-display-demo:
    cargo build {{release2040}} --package display-demo

run-display-demo:
    cargo run {{release2040}} --package display-demo

build-display-demo-2:
    cargo build {{release2350}} --package display-demo

run-display-demo-2:
    cargo run {{release2350}} --package display-demo

build-wifi-demo:
    cargo build {{release2040}} --package wifi-demo

run-wifi-demo:
    cargo run {{release2040}} --package wifi-demo

build-wifi-demo-2:
    cargo build {{release2350}} --package wifi-demo

run-wifi-demo-2:
    cargo run {{release2350}} --package wifi-demo

build-pico-display:
    cargo build {{release2040}} --package pico-display

run-pico-display:
    cargo run {{release2040}} --package pico-display

build-pico-display-2:
    cargo build {{release2350}} --package pico-display

run-pico-display-2:
    cargo run {{release2350}} --package pico-display
