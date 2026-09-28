# Throughput of mail-parser 1.0 and other email parsers

Measured on 2026-09-28 on a Mac mini with an Apple M4 (4 performance and 6
efficiency cores, 16 GB), macOS 26.6.2, rustc 1.98.1, Apple clang 21.0.0
(clang-2100.3.34.2), Homebrew GLib 2.90.0 and CMake 4.4.3. Library versions
and build flags are in the [README](../README.md#implementations).

Method: three runs of the whole matrix (`./measure.sh 1`, `2`, `3`), each
run as six sessions of about two and a half minutes. Every session ran under
a machine-wide lock that waited for at least 92% idle CPU before starting
and sampled the idle time during the run; none of the 18 sessions was
flagged as busy, and none was repeated. Each cell is the median over the
three runs of criterion's median (30 samples, 3 s of measurement after 0.5 s
of warm-up). Throughput is the size of the corpus divided by the time to
handle all of its messages. `1.0 / x` is the throughput of mail-parser 1.0
divided by that of the other library: above 1 means mail-parser 1.0 is
faster.

Reading the table:

- The `dashes-crlf` cells of mailparse and Dovecot are not comparable: both
  split that message at lines that only start with the boundary and decode
  1,402 bytes where the other libraries decode 611,851 (see the
  [equivalence check](check.md)).
- `plain-large-crlf` is one message with a single 1 MB text body. In
  `structure` and `headers` no library needs to read that body, so those
  cells measure a header parse against a megabyte of input and reach
  hundreds of GB/s.
- The equivalence check behind these numbers is in [check.md](check.md).

Median of 3 runs: MiB/s, and mail-parser 1.0 divided by each other implementation.

