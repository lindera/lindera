fn main() {
    // PHP extensions are shared libraries loaded by the PHP runtime at load time.
    // Zend/PHP symbols (e.g. _zend_*, _zval_*) are provided by the PHP runtime
    // and are not available at link time. On macOS, the linker requires all symbols
    // to be resolved by default, so we must allow undefined symbols via
    // `-undefined dynamic_lookup`.
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-cdylib-link-arg=-undefined");
        println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
    }
}
