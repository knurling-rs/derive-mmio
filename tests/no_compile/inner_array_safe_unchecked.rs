mod inner {
    #[derive(derive_mmio::Mmio)]
    #[repr(C)]
    pub struct UartBank {
        data: u32,
    }

    impl UartBank {
        pub fn fake() -> UartBank {
            UartBank {
                data: 0x2,
            }
        }
    }
}

#[derive(derive_mmio::Mmio)]
#[repr(C)]
struct Uart {
    #[mmio(Inner)]
    array: [inner::UartBank; 2],
}

fn main() {
    let mut uart = Uart { array: [inner::UartBank::fake(), inner::UartBank::fake()] };

    // Safety: We're pointing at a real object
    let mut mmio_uart = unsafe { MmioUart::new(core::ptr::addr_of_mut!(uart)) };

    // we cannot safely access the array field without bounds checks

    let _inner_bank = mmio_uart.array_shared_unchecked(5);

    let _inner_bank = mmio_uart.array_unchecked(5);
}
