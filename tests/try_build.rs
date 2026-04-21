// Compile tests using trybuild.

#[test]
fn test_compilation() {
    let t = trybuild::TestCases::new();
    t.pass("tests/try_build/pass/*.rs");
    t.compile_fail("tests/try_build/fail/*.rs");
}
