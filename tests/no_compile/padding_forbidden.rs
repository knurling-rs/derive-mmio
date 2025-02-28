#[derive(derive_mmio::Mmio)]
#[repr(C)]
struct Uart {
    // this is read-only by default
    data: u32,
    // this will introduce padding, which will fail the compilation
    blub: u16,
    // this will introduce padding, which will fail the compilation
    _reserved2: u8,
}

fn main() {}
