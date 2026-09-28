# Equivalence check

| corpus | messages | bytes | MiB | FNV-1a digest |
|---|---:|---:|---:|---|
| testsuite-lf | 123 | 316654 | 0.30 | `af8205bb38c165c5` |
| testsuite-crlf | 123 | 323727 | 0.31 | `85b9d080427109fc` |
| enron-crlf | 3000 | 6233382 | 5.94 | `ebbdb99ac861ae3b` |
| attachments-crlf | 3 | 4132787 | 3.94 | `00b8baeef0f9bbd4` |
| newsletter-crlf | 6 | 772947 | 0.74 | `0e0201fa1f217fc7` |
| modern-headers-crlf | 20 | 266601 | 0.25 | `20154783c10bbdf1` |
| forwarded-crlf | 6 | 110231 | 0.11 | `0b5bbf02c3108c4a` |
| dashes-crlf | 1 | 612915 | 0.58 | `5454fa53e5cb9b78` |
| plain-large-crlf | 1 | 1019901 | 0.97 | `bc625341e62c7a37` |
| attachments-lf | 3 | 4079113 | 3.89 | `1acc9c82b41557e9` |
| newsletter-lf | 6 | 759992 | 0.72 | `289c346c5a2549d8` |

## structure

| corpus | implementation | parsed | rejected | panics | crashes | messages | parts | differing messages |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| testsuite-lf | mail-parser-1.0 | 123 | 0 | 0 | 0 | 143 | 479 | 0 |
| testsuite-lf | mailparse | 122 | 1 | 0 | 0 | 143 | 475 (-4) | 10 |
| testsuite-lf | gmime | 122 | 1 | 0 | 0 | 143 | 474 (-5) | 5 |
| testsuite-lf | vmime | 123 | 0 | 0 | 0 | 138 (-5) | 467 (-12) | 13 |
| testsuite-lf | libetpan | 123 | 0 | 0 | 0 | 143 | 467 (-12) | 12 |
| testsuite-lf | dovecot | 123 | 0 | 0 | 0 | 144 (+1) | 486 (+7) | 10 |
| testsuite-crlf | mail-parser-1.0 | 123 | 0 | 0 | 0 | 143 | 479 | 0 |
| testsuite-crlf | mailparse | 122 | 1 | 0 | 0 | 143 | 479 | 12 |
| testsuite-crlf | gmime | 122 | 1 | 0 | 0 | 143 | 474 (-5) | 5 |
| testsuite-crlf | vmime | 123 | 0 | 0 | 0 | 138 (-5) | 467 (-12) | 13 |
| testsuite-crlf | libetpan | 123 | 0 | 0 | 0 | 143 | 467 (-12) | 12 |
| testsuite-crlf | dovecot | 123 | 0 | 0 | 0 | 144 (+1) | 486 (+7) | 10 |
| enron-crlf | mail-parser-1.0 | 3000 | 0 | 0 | 0 | 3000 | 3000 | 0 |
| enron-crlf | mailparse | 3000 | 0 | 0 | 0 | 3000 | 3000 | 0 |
| enron-crlf | gmime | 3000 | 0 | 0 | 0 | 3000 | 3000 | 0 |
| enron-crlf | vmime | 3000 | 0 | 0 | 0 | 3000 | 3000 | 0 |
| enron-crlf | libetpan | 3000 | 0 | 0 | 0 | 3000 | 3000 | 0 |
| enron-crlf | dovecot | 3000 | 0 | 0 | 0 | 3000 | 3000 | 0 |
| attachments-crlf | mail-parser-1.0 | 3 | 0 | 0 | 0 | 3 | 18 | 0 |
| attachments-crlf | mailparse | 3 | 0 | 0 | 0 | 3 | 18 | 0 |
| attachments-crlf | gmime | 3 | 0 | 0 | 0 | 3 | 18 | 0 |
| attachments-crlf | vmime | 3 | 0 | 0 | 0 | 3 | 18 | 0 |
| attachments-crlf | libetpan | 3 | 0 | 0 | 0 | 3 | 18 | 0 |
| attachments-crlf | dovecot | 3 | 0 | 0 | 0 | 3 | 18 | 0 |
| newsletter-crlf | mail-parser-1.0 | 6 | 0 | 0 | 0 | 6 | 30 | 0 |
| newsletter-crlf | mailparse | 6 | 0 | 0 | 0 | 6 | 30 | 0 |
| newsletter-crlf | gmime | 6 | 0 | 0 | 0 | 6 | 30 | 0 |
| newsletter-crlf | vmime | 6 | 0 | 0 | 0 | 6 | 30 | 0 |
| newsletter-crlf | libetpan | 6 | 0 | 0 | 0 | 6 | 30 | 0 |
| newsletter-crlf | dovecot | 6 | 0 | 0 | 0 | 6 | 30 | 0 |
| modern-headers-crlf | mail-parser-1.0 | 20 | 0 | 0 | 0 | 20 | 60 | 0 |
| modern-headers-crlf | mailparse | 20 | 0 | 0 | 0 | 20 | 60 | 0 |
| modern-headers-crlf | gmime | 20 | 0 | 0 | 0 | 20 | 60 | 0 |
| modern-headers-crlf | vmime | 20 | 0 | 0 | 0 | 20 | 60 | 0 |
| modern-headers-crlf | libetpan | 20 | 0 | 0 | 0 | 20 | 60 | 0 |
| modern-headers-crlf | dovecot | 20 | 0 | 0 | 0 | 20 | 60 | 0 |
| forwarded-crlf | mail-parser-1.0 | 6 | 0 | 0 | 0 | 27 | 93 | 0 |
| forwarded-crlf | mailparse | 6 | 0 | 0 | 0 | 27 | 93 | 0 |
| forwarded-crlf | gmime | 6 | 0 | 0 | 0 | 27 | 93 | 0 |
| forwarded-crlf | vmime | 6 | 0 | 0 | 0 | 27 | 93 | 0 |
| forwarded-crlf | libetpan | 6 | 0 | 0 | 0 | 27 | 93 | 0 |
| forwarded-crlf | dovecot | 6 | 0 | 0 | 0 | 27 | 93 | 0 |
| dashes-crlf | mail-parser-1.0 | 1 | 0 | 0 | 0 | 1 | 4 | 0 |
| dashes-crlf | mailparse | 1 | 0 | 0 | 0 | 1 | 1065 (+1061) | 1 |
| dashes-crlf | gmime | 1 | 0 | 0 | 0 | 1 | 4 | 0 |
| dashes-crlf | vmime | 1 | 0 | 0 | 0 | 1 | 4 | 0 |
| dashes-crlf | libetpan | 1 | 0 | 0 | 0 | 1 | 4 | 0 |
| dashes-crlf | dovecot | 1 | 0 | 0 | 0 | 1 | 1065 (+1061) | 1 |
| plain-large-crlf | mail-parser-1.0 | 1 | 0 | 0 | 0 | 1 | 1 | 0 |
| plain-large-crlf | mailparse | 1 | 0 | 0 | 0 | 1 | 1 | 0 |
| plain-large-crlf | gmime | 1 | 0 | 0 | 0 | 1 | 1 | 0 |
| plain-large-crlf | vmime | 1 | 0 | 0 | 0 | 1 | 1 | 0 |
| plain-large-crlf | libetpan | 1 | 0 | 0 | 0 | 1 | 1 | 0 |
| plain-large-crlf | dovecot | 1 | 0 | 0 | 0 | 1 | 1 | 0 |
| attachments-lf | mail-parser-1.0 | 3 | 0 | 0 | 0 | 3 | 18 | 0 |
| attachments-lf | mailparse | 3 | 0 | 0 | 0 | 3 | 18 | 0 |
| attachments-lf | gmime | 3 | 0 | 0 | 0 | 3 | 18 | 0 |
| attachments-lf | vmime | 3 | 0 | 0 | 0 | 3 | 18 | 0 |
| attachments-lf | libetpan | 3 | 0 | 0 | 0 | 3 | 18 | 0 |
| attachments-lf | dovecot | 3 | 0 | 0 | 0 | 3 | 18 | 0 |
| newsletter-lf | mail-parser-1.0 | 6 | 0 | 0 | 0 | 6 | 30 | 0 |
| newsletter-lf | mailparse | 6 | 0 | 0 | 0 | 6 | 30 | 0 |
| newsletter-lf | gmime | 6 | 0 | 0 | 0 | 6 | 30 | 0 |
| newsletter-lf | vmime | 6 | 0 | 0 | 0 | 6 | 30 | 0 |
| newsletter-lf | libetpan | 6 | 0 | 0 | 0 | 6 | 30 | 0 |
| newsletter-lf | dovecot | 6 | 0 | 0 | 0 | 6 | 30 | 0 |

