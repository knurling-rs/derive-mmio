#[derive(derive_mmio::Mmio)]
#[repr(C)]
struct Uart {
    // this is read-write by default
    data: u32,
    // you can be explicit if you like
    #[mmio(Read, Write, Modify)]
    control: u32,
    // this field is read-only, with no side effects for a read.
    #[mmio(PureRead)]
    status: u32,
    // this field is read-only, but has side effects (e.g. read clears error bits)
    #[mmio(Read)]
    errors: u32,
    // this is ignored
    _reserved: u32,
    // this will introduce padding, which will fail the compilation
    // _reserved2: u8,
}

fn main() {
    let mut uart = Uart {
        data: 0xA,
        control: 0xC,
        status: 0xF,
        errors: 0x2,
        _reserved: 0,
        // _reserved2: 0,
    };

    // Safety: We're pointing at a real object
    let mut mmio_uart = unsafe { Uart::new_mmio(core::ptr::addr_of_mut!(uart)) };

    let ptr_raw = core::ptr::addr_of_mut!(uart);
    assert_eq!(mmio_uart.read_data(), 0xA);
    mmio_uart.write_data(0x0B);
    assert_eq!(mmio_uart.read_data(), 0xB);
    assert_eq!(mmio_uart.pointer_to_data(), ptr_raw as _);
    assert_eq!(mmio_uart.read_errors(), 0x2);

    mmio_uart.modify_control(|f| {
        assert_eq!(f, 0xC);
        32
    });
    assert!(mmio_uart.read_control() == 32);
    assert_eq!(mmio_uart.read_status(), 0xF);
    let ptr_to_u32s = ptr_raw as *const u32;
    assert_eq!(
        mmio_uart.pointer_to_status(),
        ptr_to_u32s.wrapping_add(2) as _
    );

    // We can unsafely clone the MMIO object.
    let mmio_uart_clone = unsafe { mmio_uart.clone() };
    assert_eq!(mmio_uart_clone.read_data(), 0xB);

    // Non-mutable block can be used to perform pure reads.
    let mmio_uart_clone = unsafe { mmio_uart.clone() };
    let status = mmio_uart_clone.read_status();
    assert_eq!(status, 0xF);
}
