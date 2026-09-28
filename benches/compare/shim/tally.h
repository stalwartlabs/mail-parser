#ifndef CMP_TALLY_H
#define CMP_TALLY_H

#include <stddef.h>
#include <stdint.h>
#include <string.h>

#define CMP_MAX_NESTING 8

typedef struct cmp_tally {
    uint64_t digest;
    uint64_t messages;
    uint64_t parts;
    uint64_t leaves;
    uint64_t text_leaves;
    uint64_t decoded_bytes;
    uint64_t text_bytes;
    uint64_t failures;
    uint64_t subject;
    uint64_t from;
    uint64_t date;
    uint64_t message_id;
} cmp_tally;

enum cmp_mode { CMP_STRUCTURE, CMP_HEADERS, CMP_FULL };

#ifdef __cplusplus
extern "C" {
#endif
void cmp_utf8_leaf(cmp_tally *tally, const char *bytes, size_t len);
#ifdef __cplusplus
}
#endif

static inline void cmp_add(cmp_tally *tally, uint64_t value)
{
    tally->digest = ((tally->digest << 5) | (tally->digest >> 59)) ^ value;
}

static inline void cmp_str(cmp_tally *tally, const char *value)
{
    if (value != NULL)
        cmp_add(tally, strlen(value));
}

static inline void cmp_leaf(cmp_tally *tally, size_t len, int text)
{
    tally->leaves++;
    tally->decoded_bytes += len;
    if (text) {
        tally->text_leaves++;
        tally->text_bytes += len;
    }
    cmp_add(tally, len);
}

static inline int64_t cmp_timestamp(int64_t year, int64_t month, int64_t day, int64_t hour,
                                    int64_t minute, int64_t second, int64_t offset_minutes)
{
    int64_t y = month <= 2 ? year - 1 : year;
    int64_t era = (y >= 0 ? y : y - 399) / 400;
    int64_t yoe = y - era * 400;
    int64_t mp = (month + 9) % 12;
    int64_t doy = (153 * mp + 2) / 5 + day - 1;
    int64_t doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    int64_t days = era * 146097 + doe - 719468;
    return days * 86400 + hour * 3600 + minute * 60 + second - offset_minutes * 60;
}

#endif
