#[derive(derive_mmio::Mmio)]
#[repr(C)]
struct Uart {
    // this is read-write by default
    data: u32,
}

fn send_check<T: Send>(_: &T) {}

fn main() {
    let mut uart = Uart { data: 0xA };

    // Safety: We're pointing at a real object
    let mmio_uart = unsafe { MmioUart::new(core::ptr::addr_of_mut!(uart)) };
    send_check(&mmio_uart);
}
