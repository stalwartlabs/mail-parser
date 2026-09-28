#include "lib.h"
#include "buffer.h"
#include "istream.h"
#include "str.h"
#include "message-address.h"
#include "message-date.h"
#include "message-decoder.h"
#include "message-header-decode.h"
#include "message-id.h"
#include "message-parser.h"
#include "rfc822-parser.h"

#include "tally.h"

enum { SEEN_SUBJECT = 1, SEEN_FROM = 2, SEEN_DATE = 4, SEEN_MESSAGE_ID = 8 };

void cmp_dovecot_init(void)
{
    lib_init();
}

static bool is_header(const struct message_header_line *hdr, const char *name, size_t len)
{
    return hdr->name_len == len && strcasecmp(hdr->name, name) == 0;
}

static void content_type(const struct message_header_line *hdr, cmp_tally *tally)
{
    struct rfc822_parser_context parser;
    string_t *value = t_str_new(64);
    const char *type;
    const char *subtype;

    rfc822_parser_init(&parser, hdr->full_value, hdr->full_value_len, NULL);
    rfc822_skip_lwsp(&parser);
    (void)rfc822_parse_content_type(&parser, value);
    rfc822_parser_deinit(&parser);
    type = str_c(value);
    subtype = strchr(type, '/');
    if (subtype != NULL) {
        cmp_add(tally, (uint64_t)(subtype - type));
        cmp_str(tally, subtype + 1);
    } else {
        cmp_str(tally, type);
    }
}

static void decoded_text(const unsigned char *value, size_t len, cmp_tally *tally)
{
    buffer_t *decoded = t_buffer_create(len + 16);

    message_header_decode_utf8(value, len, decoded, NULL);
    cmp_add(tally, decoded->used);
}

static void from(const struct message_header_line *hdr, cmp_tally *tally)
{
    const struct message_address *address =
        message_address_parse(pool_datastack_create(), hdr->full_value, hdr->full_value_len,
                              UINT_MAX, 0);

    for (; address != NULL; address = address->next) {
        if (address->domain == NULL)
            continue;
        tally->from++;
        if (address->name != NULL)
            decoded_text((const unsigned char *)address->name, strlen(address->name), tally);
        cmp_add(tally, strlen(address->mailbox) + 1 + strlen(address->domain));
        return;
    }
}

static void root_header(const struct message_header_line *hdr, unsigned *seen, cmp_tally *tally)
{
    time_t timestamp;
    int timezone_offset;
    const char *ids;
    const char *id;

    if (!(*seen & SEEN_SUBJECT) && is_header(hdr, "Subject", 7)) {
        *seen |= SEEN_SUBJECT;
        tally->subject++;
        decoded_text(hdr->full_value, hdr->full_value_len, tally);
    } else if (!(*seen & SEEN_FROM) && is_header(hdr, "From", 4)) {
        *seen |= SEEN_FROM;
        from(hdr, tally);
    } else if (!(*seen & SEEN_DATE) && is_header(hdr, "Date", 4)) {
        *seen |= SEEN_DATE;
        if (message_date_parse(hdr->full_value, hdr->full_value_len, &timestamp,
                               &timezone_offset)) {
            tally->date++;
            cmp_add(tally, (uint64_t)timestamp);
        }
    } else if (!(*seen & SEEN_MESSAGE_ID) && is_header(hdr, "Message-ID", 10)) {
        *seen |= SEEN_MESSAGE_ID;
        ids = t_strndup(hdr->full_value, hdr->full_value_len);
        id = message_id_get_next(&ids);
        if (id != NULL) {
            tally->message_id++;
            cmp_str(tally, id);
        }
    }
}

static bool is_root_field(const struct message_header_line *hdr)
{
    return is_header(hdr, "Subject", 7) || is_header(hdr, "From", 4) ||
           is_header(hdr, "Date", 4) || is_header(hdr, "Message-ID", 10);
}

