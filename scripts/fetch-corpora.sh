#!/bin/sh
# Downloads the external test and benchmark corpora into one directory.
#
# Usage: scripts/fetch-corpora.sh [enron] [apache] [stalwart] [mail-auth]
# With no argument every corpus is fetched. The destination is
# $MAIL_PARSER_CORPORA, or target/corpora in the repository when unset; the
# benches and harnesses read the same variable. Every download is checked
# against scripts/corpora.sha256, and a second run only fetches what is
# missing or does not match.
#
# Layout:
#   enron/maildir/        Enron maildir (CMU, May 7 2015 release)
#   apache-mbox/          Apache mailing lists, January to August 2026
#   stalwart/tests/       Stalwart test resources (main branch)
#   stalwart-v1/tests/    Stalwart test resources (v1.0.0 branch)
#   stalwart-smtp/        Stalwart SMTP test messages (v1.0.0 branch)
#   mail-auth/resources/  mail-auth test messages
#
# Requires sh, curl, tar and sha256sum or shasum.

set -eu

ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
DEST=${MAIL_PARSER_CORPORA:-$ROOT/target/corpora}
MANIFEST=$ROOT/scripts/corpora.sha256

ENRON_URL=https://www.cs.cmu.edu/~enron/enron_mail_20150507.tar.gz
GITHUB=https://codeload.github.com/stalwartlabs
STALWART_MAIN=02503015580abd6706923d34e9d8e501e76f8634
STALWART_V1=f2aa517b994864ad027c6000a5cbb2f1eb199805
MAIL_AUTH=2e87f9ed1246bbf9ffb07f185ad02e3b1ca4b4e6
APACHE_URL=https://lists.apache.org/api/mbox.lua
APACHE_PAUSE=1

failures=0
downloaded=0

sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1"
    else
        shasum -a 256 "$1"
    fi | cut -d ' ' -f 1
}

expected() {
    while read -r sum name; do
        if [ "$name" = "$1" ]; then
            echo "$sum"
            return 0
        fi
    done <"$MANIFEST"
    echo "error: no checksum for $1 in $MANIFEST" >&2
    return 1
}

fail() {
    echo "error: $*" >&2
    failures=$((failures + 1))
    return 1
}

# fetch NAME URL SHA256: download DEST/NAME unless it is present and verifies.
fetch() {
    target=$DEST/$1
    downloaded=0
    if [ -f "$target" ] && [ "$(sha256 "$target")" = "$3" ]; then
        return 0
    fi
    mkdir -p "$(dirname "$target")"
    echo "fetching $1"
    if ! curl -fsSL --retry 3 --connect-timeout 30 --speed-limit 1024 --speed-time 120 \
        -o "$target.part" "$2"; then
        rm -f "$target.part"
        fail "download failed: $2"
        return 1
    fi
    downloaded=1
    actual=$(sha256 "$target.part")
    if [ "$actual" != "$3" ]; then
        rm -f "$target.part"
        fail "checksum mismatch for $1 (expected $3, got $actual; source $2)"
        return 1
    fi
    mv "$target.part" "$target"
}

# unpack ARCHIVE URL DIR STRIP [MEMBER]: extract MEMBER of ARCHIVE into DEST/DIR.
unpack() {
    archive=downloads/$1
    sum=$(expected "$archive") || {
        failures=$((failures + 1))
        return 1
    }
    marker=$DEST/$3.sha256
    if [ -d "$DEST/$3" ] && [ -f "$marker" ] && [ "$(cat "$marker")" = "$sum" ]; then
        echo "up to date: $3"
        return 0
    fi
    fetch "$archive" "$2" "$sum" || return 1
    echo "extracting $3"
    rm -rf "${DEST:?}/${3:?}" "$DEST/$3.tmp" "$marker"
    mkdir -p "$DEST/$3.tmp"
    if ! tar -xzf "$DEST/$archive" -C "$DEST/$3.tmp" --strip-components="$4" ${5:+"$5"}; then
        rm -rf "$DEST/$3.tmp"
        fail "cannot extract $5 from $archive"
        return 1
    fi
    mv "$DEST/$3.tmp" "$DEST/$3"
    echo "$sum" >"$marker"
}

enron() {
    unpack enron_mail_20150507.tar.gz "$ENRON_URL" enron 0 || true
}

apache() {
    count=0
    while read -r sum name; do
        case $name in
            apache-mbox/*.mbox) ;;
            *) continue ;;
        esac
        base=${name#apache-mbox/}
        base=${base%.mbox}
        month=${base##*.}
        rest=${base%.*}
        list=${rest##*.}
        domain=${rest%.*}
        fetch "$name" "$APACHE_URL?list=$list&domain=$domain&d=$month" "$sum" || true
        if [ "$downloaded" = 1 ]; then
            sleep "$APACHE_PAUSE"
        fi
        count=$((count + 1))
    done <"$MANIFEST"
    echo "apache-mbox: $count archives checked"
}

stalwart() {
    unpack "stalwart-$STALWART_MAIN.tar.gz" "$GITHUB/stalwart/tar.gz/$STALWART_MAIN" \
        stalwart 1 "stalwart-$STALWART_MAIN/tests/resources" || true
    unpack "stalwart-$STALWART_V1.tar.gz" "$GITHUB/stalwart/tar.gz/$STALWART_V1" \
        stalwart-v1 1 "stalwart-$STALWART_V1/tests/resources" || true
    unpack "stalwart-$STALWART_V1.tar.gz" "$GITHUB/stalwart/tar.gz/$STALWART_V1" \
        stalwart-smtp 5 "stalwart-$STALWART_V1/tests/resources/smtp/messages" || true
}

mail_auth() {
    unpack "mail-auth-$MAIL_AUTH.tar.gz" "$GITHUB/mail-auth/tar.gz/$MAIL_AUTH" \
        mail-auth 1 "mail-auth-$MAIL_AUTH/resources" || true
}

if [ "$#" -eq 0 ]; then
    set -- enron apache stalwart mail-auth
fi
mkdir -p "$DEST"
echo "corpora directory: $DEST"
for corpus in "$@"; do
    case $corpus in
        enron) enron ;;
        apache) apache ;;
        stalwart) stalwart ;;
        mail-auth) mail_auth ;;
        *) fail "unknown corpus $corpus (expected enron, apache, stalwart or mail-auth)" || true ;;
    esac
done

if [ "$failures" -ne 0 ]; then
    echo "$failures failure(s); run the script again to retry" >&2
    exit 1
fi
rm -rf "$DEST/downloads"
echo "all requested corpora are present and verified"
