//! A basic example of using the the inner MMIO attribute.
//!
//! You can expand this example by running
//!
//! ```rs
//! cargo expand --example inner_block
//! ```

mod inner {
    #[derive(derive_mmio::Mmio)]
    #[repr(C)]
    pub struct UartBank {
        data: u32,
        status: u32,
    }

    impl UartBank {
        pub fn fake() -> UartBank {
            UartBank {
                data: 0x2,
                status: 0x3,
            }
        }
    }
}

#[derive(derive_mmio::Mmio)]
#[repr(C)]
pub struct Uart {
    control: u32,
    #[mmio(Inner)]
    banks: [inner::UartBank; 2],
}

fn main() {
    let mut uart = Uart {
        control: 0xC,
        banks: [inner::UartBank::fake(), inner::UartBank::fake()],
    };

    // Safety: We're pointing at a real object
    let mut mmio_uart = unsafe { Uart::new_mmio(core::ptr::addr_of_mut!(uart)) };
    assert_eq!(mmio_uart.len_banks(), 2);
    let bank0 = mmio_uart.banks_shared(0).unwrap();
    assert_eq!(bank0.read_data(), 0x2);
    assert_eq!(bank0.read_status(), 0x3);
    let bank1 = mmio_uart.banks_shared(1).unwrap();
    assert_eq!(bank1.read_data(), 0x2);
    assert_eq!(bank1.read_status(), 0x3);

    let mut bank0 = mmio_uart.banks(0).unwrap();
    bank0.write_data(0x42);
    bank0.write_status(0x13);
    assert_eq!(bank0.read_data(), 0x42);
    assert_eq!(bank0.read_status(), 0x13);

    unsafe {
        let inner_owned_for_p0 = mmio_uart.steal_banks_shared(0).unwrap();
        let inner_owned_for_p1 = mmio_uart.steal_banks_shared(1).unwrap();
        // Can be used independently now.
        assert_eq!(inner_owned_for_p0.read_data(), 0x42);
        assert_eq!(inner_owned_for_p1.read_data(), 0x2);
    }

    unsafe {
        let mut inner_owned_for_p0 = mmio_uart.steal_banks(0).unwrap();
        let mut inner_owned_for_p1 = mmio_uart.steal_banks(1).unwrap();
        // Can be used independently now and also allow mutable access.
        inner_owned_for_p0.write_data(0x12);
        inner_owned_for_p1.write_status(0x13);
        assert_eq!(inner_owned_for_p0.read_data(), 0x12);
        assert_eq!(inner_owned_for_p1.read_status(), 0x13);
    }

    // Bound checks for default methods.
    matches!(mmio_uart.banks(2), Err(derive_mmio::OutOfBoundsError(2)));
    matches!(
        mmio_uart.banks_shared(2),
        Err(derive_mmio::OutOfBoundsError(2))
    );
    matches!(
        unsafe { mmio_uart.steal_banks(2) },
        Err(derive_mmio::OutOfBoundsError(2))
    );
    matches!(
        unsafe { mmio_uart.steal_banks_shared(2) },
        Err(derive_mmio::OutOfBoundsError(2))
    );
}