static bool is_decoder_field(const struct message_header_line *hdr)
{
    return is_header(hdr, "Content-Type", 12) || is_header(hdr, "Content-Transfer-Encoding", 25);
}

static void header_block(struct message_block *block, const struct message_part *root,
                         enum cmp_mode mode, unsigned *seen, cmp_tally *tally)
{
    struct message_header_line *hdr = block->hdr;

    if (mode == CMP_STRUCTURE && !hdr->continues && is_header(hdr, "Content-Type", 12)) {
        T_BEGIN {
            content_type(hdr, tally);
        } T_END;
    } else if (mode == CMP_HEADERS && block->part == root && is_root_field(hdr)) {
        if (hdr->continues) {
            hdr->use_full_value = TRUE;
        } else {
            T_BEGIN {
                root_header(hdr, seen, tally);
            } T_END;
        }
    }
}

static void body_block(struct message_decoder_context *decoder, struct message_block *block,
                       cmp_tally *tally)
{
    struct message_block decoded;

    if (block->hdr != NULL && !is_decoder_field(block->hdr))
        return;
    if (!message_decoder_decode_next_block(decoder, block, &decoded) || decoded.hdr != NULL ||
        decoded.size == 0)
        return;
    tally->decoded_bytes += decoded.size;
    if ((decoded.part->flags & MESSAGE_PART_FLAG_TEXT) != 0)
        tally->text_bytes += decoded.size;
    cmp_add(tally, decoded.size);
}

static void walk_parts(const struct message_part *part, enum cmp_mode mode, cmp_tally *tally)
{
    for (; part != NULL; part = part->next) {
        tally->parts++;
        if ((part->flags & MESSAGE_PART_FLAG_MESSAGE_RFC822) != 0) {
            if (part->children != NULL)
                tally->messages++;
        } else if (mode == CMP_FULL && (part->flags & MESSAGE_PART_FLAG_MULTIPART) == 0) {
            tally->leaves++;
            if ((part->flags & MESSAGE_PART_FLAG_TEXT) != 0)
                tally->text_leaves++;
        }
        walk_parts(part->children, mode, tally);
    }
}

static int run(const uint8_t *raw, size_t len, enum cmp_mode mode, cmp_tally *tally)
{
    const struct message_parser_settings settings = {
        .flags = mode == CMP_FULL ? 0 : MESSAGE_PARSER_FLAG_SKIP_BODY_BLOCK,
    };
    pool_t pool = pool_alloconly_create("cmp_dovecot", 4096);
    struct istream *input = i_stream_create_from_data(raw, len);
    struct message_parser_ctx *parser = message_parser_init(pool, input, &settings);
    struct message_decoder_context *decoder =
        mode == CMP_FULL ? message_decoder_init(NULL, MESSAGE_DECODER_FLAG_RETURN_BINARY) : NULL;
    const struct message_part *root = NULL;
    struct message_part *parts = NULL;
    struct message_block block;
    unsigned seen = 0;

    while (message_parser_parse_next_block(parser, &block) > 0) {
        if (root == NULL)
            root = block.part;
        if (decoder != NULL)
            body_block(decoder, &block, tally);
        else if (block.hdr != NULL)
            header_block(&block, root, mode, &seen, tally);
    }
    if (decoder != NULL)
        message_decoder_deinit(&decoder);
    message_parser_deinit(&parser, &parts);
    tally->messages++;
    walk_parts(parts, mode, tally);
    i_stream_unref(&input);
    pool_unref(&pool);
    return 1;
}

int cmp_dovecot_structure(const uint8_t *raw, size_t len, cmp_tally *tally)
{
    return run(raw, len, CMP_STRUCTURE, tally);
}

int cmp_dovecot_headers(const uint8_t *raw, size_t len, cmp_tally *tally)
{
    return run(raw, len, CMP_HEADERS, tally);
}

int cmp_dovecot_full(const uint8_t *raw, size_t len, cmp_tally *tally)
{
    return run(raw, len, CMP_FULL, tally);
}
