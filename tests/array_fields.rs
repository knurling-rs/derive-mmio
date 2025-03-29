use derive_mmio::OutOfBoundsError;

#[derive(derive_mmio::Mmio)]
#[repr(C)]
struct Uart {
    control: u32,
    array_0: [u32; 4],
    other_field: u32,
    array_1: [u32; 2],
    #[mmio(PureRead)]
    array_read_only: [u32; 4],

    // Write-only e.g. 1WC regs.
    #[mmio(Write)]
    array_write_only: [u32; 2],
}

fn main() {
    let mut uart = Uart {
        control: 0xC,
        other_field: 0,
        array_0: [0x1, 0x2, 0x3, 0x4],
        array_1: [0x12, 0x44],
        array_read_only: [0x4, 0x3, 0x2, 0x1],
        array_write_only: [0, 0],
    };

    // Safety: We're pointing at a real object
    let mut mmio_uart = unsafe { Uart::new_mmio(core::ptr::addr_of_mut!(uart)) };
    assert_eq!(mmio_uart.read_array_0(0).unwrap(), 0x1);
    assert_eq!(mmio_uart.read_array_0(1).unwrap(), 0x2);
    assert_eq!(mmio_uart.read_array_0(2).unwrap(), 0x3);
    assert_eq!(mmio_uart.read_array_0(3).unwrap(), 0x4);
    unsafe {
        assert_eq!(mmio_uart.read_array_0_unchecked(0), 0x1);
        assert_eq!(mmio_uart.read_array_0_unchecked(1), 0x2);
        assert_eq!(mmio_uart.read_array_0_unchecked(2), 0x3);
        assert_eq!(mmio_uart.read_array_0_unchecked(3), 0x4);
    }

    assert_eq!(mmio_uart.read_array_1(0).unwrap(), 0x12);
    assert_eq!(mmio_uart.read_array_1(1).unwrap(), 0x44);
    unsafe {
        assert_eq!(mmio_uart.read_array_1_unchecked(0), 0x12);
        assert_eq!(mmio_uart.read_array_1_unchecked(1), 0x44);
    }

    for (idx, val) in (0..4).rev().enumerate() {
        mmio_uart.write_array_0(idx, val).unwrap();
    }
    for (idx, val) in (0..4).rev().enumerate() {
        assert_eq!(mmio_uart.read_array_0(idx).unwrap(), val);
    }

    unsafe {
        for (idx, val) in (0..4).rev().enumerate() {
            mmio_uart.write_array_0_unchecked(idx, val);
        }
        for (idx, val) in (0..4).rev().enumerate() {
            assert_eq!(mmio_uart.read_array_0_unchecked(idx), val);
        }
    }

    for (idx, val) in (0..2).rev().enumerate() {
        mmio_uart.write_array_1(idx, val).unwrap();
    }
    for (idx, val) in (0..2).rev().enumerate() {
        assert_eq!(mmio_uart.read_array_1(idx).unwrap(), val);
    }
    unsafe {
        for (idx, val) in (0..2).rev().enumerate() {
            mmio_uart.write_array_1_unchecked(idx, val);
        }
        for (idx, val) in (0..2).rev().enumerate() {
            assert_eq!(mmio_uart.read_array_1_unchecked(idx), val);
        }
    }

    // Out of bounds.
    let error = mmio_uart.read_array_0(4);
    if error.is_ok() {
        panic!("Expected error, but read was okay");
    }
    if let Err(OutOfBoundsError(index)) = error {
        assert_eq!(index, 4);
    }

    // Out of bounds.
    let error = mmio_uart.read_array_1(2);
    if error.is_ok() {
        panic!("Expected error, but read was okay");
    }
    if let Err(OutOfBoundsError(index)) = error {
        assert_eq!(index, 2);
    }

    mmio_uart.write_array_write_only(0, 0xFF).unwrap();
    mmio_uart.write_array_write_only(1, 0xFF).unwrap();

    let mmio_uart = unsafe { Uart::new_mmio(core::ptr::addr_of_mut!(uart)) };
    // We can only use this to read the pure read-only array.
    assert_eq!(mmio_uart.read_array_read_only(0).unwrap(), 0x4);
    assert_eq!(mmio_uart.read_array_read_only(1).unwrap(), 0x3);
    assert_eq!(mmio_uart.read_array_read_only(2).unwrap(), 0x2);
    assert_eq!(mmio_uart.read_array_read_only(3).unwrap(), 0x1);
}