| workload | corpus | mail-parser-1.0 MiB/s | mailparse MiB/s | gmime MiB/s | vmime MiB/s | libetpan MiB/s | dovecot MiB/s | 1.0 / mailparse | 1.0 / gmime | 1.0 / vmime | 1.0 / libetpan | 1.0 / dovecot |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| structure | testsuite-lf | 2078 | 830 | 120 | 19 | 228 | 766 | 2.50x | 17.25x | 107.87x | 9.11x | 2.71x |
| structure | testsuite-crlf | 2113 | 851 | 123 | 18 | 233 | 784 | 2.48x | 17.11x | 118.18x | 9.08x | 2.69x |
| structure | enron-crlf | 2774 | 1326 | 133 | 155 | 342 | 1223 | 2.09x | 20.90x | 17.85x | 8.10x | 2.27x |
| structure | attachments-crlf | 81210 | 32013 | 4707 | 1479 | 1362 | 5254 | 2.54x | 17.25x | 54.91x | 59.63x | 15.46x |
| structure | newsletter-crlf | 25502 | 4826 | 2173 | 650 | 526 | 3237 | 5.28x | 11.73x | 39.21x | 48.46x | 7.88x |
| structure | modern-headers-crlf | 3839 | 1691 | 471 | 337 | 588 | 1908 | 2.27x | 8.15x | 11.39x | 6.53x | 2.01x |
| structure | forwarded-crlf | 2951 | 969 | 225 | 163 | 154 | 1319 | 3.05x | 13.14x | 18.14x | 19.12x | 2.24x |
| structure | dashes-crlf | 11912 | 414 | 2295 | 1933 | 1092 | 614 | 28.80x | 5.19x | 6.16x | 10.91x | 19.39x |
| structure | plain-large-crlf | 836089 | 658492 | 3190 | 14725 | 237513 | 6972 | 1.27x | 262.07x | 56.78x | 3.52x | 119.92x |
| structure | attachments-lf | 80959 | 31830 | 4493 | 1472 | 1337 | 4917 | 2.54x | 18.02x | 55.00x | 60.55x | 16.46x |
| structure | newsletter-lf | 25381 | 4762 | 2130 | 648 | 524 | 3200 | 5.33x | 11.91x | 39.18x | 48.46x | 7.93x |
| headers | testsuite-lf | 2034 | 671 | 121 | 20 | 174 | 504 | 3.03x | 16.84x | 101.46x | 11.71x | 4.04x |
| headers | testsuite-crlf | 2089 | 687 | 122 | 16 | 176 | 514 | 3.04x | 17.06x | 127.70x | 11.84x | 4.06x |
| headers | enron-crlf | 2638 | 873 | 132 | 137 | 266 | 480 | 3.02x | 19.96x | 19.28x | 9.92x | 5.49x |
| headers | attachments-crlf | 80868 | 31344 | 4662 | 1471 | 1362 | 5205 | 2.58x | 17.35x | 54.96x | 59.36x | 15.54x |
| headers | newsletter-crlf | 25529 | 4671 | 2178 | 643 | 522 | 3074 | 5.46x | 11.72x | 39.71x | 48.94x | 8.30x |
| headers | modern-headers-crlf | 3805 | 1483 | 469 | 299 | 508 | 1390 | 2.57x | 8.12x | 12.71x | 7.48x | 2.74x |
| headers | forwarded-crlf | 2962 | 924 | 226 | 161 | 153 | 1179 | 3.21x | 13.11x | 18.43x | 19.37x | 2.51x |
| headers | dashes-crlf | 11781 | 400 | 2298 | 1913 | 1083 | 612 | 29.44x | 5.13x | 6.16x | 10.88x | 19.26x |
| headers | plain-large-crlf | 811217 | 424327 | 3188 | 14467 | 172706 | 6885 | 1.91x | 254.46x | 56.07x | 4.70x | 117.82x |
| headers | attachments-lf | 80278 | 31193 | 4496 | 1464 | 1341 | 4872 | 2.57x | 17.86x | 54.82x | 59.86x | 16.48x |
| headers | newsletter-lf | 25192 | 4589 | 2127 | 645 | 517 | 3038 | 5.49x | 11.85x | 39.05x | 48.73x | 8.29x |
| full | testsuite-lf | 1582 | 544 | 74 | 20 | 91 | 150 | 2.91x | 21.49x | 79.55x | 17.36x | 10.57x |
| full | testsuite-crlf | 1597 | 558 | 75 | 20 | 93 | 153 | 2.86x | 21.40x | 77.94x | 17.10x | 10.42x |
| full | enron-crlf | 2420 | 942 | 65 | 70 | 96 | 366 | 2.57x | 37.10x | 34.77x | 25.23x | 6.60x |
| full | attachments-crlf | 12747 | 2781 | 960 | 380 | 185 | 522 | 4.58x | 13.28x | 33.51x | 68.80x | 24.41x |
| full | newsletter-crlf | 2942 | 638 | 113 | 299 | 257 | 353 | 4.61x | 26.03x | 9.83x | 11.44x | 8.33x |
| full | modern-headers-crlf | 1989 | 556 | 116 | 230 | 352 | 391 | 3.58x | 17.09x | 8.67x | 5.66x | 5.09x |
| full | forwarded-crlf | 2639 | 897 | 79 | 152 | 150 | 606 | 2.94x | 33.58x | 17.41x | 17.65x | 4.35x |
| full | dashes-crlf | 7512 | 321 | 96 | 1250 | 868 | 582 | 23.38x | 77.92x | 6.01x | 8.65x | 12.90x |
| full | plain-large-crlf | 17171 | 13732 | 97 | 2212 | 2796 | 856 | 1.25x | 176.61x | 7.76x | 6.14x | 20.05x |
| full | attachments-lf | 12701 | 2836 | 957 | 372 | 183 | 514 | 4.48x | 13.27x | 34.13x | 69.31x | 24.72x |
| full | newsletter-lf | 3070 | 632 | 114 | 287 | 251 | 352 | 4.86x | 27.02x | 10.70x | 12.22x | 8.71x |

Spread of the runs, (max - min) / median:

