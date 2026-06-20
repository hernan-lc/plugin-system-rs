// build.rs is intentionally empty: the cdylib is sufficient on its own.
// The matching `*.manifest.json` sidecar (which selects the C-ABI loader) is
// copied next to the produced `.so`/`.dll` by the integration test in
// `cabi_tests.rs` so the file can stay under version control.
fn main() {}
