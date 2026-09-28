#include <glib.h>

#include "tally.h"

void cmp_utf8_leaf(cmp_tally *tally, const char *bytes, size_t len)
{
    cmp_add(tally, g_utf8_validate_len(bytes, len, NULL));
    cmp_leaf(tally, len, 1);
}