| workload | corpus | mail-parser-1.0 | mailparse | gmime | vmime | libetpan | dovecot |
|---|---|---:|---:|---:|---:|---:|---:|
| structure | testsuite-lf | 0.9% | 1.2% | 1.9% | 15.0% | 1.8% | 0.7% |
| structure | testsuite-crlf | 0.6% | 0.6% | 1.0% | 14.8% | 1.7% | 0.0% |
| structure | enron-crlf | 1.1% | 1.8% | 0.6% | 0.7% | 1.2% | 0.5% |
| structure | attachments-crlf | 1.5% | 0.6% | 1.2% | 0.6% | 1.1% | 0.9% |
| structure | newsletter-crlf | 0.6% | 1.0% | 1.0% | 0.4% | 1.9% | 3.4% |
| structure | modern-headers-crlf | 0.4% | 0.7% | 1.0% | 0.8% | 1.2% | 0.4% |
| structure | forwarded-crlf | 1.5% | 1.8% | 1.2% | 0.2% | 1.1% | 1.0% |
| structure | dashes-crlf | 0.9% | 4.7% | 4.2% | 0.9% | 0.2% | 0.5% |
| structure | plain-large-crlf | 0.5% | 7.9% | 1.2% | 1.0% | 1.1% | 1.1% |
| structure | attachments-lf | 1.1% | 0.4% | 0.5% | 0.3% | 1.4% | 0.8% |
| structure | newsletter-lf | 0.0% | 0.9% | 1.5% | 1.7% | 0.0% | 2.3% |
| headers | testsuite-lf | 1.0% | 0.2% | 1.3% | 4.0% | 1.4% | 0.6% |
| headers | testsuite-crlf | 0.5% | 0.4% | 1.8% | 9.2% | 1.4% | 0.3% |
| headers | enron-crlf | 0.8% | 1.7% | 0.8% | 0.6% | 0.7% | 0.3% |
| headers | attachments-crlf | 0.8% | 0.3% | 0.8% | 0.3% | 0.4% | 0.7% |
| headers | newsletter-crlf | 0.3% | 0.3% | 2.3% | 0.4% | 0.6% | 1.9% |
| headers | modern-headers-crlf | 0.2% | 1.2% | 1.0% | 1.3% | 1.1% | 0.4% |
| headers | forwarded-crlf | 0.2% | 0.9% | 1.0% | 1.3% | 1.1% | 1.0% |
| headers | dashes-crlf | 1.2% | 6.9% | 3.1% | 1.2% | 0.3% | 0.3% |
| headers | plain-large-crlf | 0.6% | 2.9% | 1.2% | 6.1% | 0.7% | 1.2% |
| headers | attachments-lf | 0.6% | 0.0% | 0.2% | 0.5% | 1.0% | 0.2% |
| headers | newsletter-lf | 0.2% | 1.2% | 1.1% | 0.5% | 0.2% | 1.0% |
| full | testsuite-lf | 0.8% | 2.9% | 0.4% | 3.8% | 3.2% | 1.6% |
| full | testsuite-crlf | 1.0% | 1.0% | 1.3% | 1.1% | 0.5% | 0.4% |
| full | enron-crlf | 0.6% | 0.6% | 0.6% | 0.9% | 1.1% | 2.0% |
| full | attachments-crlf | 0.6% | 0.2% | 0.4% | 1.6% | 1.7% | 0.5% |
| full | newsletter-crlf | 1.1% | 0.4% | 1.3% | 4.2% | 0.9% | 3.5% |
| full | modern-headers-crlf | 0.7% | 1.9% | 2.8% | 4.3% | 2.0% | 2.0% |
| full | forwarded-crlf | 0.5% | 2.2% | 0.5% | 1.1% | 0.3% | 5.9% |
| full | dashes-crlf | 0.7% | 3.6% | 0.3% | 2.8% | 0.6% | 1.0% |
| full | plain-large-crlf | 0.5% | 0.2% | 0.6% | 2.1% | 1.3% | 10.0% |
| full | attachments-lf | 0.3% | 0.4% | 0.2% | 2.2% | 1.5% | 0.1% |
| full | newsletter-lf | 1.4% | 0.1% | 0.3% | 4.3% | 1.2% | 3.8% |
