#!/usr/bin/env bash
# Temporary diagnosis for #1326, run by .github/workflows/diagnose-php-libclang.yml
# on the ubuntu-latest runner. Removed before the fix is reviewed.
#
# Lists the clang, llvm-config and libclang instances on the runner, then
# builds ext-php-rs once per environment, each in its own target directory so
# that every build script runs again, and records which libclang clang-sys
# links, which libclang the ext-php-rs build script loads, and whether the
# bindings are generated.
set -uo pipefail

results=()

section() {
    printf '\n=== %s ===\n' "$1"
}

# Builds ext-php-rs with the given environment assignments and reports the
# libclang it used and the result as a "CASE_RESULT <name> PASS|FAIL" line.
#
# Arguments: a case name, then NAME=value assignments for `env`.
run_case() {
    local name=$1
    shift
    local target="/tmp/diag-target-${name}"
    local log="/tmp/diag-${name}.log"
    section "Case ${name}: ${*:-(default environment)}"
    env "$@" CARGO_TARGET_DIR="${target}" cargo build --locked -p ext-php-rs >"${log}" 2>&1
    local status=$?
    echo "exit status: ${status}"
    echo "clang-sys link directives:"
    cat "${target}"/debug/build/clang-sys-*/output 2>/dev/null | grep -E 'rustc-link-(search|lib)' || echo "  (none)"
    echo "libclang loaded by the ext-php-rs build script:"
    for bin in "${target}"/debug/build/ext-php-rs-*/build-script-build; do
        [ -e "${bin}" ] && ldd "${bin}" | grep -i clang
    done
    echo "errors:"
    grep -E 'error:|Unable to generate' "${log}" | head -5 || echo "  (none)"
    local result=FAIL
    [ "${status}" -eq 0 ] && result=PASS
    echo "CASE_RESULT ${name} ${result}"
    results+=("CASE_RESULT ${name} ${result}")
    rm -rf "${target}"
}

section "System"
grep -E '^(PRETTY_NAME|VERSION_ID)=' /etc/os-release
uname -m

section "clang and llvm-config on PATH"
for tool in clang llvm-config; do
    path=$(command -v "${tool}" || true)
    if [ -n "${path}" ]; then
        echo "${tool}: ${path} -> $(readlink -f "${path}")"
    else
        echo "${tool}: not found"
    fi
done
clang --version | head -1 || true
llvm-config --version --prefix --bindir --libdir || true

section "Installed LLVM"
for dir in /usr/lib/llvm-*; do
    echo "${dir}:"
    find "${dir}/bin" -maxdepth 1 -name clang -printf '  %p -> %l\n'
    find "${dir}/lib" -maxdepth 1 -name 'libclang.so*' -printf '  %p -> %l\n'
done
echo "libclang in the library directories clang-sys searches:"
find /usr/lib/x86_64-linux-gnu /usr/lib -maxdepth 1 -name 'libclang*' -printf '  %p -> %l\n'

section "PHP"
php --version | head -1
php-config --include-dir
echo "PHP headers that include the x86 SIMD headers:"
grep -rlE '#\s*include\s*<(x|e|im)mintrin\.h>' "$(php-config --include-dir)" 2>/dev/null | sed 's/^/  /' || echo "  (none)"

# clang-sys's Clang::find, which bindgen calls, takes the clang in
# `llvm-config --bindir` before the one on PATH (CLANG_PATH unset).
bindgen_clang=""
if bindir=$(llvm-config --bindir 2>/dev/null) && [ -x "${bindir}/clang" ]; then
    bindgen_clang=$(readlink -f "${bindir}/clang")
else
    bindgen_clang=$(readlink -f "$(command -v clang)")
fi
bindgen_llvm=$(dirname "$(dirname "${bindgen_clang}")")
echo
echo "clang that bindgen asks for the include paths: ${bindgen_clang}"
echo "its LLVM: ${bindgen_llvm}"

run_case default
run_case libclang-of-bindgen-clang LIBCLANG_PATH="${bindgen_llvm}/lib"
for dir in /usr/lib/llvm-*; do
    version=${dir##*/llvm-}
    run_case "pair-${version}" LIBCLANG_PATH="${dir}/lib" CLANG_PATH="${dir}/bin/clang"
done
newest=$(find /usr/lib -maxdepth 1 -name 'llvm-*' | sort -V | tail -1)
if [ "${newest}" != "${bindgen_llvm}" ]; then
    run_case "mismatch-${newest##*/llvm-}-lib-with-bindgen-clang" \
        LIBCLANG_PATH="${newest}/lib" CLANG_PATH="${bindgen_clang}"
fi

section "Summary"
printf '%s\n' "${results[@]}"
echo "DIAGNOSIS_DONE"
