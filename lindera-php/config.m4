dnl config.m4 for the lindera PHP extension (Rust, built with ext-php-rs).
dnl
dnl PIE (and anyone running phpize by hand) drives the usual
dnl `phpize && ./configure && make && make install` sequence. There are no C
dnl sources, so PHP_NEW_EXTENSION is deliberately not used: configure only
dnl (1) checks that the Cargo workspace is present, (2) finds cargo,
dnl (3) records which PHP to build against and (4) appends Makefile.frag,
dnl which runs `cargo build` from the workspace root.

PHP_ARG_ENABLE([lindera],
  [whether to enable the lindera extension],
  [AS_HELP_STRING([--enable-lindera],
    [Enable the lindera morphological analysis extension (needs a Rust toolchain)])],
  [yes])

if test "$PHP_LINDERA" != "no"; then
  dnl The Cargo workspace root is the parent of this directory. PIE extracts
  dnl the whole repository and runs phpize inside lindera-php/, so this holds
  dnl there too; a copy of lindera-php/ on its own cannot be built.
  AC_MSG_CHECKING([for the lindera Cargo workspace])
  if test -f "$abs_srcdir/../Cargo.toml"; then
    AC_MSG_RESULT([$abs_srcdir/..])
  else
    AC_MSG_RESULT([not found])
    AC_MSG_ERROR([$abs_srcdir/../Cargo.toml does not exist. lindera-php must be built from a full lindera repository checkout, not from a copy of this directory alone.])
  fi

  dnl cargo: search PATH plus rustup's default install dir (often missing from
  dnl PATH in non-login shells), then fall back to asking rustup.
  AC_PATH_PROG([CARGO], [cargo], [], [$PATH$PATH_SEPARATOR$HOME/.cargo/bin])
  if test -z "$CARGO"; then
    AC_PATH_PROG([RUSTUP], [rustup], [], [$PATH$PATH_SEPARATOR$HOME/.cargo/bin])
    if test -n "$RUSTUP"; then
      CARGO=$($RUSTUP which cargo 2>/dev/null)
    fi
  fi
  if test -z "$CARGO" || test ! -x "$CARGO"; then
    AC_MSG_ERROR([cargo (the Rust build tool) was not found. Install Rust from https://rustup.rs/ and make sure cargo is on PATH. Building also needs libclang for bindgen (Debian/Ubuntu: apt install libclang-dev; macOS: xcode-select --install or brew install llvm).])
  fi

  dnl Make ext-php-rs build against the PHP that phpize / --with-php-config
  dnl selected, not whatever php-config and php come first in PATH. Its build
  dnl script reads the environment variables PHP_CONFIG and PHP. phpize.m4
  dnl sets the shell variables PHP_CONFIG (possibly the bare word "php-config")
  dnl and PHP_EXECUTABLE (from `php-config --php-binary`) but only substitutes
  dnl the latter into the Makefile, so PHP_CONFIG is substituted here.
  dnl ext-php-rs checks the value with exists(), which fails for a bare command
  dnl name, so it is made absolute first.
  AS_CASE([$PHP_CONFIG],
    [/*], [],
    [PHP_CONFIG=$(command -v "$PHP_CONFIG" 2>/dev/null)])
  if test -z "$PHP_CONFIG" || test ! -x "$PHP_CONFIG"; then
    AC_MSG_ERROR([cannot resolve php-config to an executable path; pass --with-php-config=/path/to/php-config])
  fi
  dnl `php-config --php-binary` is "NONE" in some builds (Homebrew's
  dnl php@X.Y-zts formulae, which setup-php installs for thread-safe CI).
  dnl Fall back to the php on PATH, but only if it reports the same version
  dnl as php-config, so the headers and the binary belong to one install.
  if test -z "$PHP_EXECUTABLE" || test "$PHP_EXECUTABLE" = "NONE" || test ! -x "$PHP_EXECUTABLE"; then
    php_config_binary="$PHP_EXECUTABLE"
    PHP_EXECUTABLE=$(command -v php 2>/dev/null)
    if test -z "$PHP_EXECUTABLE" || test ! -x "$PHP_EXECUTABLE"; then
      AC_MSG_ERROR([$PHP_CONFIG --php-binary returned '$php_config_binary' and no php was found on PATH. ext-php-rs needs the php CLI of the target installation; put it on PATH or set PHP=/path/to/php.])
    fi
    php_bin_version=$("$PHP_EXECUTABLE" -n -r 'echo PHP_VERSION;' 2>/dev/null)
    php_config_version=$("$PHP_CONFIG" --version 2>/dev/null)
    if test "$php_bin_version" != "$php_config_version"; then
      AC_MSG_ERROR([$PHP_CONFIG describes PHP $php_config_version but the php on PATH ($PHP_EXECUTABLE) is PHP $php_bin_version. Pass --with-php-config for the installation whose php is on PATH.])
    fi
    AC_MSG_NOTICE([php-config reports no php binary; using $PHP_EXECUTABLE (PHP $php_bin_version) from PATH])
  fi
  AC_MSG_CHECKING([for php-config to pass to ext-php-rs])
  AC_MSG_RESULT([$PHP_CONFIG])
  AC_MSG_CHECKING([for php binary to pass to ext-php-rs])
  AC_MSG_RESULT([$PHP_EXECUTABLE])

  PHP_SUBST([CARGO])
  PHP_SUBST([PHP_CONFIG])

  dnl Without PHP_NEW_EXTENSION, $ext_srcdir and $ext_builddir are unset, so
  dnl the fragment path and the $(srcdir) / $(builddir) replacements must be
  dnl given explicitly (the default would look for /Makefile.frag and
  dnl silently append nothing).
  PHP_ADD_MAKEFILE_FRAGMENT([$abs_srcdir/Makefile.frag], [$abs_srcdir], [$abs_builddir])
fi
