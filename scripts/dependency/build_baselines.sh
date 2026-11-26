#!/usr/bin/env bash
# Builds the tools the baselines of lindera-dependency are measured with,
# from their sources, outside the repository: CRF++, MeCab (and its IPA
# dictionary), CaboCha and J.DepP, at fixed versions. Nothing of them is
# committed here.
#
# Usage: build_baselines.sh [<tools directory>]
#   <tools directory> defaults to scripts/dependency/data/tools, which git
#   ignores. The archives go to its `archives`, the build trees to its
#   `build`, and the tools are installed into it (`bin`, `lib`, `libexec`).
#   A directory that holds the tools of these versions already is left as
#   it is.
#
# It needs a C++ compiler, make, curl, tar, sha256sum and the helper scripts
# of automake (`automake --print-libdir`). Built here with GCC 15.2 on Linux
# (x86_64); other systems and compilers are not tried.
#
# What the sources need to build today, each done below:
#
# - CRF++: its repository lacks `winmain.h`, which `crf_learn.cpp` and
#   `crf_test.cpp` include. The header has content on Windows only (MeCab
#   has the same file), so an empty one is written.
# - CaboCha: its repository has `configure` and lacks the helper scripts it
#   runs (`install-sh`, `missing`, `depcomp`, `compile`); they are copied
#   from automake. `autoreconf` is not an option: it wants a macro
#   (`AM_ICONV`) that is not there.
# - J.DepP: `kernel.cc` calls a member template that `classify.cc` defines
#   and does not instantiate for it; an older GCC emitted the symbol all the
#   same, a current one inlines it and the link fails with undefined
#   references. The four explicit instantiations are added. Its `configure`
#   also wants the IPA dictionary of MeCab to be installed, although the
#   baselines give it the morphemes of Lindera: the dictionary of the same
#   MeCab repository is built for it.
# - The build trees are unpacked with the times of the archives. A copy
#   with new times makes `make` regenerate `configure` with the automake of
#   the machine, which fails.
#
# MeCab is built for its library, which CaboCha and J.DepP link. The tools
# find their libraries without LD_LIBRARY_PATH (an rpath).
set -euo pipefail

here=$(dirname "$(readlink -f "$0")")
tools=${1:-$here/data/tools}
mkdir -p "$tools"
tools=$(readlink -f "$tools")
archives=$tools/archives
build=$tools/build
mkdir -p "$archives" "$build"

# name, URL, SHA-256 of the archive.
sources="
crfpp https://github.com/taku910/crfpp/archive/c603ea6fc2cc8e5317d2754971bc787da383f7e5.tar.gz 1d3e4be6e7d944cdd1db66ddbb5417f6bda35ab3530e11a5970985499a530be9
mecab https://github.com/taku910/mecab/archive/61b90ba6e669dc2d7d533d4a80d206f3b31d52b1.tar.gz 7ad44f987ae0b7fd345c72a4b67e14dbe0f4bd1669d247a8aaeb0f15218a3fd1
cabocha https://github.com/taku910/cabocha/archive/96e100a62f6dadceb490cdd61c8c15fb561f6674.tar.gz 65a8ee45068a0dc942c5443714287f5cbbababad79ac3dea457d12f6386e0115
jdepp https://www.tkl.iis.u-tokyo.ac.jp/~ynaga/jdepp/jdepp-2022-03-18.tar.gz e4a4328e1ad38970185471129a85da6db7079f41439f9f2abb34e7b62ec09232
"
versions=$tools/versions.txt
if [ -f "$versions" ] && echo "$sources" | cmp -s - "$versions" && [ -x "$tools/bin/jdepp" ]; then
    echo "the tools of these versions are in $tools"
    exit 0
fi
rm -f "$versions"

export PATH=$tools/bin:$PATH
export CPPFLAGS="-I$tools/include"
export LDFLAGS="-L$tools/lib -Wl,-rpath,$tools/lib"
jobs=$(getconf _NPROCESSORS_ONLN 2> /dev/null || echo 2)