## headers

| corpus | implementation | parsed | rejected | panics | crashes | subject | from | date | message-id | differing messages |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| testsuite-lf | mail-parser-1.0 | 123 | 0 | 0 | 0 | 94 | 92 | 83 | 73 | 0 |
| testsuite-lf | mailparse | 122 | 1 | 0 | 0 | 93 (-1) | 89 (-3) | 83 | 72 (-1) | 4 |
| testsuite-lf | gmime | 122 | 1 | 0 | 0 | 94 | 92 | 83 | 73 | 1 |
| testsuite-lf | vmime | 123 | 0 | 0 | 0 | 94 | 92 | 84 (+1) | 73 | 1 |
| testsuite-lf | libetpan | 123 | 0 | 0 | 0 | 94 | 92 | 83 | 73 | 0 |
| testsuite-lf | dovecot | 123 | 0 | 0 | 0 | 94 | 92 | 82 (-1) | 70 (-3) | 4 |
| testsuite-crlf | mail-parser-1.0 | 123 | 0 | 0 | 0 | 94 | 92 | 83 | 73 | 0 |
| testsuite-crlf | mailparse | 122 | 1 | 0 | 0 | 93 (-1) | 89 (-3) | 83 | 72 (-1) | 4 |
| testsuite-crlf | gmime | 122 | 1 | 0 | 0 | 94 | 92 | 83 | 73 | 1 |
| testsuite-crlf | vmime | 123 | 0 | 0 | 0 | 94 | 92 | 84 (+1) | 73 | 1 |
| testsuite-crlf | libetpan | 123 | 0 | 0 | 0 | 94 | 92 | 83 | 73 | 0 |
| testsuite-crlf | dovecot | 123 | 0 | 0 | 0 | 94 | 92 | 82 (-1) | 70 (-3) | 4 |
| enron-crlf | mail-parser-1.0 | 3000 | 0 | 0 | 0 | 3000 | 3000 | 3000 | 3000 | 0 |
| enron-crlf | mailparse | 3000 | 0 | 0 | 0 | 3000 | 3000 | 3000 | 3000 | 0 |
| enron-crlf | gmime | 3000 | 0 | 0 | 0 | 3000 | 2673 (-327) | 3000 | 3000 | 327 |
| enron-crlf | vmime | 3000 | 0 | 0 | 0 | 3000 | 3000 | 3000 | 3000 | 0 |
| enron-crlf | libetpan | 3000 | 0 | 0 | 0 | 3000 | 3000 | 3000 | 3000 | 0 |
| enron-crlf | dovecot | 3000 | 0 | 0 | 0 | 3000 | 3000 | 3000 | 3000 | 0 |
| attachments-crlf | mail-parser-1.0 | 3 | 0 | 0 | 0 | 3 | 3 | 3 | 3 | 0 |
| attachments-crlf | mailparse | 3 | 0 | 0 | 0 | 3 | 3 | 3 | 3 | 0 |
| attachments-crlf | gmime | 3 | 0 | 0 | 0 | 3 | 3 | 3 | 3 | 0 |
| attachments-crlf | vmime | 3 | 0 | 0 | 0 | 3 | 3 | 3 | 3 | 0 |
| attachments-crlf | libetpan | 3 | 0 | 0 | 0 | 3 | 3 | 3 | 3 | 0 |
| attachments-crlf | dovecot | 3 | 0 | 0 | 0 | 3 | 3 | 3 | 3 | 0 |
| newsletter-crlf | mail-parser-1.0 | 6 | 0 | 0 | 0 | 6 | 6 | 6 | 6 | 0 |
| newsletter-crlf | mailparse | 6 | 0 | 0 | 0 | 6 | 6 | 6 | 6 | 0 |
| newsletter-crlf | gmime | 6 | 0 | 0 | 0 | 6 | 6 | 6 | 6 | 0 |
| newsletter-crlf | vmime | 6 | 0 | 0 | 0 | 6 | 6 | 6 | 6 | 0 |
| newsletter-crlf | libetpan | 6 | 0 | 0 | 0 | 6 | 6 | 6 | 6 | 0 |
| newsletter-crlf | dovecot | 6 | 0 | 0 | 0 | 6 | 6 | 6 | 6 | 0 |
| modern-headers-crlf | mail-parser-1.0 | 20 | 0 | 0 | 0 | 20 | 20 | 20 | 20 | 0 |
| modern-headers-crlf | mailparse | 20 | 0 | 0 | 0 | 20 | 20 | 20 | 20 | 0 |
| modern-headers-crlf | gmime | 20 | 0 | 0 | 0 | 20 | 20 | 20 | 20 | 0 |
| modern-headers-crlf | vmime | 20 | 0 | 0 | 0 | 20 | 20 | 20 | 20 | 0 |
| modern-headers-crlf | libetpan | 20 | 0 | 0 | 0 | 20 | 20 | 20 | 20 | 0 |
| modern-headers-crlf | dovecot | 20 | 0 | 0 | 0 | 20 | 20 | 20 | 20 | 0 |
| forwarded-crlf | mail-parser-1.0 | 6 | 0 | 0 | 0 | 6 | 6 | 6 | 6 | 0 |
| forwarded-crlf | mailparse | 6 | 0 | 0 | 0 | 6 | 6 | 6 | 6 | 0 |
| forwarded-crlf | gmime | 6 | 0 | 0 | 0 | 6 | 6 | 6 | 6 | 0 |
| forwarded-crlf | vmime | 6 | 0 | 0 | 0 | 6 | 6 | 6 | 6 | 0 |
| forwarded-crlf | libetpan | 6 | 0 | 0 | 0 | 6 | 6 | 6 | 6 | 0 |
| forwarded-crlf | dovecot | 6 | 0 | 0 | 0 | 6 | 6 | 6 | 6 | 0 |
| dashes-crlf | mail-parser-1.0 | 1 | 0 | 0 | 0 | 1 | 1 | 1 | 1 | 0 |
| dashes-crlf | mailparse | 1 | 0 | 0 | 0 | 1 | 1 | 1 | 1 | 0 |
| dashes-crlf | gmime | 1 | 0 | 0 | 0 | 1 | 1 | 1 | 1 | 0 |
| dashes-crlf | vmime | 1 | 0 | 0 | 0 | 1 | 1 | 1 | 1 | 0 |
| dashes-crlf | libetpan | 1 | 0 | 0 | 0 | 1 | 1 | 1 | 1 | 0 |
| dashes-crlf | dovecot | 1 | 0 | 0 | 0 | 1 | 1 | 1 | 1 | 0 |
| plain-large-crlf | mail-parser-1.0 | 1 | 0 | 0 | 0 | 1 | 1 | 1 | 1 | 0 |
| plain-large-crlf | mailparse | 1 | 0 | 0 | 0 | 1 | 1 | 1 | 1 | 0 |
| plain-large-crlf | gmime | 1 | 0 | 0 | 0 | 1 | 1 | 1 | 1 | 0 |
| plain-large-crlf | vmime | 1 | 0 | 0 | 0 | 1 | 1 | 1 | 1 | 0 |
| plain-large-crlf | libetpan | 1 | 0 | 0 | 0 | 1 | 1 | 1 | 1 | 0 |
| plain-large-crlf | dovecot | 1 | 0 | 0 | 0 | 1 | 1 | 1 | 1 | 0 |
| attachments-lf | mail-parser-1.0 | 3 | 0 | 0 | 0 | 3 | 3 | 3 | 3 | 0 |
| attachments-lf | mailparse | 3 | 0 | 0 | 0 | 3 | 3 | 3 | 3 | 0 |
| attachments-lf | gmime | 3 | 0 | 0 | 0 | 3 | 3 | 3 | 3 | 0 |
| attachments-lf | vmime | 3 | 0 | 0 | 0 | 3 | 3 | 3 | 3 | 0 |
| attachments-lf | libetpan | 3 | 0 | 0 | 0 | 3 | 3 | 3 | 3 | 0 |
| attachments-lf | dovecot | 3 | 0 | 0 | 0 | 3 | 3 | 3 | 3 | 0 |
| newsletter-lf | mail-parser-1.0 | 6 | 0 | 0 | 0 | 6 | 6 | 6 | 6 | 0 |
| newsletter-lf | mailparse | 6 | 0 | 0 | 0 | 6 | 6 | 6 | 6 | 0 |
| newsletter-lf | gmime | 6 | 0 | 0 | 0 | 6 | 6 | 6 | 6 | 0 |
| newsletter-lf | vmime | 6 | 0 | 0 | 0 | 6 | 6 | 6 | 6 | 0 |
| newsletter-lf | libetpan | 6 | 0 | 0 | 0 | 6 | 6 | 6 | 6 | 0 |
| newsletter-lf | dovecot | 6 | 0 | 0 | 0 | 6 | 6 | 6 | 6 | 0 |

