// Trying to fake the inner block with a custom struct will not work, marker
// macro must be implemented.
#[repr(C)]
struct UartBank {
    data: u32,
    status: u32
}

struct MmioUartBank<'a> {
    ptr: *mut UartBank,
    phantom: core::marker::PhantomData<&'a mut ()>,
}

#[derive(derive_mmio::Mmio)]
#[repr(C)]
struct Uart {
    // you can be explicit if you like
    control: u32,
    #[mmio(inner)]
    bank_0: UartBank,
    #[mmio(inner)]
    bank_1: UartBank,
}

fn main() {}
