//! A basic example of using the Mmio trait.
//!
//! We use a 'fake' UART so this doesn't need any specific hardware to run.
//!
//! You can expand this example by running
//!
//! ```rs
//! cargo expand --example basic
//! ```

#[derive(derive_mmio::Mmio)]
#[repr(C)]
struct Uart {
    // this is read-write by default
    data: u32,
    // you can be explicit if you like
    #[mmio(RW)]
    control: u32,
    // this field is read-only (no write_x or modify_x method)
    #[mmio(RO)]
    status: u32,
    // this is ignored
    _reserved: u32,
    // this will introduce padding, which will fail the compilation
    //_reserved2: u8,
}

fn main() {
    let mut uart = Uart {
        data: 0xA,
        control: 0xC,
        status: 0xF,
        _reserved: 0,
        //_reserved2: 0,
    };

    // Safety: We're pointing at a real object
    let mut mmio_uart = unsafe { Uart::new_mmio(core::ptr::addr_of_mut!(uart)) };
    println!("sample UART is @ {:p}", core::ptr::addr_of_mut!(uart));

    println!("data = {}", mmio_uart.read_data());
    mmio_uart.write_data(0x0B);
    println!("data = {}", mmio_uart.read_data());
    println!("data register is at = {:p}", mmio_uart.pointer_to_data());

    mmio_uart.modify_control(|f| {
        println!("control was {f}, is now 32");
        32
    });
    println!("control = {}", mmio_uart.read_control());
    println!("control register is @ {:p}", mmio_uart.pointer_to_control());

    println!("status = {}", mmio_uart.read_status());
    println!("status register is @ {:p}", mmio_uart.pointer_to_status());
}
