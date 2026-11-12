#[derive(derive_mmio::Mmio)]
#[repr(C)]
struct CpuPrivateRegBlock {
    // this is read-write by default
    data: u32,
}

#[negative_impl::negative_impl]
impl !Send for CpuPrivateRegBlock {}

fn send_check<T: Send>(_: &T){}

fn main() {
    let mut private_peripheral = CpuPrivateRegBlock {
        data: 0xA,
    };

    // Safety: We're pointing at a real object
    let mmio_uart = unsafe { MmioCpuPrivateRegBlock::new(core::ptr::addr_of_mut!(private_peripheral)) };
    send_check(&mmio_uart);
}
