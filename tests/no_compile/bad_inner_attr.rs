#[derive(derive_mmio::Mmio)]
#[repr(C)]
struct Uart {
    #[mmio(RW)]
    status: u32,
}

fn main() {
}
