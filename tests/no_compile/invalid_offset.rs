#[derive(derive_mmio::Mmio)]
#[repr(C)]
struct Uart {
    #[mmio(offset_bytes(0x20))]
    data: u32,
}

fn main() {}
