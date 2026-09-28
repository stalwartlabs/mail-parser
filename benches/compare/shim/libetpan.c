#include <stdlib.h>
#include <strings.h>

#include <libetpan/charconv.h>
#include <libetpan/mailimf.h>
#include <libetpan/mailmime.h>
#include <libetpan/mailmime_content.h>
#include <libetpan/mailmime_decode.h>

#include "tally.h"

static struct mailmime *parse(const uint8_t *raw, size_t len)
{
    struct mailmime *mime = NULL;
    size_t index = 0;

    if (mailmime_parse((const char *)raw, len, &index, &mime) != MAILIMF_NO_ERROR)
        return NULL;
    return mime;
}

static const char *media_type(const struct mailmime_content *content)
{
    const struct mailmime_type *type = content != NULL ? content->ct_type : NULL;

    if (type == NULL)
        return NULL;
    if (type->tp_type == MAILMIME_TYPE_DISCRETE_TYPE) {
        const struct mailmime_discrete_type *discrete = type->tp_data.tp_discrete_type;

        switch (discrete->dt_type) {
        case MAILMIME_DISCRETE_TYPE_TEXT:
            return "text";
        case MAILMIME_DISCRETE_TYPE_IMAGE:
            return "image";
        case MAILMIME_DISCRETE_TYPE_AUDIO:
            return "audio";
        case MAILMIME_DISCRETE_TYPE_VIDEO:
            return "video";
        case MAILMIME_DISCRETE_TYPE_APPLICATION:
            return "application";
        default:
            return discrete->dt_extension;
        }
    }
    switch (type->tp_data.tp_composite_type->ct_type) {
    case MAILMIME_COMPOSITE_TYPE_MESSAGE:
        return "message";
    case MAILMIME_COMPOSITE_TYPE_MULTIPART:
        return "multipart";
    default:
        return type->tp_data.tp_composite_type->ct_token;
    }
}

static const char *media_subtype(const struct mailmime_content *content)
{
    return content != NULL ? content->ct_subtype : NULL;
}

static int is_type(const struct mailmime_content *content, const char *type)
{
    const char *name = media_type(content);

    return name != NULL && strcasecmp(name, type) == 0;
}

static int is_nested(const struct mailmime_content *content)
{
    const char *subtype = media_subtype(content);

    return is_type(content, "message") && subtype != NULL &&
           (strcasecmp(subtype, "rfc822") == 0 || strcasecmp(subtype, "global") == 0);
}

static void walk_mime(const struct mailmime *mime, enum cmp_mode mode, cmp_tally *tally, int depth);

static void walk_root(const struct mailmime *root, enum cmp_mode mode, cmp_tally *tally, int depth)
{
    tally->messages++;
    if (root->mm_type == MAILMIME_MESSAGE && root->mm_data.mm_message.mm_msg_mime != NULL)
        walk_mime(root->mm_data.mm_message.mm_msg_mime, mode, tally, depth);
}

static void walk_nested_bytes(const char *bytes, size_t len, enum cmp_mode mode, cmp_tally *tally,
                              int depth)
{
    struct mailmime *nested = parse((const uint8_t *)bytes, len);

    if (nested == NULL) {
        tally->failures++;
        return;
    }
    walk_root(nested, mode, tally, depth + 1);
    mailmime_free(nested);
}

static void full_text(const struct mailmime_content *content, const char *bytes, size_t len,
                      cmp_tally *tally)
{
    const char *charset = content != NULL ? mailmime_content_charset_get((struct mailmime_content *)content) : NULL;
    char *converted = NULL;
    size_t converted_len = 0;

    if (charset != NULL && strcasecmp(charset, "utf-8") != 0 && strcasecmp(charset, "utf8") != 0 &&
        charconv_buffer("utf-8", charset, bytes, len, &converted, &converted_len) ==
            MAIL_CHARCONV_NO_ERROR) {
        cmp_leaf(tally, converted_len, 1);
        charconv_buffer_free(converted);
    } else {
        cmp_utf8_leaf(tally, bytes, len);
    }
}

static void walk_leaf(const struct mailmime *mime, enum cmp_mode mode, cmp_tally *tally, int depth)
{
    const struct mailmime_data *data = mime->mm_data.mm_single;
    int nested = depth < CMP_MAX_NESTING && is_nested(mime->mm_content_type);
    char *decoded = NULL;
    size_t decoded_len = 0;
    size_t index = 0;

    if (!nested && mode != CMP_FULL)
        return;
    if (data == NULL || data->dt_type != MAILMIME_DATA_TEXT) {
        if (mode == CMP_FULL)
            cmp_leaf(tally, 0, is_type(mime->mm_content_type, "text"));
        return;
    }
    if (mailmime_part_parse(data->dt_data.dt_text.dt_data, data->dt_data.dt_text.dt_length,
                            &index, data->dt_encoded ? data->dt_encoding : MAILMIME_MECHANISM_8BIT,
                            &decoded, &decoded_len) != MAILIMF_NO_ERROR) {
        tally->failures++;
        return;
    }
    if (nested)
        walk_nested_bytes(decoded, decoded_len, mode, tally, depth);
    else if (is_type(mime->mm_content_type, "text"))
        full_text(mime->mm_content_type, decoded, decoded_len, tally);
    else
        cmp_leaf(tally, decoded_len, 0);
    mailmime_decoded_part_free(decoded);
}

