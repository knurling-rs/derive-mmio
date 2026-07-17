#[derive(derive_mmio::Mmio)]
#[repr(C)]
struct Uart {
    _reserved: [u32; 0x8],
    #[mmio(offset_bytes(0x20))]
    data: u32,
    #[mmio(PureRead, offset_bytes(36))]
    status: u32,
}

fn main() {}
