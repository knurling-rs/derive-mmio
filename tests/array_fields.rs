use derive_mmio::OutOfBoundsError;

#[derive(derive_mmio::Mmio)]
#[repr(C)]
struct Uart {
    control: u32,
    some_array_0: [u32; 4],
    other_field: u32,
    some_array_1: [u32; 2],
}

fn main() {
    let mut uart = Uart {
        control: 0xC,
        other_field: 0,
        some_array_0: [0x1, 0x2, 0x3, 0x4],
        some_array_1: [0x12, 0x44],
    };

    // Safety: We're pointing at a real object
    let mut mmio_uart = unsafe { Uart::new_mmio(core::ptr::addr_of_mut!(uart)) };
    assert_eq!(mmio_uart.read_some_array_0(0).unwrap(), 0x1);
    assert_eq!(mmio_uart.read_some_array_0(1).unwrap(), 0x2);
    assert_eq!(mmio_uart.read_some_array_0(2).unwrap(), 0x3);
    assert_eq!(mmio_uart.read_some_array_0(3).unwrap(), 0x4);
    unsafe {
        assert_eq!(mmio_uart.read_some_array_0_unchecked(0), 0x1);
        assert_eq!(mmio_uart.read_some_array_0_unchecked(1), 0x2);
        assert_eq!(mmio_uart.read_some_array_0_unchecked(2), 0x3);
        assert_eq!(mmio_uart.read_some_array_0_unchecked(3), 0x4);
    }

    assert_eq!(mmio_uart.read_some_array_1(0).unwrap(), 0x12);
    assert_eq!(mmio_uart.read_some_array_1(1).unwrap(), 0x44);
    unsafe {
        assert_eq!(mmio_uart.read_some_array_1_unchecked(0), 0x12);
        assert_eq!(mmio_uart.read_some_array_1_unchecked(1), 0x44);
    }

    for (idx, val) in (0..3).rev().enumerate() {
        mmio_uart.write_some_array_0(idx, val).unwrap();
    }
    for (idx, val) in (0..3).rev().enumerate() {
        assert_eq!(mmio_uart.read_some_array_0(idx).unwrap(), val);
    }

    unsafe {
        for (idx, val) in (0..3).rev().enumerate() {
            mmio_uart.write_some_array_0_unchecked(idx, val);
        }
        for (idx, val) in (0..3).rev().enumerate() {
            assert_eq!(mmio_uart.read_some_array_0_unchecked(idx), val);
        }
    }

    for (idx, val) in (0..1).rev().enumerate() {
        mmio_uart.write_some_array_1(idx, val).unwrap();
    }
    for (idx, val) in (0..1).rev().enumerate() {
        assert_eq!(mmio_uart.read_some_array_1(idx).unwrap(), val);
    }
    unsafe {
        for (idx, val) in (0..1).rev().enumerate() {
            mmio_uart.write_some_array_1_unchecked(idx, val);
        }
        for (idx, val) in (0..1).rev().enumerate() {
            assert_eq!(mmio_uart.read_some_array_1_unchecked(idx), val);
        }
    }

    // Out of bounds.
    let error = mmio_uart.read_some_array_0(4);
    if error.is_ok() {
        panic!("Expected error, but read was okay");
    }
    if let Err(OutOfBoundsError(index)) = error {
        assert_eq!(index, 4);
    }

    // Out of bounds.
    let error = mmio_uart.read_some_array_1(2);
    if error.is_ok() {
        panic!("Expected error, but read was okay");
    }
    if let Err(OutOfBoundsError(index)) = error {
        assert_eq!(index, 2);
    }
}