## full

| corpus | implementation | parsed | rejected | panics | crashes | messages | leaves | text leaves | decoded MiB | decoded vs 1.0 | failures | differing messages |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| testsuite-lf | mail-parser-1.0 | 123 | 0 | 0 | 0 | 143 | 316 | 223 | 0.17 | = | 0 | 0 |
| testsuite-lf | mailparse | 122 | 1 | 0 | 0 | 143 | 303 (-13) | 213 (-10) | 0.17 | -0.33% | 4 | 59 |
| testsuite-lf | gmime | 122 | 1 | 0 | 0 | 143 | 308 (-8) | 216 (-7) | 0.16 | -1.91% | 0 | 18 |
| testsuite-lf | vmime | 123 | 0 | 0 | 0 | 138 (-5) | 301 (-15) | 209 (-14) | 0.16 | -1.72% | 3 | 31 |
| testsuite-lf | libetpan | 123 | 0 | 0 | 0 | 143 | 307 (-9) | 214 (-9) | 0.17 | +1.46% | 0 | 69 |
| testsuite-lf | dovecot | 123 | 0 | 0 | 0 | 144 (+1) | 318 (+2) | 224 (+1) | 0.16 | -6.04% | 0 | 63 |
| testsuite-crlf | mail-parser-1.0 | 123 | 0 | 0 | 0 | 143 | 316 | 223 | 0.17 | = | 0 | 0 |
| testsuite-crlf | mailparse | 122 | 1 | 0 | 0 | 143 | 307 (-9) | 217 (-6) | 0.17 | -0.70% | 4 | 24 |
| testsuite-crlf | gmime | 122 | 1 | 0 | 0 | 143 | 308 (-8) | 216 (-7) | 0.17 | -1.99% | 0 | 19 |
| testsuite-crlf | vmime | 123 | 0 | 0 | 0 | 138 (-5) | 301 (-15) | 209 (-14) | 0.17 | -1.80% | 3 | 31 |
| testsuite-crlf | libetpan | 123 | 0 | 0 | 0 | 143 | 307 (-9) | 214 (-9) | 0.17 | +1.11% | 0 | 33 |
| testsuite-crlf | dovecot | 123 | 0 | 0 | 0 | 144 (+1) | 318 (+2) | 224 (+1) | 0.16 | -6.45% | 0 | 21 |
| enron-crlf | mail-parser-1.0 | 3000 | 0 | 0 | 0 | 3000 | 3000 | 3000 | 3.51 | = | 0 | 0 |
| enron-crlf | mailparse | 3000 | 0 | 0 | 0 | 3000 | 3000 | 3000 | 3.51 | = | 0 | 0 |
| enron-crlf | gmime | 3000 | 0 | 0 | 0 | 3000 | 3000 | 3000 | 3.51 | +0.00% | 0 | 2 |
| enron-crlf | vmime | 3000 | 0 | 0 | 0 | 3000 | 3000 | 3000 | 3.51 | +0.00% | 0 | 2 |
| enron-crlf | libetpan | 3000 | 0 | 0 | 0 | 3000 | 3000 | 3000 | 3.51 | +0.00% | 0 | 2 |
| enron-crlf | dovecot | 3000 | 0 | 0 | 0 | 3000 | 3000 | 3000 | 3.51 | = | 0 | 0 |
| attachments-crlf | mail-parser-1.0 | 3 | 0 | 0 | 0 | 3 | 12 | 6 | 2.89 | = | 0 | 0 |
| attachments-crlf | mailparse | 3 | 0 | 0 | 0 | 3 | 12 | 6 | 2.89 | = | 0 | 0 |
| attachments-crlf | gmime | 3 | 0 | 0 | 0 | 3 | 12 | 6 | 2.89 | = | 0 | 0 |
| attachments-crlf | vmime | 3 | 0 | 0 | 0 | 3 | 12 | 6 | 2.89 | = | 0 | 0 |
| attachments-crlf | libetpan | 3 | 0 | 0 | 0 | 3 | 12 | 6 | 2.89 | = | 0 | 0 |
| attachments-crlf | dovecot | 3 | 0 | 0 | 0 | 3 | 12 | 6 | 2.89 | = | 0 | 0 |
| newsletter-crlf | mail-parser-1.0 | 6 | 0 | 0 | 0 | 6 | 18 | 12 | 0.64 | = | 0 | 0 |
| newsletter-crlf | mailparse | 6 | 0 | 0 | 0 | 6 | 18 | 12 | 0.64 | = | 0 | 0 |
| newsletter-crlf | gmime | 6 | 0 | 0 | 0 | 6 | 18 | 12 | 0.64 | = | 0 | 0 |
| newsletter-crlf | vmime | 6 | 0 | 0 | 0 | 6 | 18 | 12 | 0.64 | = | 0 | 0 |
| newsletter-crlf | libetpan | 6 | 0 | 0 | 0 | 6 | 18 | 12 | 0.64 | = | 0 | 0 |
| newsletter-crlf | dovecot | 6 | 0 | 0 | 0 | 6 | 18 | 12 | 0.64 | = | 0 | 0 |
| modern-headers-crlf | mail-parser-1.0 | 20 | 0 | 0 | 0 | 20 | 40 | 40 | 0.15 | = | 0 | 0 |
| modern-headers-crlf | mailparse | 20 | 0 | 0 | 0 | 20 | 40 | 40 | 0.15 | = | 0 | 0 |
| modern-headers-crlf | gmime | 20 | 0 | 0 | 0 | 20 | 40 | 40 | 0.15 | = | 0 | 0 |
| modern-headers-crlf | vmime | 20 | 0 | 0 | 0 | 20 | 40 | 40 | 0.15 | = | 0 | 0 |
| modern-headers-crlf | libetpan | 20 | 0 | 0 | 0 | 20 | 40 | 40 | 0.15 | = | 0 | 0 |
| modern-headers-crlf | dovecot | 20 | 0 | 0 | 0 | 20 | 40 | 40 | 0.15 | = | 0 | 0 |
| forwarded-crlf | mail-parser-1.0 | 6 | 0 | 0 | 0 | 27 | 39 | 39 | 0.08 | = | 0 | 0 |
| forwarded-crlf | mailparse | 6 | 0 | 0 | 0 | 27 | 39 | 39 | 0.08 | = | 0 | 0 |
| forwarded-crlf | gmime | 6 | 0 | 0 | 0 | 27 | 39 | 39 | 0.08 | = | 0 | 0 |
| forwarded-crlf | vmime | 6 | 0 | 0 | 0 | 27 | 39 | 39 | 0.08 | = | 0 | 0 |
| forwarded-crlf | libetpan | 6 | 0 | 0 | 0 | 27 | 39 | 39 | 0.08 | = | 0 | 0 |
| forwarded-crlf | dovecot | 6 | 0 | 0 | 0 | 27 | 39 | 39 | 0.08 | = | 0 | 0 |
| dashes-crlf | mail-parser-1.0 | 1 | 0 | 0 | 0 | 1 | 3 | 3 | 0.58 | = | 0 | 0 |
| dashes-crlf | mailparse | 1 | 0 | 0 | 0 | 1 | 1064 (+1061) | 1064 (+1061) | 0.00 | -99.77% | 0 | 1 |
| dashes-crlf | gmime | 1 | 0 | 0 | 0 | 1 | 3 | 3 | 0.58 | = | 0 | 0 |
| dashes-crlf | vmime | 1 | 0 | 0 | 0 | 1 | 3 | 3 | 0.58 | = | 0 | 0 |
| dashes-crlf | libetpan | 1 | 0 | 0 | 0 | 1 | 3 | 3 | 0.58 | = | 0 | 0 |
| dashes-crlf | dovecot | 1 | 0 | 0 | 0 | 1 | 1064 (+1061) | 1064 (+1061) | 0.00 | -99.77% | 0 | 1 |
| plain-large-crlf | mail-parser-1.0 | 1 | 0 | 0 | 0 | 1 | 1 | 1 | 0.97 | = | 0 | 0 |
| plain-large-crlf | mailparse | 1 | 0 | 0 | 0 | 1 | 1 | 1 | 0.97 | = | 0 | 0 |
| plain-large-crlf | gmime | 1 | 0 | 0 | 0 | 1 | 1 | 1 | 0.97 | = | 0 | 0 |
| plain-large-crlf | vmime | 1 | 0 | 0 | 0 | 1 | 1 | 1 | 0.97 | = | 0 | 0 |
| plain-large-crlf | libetpan | 1 | 0 | 0 | 0 | 1 | 1 | 1 | 0.97 | = | 0 | 0 |
| plain-large-crlf | dovecot | 1 | 0 | 0 | 0 | 1 | 1 | 1 | 0.97 | = | 0 | 0 |
| attachments-lf | mail-parser-1.0 | 3 | 0 | 0 | 0 | 3 | 12 | 6 | 2.89 | = | 0 | 0 |
| attachments-lf | mailparse | 3 | 0 | 0 | 0 | 3 | 12 | 6 | 2.89 | +0.04% | 0 | 3 |
| attachments-lf | gmime | 3 | 0 | 0 | 0 | 3 | 12 | 6 | 2.89 | = | 0 | 0 |
| attachments-lf | vmime | 3 | 0 | 0 | 0 | 3 | 12 | 6 | 2.89 | = | 0 | 0 |
| attachments-lf | libetpan | 3 | 0 | 0 | 0 | 3 | 12 | 6 | 2.89 | +0.04% | 0 | 3 |
| attachments-lf | dovecot | 3 | 0 | 0 | 0 | 3 | 12 | 6 | 2.89 | +0.04% | 0 | 3 |
| newsletter-lf | mail-parser-1.0 | 6 | 0 | 0 | 0 | 6 | 18 | 12 | 0.63 | = | 0 | 0 |
| newsletter-lf | mailparse | 6 | 0 | 0 | 0 | 6 | 18 | 12 | 0.64 | +0.76% | 0 | 6 |
| newsletter-lf | gmime | 6 | 0 | 0 | 0 | 6 | 18 | 12 | 0.63 | = | 0 | 0 |
| newsletter-lf | vmime | 6 | 0 | 0 | 0 | 6 | 18 | 12 | 0.63 | = | 0 | 0 |
| newsletter-lf | libetpan | 6 | 0 | 0 | 0 | 6 | 18 | 12 | 0.64 | +0.76% | 0 | 6 |
| newsletter-lf | dovecot | 6 | 0 | 0 | 0 | 6 | 18 | 12 | 0.64 | +0.76% | 0 | 6 |

