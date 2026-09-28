#include <vmime/vmime.hpp>

#include "tally.h"

namespace {

const vmime::charset &utf8()
{
    static const vmime::charset charset(vmime::charsets::UTF_8);
    return charset;
}

std::shared_ptr<vmime::message> parse(const char *raw, size_t len)
{
    auto message = std::make_shared<vmime::message>();
    message->parse(vmime::string(raw, len));
    return message;
}

bool is_nested(const vmime::mediaType &type)
{
    return type.getType() == vmime::mediaTypes::MESSAGE &&
           (type.getSubType() == vmime::mediaTypes::MESSAGE_RFC822 || type.getSubType() == "global");
}

void walk_part(const vmime::bodyPart &part, cmp_mode mode, cmp_tally *tally, int depth);

void walk_message(const std::string &bytes, cmp_mode mode, cmp_tally *tally, int depth)
{
    auto nested = std::make_shared<vmime::message>();
    nested->parse(bytes);
    tally->messages++;
    walk_part(*nested, mode, tally, depth);
}

void full_text(const vmime::body &body, const std::string &decoded, cmp_tally *tally)
{
    const vmime::charset charset = body.getCharset();

    if (charset == utf8()) {
        cmp_utf8_leaf(tally, decoded.data(), decoded.size());
        return;
    }
    try {
        std::string converted;
        vmime::charset::convert(decoded, converted, charset, utf8());
        cmp_leaf(tally, converted.size(), 1);
    } catch (const vmime::exception &) {
        cmp_utf8_leaf(tally, decoded.data(), decoded.size());
    }
}

void walk_leaf(const vmime::body &body, const vmime::mediaType &type, cmp_mode mode,
               cmp_tally *tally, int depth)
{
    const bool nested = depth < CMP_MAX_NESTING && is_nested(type);
    std::string decoded;

    if (!nested && mode != CMP_FULL)
        return;
    try {
        vmime::utility::outputStreamStringAdapter out(decoded);
        body.getContents()->extract(out);
    } catch (const vmime::exception &) {
        tally->failures++;
        return;
    }
    if (nested)
        walk_message(decoded, mode, tally, depth + 1);
    else if (type.getType() == vmime::mediaTypes::TEXT)
        full_text(body, decoded, tally);
    else
        cmp_leaf(tally, decoded.size(), 0);
}

void walk_part(const vmime::bodyPart &part, cmp_mode mode, cmp_tally *tally, int depth)
{
    auto body = part.getBody();
    const vmime::mediaType type = body->getContentType();
    const size_t count = body->getPartCount();

    tally->parts++;
    if (mode == CMP_STRUCTURE) {
        cmp_add(tally, type.getType().size());
        cmp_add(tally, type.getSubType().size());
    }
    if (count > 0 || type.getType() == vmime::mediaTypes::MULTIPART) {
        for (size_t i = 0; i < count; ++i)
            walk_part(*body->getPartAt(i), mode, tally, depth);
        return;
    }
    walk_leaf(*body, type, mode, tally, depth);
}

template <typename T> std::shared_ptr<T> field_value(const vmime::header &header, const char *name)
{
    auto field = header.findField(name);
    return field ? field->getValue<T>() : nullptr;
}

void headers(const vmime::message &message, cmp_tally *tally)
{
    const vmime::header &header = *message.getHeader();

    if (auto subject = field_value<vmime::text>(header, vmime::fields::SUBJECT)) {
        tally->subject++;
        cmp_add(tally, subject->getConvertedText(utf8()).size());
    }
    if (auto from = field_value<vmime::mailbox>(header, vmime::fields::FROM)) {
        if (!from->isEmpty()) {
            tally->from++;
            cmp_add(tally, from->getName().getConvertedText(utf8()).size());
            cmp_add(tally, from->getEmail().toString().size());
        }
    }
    if (auto date = field_value<vmime::datetime>(header, vmime::fields::DATE)) {
        tally->date++;
        cmp_add(tally, (uint64_t)cmp_timestamp(date->getYear(), date->getMonth(), date->getDay(),
                                               date->getHour(), date->getMinute(), date->getSecond(),
                                               date->getZone()));
    }
    if (auto id = field_value<vmime::messageId>(header, vmime::fields::MESSAGE_ID)) {
        tally->message_id++;
        cmp_add(tally, id->getId().size());
    }
}

int run(const uint8_t *raw, size_t len, cmp_mode mode, cmp_tally *tally)
{
    try {
        auto message = parse(reinterpret_cast<const char *>(raw), len);
        if (mode == CMP_HEADERS)
            headers(*message, tally);
        tally->messages++;
        walk_part(*message, mode, tally, 0);
        return 1;
    } catch (const std::exception &) {
        return 0;
    }
}

}

extern "C" int cmp_vmime_structure(const uint8_t *raw, size_t len, cmp_tally *tally)
{
    return run(raw, len, CMP_STRUCTURE, tally);
}

extern "C" int cmp_vmime_headers(const uint8_t *raw, size_t len, cmp_tally *tally)
{
    return run(raw, len, CMP_HEADERS, tally);
}

extern "C" int cmp_vmime_full(const uint8_t *raw, size_t len, cmp_tally *tally)
{
    return run(raw, len, CMP_FULL, tally);
}
