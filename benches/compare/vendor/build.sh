#!/bin/sh
set -eu

cd "$(dirname "$0")"
VENDOR=$(pwd)
BUILD="$VENDOR/build"
PREFIX="$VENDOR/prefix"
LOGS="$VENDOR/logs"
DOWNLOADS="$VENDOR/downloads"
OPT=-O3
JOBS=${JOBS:-8}
BUILD_TIME_LIMIT=${BUILD_TIME_LIMIT:-10800}

case "$(uname -s)" in
    Darwin)
        CC=${CC:-/usr/bin/clang}
        CXX=${CXX:-/usr/bin/clang++}
        if [ -z "${HOMEBREW_PREFIX:-}" ]; then
            HOMEBREW_PREFIX=/opt/homebrew
            if [ ! -d "$HOMEBREW_PREFIX/opt/glib" ] && [ -d /usr/local/opt/glib ]; then
                HOMEBREW_PREFIX=/usr/local
            fi
        fi
        GLIB=${GLIB_PREFIX:-$HOMEBREW_PREFIX/opt/glib}
        GETTEXT=${GETTEXT_PREFIX:-$HOMEBREW_PREFIX/opt/gettext}
        OPENSSL_PREFIX=${OPENSSL_PREFIX:-$HOMEBREW_PREFIX/opt/openssl@3}
        GLIB_CFLAGS="-I$GLIB/include/glib-2.0 -I$GLIB/lib/glib-2.0/include -I$GETTEXT/include"
        GLIB_LIBS="-L$GLIB/lib -lgio-2.0 -lgobject-2.0 -lgmodule-2.0 -lglib-2.0 -L$GETTEXT/lib -lintl"
        ;;
    *)
        CC=${CC:-cc}
        CXX=${CXX:-c++}
        OPENSSL_PREFIX=${OPENSSL_PREFIX:-}
        GLIB_CFLAGS=$(pkg-config --cflags gio-2.0 gmodule-2.0)
        GLIB_LIBS=$(pkg-config --libs gio-2.0 gmodule-2.0)
        ;;
esac

GMIME_VERSION=3.2.15
GMIME_TARBALL=gmime-$GMIME_VERSION.tar.xz
GMIME_URL=https://github.com/jstedfast/gmime/releases/download/$GMIME_VERSION/$GMIME_TARBALL
GMIME_SHA256=84cd2a481a27970ec39b5c95f72db026722904a2ccf3fdbd57b280cf2d02b5c4

VMIME_COMMIT=5b0191136f84c177b737c9cf9aa7cf59d1c65ef1
VMIME_TARBALL=vmime-5b0191136f84.tar.gz
VMIME_URL=https://github.com/kisli/vmime/archive/$VMIME_COMMIT.tar.gz
VMIME_SHA256=41d31b91f436d3ffbb4c5bc339770dadb35a53425bf22f36b593d196f02c0c56

LIBETPAN_VERSION=1.10.1
LIBETPAN_TARBALL=libetpan-$LIBETPAN_VERSION.tar.gz
LIBETPAN_URL=https://github.com/dinhvh/libetpan/archive/refs/tags/$LIBETPAN_VERSION.tar.gz
LIBETPAN_SHA256=87bacdc62661a2a7aa5fe9f1f28d2f7c7a53256633ac5129903916c59f80c4c2

DOVECOT_VERSION=2.4.5
DOVECOT_TARBALL=dovecot-$DOVECOT_VERSION.tar.gz
DOVECOT_URL=https://dovecot.org/releases/2.4/$DOVECOT_TARBALL
DOVECOT_SHA256=868c2686a61b5f8e00a3e4721789b1ab46e6528fd773a5fbed07a6ecba7731e6

step() {
    if [ -n "${BUILD_WRAP:-}" ]; then
        "$BUILD_WRAP" perl -e 'alarm shift; exec @ARGV' "$BUILD_TIME_LIMIT" "$@"
    else
        "$@"
    fi
}

sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1"
    else
        shasum -a 256 "$1"
    fi | cut -d ' ' -f 1
}

fetch() {
    file="$DOWNLOADS/$1"
    if [ ! -f "$file" ]; then
        curl -fsSL -m 300 -o "$file.part" "$2"
        mv "$file.part" "$file"
    fi
    actual=$(sha256 "$file")
    if [ "$actual" != "$3" ]; then
        echo "checksum mismatch for $1: expected $3, got $actual" >&2
        exit 1
    fi
    echo "verified $1"
}

