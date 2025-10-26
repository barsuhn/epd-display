# E-Paper Display for Raspberry Pi Pico

## Building and Running

The binary crates use rp2040 as a default feature. Those can be compiled and run using the usual cargo 
commands:

```sh
cargo build --release
cargo run --release
```

To build the libraries for rp2040, the feature rp2040 must be explicitly set:

```sh
cargo build --release --features rp2040
cargo run --release --features rp2040
```

Building for RP2350 is also possible by setting the feature rp2350, choosing the desired target and 
disabling default features:

```sh
cargo build --release --no-default-features --features rp2350 --target thumbv8m.main-none-eabihf
cargo run --release --no-default-features --features rp2350 --target thumbv8m.main-none-eabihf
```

## Static memory usage analysis

For RP2040 the static memory usage can be analyzed using:

```sh
cargo size --release --features rp2040 --target thumbv6m-none-eabi -- -A
```

For RP2350 the static memory usage can be analyzed using:

```sh
cargo size --release --no-default-features --features rp2350 --target thumbv8m.main-none-eabihf -- -A
```

The memory needed for statically defined data is the sum of the `.data`, `.bss` and `.uninit` sections.
For the following example output the static memory usage is `240 + 12048 + 1024 = 13312` bytes.

```
display  :
section               size        addr
.vector_table          276  0x10000000
.start_block            40  0x10000114
.text                24432  0x1000013c
.bi_entries              0  0x100060ac
.rodata               6496  0x100060b0
.data                  240  0x20000000
.gnu.sgstubs             0  0x10007b00
.bss                 12048  0x200000f0
.uninit               1024  0x20003000
.end_block               0  0x10007b00
.defmt                  49         0x0
.debug_loc           39205         0x0
.debug_abbrev         7403         0x0
.debug_info         328669         0x0
.debug_aranges       10544         0x0
.debug_ranges        20504         0x0
.debug_str          536760         0x0
.comment               153         0x0
.ARM.attributes         56         0x0
.debug_frame          8264         0x0
.debug_line          87637         0x0
.debug_pubnames        803         0x0
.debug_pubtypes         71         0x0
Total              1084674
```

## License

- MIT license
  ([LICENSE](LICENSE.txt) or http://opensource.org/licenses/MIT)

