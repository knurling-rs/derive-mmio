#[test]
fn all_tests() {
    let t = trybuild::TestCases::new();

    // tests that pass

    t.pass("tests/array_fields.rs");
    t.pass("tests/basic.rs");
    t.pass("tests/offset_syntax.rs");
    t.pass("tests/inner_mmio.rs");
    t.pass("tests/inner_mmio_array.rs");
    t.pass("tests/no_ctors.rs");
    t.pass("tests/derives_debug.rs");
    t.pass("tests/owned_handle.rs");

    // tests that pass but need an specific rustc version

    if rustversion::cfg!(since(1.82)) {
        t.pass("tests/constness.rs");
    }

    // tests that fail

    t.compile_fail("tests/no_compile/array_safe_unchecked.rs");
    t.compile_fail("tests/no_compile/bad_inner_attr.rs");
    t.compile_fail("tests/no_compile/bad_outer_attr.rs");
    t.compile_fail("tests/no_compile/cant_fake_inner_block.rs");
    t.compile_fail("tests/no_compile/double_read.rs");
    t.compile_fail("tests/no_compile/duplicate_field_attr.rs");
    t.compile_fail("tests/no_compile/inner_array_invalid_type.rs");
    t.compile_fail("tests/no_compile/inner_array_safe_unchecked.rs");
    t.compile_fail("tests/no_compile/inner_mmio_double_borrow.rs");
    t.compile_fail("tests/no_compile/inner_only_shared.rs");
    t.compile_fail("tests/no_compile/offset_syntax.rs");
    t.compile_fail("tests/no_compile/modify_standalone.rs");
    t.compile_fail("tests/no_compile/modify_without_read.rs");
    t.compile_fail("tests/no_compile/modify_without_write.rs");
    t.compile_fail("tests/no_compile/no_modify.rs");
    t.compile_fail("tests/no_compile/no_owned.rs");
    t.compile_fail("tests/no_compile/owned_borrow.rs");
    t.compile_fail("tests/no_compile/padding_forbidden.rs");
    t.compile_fail("tests/no_compile/read_only.rs");
    t.compile_fail("tests/no_compile/repr_c_mandatory.rs");
    t.compile_fail("tests/no_compile/unimpl_send.rs");
}
