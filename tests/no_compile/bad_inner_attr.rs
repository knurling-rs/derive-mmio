#[derive(derive_mmio::Mmio)]
#[repr(C)]
struct Uart {
    #[mmio(RX)]
    status: u32,
}

fn main() {
}