static void walk_mime(const struct mailmime *mime, enum cmp_mode mode, cmp_tally *tally, int depth)
{
    tally->parts++;
    if (mode == CMP_STRUCTURE) {
        cmp_str(tally, media_type(mime->mm_content_type));
        cmp_str(tally, media_subtype(mime->mm_content_type));
    }
    switch (mime->mm_type) {
    case MAILMIME_MULTIPLE:
        for (clistiter *cur = clist_begin(mime->mm_data.mm_multipart.mm_mp_list); cur != NULL;
             cur = clist_next(cur))
            walk_mime(clist_content(cur), mode, tally, depth);
        break;
    case MAILMIME_MESSAGE:
        if (mime->mm_data.mm_message.mm_msg_mime != NULL) {
            tally->messages++;
            walk_mime(mime->mm_data.mm_message.mm_msg_mime, mode, tally, depth);
        }
        break;
    default:
        walk_leaf(mime, mode, tally, depth);
        break;
    }
}

static int run(const uint8_t *raw, size_t len, enum cmp_mode mode, cmp_tally *tally)
{
    struct mailmime *root = parse(raw, len);

    if (root == NULL)
        return 0;
    walk_root(root, mode, tally, 0);
    mailmime_free(root);
    return 1;
}

int cmp_libetpan_structure(const uint8_t *raw, size_t len, cmp_tally *tally)
{
    return run(raw, len, CMP_STRUCTURE, tally);
}

static int decode_phrase(const char *value, cmp_tally *tally)
{
    char *decoded = NULL;
    size_t index = 0;

    if (mailmime_encoded_phrase_parse("utf-8", value, strlen(value), &index, "utf-8", &decoded) !=
        MAILIMF_NO_ERROR)
        return 0;
    cmp_str(tally, decoded);
    free(decoded);
    return 1;
}

enum { SEEN_SUBJECT = 1, SEEN_FROM = 2, SEEN_DATE = 4, SEEN_MESSAGE_ID = 8 };

static void header_field(const struct mailimf_field *field, unsigned *seen, cmp_tally *tally)
{
    switch (field->fld_type) {
    case MAILIMF_FIELD_SUBJECT:
        if (!(*seen & SEEN_SUBJECT)) {
            *seen |= SEEN_SUBJECT;
            if (decode_phrase(field->fld_data.fld_subject->sbj_value, tally))
                tally->subject++;
        }
        break;
    case MAILIMF_FIELD_FROM:
        if (!(*seen & SEEN_FROM)) {
            clistiter *first = clist_begin(field->fld_data.fld_from->frm_mb_list->mb_list);
            const struct mailimf_mailbox *mailbox = first != NULL ? clist_content(first) : NULL;

            *seen |= SEEN_FROM;
            if (mailbox != NULL) {
                tally->from++;
                if (mailbox->mb_display_name != NULL)
                    decode_phrase(mailbox->mb_display_name, tally);
                cmp_str(tally, mailbox->mb_addr_spec);
            }
        }
        break;
    case MAILIMF_FIELD_ORIG_DATE:
        if (!(*seen & SEEN_DATE)) {
            const struct mailimf_date_time *date = field->fld_data.fld_orig_date->dt_date_time;
            int64_t offset = (date->dt_zone / 100) * 60 + date->dt_zone % 100;

            *seen |= SEEN_DATE;
            tally->date++;
            cmp_add(tally, (uint64_t)cmp_timestamp(date->dt_year, date->dt_month, date->dt_day,
                                                   date->dt_hour, date->dt_min, date->dt_sec,
                                                   offset));
        }
        break;
    case MAILIMF_FIELD_MESSAGE_ID:
        if (!(*seen & SEEN_MESSAGE_ID)) {
            *seen |= SEEN_MESSAGE_ID;
            tally->message_id++;
            cmp_str(tally, field->fld_data.fld_message_id->mid_value);
        }
        break;
    default:
        break;
    }
}

int cmp_libetpan_headers(const uint8_t *raw, size_t len, cmp_tally *tally)
{
    struct mailmime *root = parse(raw, len);
    const struct mailimf_fields *fields;
    unsigned seen = 0;

    if (root == NULL)
        return 0;
    fields = root->mm_type == MAILMIME_MESSAGE ? root->mm_data.mm_message.mm_fields : NULL;
    if (fields != NULL) {
        for (clistiter *cur = clist_begin(fields->fld_list); cur != NULL; cur = clist_next(cur))
            header_field(clist_content(cur), &seen, tally);
    }
    walk_root(root, CMP_HEADERS, tally, 0);
    mailmime_free(root);
    return 1;
}

int cmp_libetpan_full(const uint8_t *raw, size_t len, cmp_tally *tally)
{
    return run(raw, len, CMP_FULL, tally);
}