only=${1:-all}
case "$only" in
    all | gmime | vmime | libetpan | dovecot) ;;
    *)
        echo "usage: $0 [all|gmime|vmime|libetpan|dovecot]" >&2
        exit 2
        ;;
esac

mkdir -p "$DOWNLOADS" "$BUILD" "$PREFIX" "$LOGS"
fetch "$GMIME_TARBALL" "$GMIME_URL" "$GMIME_SHA256"
fetch "$VMIME_TARBALL" "$VMIME_URL" "$VMIME_SHA256"
fetch "$LIBETPAN_TARBALL" "$LIBETPAN_URL" "$LIBETPAN_SHA256"
fetch "$DOVECOT_TARBALL" "$DOVECOT_URL" "$DOVECOT_SHA256"

build_gmime() {
    src="$BUILD/gmime-$GMIME_VERSION"
    rm -rf "$src" "$PREFIX/gmime"
    step tar -xJf "$DOWNLOADS/$GMIME_TARBALL" -C "$BUILD"
    cd "$src"
    step ./configure --prefix="$PREFIX/gmime" \
        CC="$CC" CFLAGS="$OPT" \
        GLIB_CFLAGS="$GLIB_CFLAGS" GLIB_LIBS="$GLIB_LIBS" \
        ZLIB_CFLAGS=" " ZLIB_LIBS="-lz" \
        --disable-crypto --without-libidn --enable-introspection=no --enable-vala=no \
        --disable-gtk-doc --enable-static --disable-shared --disable-dependency-tracking \
        > "$LOGS/gmime-configure.log" 2>&1
    step make -j"$JOBS" -C util > "$LOGS/gmime-make.log" 2>&1
    step make -j"$JOBS" -C gmime >> "$LOGS/gmime-make.log" 2>&1
    step make -C gmime install >> "$LOGS/gmime-make.log" 2>&1
    cd "$VENDOR"
    echo "gmime $GMIME_VERSION: $PREFIX/gmime"
}

build_vmime() {
    src="$BUILD/vmime-$VMIME_COMMIT"
    out="$BUILD/vmime-build"
    rm -rf "$src" "$out" "$PREFIX/vmime"
    step tar -xzf "$DOWNLOADS/$VMIME_TARBALL" -C "$BUILD"
    mkdir -p "$out"
    step cmake -S "$src" -B "$out" -G "Unix Makefiles" \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_C_COMPILER="$CC" -DCMAKE_CXX_COMPILER="$CXX" \
        -DCMAKE_CXX_FLAGS_RELEASE="$OPT -DNDEBUG" \
        -DCMAKE_INSTALL_PREFIX="$PREFIX/vmime" \
        -DCMAKE_INSTALL_LIBDIR=lib \
        -DVMIME_BUILD_SHARED_LIBRARY=OFF -DVMIME_BUILD_STATIC_LIBRARY=ON \
        -DVMIME_HAVE_MESSAGING_FEATURES=OFF -DVMIME_HAVE_SASL_SUPPORT=OFF \
        -DVMIME_HAVE_TLS_SUPPORT=OFF -DVMIME_CHARSETCONV_LIB=iconv \
        -DVMIME_BUILD_SAMPLES=OFF -DVMIME_BUILD_TESTS=OFF -DVMIME_BUILD_DOCUMENTATION=OFF \
        > "$LOGS/vmime-cmake.log" 2>&1
    step make -j"$JOBS" -C "$out" install > "$LOGS/vmime-make.log" 2>&1
    echo "vmime $VMIME_COMMIT: $PREFIX/vmime"
}

LIBETPAN_SOURCES="
src/low-level/imf/mailimf.c
src/low-level/imf/mailimf_types.c
src/low-level/imf/mailimf_types_helper.c
src/low-level/imf/mailimf_write_file.c
src/low-level/imf/mailimf_write_generic.c
src/low-level/imf/mailimf_write_mem.c
src/low-level/mime/mailmime.c
src/low-level/mime/mailmime_content.c
src/low-level/mime/mailmime_decode.c
src/low-level/mime/mailmime_disposition.c
src/low-level/mime/mailmime_types.c
src/low-level/mime/mailmime_types_helper.c
src/low-level/mime/mailmime_write_file.c
src/low-level/mime/mailmime_write_generic.c
src/low-level/mime/mailmime_write_mem.c
src/data-types/base64.c
src/data-types/carray.c
src/data-types/charconv.c
src/data-types/chash.c
src/data-types/clist.c
src/data-types/mmapstring.c
src/data-types/timeutils.c
"

