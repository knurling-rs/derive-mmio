#[derive(derive_mmio::Mmio)]
#[mmio(no_ctors)]
#[repr(C)]
struct Uart {
    data: u32,
}

impl Uart {
    // No name clashes, we can implement it ourselves.
    #[allow(dead_code)]
    pub fn new_mmio() -> MmioUart<'static> {
        MmioUart {
            ptr: 0x10002000 as *mut Uart,
            phantom: core::marker::PhantomData,
        }
    }
}
fn main() {
    // Can not really test this on a host environment. Simply verify that it builds..
}
