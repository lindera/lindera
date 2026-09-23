//! Build script for the lindera PHP extension: re-runs when the target PHP
//! installation changes and adds the linker flags a PHP extension needs on
//! macOS.

use std::env;

fn main() {
    // ext-php-rs reads the PHP headers through `php-config`; PIE's phpize path
    // (Makefile.frag) selects the installation with these variables.
    println!("cargo:rerun-if-env-changed=PHP");
    println!("cargo:rerun-if-env-changed=PHP_CONFIG");

    // PHP extensions are shared libraries loaded by the PHP runtime at load time.
    // Zend/PHP symbols (e.g. _zend_*, _zval_*) are provided by the PHP runtime
    // and are not available at link time. On macOS, the linker requires all symbols
    // to be resolved by default, so we must allow undefined symbols via
    // `-undefined dynamic_lookup`. The target OS is read from Cargo rather than
    // `cfg!`, which would describe the host running this build script.
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-cdylib-link-arg=-undefined");
        println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
    }
}
