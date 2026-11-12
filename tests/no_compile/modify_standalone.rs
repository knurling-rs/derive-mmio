
#[derive(derive_mmio::Mmio)]
#[repr(C)]
struct Uart {
    // Standalone modification is not allowed, Read and Write access need to be specified
    // explicitly.
    #[mmio(Modify)]
    fifo: u32,
}

fn main() {
    let mut uart = Uart { fifo: 0x0 };

    // Safety: We're pointing at a real object
    let mut mmio_uart = unsafe { MmioUart::new(core::ptr::addr_of_mut!(uart)) };
    mmio_uart.modify_uart();
}
