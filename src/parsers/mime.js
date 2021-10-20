function isInlineMediaType(type) {
    return type.startsWith('image/') ||
        type.startsWith('audio/') ||
        type.startsWith('video/');
}

function parseStructure(parts, multipartType, inAlternative,
    htmlBody, textBody, attachments) {

    // For multipartType == alternative
    let textLength = textBody ? textBody.length : -1;
    let htmlLength = htmlBody ? htmlBody.length : -1;

    for (let i = 0; i < parts.length; i += 1) {
        let part = parts[i];
        let isMultipart = part.type.startsWith('multipart/');
        // Is this a body part rather than an attachment
        let isInline = part.disposition != "attachment" &&
            // Must be one of the allowed body types
            (part.type == "text/plain" ||
                part.type == "text/html" ||
                isInlineMediaType(part.type)) &&
            // If multipart/related, only the first part can be inline
            // If a text part with a filename, and not the first item
            // in the multipart, assume it is an attachment
            (i === 0 ||
                (multipartType != "related" &&
                    (isInlineMediaType(part.type) || !part.name)));

        if (isMultipart) {
            let subMultiType = part.type.split('/')[1];
            parseStructure(part.subParts, subMultiType,
                inAlternative || (subMultiType == 'alternative'),
                htmlBody, textBody, attachments);
        } else if (isInline) {
            if (multipartType == 'alternative') {
                switch (part.type) {
                    case 'text/plain':
                        textBody.push(part);
                        break;
                    case 'text/html':
                        htmlBody.push(part);
                        break;
                    default:
                        attachments.push(part);
                        break;
                }
                continue;
            } else if (inAlternative) {
                if (part.type == 'text/plain') {
                    htmlBody = null;
                }
                if (part.type == 'text/html') {
                    textBody = null;
                }
            }
            if (textBody) {
                textBody.push(part);
            }
            if (htmlBody) {
                htmlBody.push(part);
            }
            if ((!textBody || !htmlBody) &&
                isInlineMediaType(part.type)) {
                attachments.push(part);
            }
        } else {
            attachments.push(part);
        }
    }

    if (multipartType == 'alternative' && textBody && htmlBody) {
        // Found HTML part only
        if (textLength == textBody.length &&
            htmlLength != htmlBody.length) {
            for (let i = htmlLength; i < htmlBody.length; i += 1) {
                textBody.push(htmlBody[i]);
            }
        }
        // Found plaintext part only
        if (htmlLength == htmlBody.length &&
            textLength != textBody.length) {
            for (let i = textLength; i < textBody.length; i += 1) {
                htmlBody.push(textBody[i]);
            }
        }
    }
}


