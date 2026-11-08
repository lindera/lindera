#!/usr/bin/env bash
# Points bindgen at one LLVM: the LLVM of the clang on PATH (#1326).
#
# bindgen, through clang-sys, opens the libclang with the highest version it
# finds, but takes the system include paths, and with them clang's builtin
# headers such as xmmintrin.h, from CLANG_PATH or else the clang on PATH. When
# several LLVM versions are installed (the Ubuntu 26.04 runner image has 20,
# 21 and 22, with clang 21 on PATH), the two can come from different versions,
# and the headers then call builtins that the libclang does not have, so the
# bindings of ext-php-rs fail to generate. Setting both CLANG_PATH and
# LIBCLANG_PATH to the LLVM of the clang on PATH makes the pair match whatever
# the image installs.
#
# The variables are appended to the file named by GITHUB_ENV, so that the
# later steps of a GitHub Actions job use them.
set -euo pipefail

: "${GITHUB_ENV:?GITHUB_ENV must name the file to write the variables to}"

# Stops with an error annotation that points to the issue.
#
# Arguments: the message.
fail() {
    echo "::error::$1 (see https://github.com/lindera/lindera/issues/1326)" >&2
    exit 1
}

clang_on_path=$(command -v clang) ||
    fail "no clang on PATH; install clang and libclang of one LLVM version"
clang_bin=$(readlink -f "${clang_on_path}")
llvm_dir=$(dirname "$(dirname "${clang_bin}")")
lib_dir="${llvm_dir}/lib"

# The file names that clang-sys accepts when it opens libclang at run time.
# Each candidate is checked, since a pattern that matches nothing, and the
# plain `libclang.so`, stay in the list as they are written. Like clang-sys,
# skip libclang-cpp, the C++ library, whose name matches `libclang-*.so.*`.
libraries=()
for library in \
    "${lib_dir}"/libclang.so \
    "${lib_dir}"/libclang-*.so \
    "${lib_dir}"/libclang.so.* \
    "${lib_dir}"/libclang-*.so.*; do
    if [ -e "${library}" ] && [[ "${library##*/}" != *-cpp.* ]]; then
        libraries+=("${library}")
    fi
done
if [ "${#libraries[@]}" -eq 0 ]; then
    fail "no libclang in ${lib_dir} for ${clang_bin}; install the libclang of the same LLVM version"
fi

{
    echo "CLANG_PATH=${clang_bin}"
    echo "LIBCLANG_PATH=${lib_dir}"
} >>"${GITHUB_ENV}"

echo "CLANG_PATH=${clang_bin} ($("${clang_bin}" --version | head -1))"
echo "LIBCLANG_PATH=${lib_dir} (${libraries[*]})"
