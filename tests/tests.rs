#[test]
fn all_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/basic.rs");
    t.pass("tests/inner_mmio.rs");
    t.pass("tests/no_ctors.rs");
    t.pass("tests/array_fields.rs");

    t.compile_fail("tests/no_compile/bad_inner_attr.rs");
    t.compile_fail("tests/no_compile/bad_outer_attr.rs");
    t.compile_fail("tests/no_compile/cant_fake_inner_block.rs");
    t.compile_fail("tests/no_compile/inner_mmio_double_borrow.rs");
    t.compile_fail("tests/no_compile/padding_forbidden.rs");
    t.compile_fail("tests/no_compile/read_only.rs");
    t.compile_fail("tests/no_compile/repr_c_mandatory.rs");
}
