# mail-parser compared with other email parsers

This crate measures the throughput of mail-parser 1.0 and of other Rust and
C/C++ email parsers on the same messages, with every library doing the same
work. It is a standalone crate (its own `[workspace]`, never published) that
depends on the mail-parser in this repository by path and builds the C and
C++ libraries from their release sources.

Results measured on a Mac mini with an Apple M4 on 2026-09-28 are in
[`results/summary.md`](results/summary.md) (throughput, median of three
runs) and [`results/check.md`](results/check.md) (equivalence check).
mail-parser 1.0 is the fastest implementation in every workload and corpus.

## Requirements

- macOS or Linux, Rust 1.98 or later, a C and C++ toolchain (Apple clang on
  macOS), `make`, `curl`, `tar` with xz support, `perl`.
- GLib 2.x headers and libraries (GMime needs them; the C shims also use
  GLib's UTF-8 validator). On macOS: `brew install glib`. On Linux: the GLib
  development package; `build.rs` and `vendor/build.sh` find it with
  `pkg-config`.
- CMake (VMime). On macOS: `brew install cmake`.
- OpenSSL headers, only because Dovecot's `configure` refuses to run without
  them; nothing from OpenSSL is compiled into or linked with the benchmark.
  On macOS `vendor/build.sh` uses Homebrew's `openssl@3`; set
  `OPENSSL_PREFIX` to use another installation.
- About 3 GB of disk for the Enron corpus (archive and extracted maildir).

Only macOS (Apple silicon, Homebrew) has been verified. The Linux paths are
there to make a port cheap, not tested.

## Run the comparison

All commands run from this directory (`benches/compare`).

1. Fetch the external corpora (Enron and the Stalwart SMTP test messages)
   into the repository's `target/corpora`, or into `$MAIL_PARSER_CORPORA`:

   ```sh
   ../../scripts/fetch-corpora.sh enron stalwart
   ```

2. Download, verify and build the C/C++ libraries into `vendor/prefix/`
   (a few minutes; logs in `vendor/logs/`):

   ```sh
   vendor/build.sh
   ```

   The script checks the SHA-256 of each source archive and stops on a
   mismatch. It takes `all` (default), `gmime`, `vmime`, `libetpan` or
   `dovecot`.

3. Build, check and measure:

   ```sh
   ./run.sh
   ```

   `run.sh` builds the crate and the benchmark, runs the equivalence check
   (`results/check.md`), measures three runs of the whole matrix (231
   benchmarks, about 20 minutes per run) and writes the median of the runs to
   `results/summary.md`. `RUNS=1 ./run.sh` measures once.

Close other applications first: the numbers are only as good as the machine
is idle.

### Individual steps

```sh
cargo build --release --bins
cargo bench --no-run > target/bench-build.log 2>&1
target/release/check --out results/check.md
target/release/check --corpus enron-crlf --workload full --examples 10
./measure.sh 1
./measure.sh 1 full-a full-b
cargo bench --bench compare -- '^headers/testsuite-crlf/'
target/release/summary target/compare-runs/r*/criterion --out results/summary.md
```

`measure.sh <run> [session...]` measures one run of the matrix as six
sessions of a few minutes each (`structure-a`, `structure-b`, `headers-a`,
`headers-b`, `full-a`, `full-b`: one workload and half of the corpora each)
and copies each session's criterion results to
`target/compare-runs/r<run>/criterion`. It logs the busiest processes every
10 seconds next to each session's output, and stops before the next session
when `target/compare-runs/STOP` exists.

`summary` reads one or more criterion directories (default
`target/criterion`) and prints the median MiB/s of every implementation,
workload and corpus, the speed of mail-parser 1.0 divided by the speed of
each other implementation (`1.0 / x`: above 1 means mail-parser 1.0 is
faster) and, with several runs, the spread of each cell ((max - min) /
median). `--exclude <implementation>` leaves an implementation out of the
table; `check` takes the same option.

Benchmark ids are `<workload>/<corpus>/<implementation>`, so criterion's
filter selects any subset. Criterion settings: 0.5 s warm-up, 3 s
measurement, 30 samples, throughput in bytes of the corpus.

### Environment variables

| Variable | Used by | Effect |
|---|---|---|
| `MAIL_PARSER_CORPORA` | corpus loader | directory of the external corpora (default: `target/corpora` of the repository) |
| `RUNS` | `run.sh` | number of measured runs (default 3) |
| `EXCLUDE` | `run.sh` | implementations left out of `check.md` and `summary.md`, separated by spaces |
| `BUILD_WRAP` | all scripts | command that wraps every build step, for example a machine-wide build lock; the step also gets a time limit (`BUILD_TIME_LIMIT`, default 10800 s) |
| `BENCH_LOCK` | `measure.sh` | path of a [bench-lock](#machine-wide-locking)-style wrapper for the measurements (`<wrapper> <label> <command>`); flagged sessions are rerun (`ATTEMPTS`, default 5, `PAUSE` seconds apart, default 240) |
| `GLIB_PREFIX`, `GETTEXT_PREFIX`, `HOMEBREW_PREFIX`, `OPENSSL_PREFIX` | `build.rs`, `vendor/build.sh` | library locations on macOS (default: Homebrew's `opt` links) |
| `CC`, `CXX`, `JOBS` | `vendor/build.sh`, `build.rs` | compilers (default `/usr/bin/clang` and `/usr/bin/clang++` on macOS, `cc` and `c++` elsewhere) and `make -j` |

### Machine-wide locking

The published numbers were measured on a machine shared with other work,
through two wrappers: one that holds builds back while a measurement runs
(`BUILD_WRAP`), and one that takes a machine-wide lock, waits until the CPU
is idle, samples the idle time during the run and records a session as noisy
in `state/noisy.log` next to itself when the machine got busy (`BENCH_LOCK`).
Without these variables the scripts run every command directly.

## Implementations

| Id | Library | Version | Language | Source |
|---|---|---|---|---|
| `mail-parser-1.0` | mail-parser | 1.0.0 (this repository, feature `full_encoding`) | Rust | `../..` |
| `mail-parser-0.11` | mail-parser | 0.11.9 (feature `full_encoding`) | Rust | crates.io |
| `mailparse` | mailparse | 0.17.0 | Rust | crates.io |
| `gmime` | GMime | 3.2.15 (2024-06-20, latest release) | C (GLib) | release tarball |
| `vmime` | VMime | master at `5b0191136f84c177b737c9cf9aa7cf59d1c65ef1` (2026-03-24) | C++ | GitHub archive of that commit |
| `libetpan` | libEtPan! | 1.10.1 (2026-06-14, latest release) | C | GitHub tag archive |
| `dovecot` | Dovecot lib-mail, lib-charset and lib | 2.4.5 (2026-08-28, latest release) | C | dovecot.org release tarball |

mail-parser 0.11 is part of the harness to measure the rewrite against the
previous release; the published tables leave it out.

### Choice of libraries

Rust crates, from the crates.io API on 2026-09-26 (total downloads,
downloads in the last 90 days, crates depending on it):

| Crate | Version | Total | Last 90 days | Reverse deps | Compared |
|---|---|---:|---:|---:|---|
| mailparse | 0.17.0 (2026-09-06) | 14,169,109 | 3,808,629 | 106 | yes |
| mail-parser | 0.11.9 | 3,820,512 | 1,531,183 | 143 | yes |
| eml-parser | 0.1.5 (2025-11-30) | 1,279,717 | 224,356 | 2 | no |
| email (rust-email) | 0.0.21 (2020-02-16) | 1,112,663 | 77,620 | 8 | no |
| email-format | 0.8.1 (2022-07-27) | 67,242 | 603 | 3 | no |
| melib | 0.8.13 (2026-01-05) | 35,027 | 362 | 1 | no |
| rustyknife | 0.2.11 (2021-01-04) | 30,215 | 493 | 3 | no |
| email-parser | 0.5.0 (2020-12-17) | 24,057 | 499 | 2 | no |
| eml-codec | 0.4.1 (2026-08-18) | 4,411 | 153 | 0 | no |
| mailparsing (KumoMTA) | not published on crates.io | | | | no |

- mailparse is the most used Rust email parser by a wide margin.
- eml-parser's downloads come from one dependent, Nushell's `from eml`
  plugin. It does no MIME work (no part structure, no transfer or charset
  decoding of bodies, no Date or Message-ID parsing) and only takes a UTF-8
  `String`, so it cannot run any of the three workloads.
- email (rust-email) has had no release since 2020; most of its downloads
  come through the builder of the old lettre_email 0.9.
- The others have under 1,000 downloads in the last 90 days.

C and C++ libraries:

- GMime is the most widely used C MIME library (notmuch, Balsa, Pan).
- VMime is the maintained C++ mail library. Its only tagged release, v0.9.2,
  is from 2017 and development continues on master, so the benchmark pins a
  master commit.
- libEtPan! is maintained (1.10 and 1.10.1 were released in 2026) and used
  by Claws Mail.
- Dovecot's lib-mail is the MIME parser of the most deployed IMAP server:
  `message-parser` (streaming MIME structure), `message-header-parser`,
  `message-address`, `message-date`, `message-id`, `message-header-decode`,
  `rfc822-parser`, `rfc2231-parser` and `message-decoder` (transfer decoding
  plus conversion to UTF-8 through lib-charset and iconv). It is not a
  standalone library, but its static convenience libraries build on their
  own from the release tarball.
- mimetic (C++) is not compared: its last release, 0.9.8, is from 2014.
- Not considered further: KMime (needs Qt), mailio (needs Boost), POCO's
  `MailMessage` (a small part of a large framework), and the MIME parsers
  built into Cyrus, rspamd, Postfix or Thunderbird (not standalone
  libraries).

### Build of the C and C++ libraries

`vendor/build.sh` builds each library from its source archive, static, at
`-O3`, into `vendor/prefix/<library>`; nothing is installed elsewhere.

| Library | Build | Flags |
|---|---|---|
| GMime 3.2.15 | release `configure`, `make` of `util` and `gmime` | `CFLAGS=-O3`, `--disable-crypto --without-libidn --enable-introspection=no --enable-vala=no`; `GLIB_CFLAGS`/`GLIB_LIBS` given explicitly (Homebrew GLib on macOS, `pkg-config` elsewhere), `ZLIB_LIBS=-lz`; charset conversion through the system iconv |
| VMime master | CMake (Unix Makefiles), Release | `CMAKE_CXX_FLAGS_RELEASE="-O3 -DNDEBUG"`, messaging, TLS and SASL off, `VMIME_CHARSETCONV_LIB=iconv` |
| libEtPan! 1.10.1 | the tag has no `configure`; `build-mac/autogen-result.tar.gz` (the autogen output its Xcode build uses) provides one, run only to generate `config.h` (its version string says 1.6: the archive is stale, the sources are 1.10.1); the 22 IMF, MIME and data-type sources the parser needs are compiled directly into `libetpan-mime.a`, as the Xcode project does | `-O3`, iconv on, no SSL, SASL, DB, curl, expat or zlib |
| Dovecot 2.4.5 | release `configure`, then `make` of the `liblib.la`, `libcharset.la` and `libmail.la` targets only, installed as `libdovecot-{lib,charset,mail}.a` with their headers and `config.h` | `CFLAGS=-O3`, `--enable-static --disable-shared --without-shared-libs`, every optional dependency off; `am_cv_func_iconv_works=yes` (gnulib's iconv self-test rejects macOS's iconv, which the other C libraries use anyway; without the override lib-charset would convert nothing); OpenSSL include and library paths for `configure`'s checks only; `ZLIB_CFLAGS=" " ZLIB_LIBS=-lz` |

The Dovecot archive's SHA-256 equals Homebrew's formula checksum, and its
GPG signature (`dovecot-2.4.5.tar.gz.sig` on dovecot.org) verifies with the
Dovecot CE key `EF0882079FD4ED32BF8B23B2A1B09EF84EDC5219`.

The shims in `shim/` (`gmime.c`, `vmime.cpp`, `libetpan.c`, `dovecot.c`,
and `utf8.c`) are compiled by `build.rs` with the `cc` crate at `-O3`
(`-std=gnu11`, `-std=c++17`). The Rust code uses the release profile
(opt-level 3). `Cargo.lock` pins mailparse 0.17.0 (charset 0.1.5), mail-parser
0.11.9, criterion 0.8.2 and cc 1.5.1. `src/ffi.rs` is the only module with
`unsafe`.

## Corpora

Eleven corpora, 17.8 MiB and 3,292 messages in total:

| Corpus | Messages | Content |
|---|---:|---|
| `testsuite-lf`, `testsuite-crlf` | 123 | the RFC, legacy, third-party and malformed test messages of `resources/eml`, the Stalwart SMTP test messages and `resources/sieve`, with LF and with CRLF line endings |
| `enron-crlf` | 3,000 | the first 20 messages of each mailbox of the Enron corpus (CMU, 2015-05-07 release) |
| `attachments-crlf`, `attachments-lf` | 3 | generated: quoted-printable text and HTML alternatives with large base64 PDF and PNG attachments |
| `newsletter-crlf`, `newsletter-lf` | 6 | generated: 8bit text and quoted-printable HTML with an inline base64 image (`multipart/related`) |
| `modern-headers-crlf` | 20 | generated: long header blocks (Received chains, ARC, DKIM, List-Unsubscribe) over short quoted-printable alternatives |
| `forwarded-crlf` | 6 | generated: 1 to 6 levels of forwarded `message/rfc822` |
| `dashes-crlf` | 1 | generated: three 200 KB parts whose bodies contain lines that start with the boundary |
| `plain-large-crlf` | 1 | generated: one 1 MB `text/plain; charset=utf-8` body |

The generated corpora come from a fixed seed (`src/corpus/synthetic.rs`), so
every machine gets the same bytes. `check.md` lists the size and an FNV-1a
digest of every corpus: equal digests mean equal input. `check` and the
benchmark warn when a corpus cannot be loaded.

## Workloads

Each benchmark iteration handles every message of a corpus. Each message is
parsed, read and freed as a user would, with no state kept between messages
besides one-time library initialization (`g_mime_init`, Dovecot's
`lib_init`). Every value read feeds a digest (lengths, timestamps;
`black_box` in Rust), and C/C++ calls go through one function per workload
taking `(pointer, length)`.

1. `structure`: parse, then walk every MIME part, nested messages included,
   and read its content type and subtype.
2. `headers`: parse, then read the root message's decoded Subject, the first
   From mailbox (display name and address; groups flattened), the Date as a
   Unix timestamp and the Message-ID (without angle brackets); then walk the
   parts, nested messages included.
3. `full`: parse, then read every leaf body decoded: transfer decoding for
   every leaf and charset conversion to UTF-8 for `text/*` leaves, ending with
   text the caller can use as UTF-8.

Nested messages are walked the same way in all three workloads. mail-parser
parses `message/rfc822` and `message/global` parts during `parse`. When a
library hands such a body back as bytes, the shim transfer-decodes the bytes
and parses them again with the same library (up to 8 levels). In `headers`
the walk reads nothing: it only makes each library parse what mail-parser's
`parse` parses. Each implementation has one tree walk shared by the three
workloads.

UTF-8 in `full`: mail-parser's `text()` validates UTF-8 even when no
conversion is needed. A C library that returns the raw bytes of a UTF-8 (or
undeclared) text body without checking them gives the caller bytes that may
not be UTF-8, so its shim validates them as a user would, with GLib's
`g_utf8_validate_len` (`shim/utf8.c`). This applies to libEtPan! (charset
UTF-8, or a failed `charconv_buffer`), VMime (charset UTF-8, or a failed
`charset::convert`) and GMime (no charset parameter: `g_mime_text_part_get_text`
then returns the bytes unconverted). GMime with a charset converts through
iconv, Dovecot's `message-decoder` validates UTF-8 itself (U+FFFD for
invalid sequences), mailparse decodes through `charset` and encoding_rs, and
mail-parser 0.11 builds `String`s: none of these needs a separate check.

### Calls per implementation

| Implementation | Parse | structure | headers | full |
|---|---|---|---|---|
| mail-parser 1.0 | `MessageParser::default().parse(raw)` | `Message::parts()` (every part of every nested message), `content_type()`, `ctype()`, `subtype()` | `subject()`, `from()?.first()` with `name()`, `address()`; `date()?.to_timestamp()`; `message_id()` | `PartKind::Text`/`Html`: `text()`; `Binary`/`InlineBinary`: `decoded()` |
| mail-parser 0.11 | `MessageParser::default().parse(raw)` (decodes every body) | `message.parts` of the root and of every `PartType::Message`, `MimeHeaders::content_type()` | the same getters as 1.0 | the already decoded `PartType::Text`/`Html`/`Binary`/`InlineBinary` bodies |
| mailparse | `parse_mail(raw)` | `subparts` tree, `ctype.mimetype` split at `/`; nested messages are leaves: `get_body_raw()`, then `parse_mail` again | `headers.get_first_value("Subject")`, `addrparse_header(get_first_header("From"))` first mailbox, `dateparse(get_first_value("Date"))`, `msgidparse(get_first_value("Message-ID"))` first id | `text/*`: `get_body()`; others: `get_body_raw()` |
| GMime | `g_mime_stream_mem_new_with_buffer`, `g_mime_parser_new_with_stream`, `g_mime_parser_construct_message` | `g_mime_message_get_mime_part`, `GMimeMultipart` children, `g_mime_message_part_get_message`; `g_mime_object_get_content_type`, `g_mime_content_type_get_media_type`/`_subtype` | `g_mime_message_get_subject`, `g_mime_message_get_from` first mailbox (`internet_address_get_name`, `internet_address_mailbox_get_addr`), `g_mime_message_get_date` with `g_date_time_to_unix`, `g_mime_message_get_message_id` | `GMimeTextPart`: `g_mime_text_part_get_text`; other `GMimePart`: `g_mime_data_wrapper_write_to_stream` into a memory stream; transfer-encoded `message/rfc822` (kept as leaves): decoded, then parsed from the memory stream |
| VMime | `vmime::message::parse(std::string)` | `body::getContentType()` over `getPartAt()`; nested messages are leaves: `contentHandler::extract()`, then parsed as a new `vmime::message` | `findField` for Subject (`text::getConvertedText(utf-8)`), From (`mailbox`: `getName().getConvertedText`, `getEmail().toString()`), Date (`datetime` to a timestamp), Message-ID (`messageId::getId()`) | `contentHandler::extract()`; `text/*`: `charset::convert(..., utf-8)` unless the charset is UTF-8 |
| libEtPan! | `mailmime_parse(raw, len, &index, &mime)` | `mailmime` tree (`MAILMIME_MULTIPLE` children, `MAILMIME_MESSAGE` nested root), `mm_content_type` type and subtype; transfer-encoded nested messages (kept as leaves): `mailmime_part_parse`, then `mailmime_parse` again | root `mm_fields`: Subject and the From display name through `mailmime_encoded_phrase_parse(..., "utf-8")`, first `mailimf_mailbox`, `mailimf_date_time` to a timestamp, `mid_value` | `MAILMIME_SINGLE`: `mailmime_part_parse`; `text/*`: `charconv_buffer` to UTF-8 unless the charset is UTF-8 |
| Dovecot | `i_stream_create_from_data`, `message_parser_init`, `message_parser_parse_next_block` until the end, `message_parser_deinit` (gives the `message_part` tree); `MESSAGE_PARSER_FLAG_SKIP_BODY_BLOCK` in `structure` and `headers` | `Content-Type` header blocks through `rfc822_parse_content_type`; the `message_part` tree (`children`, `next`) | root header blocks: Subject through `message_header_decode_utf8`, From through `message_address_parse` (display name through `message_header_decode_utf8`), `message_date_parse`, `message_id_get_next` | `message_decoder_init(NULL, MESSAGE_DECODER_FLAG_RETURN_BINARY)` fed every body block plus the `Content-Type` and `Content-Transfer-Encoding` header blocks |

## Equivalence check

`check` runs every implementation, workload and corpus once, in one worker
process per implementation, workload and corpus (a worker that crashes is
restarted after the message that crashed it, and the crash is counted), with
`catch_unwind` around every Rust call. For each cell it prints messages
parsed and rejected, panics, crashes, and the counts the workload produced
(messages walked, parts, leaves, text leaves, decoded bytes, headers found),
each with its difference from mail-parser 1.0 and the number of messages
whose counts differ. `--examples N` lists the first N differing messages of
each cell. The check shows whether the libraries do comparable work; it does
not judge their correctness.

Findings (`results/check.md`):

- No panic and no crash in any implementation, workload or corpus.
- Rejected messages, out of 123 in each test suite: mailparse rejects
  `malformed/019.eml` (a header line `Fro :...` with a space before the
  colon) and GMime rejects `malformed/023.eml` (its first line is not a
  header field). Every other library parses every message.
- Enron and the generated corpora give identical message, part and leaf
  counts for every implementation in all three workloads (`forwarded-crlf`:
  27 messages, 93 parts, 39 leaves everywhere), and decoded byte counts
  equal to 0.01%, except `dashes-crlf` and quoted-printable in LF (below).
- `dashes-crlf`: its bodies contain lines `--<boundary>x not a delimiter`.
  mailparse and Dovecot match boundaries by prefix and split the message
  into 1,064 leaves (1,402 decoded bytes instead of 611,851); mail-parser
  0.11 finds the 3 parts but also decodes only 1,402 bytes. mail-parser 1.0,
  GMime, VMime and libEtPan! follow RFC 2046 (a delimiter line is the
  boundary plus optional whitespace) and decode all 611,851 bytes. The
  `dashes-crlf` cells of mailparse and Dovecot measure different, and in
  `full` much less, work: they are not comparable.
- `headers`: all libraries find the same Subject, From, Date and Message-ID
  on the generated corpora. Enron: GMime finds no From mailbox in 327
  messages whose local part is not a valid dot-atom (`k..allen@enron.com`,
  `.hall@enron.com`), which the others accept. Test suites: mailparse finds
  3 fewer From mailboxes (`addrparse` rejects From fields without a domain)
  and 1 fewer Subject and Message-ID (the rejected message); mailparse and
  VMime return a date for `Date: Whenever`, mail-parser 1.0 does not;
  Dovecot finds 3 fewer Message-IDs (`message_id_get_next` requires an `@`)
  and one fewer Date.
- `structure`, test suites: part counts within 12 of 479. The differences
  come from malformed messages recovered differently, VMime ignoring the
  `multipart/digest` default type (digest parts stay text leaves, 5 fewer
  messages), and Dovecot parsing a base64-encoded `message/rfc822` body as
  if it were not encoded (its parser ignores the transfer encoding of nested
  messages).
- `full`, test suites: decoded bytes within 2% of mail-parser 1.0 for every
  library except Dovecot (-6%). The differences come from invalid or
  undeclared 8-bit text (mail-parser writes U+FFFD, 3 bytes, per invalid
  sequence; iconv-based conversions substitute or drop), malformed messages
  and digests recovered differently, and for Dovecot mostly from
  `x-uuencode` bodies: `message-decoder` skips a body whose transfer
  encoding it does not know, mail-parser decodes uuencode.
- Quoted-printable in the LF corpora: mailparse, libEtPan! and Dovecot write
  CRLF for the line breaks of quoted-printable bodies (the canonical form),
  so they decode 0.04% (`attachments-lf`) to 0.76% (`newsletter-lf`) more
  bytes.
- No corpus contains `message/global`. Dovecot's parser treats it as a leaf
  (only `message/rfc822` is parsed as nested) and its shim does not parse it
  again.

## Caveats

- GMime, VMime and libEtPan! keep transfer-encoded (base64 or
  quoted-printable) `message/rfc822` parts as leaves; the shims decode and
  parse them, as mail-parser does during `parse`. Dovecot parses nested
  messages without transfer-decoding them.
- mail-parser 0.11 decodes every body during `parse`, whatever the workload
  reads.
- The C/C++ digest uses `strlen` on returned C strings (subject, names,
  GMime's decoded text), a cost the Rust implementations do not have; it is
  small next to parsing.
- GMime copies the input into its memory stream, VMime into a
  `std::string`, and libEtPan! each 7bit or 8bit body into a new buffer
  (`mailmime_part_parse`), as their users do; these copies are part of the
  measured time. Dovecot and the Rust libraries read the input in place.
- Dovecot's `message-decoder` is fed only the two header fields it needs in
  `full`. Feeding every header block would add RFC 2047 decoding of every
  header, which no other implementation does in `full`.
- Throughput is the corpus size divided by the time to handle it, whether
  or not a workload reads every byte. In `structure` and `headers` no
  library needs to read the single 1 MB body of `plain-large-crlf`, so
  those cells measure one header parse and reach hundreds of GB/s; in
  `full` they measure one scan or validation of that body.
- The numbers come from one machine (Apple M4, macOS). Other CPUs, and GLib
  or iconv builds on other systems, can change the ratios.

## Layout

| Path | Content |
|---|---|
| `vendor/build.sh` | download, verify, build and install GMime, VMime, libEtPan! and Dovecot lib-mail; `vendor/downloads/`, `vendor/build/`, `vendor/prefix/` and `vendor/logs/` are generated |
| `shim/` | the C and C++ workload functions, one per library and workload, `tally.h` (the counters shared with Rust) and `utf8.c` (UTF-8 validation shared by the C/C++ shims) |
| `build.rs` | compiles the shims and links the libraries |
| `src/corpus/` | the corpus loader and the generated corpora |
| `src/impls/` | the Rust implementations of the workloads |
| `src/ffi.rs` | calls into the shims (the only `unsafe` code) |
| `src/tally.rs`, `src/workload.rs` | digest and counters, workload and implementation lists |
| `src/bin/check/` | equivalence check |
| `src/bin/summary.rs` | criterion results to a markdown table |
| `benches/compare.rs` | the criterion benchmark |
| `run.sh` | build, check, measure and summarize |
| `measure.sh` | one run of the matrix as six short sessions |
| `results/` | published results |
