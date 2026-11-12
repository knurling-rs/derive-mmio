#![allow(deprecated)]

#[derive(derive_mmio::Mmio)]
#[repr(C)]
struct Uart {
    data: u32,
}

#[test]
fn deprecated_ctors_still_work() {
    let mut uart = Uart { data: 0x1234 };
    let mut mmio_uart = unsafe { Uart::new_mmio(core::ptr::addr_of_mut!(uart)) };
    assert_eq!(mmio_uart.read_data(), 0x1234);
    mmio_uart.write_data(0x5678);

    let addr = core::ptr::addr_of_mut!(uart) as usize;
    let mmio_uart = unsafe { Uart::new_mmio_at(addr) };
    assert_eq!(mmio_uart.read_data(), 0x5678);
}
