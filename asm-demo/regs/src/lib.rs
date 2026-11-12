#![no_std]

#[derive(derive_mmio::Mmio)]
#[repr(C)]
pub struct Bank {
    data: u32,
}

#[derive(derive_mmio::Mmio)]
#[repr(C)]
pub struct Uart {
    control: u32,
    #[mmio(Inner)]
    bank: Bank,
}