## Differing messages (first 5 per cell)

- structure testsuite-lf mailparse `legacy/015.eml`: 1 messages, 10 parts (1.0: 1 messages, 9 parts)
- structure testsuite-lf mailparse `legacy/025.eml`: 1 messages, 8 parts (1.0: 1 messages, 7 parts)
- structure testsuite-lf mailparse `legacy/051.eml`: 1 messages, 8 parts (1.0: 1 messages, 7 parts)
- structure testsuite-lf mailparse `malformed/005.eml`: 1 messages, 2 parts (1.0: 1 messages, 3 parts)
- structure testsuite-lf mailparse `malformed/007.eml`: 1 messages, 4 parts (1.0: 1 messages, 2 parts)
- structure testsuite-lf gmime `malformed/000.eml`: 1 messages, 4 parts (1.0: 1 messages, 5 parts)
- structure testsuite-lf gmime `malformed/004.eml`: 1 messages, 1 parts (1.0: 1 messages, 3 parts)
- structure testsuite-lf gmime `malformed/017.eml`: 1 messages, 1 parts (1.0: 1 messages, 2 parts)
- structure testsuite-lf gmime `malformed/022.eml`: 2 messages, 4 parts (1.0: 1 messages, 3 parts)
- structure testsuite-lf gmime `malformed/023.eml`: rejected (1.0: 1 messages, 2 parts)
- structure testsuite-lf vmime `rfc/004.eml`: 1 messages, 5 parts (1.0: 3 messages, 7 parts)
- structure testsuite-lf vmime `thirdparty/007.eml`: 1 messages, 4 parts (1.0: 4 messages, 9 parts)
- structure testsuite-lf vmime `malformed/002.eml`: 1 messages, 2 parts (1.0: 1 messages, 1 parts)
- structure testsuite-lf vmime `malformed/005.eml`: 1 messages, 2 parts (1.0: 1 messages, 3 parts)
- structure testsuite-lf vmime `malformed/008.eml`: 1 messages, 3 parts (1.0: 1 messages, 4 parts)
- structure testsuite-lf libetpan `thirdparty/007.eml`: 3 messages, 8 parts (1.0: 4 messages, 9 parts)
- structure testsuite-lf libetpan `malformed/000.eml`: 1 messages, 4 parts (1.0: 1 messages, 5 parts)
- structure testsuite-lf libetpan `malformed/002.eml`: 1 messages, 2 parts (1.0: 1 messages, 1 parts)
- structure testsuite-lf libetpan `malformed/004.eml`: 1 messages, 2 parts (1.0: 1 messages, 3 parts)
- structure testsuite-lf libetpan `malformed/005.eml`: 1 messages, 2 parts (1.0: 1 messages, 3 parts)
- structure testsuite-lf dovecot `rfc/008.eml`: 2 messages, 4 parts (1.0: 2 messages, 6 parts)
- structure testsuite-lf dovecot `malformed/007.eml`: 1 messages, 4 parts (1.0: 1 messages, 2 parts)
- structure testsuite-lf dovecot `malformed/013.eml`: 1 messages, 9 parts (1.0: 1 messages, 8 parts)
- structure testsuite-lf dovecot `malformed/014.eml`: 1 messages, 3 parts (1.0: 1 messages, 2 parts)
- structure testsuite-lf dovecot `malformed/016.eml`: 1 messages, 7 parts (1.0: 1 messages, 5 parts)
- structure testsuite-crlf mailparse `legacy/015.eml`: 1 messages, 10 parts (1.0: 1 messages, 9 parts)
- structure testsuite-crlf mailparse `legacy/025.eml`: 1 messages, 8 parts (1.0: 1 messages, 7 parts)
- structure testsuite-crlf mailparse `legacy/051.eml`: 1 messages, 8 parts (1.0: 1 messages, 7 parts)
- structure testsuite-crlf mailparse `malformed/005.eml`: 1 messages, 2 parts (1.0: 1 messages, 3 parts)
- structure testsuite-crlf mailparse `malformed/007.eml`: 1 messages, 4 parts (1.0: 1 messages, 2 parts)
- structure testsuite-crlf gmime `malformed/000.eml`: 1 messages, 4 parts (1.0: 1 messages, 5 parts)
- structure testsuite-crlf gmime `malformed/004.eml`: 1 messages, 1 parts (1.0: 1 messages, 3 parts)
- structure testsuite-crlf gmime `malformed/017.eml`: 1 messages, 1 parts (1.0: 1 messages, 2 parts)
- structure testsuite-crlf gmime `malformed/022.eml`: 2 messages, 4 parts (1.0: 1 messages, 3 parts)
- structure testsuite-crlf gmime `malformed/023.eml`: rejected (1.0: 1 messages, 2 parts)
- structure testsuite-crlf vmime `rfc/004.eml`: 1 messages, 5 parts (1.0: 3 messages, 7 parts)
- structure testsuite-crlf vmime `thirdparty/007.eml`: 1 messages, 4 parts (1.0: 4 messages, 9 parts)
- structure testsuite-crlf vmime `malformed/002.eml`: 1 messages, 2 parts (1.0: 1 messages, 1 parts)
- structure testsuite-crlf vmime `malformed/005.eml`: 1 messages, 2 parts (1.0: 1 messages, 3 parts)
- structure testsuite-crlf vmime `malformed/008.eml`: 1 messages, 3 parts (1.0: 1 messages, 4 parts)
- structure testsuite-crlf libetpan `thirdparty/007.eml`: 3 messages, 8 parts (1.0: 4 messages, 9 parts)
- structure testsuite-crlf libetpan `malformed/000.eml`: 1 messages, 4 parts (1.0: 1 messages, 5 parts)
- structure testsuite-crlf libetpan `malformed/002.eml`: 1 messages, 2 parts (1.0: 1 messages, 1 parts)
- structure testsuite-crlf libetpan `malformed/004.eml`: 1 messages, 2 parts (1.0: 1 messages, 3 parts)
- structure testsuite-crlf libetpan `malformed/005.eml`: 1 messages, 2 parts (1.0: 1 messages, 3 parts)
- structure testsuite-crlf dovecot `rfc/008.eml`: 2 messages, 4 parts (1.0: 2 messages, 6 parts)
- structure testsuite-crlf dovecot `malformed/007.eml`: 1 messages, 4 parts (1.0: 1 messages, 2 parts)
- structure testsuite-crlf dovecot `malformed/013.eml`: 1 messages, 9 parts (1.0: 1 messages, 8 parts)
- structure testsuite-crlf dovecot `malformed/014.eml`: 1 messages, 3 parts (1.0: 1 messages, 2 parts)
- structure testsuite-crlf dovecot `malformed/016.eml`: 1 messages, 7 parts (1.0: 1 messages, 5 parts)
- structure dashes-crlf mailparse `dashes`: 1 messages, 1065 parts (1.0: 1 messages, 4 parts)
- structure dashes-crlf dovecot `dashes`: 1 messages, 1065 parts (1.0: 1 messages, 4 parts)
- headers testsuite-lf mailparse `rfc/001.eml`: subject 1, from 0, date 1, message-id 1 (1.0: subject 1, from 1, date 0, message-id 1)
- headers testsuite-lf mailparse `rfc/004.eml`: subject 1, from 0, date 1, message-id 0 (1.0: subject 1, from 1, date 1, message-id 0)
- headers testsuite-lf mailparse `thirdparty/007.eml`: subject 0, from 0, date 0, message-id 0 (1.0: subject 0, from 1, date 0, message-id 0)
- headers testsuite-lf mailparse `malformed/019.eml`: rejected (1.0: subject 1, from 0, date 1, message-id 1)
- headers testsuite-lf gmime `malformed/023.eml`: rejected (1.0: subject 0, from 0, date 0, message-id 0)
- headers testsuite-lf vmime `rfc/001.eml`: subject 1, from 1, date 1, message-id 1 (1.0: subject 1, from 1, date 0, message-id 1)
- headers testsuite-lf dovecot `malformed/019.eml`: subject 1, from 0, date 0, message-id 1 (1.0: subject 1, from 0, date 1, message-id 1)
- headers testsuite-lf dovecot `sieve/headers.eml`: subject 1, from 1, date 1, message-id 0 (1.0: subject 1, from 1, date 1, message-id 1)
- headers testsuite-lf dovecot `sieve/mixed-attachment.eml`: subject 1, from 1, date 1, message-id 0 (1.0: subject 1, from 1, date 1, message-id 1)
- headers testsuite-lf dovecot `sieve/nested.eml`: subject 1, from 1, date 1, message-id 0 (1.0: subject 1, from 1, date 1, message-id 1)
- headers testsuite-crlf mailparse `rfc/001.eml`: subject 1, from 0, date 1, message-id 1 (1.0: subject 1, from 1, date 0, message-id 1)
- headers testsuite-crlf mailparse `rfc/004.eml`: subject 1, from 0, date 1, message-id 0 (1.0: subject 1, from 1, date 1, message-id 0)
- headers testsuite-crlf mailparse `thirdparty/007.eml`: subject 0, from 0, date 0, message-id 0 (1.0: subject 0, from 1, date 0, message-id 0)
- headers testsuite-crlf mailparse `malformed/019.eml`: rejected (1.0: subject 1, from 0, date 1, message-id 1)
- headers testsuite-crlf gmime `malformed/023.eml`: rejected (1.0: subject 0, from 0, date 0, message-id 0)
- headers testsuite-crlf vmime `rfc/001.eml`: subject 1, from 1, date 1, message-id 1 (1.0: subject 1, from 1, date 0, message-id 1)
- headers testsuite-crlf dovecot `malformed/019.eml`: subject 1, from 0, date 0, message-id 1 (1.0: subject 1, from 0, date 1, message-id 1)
- headers testsuite-crlf dovecot `sieve/headers.eml`: subject 1, from 1, date 1, message-id 0 (1.0: subject 1, from 1, date 1, message-id 1)
- headers testsuite-crlf dovecot `sieve/mixed-attachment.eml`: subject 1, from 1, date 1, message-id 0 (1.0: subject 1, from 1, date 1, message-id 1)
- headers testsuite-crlf dovecot `sieve/nested.eml`: subject 1, from 1, date 1, message-id 0 (1.0: subject 1, from 1, date 1, message-id 1)
- headers enron-crlf gmime `allen-p/sent_items/1.`: subject 1, from 0, date 1, message-id 1 (1.0: subject 1, from 1, date 1, message-id 1)
- headers enron-crlf gmime `allen-p/sent_items/10.`: subject 1, from 0, date 1, message-id 1 (1.0: subject 1, from 1, date 1, message-id 1)
- headers enron-crlf gmime `allen-p/sent_items/100.`: subject 1, from 0, date 1, message-id 1 (1.0: subject 1, from 1, date 1, message-id 1)
- headers enron-crlf gmime `allen-p/sent_items/101.`: subject 1, from 0, date 1, message-id 1 (1.0: subject 1, from 1, date 1, message-id 1)
- headers enron-crlf gmime `allen-p/sent_items/102.`: subject 1, from 0, date 1, message-id 1 (1.0: subject 1, from 1, date 1, message-id 1)
- full testsuite-lf mailparse `rfc/000.eml`: 2 messages, 4 leaves, 562 bytes (1.0: 2 messages, 6 leaves, 628 bytes)
- full testsuite-lf mailparse `rfc/006.eml`: 1 messages, 2 leaves, 184 bytes (1.0: 1 messages, 2 leaves, 180 bytes)
- full testsuite-lf mailparse `legacy/001.eml`: 1 messages, 1 leaves, 763 bytes (1.0: 1 messages, 1 leaves, 755 bytes)
- full testsuite-lf mailparse `legacy/002.eml`: 1 messages, 1 leaves, 969 bytes (1.0: 1 messages, 1 leaves, 947 bytes)
- full testsuite-lf mailparse `legacy/003.eml`: 1 messages, 2 leaves, 1730 bytes (1.0: 1 messages, 2 leaves, 1701 bytes)
- full testsuite-lf gmime `rfc/000.eml`: 2 messages, 6 leaves, 627 bytes (1.0: 2 messages, 6 leaves, 628 bytes)
- full testsuite-lf gmime `legacy/017.eml`: 1 messages, 3 leaves, 3381 bytes (1.0: 1 messages, 3 leaves, 4437 bytes)
- full testsuite-lf gmime `legacy/026.eml`: 1 messages, 3 leaves, 2289 bytes (1.0: 1 messages, 3 leaves, 2611 bytes)
- full testsuite-lf gmime `legacy/033.eml`: 1 messages, 1 leaves, 721 bytes (1.0: 1 messages, 1 leaves, 769 bytes)
- full testsuite-lf gmime `legacy/048.eml`: 1 messages, 4 leaves, 4829 bytes (1.0: 1 messages, 4 leaves, 6458 bytes)
- full testsuite-lf vmime `rfc/000.eml`: 2 messages, 6 leaves, 639 bytes (1.0: 2 messages, 6 leaves, 628 bytes)
- full testsuite-lf vmime `rfc/004.eml`: 1 messages, 3 leaves, 270 bytes (1.0: 3 messages, 3 leaves, 98 bytes)
- full testsuite-lf vmime `legacy/017.eml`: 1 messages, 3 leaves, 3381 bytes (1.0: 1 messages, 3 leaves, 4437 bytes)
- full testsuite-lf vmime `legacy/026.eml`: 1 messages, 3 leaves, 2289 bytes (1.0: 1 messages, 3 leaves, 2611 bytes)
- full testsuite-lf vmime `legacy/033.eml`: 1 messages, 1 leaves, 737 bytes (1.0: 1 messages, 1 leaves, 769 bytes)
- full testsuite-lf libetpan `rfc/000.eml`: 2 messages, 6 leaves, 629 bytes (1.0: 2 messages, 6 leaves, 628 bytes)
- full testsuite-lf libetpan `rfc/006.eml`: 1 messages, 2 leaves, 184 bytes (1.0: 1 messages, 2 leaves, 180 bytes)
- full testsuite-lf libetpan `legacy/001.eml`: 1 messages, 1 leaves, 763 bytes (1.0: 1 messages, 1 leaves, 755 bytes)
- full testsuite-lf libetpan `legacy/002.eml`: 1 messages, 1 leaves, 969 bytes (1.0: 1 messages, 1 leaves, 947 bytes)
- full testsuite-lf libetpan `legacy/003.eml`: 1 messages, 2 leaves, 1730 bytes (1.0: 1 messages, 2 leaves, 1701 bytes)
- full testsuite-lf dovecot `rfc/000.eml`: 2 messages, 6 leaves, 562 bytes (1.0: 2 messages, 6 leaves, 628 bytes)
- full testsuite-lf dovecot `rfc/006.eml`: 1 messages, 2 leaves, 184 bytes (1.0: 1 messages, 2 leaves, 180 bytes)
- full testsuite-lf dovecot `rfc/008.eml`: 2 messages, 2 leaves, 54 bytes (1.0: 2 messages, 3 leaves, 257 bytes)
- full testsuite-lf dovecot `legacy/001.eml`: 1 messages, 1 leaves, 763 bytes (1.0: 1 messages, 1 leaves, 755 bytes)
- full testsuite-lf dovecot `legacy/002.eml`: 1 messages, 1 leaves, 969 bytes (1.0: 1 messages, 1 leaves, 947 bytes)
- full testsuite-crlf mailparse `rfc/000.eml`: 2 messages, 4 leaves, 577 bytes (1.0: 2 messages, 6 leaves, 644 bytes)
- full testsuite-crlf mailparse `legacy/015.eml`: 1 messages, 7 leaves, 7363 bytes (1.0: 1 messages, 6 leaves, 7363 bytes)
- full testsuite-crlf mailparse `legacy/025.eml`: 1 messages, 6 leaves, 6576 bytes (1.0: 1 messages, 5 leaves, 6576 bytes)
- full testsuite-crlf mailparse `legacy/026.eml`: 1 messages, 3 leaves, 2656 bytes (1.0: 1 messages, 3 leaves, 2657 bytes)
- full testsuite-crlf mailparse `legacy/051.eml`: 1 messages, 6 leaves, 6188 bytes (1.0: 1 messages, 5 leaves, 6188 bytes)
- full testsuite-crlf gmime `rfc/000.eml`: 2 messages, 6 leaves, 643 bytes (1.0: 2 messages, 6 leaves, 644 bytes)
- full testsuite-crlf gmime `legacy/017.eml`: 1 messages, 3 leaves, 3387 bytes (1.0: 1 messages, 3 leaves, 4510 bytes)
- full testsuite-crlf gmime `legacy/026.eml`: 1 messages, 3 leaves, 2314 bytes (1.0: 1 messages, 3 leaves, 2657 bytes)
- full testsuite-crlf gmime `legacy/033.eml`: 1 messages, 1 leaves, 736 bytes (1.0: 1 messages, 1 leaves, 784 bytes)
- full testsuite-crlf gmime `legacy/048.eml`: 1 messages, 4 leaves, 4843 bytes (1.0: 1 messages, 4 leaves, 6570 bytes)
- full testsuite-crlf vmime `rfc/000.eml`: 2 messages, 6 leaves, 655 bytes (1.0: 2 messages, 6 leaves, 644 bytes)
- full testsuite-crlf vmime `rfc/004.eml`: 1 messages, 3 leaves, 281 bytes (1.0: 3 messages, 3 leaves, 101 bytes)
- full testsuite-crlf vmime `legacy/017.eml`: 1 messages, 3 leaves, 3403 bytes (1.0: 1 messages, 3 leaves, 4510 bytes)
- full testsuite-crlf vmime `legacy/026.eml`: 1 messages, 3 leaves, 2314 bytes (1.0: 1 messages, 3 leaves, 2657 bytes)
- full testsuite-crlf vmime `legacy/033.eml`: 1 messages, 1 leaves, 752 bytes (1.0: 1 messages, 1 leaves, 784 bytes)
- full testsuite-crlf libetpan `legacy/026.eml`: 1 messages, 3 leaves, 2655 bytes (1.0: 1 messages, 3 leaves, 2657 bytes)
- full testsuite-crlf libetpan `legacy/033.eml`: 1 messages, 1 leaves, 752 bytes (1.0: 1 messages, 1 leaves, 784 bytes)
- full testsuite-crlf libetpan `legacy/052.eml`: 1 messages, 1 leaves, 752 bytes (1.0: 1 messages, 1 leaves, 784 bytes)
- full testsuite-crlf libetpan `thirdparty/002.eml`: 1 messages, 3 leaves, 35 bytes (1.0: 1 messages, 3 leaves, 45 bytes)
- full testsuite-crlf libetpan `thirdparty/005.eml`: 1 messages, 5 leaves, 77 bytes (1.0: 1 messages, 5 leaves, 75 bytes)
- full testsuite-crlf dovecot `rfc/000.eml`: 2 messages, 6 leaves, 577 bytes (1.0: 2 messages, 6 leaves, 644 bytes)
- full testsuite-crlf dovecot `rfc/008.eml`: 2 messages, 2 leaves, 54 bytes (1.0: 2 messages, 3 leaves, 257 bytes)
- full testsuite-crlf dovecot `legacy/017.eml`: 1 messages, 3 leaves, 764 bytes (1.0: 1 messages, 3 leaves, 4510 bytes)
- full testsuite-crlf dovecot `legacy/026.eml`: 1 messages, 3 leaves, 1567 bytes (1.0: 1 messages, 3 leaves, 2657 bytes)
- full testsuite-crlf dovecot `legacy/048.eml`: 1 messages, 4 leaves, 767 bytes (1.0: 1 messages, 4 leaves, 6570 bytes)
- full enron-crlf gmime `davis-d/vargas_becton_lamb/12.`: 1 messages, 1 leaves, 2317 bytes (1.0: 1 messages, 1 leaves, 2316 bytes)
- full enron-crlf gmime `horton-s/sent_items/108.`: 1 messages, 1 leaves, 3006 bytes (1.0: 1 messages, 1 leaves, 3003 bytes)
- full enron-crlf vmime `davis-d/vargas_becton_lamb/12.`: 1 messages, 1 leaves, 2317 bytes (1.0: 1 messages, 1 leaves, 2316 bytes)
- full enron-crlf vmime `horton-s/sent_items/108.`: 1 messages, 1 leaves, 3006 bytes (1.0: 1 messages, 1 leaves, 3003 bytes)
- full enron-crlf libetpan `davis-d/vargas_becton_lamb/12.`: 1 messages, 1 leaves, 2317 bytes (1.0: 1 messages, 1 leaves, 2316 bytes)
- full enron-crlf libetpan `horton-s/sent_items/108.`: 1 messages, 1 leaves, 3006 bytes (1.0: 1 messages, 1 leaves, 3003 bytes)
- full dashes-crlf mailparse `dashes`: 1 messages, 1064 leaves, 1402 bytes (1.0: 1 messages, 3 leaves, 611851 bytes)
- full dashes-crlf dovecot `dashes`: 1 messages, 1064 leaves, 1402 bytes (1.0: 1 messages, 3 leaves, 611851 bytes)
- full attachments-lf mailparse `attachments-0`: 1 messages, 4 leaves, 584457 bytes (1.0: 1 messages, 4 leaves, 584077 bytes)
- full attachments-lf mailparse `attachments-1`: 1 messages, 4 leaves, 114458 bytes (1.0: 1 messages, 4 leaves, 114097 bytes)
- full attachments-lf mailparse `attachments-2`: 1 messages, 4 leaves, 2334464 bytes (1.0: 1 messages, 4 leaves, 2334079 bytes)
- full attachments-lf libetpan `attachments-0`: 1 messages, 4 leaves, 584457 bytes (1.0: 1 messages, 4 leaves, 584077 bytes)
- full attachments-lf libetpan `attachments-1`: 1 messages, 4 leaves, 114458 bytes (1.0: 1 messages, 4 leaves, 114097 bytes)
- full attachments-lf libetpan `attachments-2`: 1 messages, 4 leaves, 2334464 bytes (1.0: 1 messages, 4 leaves, 2334079 bytes)
- full attachments-lf dovecot `attachments-0`: 1 messages, 4 leaves, 584457 bytes (1.0: 1 messages, 4 leaves, 584077 bytes)
- full attachments-lf dovecot `attachments-1`: 1 messages, 4 leaves, 114458 bytes (1.0: 1 messages, 4 leaves, 114097 bytes)
- full attachments-lf dovecot `attachments-2`: 1 messages, 4 leaves, 2334464 bytes (1.0: 1 messages, 4 leaves, 2334079 bytes)
- full newsletter-lf mailparse `newsletter-0`: 1 messages, 3 leaves, 80845 bytes (1.0: 1 messages, 3 leaves, 80247 bytes)
- full newsletter-lf mailparse `newsletter-1`: 1 messages, 3 leaves, 92907 bytes (1.0: 1 messages, 3 leaves, 92219 bytes)
- full newsletter-lf mailparse `newsletter-2`: 1 messages, 3 leaves, 105017 bytes (1.0: 1 messages, 3 leaves, 104215 bytes)
- full newsletter-lf mailparse `newsletter-3`: 1 messages, 3 leaves, 117218 bytes (1.0: 1 messages, 3 leaves, 116340 bytes)
- full newsletter-lf mailparse `newsletter-4`: 1 messages, 3 leaves, 129240 bytes (1.0: 1 messages, 3 leaves, 128241 bytes)
- full newsletter-lf libetpan `newsletter-0`: 1 messages, 3 leaves, 80845 bytes (1.0: 1 messages, 3 leaves, 80247 bytes)
- full newsletter-lf libetpan `newsletter-1`: 1 messages, 3 leaves, 92907 bytes (1.0: 1 messages, 3 leaves, 92219 bytes)
- full newsletter-lf libetpan `newsletter-2`: 1 messages, 3 leaves, 105017 bytes (1.0: 1 messages, 3 leaves, 104215 bytes)
- full newsletter-lf libetpan `newsletter-3`: 1 messages, 3 leaves, 117218 bytes (1.0: 1 messages, 3 leaves, 116340 bytes)
- full newsletter-lf libetpan `newsletter-4`: 1 messages, 3 leaves, 129240 bytes (1.0: 1 messages, 3 leaves, 128241 bytes)
- full newsletter-lf dovecot `newsletter-0`: 1 messages, 3 leaves, 80845 bytes (1.0: 1 messages, 3 leaves, 80247 bytes)
- full newsletter-lf dovecot `newsletter-1`: 1 messages, 3 leaves, 92907 bytes (1.0: 1 messages, 3 leaves, 92219 bytes)
- full newsletter-lf dovecot `newsletter-2`: 1 messages, 3 leaves, 105017 bytes (1.0: 1 messages, 3 leaves, 104215 bytes)
- full newsletter-lf dovecot `newsletter-3`: 1 messages, 3 leaves, 117218 bytes (1.0: 1 messages, 3 leaves, 116340 bytes)
- full newsletter-lf dovecot `newsletter-4`: 1 messages, 3 leaves, 129240 bytes (1.0: 1 messages, 3 leaves, 128241 bytes)