build_libetpan() {
    src="$BUILD/libetpan-$LIBETPAN_VERSION"
    include="$PREFIX/libetpan/include/libetpan"
    objects="$BUILD/libetpan-objects"
    rm -rf "$src" "$objects" "$PREFIX/libetpan"
    step tar -xzf "$DOWNLOADS/$LIBETPAN_TARBALL" -C "$BUILD"
    cd "$src"
    step tar -xzf build-mac/autogen-result.tar.gz
    step ./configure CC="$CC" CFLAGS="$OPT" \
        --disable-db --without-openssl --without-gnutls --without-sasl \
        --with-curl=no --with-expat=no --without-zlib --disable-lockfile \
        --disable-dependency-tracking \
        > "$LOGS/libetpan-configure.log" 2>&1 || true
    test -f config.h
    mkdir -p "$include" "$PREFIX/libetpan/lib" "$objects"
    step sh -c '"$1" -E -I. - < libetpan-config.h.in' sh "$CC" \
        | sed -e '/^#/d;/^[ \t]*$/d;s/^@/#/' > "$include/libetpan-config.h"
    cp src/low-level/imf/*.h src/low-level/mime/*.h src/data-types/*.h "$include/"
    step sh -c '
        set -e
        src=$1 objects=$2 include=$3 cc=$4 opt=$5
        shift 5
        cd "$src"
        for file in "$@"; do
            "$cc" $opt -DHAVE_CONFIG_H -I. -iquote "$include" -I"$include/.." \
                -Isrc/low-level/imf -Isrc/low-level/mime -Isrc/data-types \
                -c "$file" -o "$objects/$(basename "$file" .c).o"
        done
    ' sh "$src" "$objects" "$include" "$CC" "$OPT" $LIBETPAN_SOURCES \
        > "$LOGS/libetpan-make.log" 2>&1
    step ar rcs "$PREFIX/libetpan/lib/libetpan-mime.a" "$objects"/*.o
    cd "$VENDOR"
    echo "libetpan $LIBETPAN_VERSION: $PREFIX/libetpan"
}

DOVECOT_LIBS="lib:lib charset:lib-charset mail:lib-mail"

build_dovecot() {
    src="$BUILD/dovecot-$DOVECOT_VERSION"
    include="$PREFIX/dovecot/include"
    rm -rf "$src" "$PREFIX/dovecot"
    step tar -xzf "$DOWNLOADS/$DOVECOT_TARBALL" -C "$BUILD"
    cd "$src"
    openssl_cppflags=
    openssl_ldflags=
    if [ -n "$OPENSSL_PREFIX" ]; then
        openssl_cppflags="-I$OPENSSL_PREFIX/include"
        openssl_ldflags="-L$OPENSSL_PREFIX/lib"
    fi
    step ./configure CC="$CC" CFLAGS="$OPT" \
        CPPFLAGS="$openssl_cppflags" LDFLAGS="$openssl_ldflags" \
        ZLIB_CFLAGS=" " ZLIB_LIBS=-lz am_cv_func_iconv_works=yes \
        --enable-static --disable-shared --without-shared-libs \
        --without-pam --without-bsdauth --without-gssapi --without-ldap --without-libunwind \
        --without-cdb --without-sql --without-pgsql --without-mysql --without-sqlite \
        --without-cassandra --without-stemmer --without-textcat --without-icu \
        --without-pcre2 --without-solr --without-flatcurve --without-sodium --without-bzlib \
        --without-lz4 --without-zstd --without-libcap --without-lua \
        --disable-dependency-tracking \
        > "$LOGS/dovecot-configure.log" 2>&1
    : > "$LOGS/dovecot-make.log"
    mkdir -p "$include" "$PREFIX/dovecot/lib"
    cp config.h "$include/"
    for entry in $DOVECOT_LIBS; do
        name=${entry%%:*}
        dir=src/${entry#*:}
        step make -j"$JOBS" -C "$dir" "lib$name.la" >> "$LOGS/dovecot-make.log" 2>&1
        cp "$dir/.libs/lib$name.a" "$PREFIX/dovecot/lib/libdovecot-$name.a"
        cp "$dir"/*.h "$include/"
    done
    cd "$VENDOR"
    echo "dovecot $DOVECOT_VERSION: $PREFIX/dovecot"
}

case "$only" in
    gmime) build_gmime ;;
    vmime) build_vmime ;;
    libetpan) build_libetpan ;;
    dovecot) build_dovecot ;;
    all)
        build_gmime
        build_vmime
        build_libetpan
        build_dovecot
        ;;
esac
