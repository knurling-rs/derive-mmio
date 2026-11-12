# Inline assembly demo

`app` uses the register blocks of `regs`. Missing `#[inline]` attributes only show up across a
crate boundary, and only with `opt-level = "s"` or `"z"` without LTO.

Requires `rustup target add thumbv7em-none-eabihf`, `rustup component add llvm-tools` and
`cargo install cargo-binutils`.

```sh
cd asm-demo

# Before: derive-mmio sources from main
git checkout main -- ../macro ../src
cargo objdump --bin app --release --target thumbv7em-none-eabihf -- -d -C --no-show-raw-insn
git checkout HEAD -- ../macro ../src

# After: current checkout
cargo objdump --bin app --release --target thumbv7em-none-eabihf -- -d -C --no-show-raw-insn
```

The relevant functions are `write_control` and `write_bank_data`.