# fetch <name>: the archive of a tool, checked against its SHA-256.
fetch() {
    local name=$1 url sha256 found
    url=$(echo "$sources" | awk -v name="$name" '$1 == name { print $2 }')
    sha256=$(echo "$sources" | awk -v name="$name" '$1 == name { print $3 }')
    if [ ! -s "$archives/$name.tar.gz" ]; then
        curl -sL --fail -o "$archives/$name.tar.gz" "$url"
    fi
    found=$(sha256sum "$archives/$name.tar.gz" | cut -d' ' -f1)
    if [ "$found" != "$sha256" ]; then
        echo "$archives/$name.tar.gz has the SHA-256 $found, not $sha256" >&2
        exit 1
    fi
}

# unpack <name> <tree> [<directory in the archive>]: a build tree, with the
# times of the archive.
unpack() {
    local name=$1 tree=$2 inner=${3:-}
    rm -rf "${build:?}/$tree" "$build/$tree.tmp"
    mkdir -p "$build/$tree.tmp"
    tar -xzf "$archives/$name.tar.gz" -C "$build/$tree.tmp" --strip-components=1
    mv "$build/$tree.tmp/$inner" "$build/$tree"
    rm -rf "$build/$tree.tmp"
}

# make_install <tree> <configure options...>: the log of a tool is in its
# build tree, and its last lines are shown when a step fails.
make_install() {
    local tree=$1
    shift
    local log=$build/$tree.log
    echo "building $tree"
    if ! (
        cd "$build/$tree"
        ./configure --prefix="$tools" "$@"
        make -j"$jobs"
        make install
    ) > "$log" 2>&1; then
        echo "the build of $tree failed; the end of $log:" >&2
        tail -n 20 "$log" >&2
        exit 1
    fi
}

for name in crfpp mecab cabocha jdepp; do
    fetch "$name"
done

unpack crfpp crfpp
: > "$build/crfpp/winmain.h"
make_install crfpp

unpack mecab mecab mecab
make_install mecab --with-charset=utf8
unpack mecab mecab-ipadic mecab-ipadic
make_install mecab-ipadic --with-charset=utf8

unpack cabocha cabocha
helpers=$(automake --print-libdir)
for file in install-sh missing depcomp compile; do
    if [ ! -e "$build/cabocha/$file" ]; then
        cp "$helpers/$file" "$build/cabocha/$file"
    fi
done
make_install cabocha --with-charset=UTF8 --with-posset=IPA --enable-utf8-only \
    --with-mecab-config="$tools/bin/mecab-config"

unpack jdepp jdepp
anchor='  template void ClassifierBase <kernel_model>::_pkeClassify <true, MULTI>  (ny::fv_t&, double*);'
if [ "$(grep -cF "$anchor" "$build/jdepp/src/classify.cc")" != 1 ]; then
    echo "src/classify.cc of J.DepP is not the file the change was written for" >&2
    exit 1
fi
arguments='(double*, ny::fv_it, const ny::fv_it\&, const ny::fv_it\&);'
sed -i "/_pkeClassify <true, MULTI>  (ny::fv_t&, double\*);/a\\
  template bool ClassifierBase <kernel_model>::_pkeClassify <true, BINARY> $arguments\\
  template bool ClassifierBase <kernel_model>::_pkeClassify <true, MULTI> $arguments\\
  template bool ClassifierBase <kernel_model>::_pkeClassify <false, BINARY> $arguments\\
  template bool ClassifierBase <kernel_model>::_pkeClassify <false, MULTI> $arguments" \
    "$build/jdepp/src/classify.cc"
if [ "$(grep -c 'template bool ClassifierBase <kernel_model>::_pkeClassify' "$build/jdepp/src/classify.cc")" != 4 ]; then
    echo "the change of src/classify.cc of J.DepP did not apply" >&2
    exit 1
fi
make_install jdepp --with-mecab-dict=IPA

for program in bin/crf_learn bin/mecab bin/cabocha bin/jdepp libexec/cabocha/cabocha-learn \
    libexec/cabocha/cabocha-system-eval; do
    if [ ! -x "$tools/$program" ]; then
        echo "$tools/$program was not built" >&2
        exit 1
    fi
done
echo "$sources" > "$versions"
echo "the tools are in $tools"
