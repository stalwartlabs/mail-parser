#include <gmime/gmime.h>

#include "tally.h"

void cmp_gmime_init(void)
{
    g_mime_init();
}

static GMimeMessage *parse_stream(GMimeStream *stream)
{
    GMimeParser *parser = g_mime_parser_new_with_stream(stream);
    GMimeMessage *message = g_mime_parser_construct_message(parser, NULL);
    g_object_unref(parser);
    return message;
}

static GMimeMessage *parse(const uint8_t *raw, size_t len)
{
    GMimeStream *stream = g_mime_stream_mem_new_with_buffer((const char *)raw, len);
    GMimeMessage *message = parse_stream(stream);
    g_object_unref(stream);
    return message;
}

static int is_nested(GMimeContentType *content_type)
{
    return content_type != NULL &&
           (g_mime_content_type_is_type(content_type, "message", "rfc822") ||
            g_mime_content_type_is_type(content_type, "message", "global"));
}

static void walk_object(GMimeObject *object, enum cmp_mode mode, cmp_tally *tally, int depth);

static void walk_message(GMimeMessage *message, enum cmp_mode mode, cmp_tally *tally, int depth)
{
    GMimeObject *root = g_mime_message_get_mime_part(message);

    tally->messages++;
    if (root != NULL)
        walk_object(root, mode, tally, depth);
}

static void walk_nested_bytes(GMimeDataWrapper *content, enum cmp_mode mode, cmp_tally *tally,
                              int depth)
{
    GMimeStream *decoded = g_mime_stream_mem_new();
    GMimeMessage *nested;

    g_mime_data_wrapper_write_to_stream(content, decoded);
    g_mime_stream_reset(decoded);
    nested = parse_stream(decoded);
    if (nested != NULL) {
        walk_message(nested, mode, tally, depth + 1);
        g_object_unref(nested);
    } else {
        tally->failures++;
    }
    g_object_unref(decoded);
}

static void full_text(GMimeTextPart *part, GMimeContentType *content_type, cmp_tally *tally)
{
    char *text = g_mime_text_part_get_text(part);
    size_t len = text != NULL ? strlen(text) : 0;

    if (text != NULL && g_mime_content_type_get_parameter(content_type, "charset") == NULL)
        cmp_utf8_leaf(tally, text, len);
    else
        cmp_leaf(tally, len, 1);
    g_free(text);
}

static void full_leaf(GMimePart *part, GMimeDataWrapper *content, GMimeContentType *content_type,
                      cmp_tally *tally)
{
    if (GMIME_IS_TEXT_PART(part)) {
        full_text((GMimeTextPart *)part, content_type, tally);
    } else if (content != NULL) {
        GMimeStream *decoded = g_mime_stream_mem_new();
        GByteArray *bytes;

        g_mime_data_wrapper_write_to_stream(content, decoded);
        bytes = g_mime_stream_mem_get_byte_array((GMimeStreamMem *)decoded);
        cmp_leaf(tally, bytes != NULL ? bytes->len : 0, 0);
        g_object_unref(decoded);
    } else {
        cmp_leaf(tally, 0, 0);
    }
}

static void walk_object(GMimeObject *object, enum cmp_mode mode, cmp_tally *tally, int depth)
{
    GMimeContentType *content_type = g_mime_object_get_content_type(object);

    tally->parts++;
    if (mode == CMP_STRUCTURE && content_type != NULL) {
        cmp_str(tally, g_mime_content_type_get_media_type(content_type));
        cmp_str(tally, g_mime_content_type_get_media_subtype(content_type));
    }
    if (GMIME_IS_MULTIPART(object)) {
        GMimeMultipart *multipart = (GMimeMultipart *)object;
        int count = g_mime_multipart_get_count(multipart);

        for (int i = 0; i < count; i++)
            walk_object(g_mime_multipart_get_part(multipart, i), mode, tally, depth);
    } else if (GMIME_IS_MESSAGE_PART(object)) {
        GMimeMessage *nested = g_mime_message_part_get_message((GMimeMessagePart *)object);

        if (nested != NULL)
            walk_message(nested, mode, tally, depth);
    } else if (GMIME_IS_PART(object)) {
        GMimePart *part = (GMimePart *)object;
        GMimeDataWrapper *content = g_mime_part_get_content(part);

        if (content != NULL && depth < CMP_MAX_NESTING && is_nested(content_type))
            walk_nested_bytes(content, mode, tally, depth);
        else if (mode == CMP_FULL)
            full_leaf(part, content, content_type, tally);
    }
}

int cmp_gmime_structure(const uint8_t *raw, size_t len, cmp_tally *tally)
{
    GMimeMessage *message = parse(raw, len);

    if (message == NULL)
        return 0;
    walk_message(message, CMP_STRUCTURE, tally, 0);
    g_object_unref(message);
    return 1;
}

static InternetAddressMailbox *first_mailbox(InternetAddressList *list)
{
    int count = list != NULL ? internet_address_list_length(list) : 0;

    for (int i = 0; i < count; i++) {
        InternetAddress *address = internet_address_list_get_address(list, i);

        if (INTERNET_ADDRESS_IS_MAILBOX(address))
            return (InternetAddressMailbox *)address;
        if (INTERNET_ADDRESS_IS_GROUP(address)) {
            InternetAddressList *members =
                internet_address_group_get_members((InternetAddressGroup *)address);
            InternetAddressMailbox *mailbox = first_mailbox(members);

            if (mailbox != NULL)
                return mailbox;
        }
    }
    return NULL;
}

int cmp_gmime_headers(const uint8_t *raw, size_t len, cmp_tally *tally)
{
    GMimeMessage *message = parse(raw, len);
    const char *subject;
    const char *message_id;
    InternetAddressMailbox *mailbox;
    GDateTime *date;

    if (message == NULL)
        return 0;
    subject = g_mime_message_get_subject(message);
    if (subject != NULL) {
        tally->subject++;
        cmp_str(tally, subject);
    }
    mailbox = first_mailbox(g_mime_message_get_from(message));
    if (mailbox != NULL) {
        tally->from++;
        cmp_str(tally, internet_address_get_name((InternetAddress *)mailbox));
        cmp_str(tally, internet_address_mailbox_get_addr(mailbox));
    }
    date = g_mime_message_get_date(message);
    if (date != NULL) {
        tally->date++;
        cmp_add(tally, (uint64_t)g_date_time_to_unix(date));
    }
    message_id = g_mime_message_get_message_id(message);
    if (message_id != NULL) {
        tally->message_id++;
        cmp_str(tally, message_id);
    }
    walk_message(message, CMP_HEADERS, tally, 0);
    g_object_unref(message);
    return 1;
}

int cmp_gmime_full(const uint8_t *raw, size_t len, cmp_tally *tally)
{
    GMimeMessage *message = parse(raw, len);

    if (message == NULL)
        return 0;
    walk_message(message, CMP_FULL, tally, 0);
    g_object_unref(message);
    return 1;
}
