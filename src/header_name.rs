/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use std::{
    borrow::Cow,
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
};

macro_rules! header_names {
    ($($variant:ident, $name:literal, $lc:literal;)+) => {
        /// A header field name: one of the known names, or any other name
        /// with its original spelling.
        ///
        /// Names compare ASCII case-insensitively, so `Other("x-custom")`
        /// equals `Other("X-Custom")`, and an `Other` spelled like a known
        /// name equals, hashes and orders as that name. Known names are
        /// parsed into their variant by [`HeaderName::parse`], by the `From`
        /// conversions (which keep any other string as written in `Other`)
        /// and by the message parser, and keep their canonical spelling in
        /// [`HeaderName::as_str`]; [`crate::Header::raw_name`] gives the
        /// spelling used in a message.
        ///
        /// ```
        /// use mail_parser::HeaderName;
        ///
        /// assert_eq!(HeaderName::parse("content-type"), Some(HeaderName::ContentType));
        /// assert_eq!(HeaderName::ContentType.as_str(), "Content-Type");
        /// let custom = HeaderName::parse("X-Custom").expect("valid name");
        /// assert!(custom.is_other());
        /// assert_eq!(custom, HeaderName::from("x-custom"));
        /// ```
        #[derive(Debug, Clone)]
        #[non_exhaustive]
        pub enum HeaderName<'x> {
            $(
                #[doc = concat!("`", $name, "`.")]
                $variant,
            )+
            /// Any other name, with its original spelling.
            Other(Cow<'x, str>),
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[repr(u16)]
        pub(crate) enum HeaderId {
            $($variant,)+
        }

        pub(crate) const KNOWN_NAMES: usize = [$($lc,)+].len();

        static CANONICAL: [&str; KNOWN_NAMES] = [$($name,)+];

        const LOWERCASE: [&str; KNOWN_NAMES] = [$($lc,)+];

        static VARIANTS: [HeaderName<'static>; KNOWN_NAMES] = [$(HeaderName::$variant,)+];

        impl HeaderName<'_> {
            pub(crate) fn id(&self) -> u16 {
                match self {
                    $(HeaderName::$variant => HeaderId::$variant as u16,)+
                    HeaderName::Other(_) => OTHER_ID,
                }
            }
        }

        pub(crate) fn lookup_lowercase(name: &[u8]) -> Option<u16> {
            hashify::fnc_map!(name,
                $($lc => Some(HeaderId::$variant as u16),)+
                _ => None
            )
        }
    };
}

pub(crate) const OTHER_ID: u16 = u16::MAX;
pub(crate) const FIRST_VENDOR: u16 = HeaderId::XOriginalTo as u16;

header_names! {
    Subject, "Subject", "subject";
    From, "From", "from";
    To, "To", "to";
    Cc, "Cc", "cc";
    Date, "Date", "date";
    Bcc, "Bcc", "bcc";
    ReplyTo, "Reply-To", "reply-to";
    Sender, "Sender", "sender";
    Comments, "Comments", "comments";
    InReplyTo, "In-Reply-To", "in-reply-to";
    Keywords, "Keywords", "keywords";
    Received, "Received", "received";
    MessageId, "Message-ID", "message-id";
    References, "References", "references";
    ReturnPath, "Return-Path", "return-path";
    MimeVersion, "MIME-Version", "mime-version";
    ContentDescription, "Content-Description", "content-description";
    ContentId, "Content-ID", "content-id";
    ContentLanguage, "Content-Language", "content-language";
    ContentLocation, "Content-Location", "content-location";
    ContentTransferEncoding, "Content-Transfer-Encoding", "content-transfer-encoding";
    ContentType, "Content-Type", "content-type";
    ContentDisposition, "Content-Disposition", "content-disposition";
    ResentTo, "Resent-To", "resent-to";
    ResentFrom, "Resent-From", "resent-from";
    ResentBcc, "Resent-Bcc", "resent-bcc";
    ResentCc, "Resent-Cc", "resent-cc";
    ResentSender, "Resent-Sender", "resent-sender";
    ResentDate, "Resent-Date", "resent-date";
    ResentMessageId, "Resent-Message-ID", "resent-message-id";
    ListArchive, "List-Archive", "list-archive";
    ListHelp, "List-Help", "list-help";
    ListId, "List-ID", "list-id";
    ListOwner, "List-Owner", "list-owner";
    ListPost, "List-Post", "list-post";
    ListSubscribe, "List-Subscribe", "list-subscribe";
    ListUnsubscribe, "List-Unsubscribe", "list-unsubscribe";
    DkimSignature, "DKIM-Signature", "dkim-signature";
    ArcAuthenticationResults, "ARC-Authentication-Results", "arc-authentication-results";
    ArcMessageSignature, "ARC-Message-Signature", "arc-message-signature";
    ArcSeal, "ARC-Seal", "arc-seal";
    Dkim2Signature, "DKIM2-Signature", "dkim2-signature";
    MessageInstance, "Message-Instance", "message-instance";
    AcceptLanguage, "Accept-Language", "accept-language";
    AlternateRecipient, "Alternate-Recipient", "alternate-recipient";
    ArchivedAt, "Archived-At", "archived-at";
    AuthenticationResults, "Authentication-Results", "authentication-results";
    AutoSubmitted, "Auto-Submitted", "auto-submitted";
    Autoforwarded, "Autoforwarded", "autoforwarded";
    Autosubmitted, "Autosubmitted", "autosubmitted";
    ContentAlternative, "Content-Alternative", "content-alternative";
    ContentDuration, "Content-Duration", "content-duration";
    ContentFeatures, "Content-features", "content-features";
    ContentMd5, "Content-MD5", "content-md5";
    ContentTranslationType, "Content-Translation-Type", "content-translation-type";
    Conversion, "Conversion", "conversion";
    ConversionWithLoss, "Conversion-With-Loss", "conversion-with-loss";
    DlExpansionHistory, "DL-Expansion-History", "dl-expansion-history";
    DeferredDelivery, "Deferred-Delivery", "deferred-delivery";
    DeliveryDate, "Delivery-Date", "delivery-date";
    DiscardedX400IpmsExtensions, "Discarded-X400-IPMS-Extensions", "discarded-x400-ipms-extensions";
    DiscardedX400MtsExtensions, "Discarded-X400-MTS-Extensions", "discarded-x400-mts-extensions";
    DiscloseRecipients, "Disclose-Recipients", "disclose-recipients";
    DispositionNotificationOptions, "Disposition-Notification-Options", "disposition-notification-options";
    DispositionNotificationTo, "Disposition-Notification-To", "disposition-notification-to";
    DowngradedFinalRecipient, "Downgraded-Final-Recipient", "downgraded-final-recipient";
    DowngradedInReplyTo, "Downgraded-In-Reply-To", "downgraded-in-reply-to";
    DowngradedMessageId, "Downgraded-Message-Id", "downgraded-message-id";
    DowngradedOriginalRecipient, "Downgraded-Original-Recipient", "downgraded-original-recipient";
    DowngradedReferences, "Downgraded-References", "downgraded-references";
    Encoding, "Encoding", "encoding";
    Expires, "Expires", "expires";
    GenerateDeliveryReport, "Generate-Delivery-Report", "generate-delivery-report";
    HpOuter, "HP-Outer", "hp-outer";
    Importance, "Importance", "importance";
    IncompleteCopy, "Incomplete-Copy", "incomplete-copy";
    Language, "Language", "language";
    LatestDeliveryTime, "Latest-Delivery-Time", "latest-delivery-time";
    ListUnsubscribePost, "List-Unsubscribe-Post", "list-unsubscribe-post";
    MessageContext, "Message-Context", "message-context";
    MessageType, "Message-Type", "message-type";
    MmhsExemptedAddress, "MMHS-Exempted-Address", "mmhs-exempted-address";
    MmhsExtendedAuthorisationInfo, "MMHS-Extended-Authorisation-Info", "mmhs-extended-authorisation-info";
    MmhsSubjectIndicatorCodes, "MMHS-Subject-Indicator-Codes", "mmhs-subject-indicator-codes";
    MmhsHandlingInstructions, "MMHS-Handling-Instructions", "mmhs-handling-instructions";
    MmhsMessageInstructions, "MMHS-Message-Instructions", "mmhs-message-instructions";
    MmhsCodressMessageIndicator, "MMHS-Codress-Message-Indicator", "mmhs-codress-message-indicator";
    MmhsOriginatorReference, "MMHS-Originator-Reference", "mmhs-originator-reference";
    MmhsPrimaryPrecedence, "MMHS-Primary-Precedence", "mmhs-primary-precedence";
    MmhsCopyPrecedence, "MMHS-Copy-Precedence", "mmhs-copy-precedence";
    MmhsMessageType, "MMHS-Message-Type", "mmhs-message-type";
    MmhsOtherRecipientsIndicatorTo, "MMHS-Other-Recipients-Indicator-To", "mmhs-other-recipients-indicator-to";
    MmhsOtherRecipientsIndicatorCc, "MMHS-Other-Recipients-Indicator-CC", "mmhs-other-recipients-indicator-cc";
    MmhsAcp127MessageIdentifier, "MMHS-Acp127-Message-Identifier", "mmhs-acp127-message-identifier";
    MmhsOriginatorPlad, "MMHS-Originator-PLAD", "mmhs-originator-plad";
    MtPriority, "MT-Priority", "mt-priority";
    Organization, "Organization", "organization";
    OriginalEncodedInformationTypes, "Original-Encoded-Information-Types", "original-encoded-information-types";
    OriginalFrom, "Original-From", "original-from";
    OriginalMessageId, "Original-Message-ID", "original-message-id";
    OriginalRecipient, "Original-Recipient", "original-recipient";
    OriginatorReturnAddress, "Originator-Return-Address", "originator-return-address";
    OriginalSubject, "Original-Subject", "original-subject";
    PicsLabel, "PICS-Label", "pics-label";
    PreventNonDeliveryReport, "Prevent-NonDelivery-Report", "prevent-nondelivery-report";
    Priority, "Priority", "priority";
    ReceivedSpf, "Received-SPF", "received-spf";
    ReplyBy, "Reply-By", "reply-by";
    RequireRecipientValidSince, "Require-Recipient-Valid-Since", "require-recipient-valid-since";
    Sensitivity, "Sensitivity", "sensitivity";
    Solicitation, "Solicitation", "solicitation";
    Supersedes, "Supersedes", "supersedes";
    TlsReportDomain, "TLS-Report-Domain", "tls-report-domain";
    TlsReportSubmitter, "TLS-Report-Submitter", "tls-report-submitter";
    TlsRequired, "TLS-Required", "tls-required";
    VbrInfo, "VBR-Info", "vbr-info";
    X400ContentIdentifier, "X400-Content-Identifier", "x400-content-identifier";
    X400ContentReturn, "X400-Content-Return", "x400-content-return";
    X400ContentType, "X400-Content-Type", "x400-content-type";
    X400MtsIdentifier, "X400-MTS-Identifier", "x400-mts-identifier";
    X400Originator, "X400-Originator", "x400-originator";
    X400Received, "X400-Received", "x400-received";
    X400Recipients, "X400-Recipients", "x400-recipients";
    X400Trace, "X400-Trace", "x400-trace";
    ApparentlyTo, "Apparently-To", "apparently-to";
    Author, "Author", "author";
    CfblAddress, "CFBL-Address", "cfbl-address";
    CfblFeedbackId, "CFBL-Feedback-ID", "cfbl-feedback-id";
    DeliveredTo, "Delivered-To", "delivered-to";
    EdiintFeatures, "EDIINT-Features", "ediint-features";
    EesstVersion, "Eesst-Version", "eesst-version";
    ErrorsTo, "Errors-To", "errors-to";
    Face, "Face", "face";
    FormSub, "Form-Sub", "form-sub";
    JabberId, "Jabber-ID", "jabber-id";
    MmhsAuthorizingUsers, "MMHS-Authorizing-Users", "mmhs-authorizing-users";
    Privicon, "Privicon", "privicon";
    SioLabel, "SIO-Label", "sio-label";
    SioLabelHistory, "SIO-Label-History", "sio-label-history";
    WrongRecipient, "Wrong-Recipient", "wrong-recipient";
    XOriginalTo, "X-Original-To", "x-original-to";
    ReturnReceiptTo, "Return-Receipt-To", "return-receipt-to";
    XSpamStatus, "X-Spam-Status", "x-spam-status";
    XSpamScore, "X-Spam-Score", "x-spam-score";
    XSpamFlag, "X-Spam-Flag", "x-spam-flag";
    XSpamResult, "X-Spam-Result", "x-spam-result";
    XPriority, "X-Priority", "x-priority";
    XMSMailPriority, "X-MSMail-Priority", "x-msmail-priority";
    XMailer, "X-Mailer", "x-mailer";
    UserAgent, "User-Agent", "user-agent";
    XMimeOLE, "X-MimeOLE", "x-mimeole";
    XOriginatingIp, "X-Originating-IP", "x-originating-ip";
    XForwardedTo, "X-Forwarded-To", "x-forwarded-to";
    XForwardedFor, "X-Forwarded-For", "x-forwarded-for";
    XAutoResponseSuppress, "X-Auto-Response-Suppress", "x-auto-response-suppress";
    Precedence, "Precedence", "precedence";
    ThreadIndex, "Thread-Index", "thread-index";
    ThreadTopic, "Thread-Topic", "thread-topic";
    FeedbackId, "Feedback-ID", "feedback-id";
    XUniversallyUniqueIdentifier, "X-Universally-Unique-Identifier", "x-universally-unique-identifier";
    XVirusScanned, "X-Virus-Scanned", "x-virus-scanned";
    XMailFrom, "X-MailFrom", "x-mailfrom";
    XMailmanVersion, "X-Mailman-Version", "x-mailman-version";
    MessageIdHash, "Message-ID-Hash", "message-id-hash";
    XMessageIdHash, "X-Message-ID-Hash", "x-message-id-hash";
    XMailmanRuleMisses, "X-Mailman-Rule-Misses", "x-mailman-rule-misses";
    XSpamLevel, "X-Spam-Level", "x-spam-level";
    XReceived, "X-Received", "x-received";
    XGmMessageState, "X-Gm-Message-State", "x-gm-message-state";
    XGoogleDkimSignature, "X-Google-DKIM-Signature", "x-google-dkim-signature";
    XMailboxLine, "X-Mailbox-Line", "x-mailbox-line";
    XGmGg, "X-Gm-Gg", "x-gm-gg";
    XOriginalFrom, "X-Original-From", "x-original-from";
    XGoogleSmtpSource, "X-Google-Smtp-Source", "x-google-smtp-source";
    XMsTnefCorrelator, "X-MS-TNEF-Correlator", "x-ms-tnef-correlator";
    XMsHasAttach, "X-MS-Has-Attach", "x-ms-has-attach";
    XMeProxy, "X-ME-Proxy", "x-me-proxy";
    XMeProxyCause, "X-ME-Proxy-Cause", "x-me-proxy-cause";
    XMeSender, "X-ME-Sender", "x-me-sender";
    XGmFeatures, "X-Gm-Features", "x-gm-features";
    XMsExchangeTransportCrossTenantHeadersStamped, "X-MS-Exchange-Transport-CrossTenantHeadersStamped", "x-ms-exchange-transport-crosstenantheadersstamped";
    XMsPublicTrafficType, "X-MS-PublicTrafficType", "x-ms-publictraffictype";
    XOriginatorOrg, "X-OriginatorOrg", "x-originatororg";
    XMsExchangeCrossTenantNetworkMessageId, "X-MS-Exchange-CrossTenant-Network-Message-Id", "x-ms-exchange-crosstenant-network-message-id";
    XMsExchangeCrossTenantAuthSource, "X-MS-Exchange-CrossTenant-AuthSource", "x-ms-exchange-crosstenant-authsource";
    XMsExchangeCrossTenantAuthAs, "X-MS-Exchange-CrossTenant-AuthAs", "x-ms-exchange-crosstenant-authas";
    XMsExchangeCrossTenantOriginalArrivalTime, "X-MS-Exchange-CrossTenant-OriginalArrivalTime", "x-ms-exchange-crosstenant-originalarrivaltime";
    XMicrosoftAntispam, "X-Microsoft-Antispam", "x-microsoft-antispam";
    XMsExchangeCrossTenantId, "X-MS-Exchange-CrossTenant-Id", "x-ms-exchange-crosstenant-id";
    XMsTrafficTypeDiagnostic, "X-MS-TrafficTypeDiagnostic", "x-ms-traffictypediagnostic";
    XMicrosoftAntispamMessageInfo, "X-Microsoft-Antispam-Message-Info", "x-microsoft-antispam-message-info";
    XMsOffice365FilteringCorrelationId, "X-MS-Office365-Filtering-Correlation-Id", "x-ms-office365-filtering-correlation-id";
    XMsExchangeCrossTenantFromEntityHeader, "X-MS-Exchange-CrossTenant-FromEntityHeader", "x-ms-exchange-crosstenant-fromentityheader";
    XMsExchangeAntiSpamMessageDataChunkCount, "X-MS-Exchange-AntiSpam-MessageData-ChunkCount", "x-ms-exchange-antispam-messagedata-chunkcount";
    XMsExchangeAntiSpamMessageData0, "X-MS-Exchange-AntiSpam-MessageData-0", "x-ms-exchange-antispam-messagedata-0";
    XMsExchangeSenderADCheck, "X-MS-Exchange-SenderADCheck", "x-ms-exchange-senderadcheck";
    XForefrontAntispamReport, "X-Forefront-Antispam-Report", "x-forefront-antispam-report";
    XMsExchangeAntiSpamRelay, "X-MS-Exchange-AntiSpam-Relay", "x-ms-exchange-antispam-relay";
    XMsExchangeCrossTenantMailboxType, "X-MS-Exchange-CrossTenant-MailboxType", "x-ms-exchange-crosstenant-mailboxtype";
    XMsExchangeCrossTenantUserPrincipalName, "X-MS-Exchange-CrossTenant-UserPrincipalName", "x-ms-exchange-crosstenant-userprincipalname";
    XGitHubReason, "X-GitHub-Reason", "x-github-reason";
    XGitHubRecipientAddress, "X-GitHub-Recipient-Address", "x-github-recipient-address";
    XGitHubRecipient, "X-GitHub-Recipient", "x-github-recipient";
    Destinations, "Destinations", "destinations";
    XGitHubSender, "X-GitHub-Sender", "x-github-sender";
    XGitHubNotifyPlatform, "X-GitHub-Notify-Platform", "x-github-notify-platform";
    XThreadId, "X-ThreadId", "x-threadid";
    XSesOutgoing, "X-SES-Outgoing", "x-ses-outgoing";
    XProofpointVirusVersion, "X-Proofpoint-Virus-Version", "x-proofpoint-virus-version";
    XGitHubAssignees, "X-GitHub-Assignees", "x-github-assignees";
    XGitHubLabels, "X-GitHub-Labels", "x-github-labels";
    BimiSelector, "BIMI-Selector", "bimi-selector";
    XStripeEid, "X-Stripe-EID", "x-stripe-eid";
    XAttachmentId, "X-Attachment-Id", "x-attachment-id";
    XTestIDTracker, "X-Test-IDTracker", "x-test-idtracker";
    XIetfIDTracker, "X-IETF-IDTracker", "x-ietf-idtracker";
    XPmMessageId, "X-Pm-Message-ID", "x-pm-message-id";
    XAntiAbuse, "X-AntiAbuse", "x-antiabuse";
    Autocrypt, "Autocrypt", "autocrypt";
    XComplaintsTo, "X-Complaints-To", "x-complaints-to";
    XProofpointOrigGuid, "X-Proofpoint-ORIG-GUID", "x-proofpoint-orig-guid";
    XProofpointGuid, "X-Proofpoint-GUID", "x-proofpoint-guid";
    XGitHubIssueState, "X-GitHub-IssueState", "x-github-issuestate";
    XMsReactions, "X-MS-Reactions", "x-ms-reactions";
    XPmMtaPool, "X-Pm-MTA-Pool", "x-pm-mta-pool";
    XPmRcpt, "X-Pm-RCPT", "x-pm-rcpt";
    XProofpointSpamDetailsEnc, "X-Proofpoint-Spam-Details-Enc", "x-proofpoint-spam-details-enc";
    XPmMessageOptions, "X-Pm-Message-Options", "x-pm-message-options";
    XSenderId, "X-Sender-ID", "x-sender-id";
    XGitHubPullRequestStatus, "X-GitHub-PullRequestStatus", "x-github-pullrequeststatus";
    XAuthorityAnalysis, "X-Authority-Analysis", "x-authority-analysis";
    XForwardedEncrypted, "X-Forwarded-Encrypted", "x-forwarded-encrypted";
    XMsExchangeAntiSpamExternalHopMessageDataChunkCount, "X-MS-Exchange-AntiSpam-ExternalHop-MessageData-ChunkCount", "x-ms-exchange-antispam-externalhop-messagedata-chunkcount";
    XMsExchangeAntiSpamExternalHopMessageData0, "X-MS-Exchange-AntiSpam-ExternalHop-MessageData-0", "x-ms-exchange-antispam-externalhop-messagedata-0";
    XMsExchangeCrossTenantRmsPersistedConsumerOrg, "X-MS-Exchange-CrossTenant-RMS-PersistedConsumerOrg", "x-ms-exchange-crosstenant-rms-persistedconsumerorg";
    XExchangeRoutingPolicyChecked, "X-Exchange-RoutingPolicyChecked", "x-exchange-routingpolicychecked";
    XMsExchangeCrossTenantOriginalAttributedTenantConnectingIp, "X-MS-Exchange-CrossTenant-OriginalAttributedTenantConnectingIp", "x-ms-exchange-crosstenant-originalattributedtenantconnectingip";
    XMsExchangeMessageSentRepresentingType, "X-MS-Exchange-MessageSentRepresentingType", "x-ms-exchange-messagesentrepresentingtype";
    XMsExchangeTransportCrossTenantHeadersStripped, "X-MS-Exchange-Transport-CrossTenantHeadersStripped", "x-ms-exchange-transport-crosstenantheadersstripped";
    XMsOffice365FilteringCorrelationIdPrvs, "X-MS-Office365-Filtering-Correlation-Id-Prvs", "x-ms-office365-filtering-correlation-id-prvs";
    XMicrosoftAntispamMessageInfoOriginal, "X-Microsoft-Antispam-Message-Info-Original", "x-microsoft-antispam-message-info-original";
    XMsExchangeAuthenticationResults, "X-MS-Exchange-Authentication-Results", "x-ms-exchange-authentication-results";
    XForefrontAntispamReportUntrusted, "X-Forefront-Antispam-Report-Untrusted", "x-forefront-antispam-report-untrusted";
    XMsExchangeAtpMessageProperties, "X-MS-Exchange-AtpMessageProperties", "x-ms-exchange-atpmessageproperties";
    XMicrosoftAntispamUntrusted, "X-Microsoft-Antispam-Untrusted", "x-microsoft-antispam-untrusted";
    XMsExchangeAntiSpamMessageData1, "X-MS-Exchange-AntiSpam-MessageData-1", "x-ms-exchange-antispam-messagedata-1";
    XMissiveId, "X-Missive-Id", "x-missive-id";
    XIadbIpReverse, "X-IADB-IP-Reverse", "x-iadb-ip-reverse";
    XReportAbuse, "X-Report-Abuse", "x-report-abuse";
    DkimFilter, "DKIM-Filter", "dkim-filter";
    XMeReceived, "X-ME-Received", "x-me-received";
    XProofpointSpamDetails, "X-Proofpoint-Spam-Details", "x-proofpoint-spam-details";
}

const fn longest(names: &[&str]) -> usize {
    let mut index = 0;
    let mut max = 0;
    while index < names.len() {
        if names[index].len() > max {
            max = names[index].len();
        }
        index += 1;
    }
    max
}

pub(crate) const MAX_NAME_LEN: usize = longest(&LOWERCASE);

pub(crate) fn known(id: u16) -> Option<&'static HeaderName<'static>> {
    VARIANTS.get(id as usize)
}

pub(crate) fn canonical(id: u16) -> &'static str {
    CANONICAL.get(id as usize).copied().unwrap_or_default()
}

pub(crate) fn lookup(name: &[u8]) -> Option<u16> {
    let mut buf = [0u8; MAX_NAME_LEN];
    let lowercase = buf.get_mut(..name.len())?;
    lowercase
        .iter_mut()
        .zip(name)
        .for_each(|(out, byte)| *out = byte.to_ascii_lowercase());
    lookup_lowercase(lowercase)
}

pub(crate) fn trim_blank_end(name: &[u8]) -> &[u8] {
    let len = name
        .iter()
        .rposition(|&byte| !matches!(byte, b' ' | b'\t'))
        .map_or(0, |last| last + 1);
    name.get(..len).unwrap_or_default()
}

#[inline(never)]
fn other_id(name: &str) -> u16 {
    lookup(name.as_bytes()).unwrap_or(OTHER_ID)
}

fn is_ftext(byte: u8) -> bool {
    matches!(byte, b'!'..=b'9' | b';'..=b'~')
}

impl<'x> HeaderName<'x> {
    /// Parses a header name. Returns `None` for an empty name or one with a
    /// byte that RFC 5322 does not allow in a field name (anything but
    /// printable US-ASCII other than the colon). Never allocates.
    pub fn parse(name: impl Into<Cow<'x, str>>) -> Option<HeaderName<'x>> {
        let name = name.into();
        (!name.is_empty() && name.bytes().all(is_ftext)).then(|| HeaderName::resolve(name))
    }

    fn resolve(name: Cow<'x, str>) -> HeaderName<'x> {
        match lookup(name.as_bytes()).and_then(known) {
            Some(known) => known.clone(),
            None => HeaderName::Other(name),
        }
    }

    /// Resolves the name once, for code that looks the same name up in many
    /// parts or messages: [`crate::Headers::all_key`] and
    /// [`crate::Headers::get_key`] then skip the name lookup that
    /// [`crate::Headers::all`] and [`crate::Headers::get`] do on every call.
    ///
    /// ```
    /// use mail_parser::{HeaderName, MessageParser};
    ///
    /// let message = MessageParser::new()
    ///     .parse(b"X-Tag: a\r\nX-Tag: b\r\n\r\nbody")
    ///     .expect("message");
    /// let name = HeaderName::from("x-tag");
    /// let key = name.key();
    /// assert_eq!(message.headers().all_key(key).count(), 2);
    /// assert_eq!(
    ///     message.headers().get_key(key).map(|header| header.raw_value()),
    ///     Some(&b" b\r\n"[..])
    /// );
    /// ```
    #[inline]
    pub fn key(&self) -> HeaderKey<'_> {
        HeaderKey {
            id: self.resolved_id(),
            name: self.as_str(),
        }
    }

    #[inline]
    pub(crate) fn resolved_id(&self) -> u16 {
        match self {
            HeaderName::Other(name) => other_id(name),
            known => known.id(),
        }
    }

    /// Converts the name into one that owns its spelling.
    pub fn into_owned(self) -> HeaderName<'static> {
        match self {
            HeaderName::Other(name) => HeaderName::Other(name.into_owned().into()),
            known => known
                .known_static()
                .cloned()
                .unwrap_or(HeaderName::Other(Cow::Borrowed(""))),
        }
    }

    fn known_static(&self) -> Option<&'static HeaderName<'static>> {
        match self {
            HeaderName::Other(_) => None,
            known => VARIANTS.get(known.id() as usize),
        }
    }

    /// Canonical spelling of a known name; empty for other names.
    pub fn as_static_str(&self) -> &'static str {
        match self {
            HeaderName::Other(_) => "",
            known => CANONICAL
                .get(known.id() as usize)
                .copied()
                .unwrap_or_default(),
        }
    }

    /// Canonical spelling of a known name, or the original spelling of any
    /// other name.
    pub fn as_str(&self) -> &str {
        match self {
            HeaderName::Other(name) => name.as_ref(),
            known => known.as_static_str(),
        }
    }

    /// Whether this is not one of the known names.
    pub fn is_other(&self) -> bool {
        matches!(self, HeaderName::Other(_))
    }

    /// Whether this is one of the seven `Content-*` MIME header fields.
    pub fn is_mime_header(&self) -> bool {
        matches!(
            self,
            HeaderName::ContentDescription
                | HeaderName::ContentId
                | HeaderName::ContentLanguage
                | HeaderName::ContentLocation
                | HeaderName::ContentTransferEncoding
                | HeaderName::ContentType
                | HeaderName::ContentDisposition
        )
    }

    /// Whether the default parser yields a structured value for this name.
    pub fn is_structured(&self) -> bool {
        matches!(
            self,
            HeaderName::Subject
                | HeaderName::Comments
                | HeaderName::ContentDescription
                | HeaderName::ContentLocation
                | HeaderName::ContentTransferEncoding
                | HeaderName::From
                | HeaderName::To
                | HeaderName::Cc
                | HeaderName::Bcc
                | HeaderName::ReplyTo
                | HeaderName::Sender
                | HeaderName::ResentTo
                | HeaderName::ResentFrom
                | HeaderName::ResentBcc
                | HeaderName::ResentCc
                | HeaderName::ResentSender
                | HeaderName::ListArchive
                | HeaderName::ListHelp
                | HeaderName::ListId
                | HeaderName::ListOwner
                | HeaderName::ListPost
                | HeaderName::ListSubscribe
                | HeaderName::ListUnsubscribe
                | HeaderName::Date
                | HeaderName::ResentDate
                | HeaderName::MessageId
                | HeaderName::References
                | HeaderName::InReplyTo
                | HeaderName::ReturnPath
                | HeaderName::ContentId
                | HeaderName::ResentMessageId
                | HeaderName::Keywords
                | HeaderName::ContentLanguage
                | HeaderName::Received
                | HeaderName::ContentType
                | HeaderName::ContentDisposition
        )
    }
}

impl PartialEq for HeaderName<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (HeaderName::Other(a), HeaderName::Other(b)) => a.eq_ignore_ascii_case(b),
            (HeaderName::Other(name), known) | (known, HeaderName::Other(name)) => {
                name.eq_ignore_ascii_case(known.as_static_str())
            }
            _ => self.id() == other.id(),
        }
    }
}

impl Eq for HeaderName<'_> {}

impl Hash for HeaderName<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match (self, self.resolved_id()) {
            (HeaderName::Other(name), OTHER_ID) => name
                .bytes()
                .for_each(|byte| byte.to_ascii_lowercase().hash(state)),
            (_, id) => id.hash(state),
        }
    }
}

impl PartialOrd for HeaderName<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HeaderName<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.resolved_id(), other.resolved_id()) {
            (OTHER_ID, OTHER_ID) => self
                .as_str()
                .bytes()
                .map(|byte| byte.to_ascii_lowercase())
                .cmp(other.as_str().bytes().map(|byte| byte.to_ascii_lowercase())),
            (a, b) => a.cmp(&b),
        }
    }
}

impl fmt::Display for HeaderName<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'x> From<&'x str> for HeaderName<'x> {
    fn from(value: &'x str) -> Self {
        HeaderName::resolve(Cow::Borrowed(value))
    }
}

impl<'x> From<Cow<'x, str>> for HeaderName<'x> {
    fn from(value: Cow<'x, str>) -> Self {
        HeaderName::resolve(value)
    }
}

impl From<String> for HeaderName<'_> {
    fn from(value: String) -> Self {
        HeaderName::resolve(Cow::Owned(value))
    }
}

impl<'x> From<&HeaderName<'x>> for HeaderName<'x> {
    fn from(value: &HeaderName<'x>) -> Self {
        value.clone()
    }
}

impl<'x> From<HeaderName<'x>> for Cow<'x, str> {
    fn from(name: HeaderName<'x>) -> Self {
        match name {
            HeaderName::Other(name) => name,
            known => Cow::Borrowed(known.as_static_str()),
        }
    }
}

impl From<HeaderName<'_>> for String {
    fn from(name: HeaderName<'_>) -> Self {
        name.as_str().to_string()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for HeaderName<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for HeaderName<'static> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(HeaderName::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASELINE_NAMES: &str = include_str!("../resources/header_names.txt");

    #[test]
    fn baseline_names_round_trip() {
        for name in BASELINE_NAMES.lines() {
            let parsed = HeaderName::parse(name).expect("valid name");
            assert!(!parsed.is_other(), "{name} is not known");
            assert_eq!(parsed.as_str(), name);
            assert_eq!(HeaderName::parse(parsed.as_str()), Some(parsed.clone()));
            assert_eq!(
                HeaderName::parse(name.to_ascii_uppercase()),
                Some(parsed.clone())
            );
        }
    }

    #[test]
    fn vendor_names_follow_the_baseline_names() {
        assert_eq!(FIRST_VENDOR as usize, BASELINE_NAMES.lines().count());
        for (id, name) in BASELINE_NAMES.lines().enumerate() {
            assert_eq!(canonical(id as u16), name);
        }
    }

    #[test]
    fn every_known_name_round_trips() {
        for (canonical, lowercase) in CANONICAL.iter().zip(LOWERCASE.iter()) {
            assert_eq!(&canonical.to_ascii_lowercase(), lowercase);
            let parsed = HeaderName::parse(*canonical).expect("valid");
            assert_eq!(parsed.as_str(), *canonical);
            assert_eq!(lookup(lowercase.as_bytes()), Some(parsed.id()));
        }
        assert_eq!(KNOWN_NAMES, 252);
        assert_eq!(MAX_NAME_LEN, 62);
    }

    #[test]
    fn other_names() {
        let custom = HeaderName::parse("X-Custom-Field").expect("valid");
        assert_eq!(custom, HeaderName::Other("x-custom-field".into()));
        assert_eq!(custom.as_str(), "X-Custom-Field");
        assert!(HeaderName::parse("").is_none());
        assert!(HeaderName::parse("mal formed").is_none());
        assert_eq!(HeaderName::parse("X-Mailer"), Some(HeaderName::XMailer));
        assert_eq!(
            HeaderName::parse("x-face").map(|n| n.is_other()),
            Some(true)
        );
        assert_eq!(
            HeaderName::parse("Original-Encoded-Information-Types-Extra").map(|n| n.is_other()),
            Some(true)
        );
        assert_eq!(lookup(b"SUBJECT"), Some(HeaderName::Subject.id()));
        for name in [
            &b" subject"[..],
            b"Sub ject",
            b"MIME-version ",
            b"From\r",
            b"F\x0crom",
            b":From",
            b"",
        ] {
            assert_eq!(lookup(name), None, "{name:?}");
        }
    }

    #[test]
    fn every_field_name_character_is_accepted() {
        for name in ["X.Spam", "A@[]{}~", "X!#$%&'*+/=?^`|", "X;Y"] {
            let parsed = HeaderName::parse(name).expect("valid field name");
            assert_eq!(parsed, HeaderName::Other(name.into()));
            assert_eq!(parsed.as_str(), name);
        }
        for name in ["", "X:Y", "X Y", "caf\u{e9}", "X\u{7f}", "X\tY"] {
            assert_eq!(HeaderName::parse(name), None, "{name:?}");
        }
        for name in ["X.Spam", "mal formed", ":From", "caf\u{e9}", ""] {
            assert_eq!(HeaderName::from(name).as_str(), name);
            assert_eq!(HeaderName::from(name.to_string()).as_str(), name);
            assert_eq!(HeaderName::from(Cow::Borrowed(name)).as_str(), name);
        }
        assert_eq!(HeaderName::from("x-mailer"), HeaderName::XMailer);
        assert!(HeaderName::from("Sub ject").is_other());
    }

    #[test]
    fn other_spelling_a_known_name_is_that_name() {
        use std::collections::{BTreeSet, HashSet};

        let pairs = [
            (HeaderName::Other("Subject".into()), HeaderName::Subject),
            (HeaderName::Other("x-MAILER".into()), HeaderName::XMailer),
            (
                HeaderName::Other("content-type".into()),
                HeaderName::ContentType,
            ),
        ];
        for (other, known) in &pairs {
            assert_eq!(other, known);
            assert_eq!(known, other);
            assert_eq!(other.cmp(known), Ordering::Equal);
            assert_eq!(other.resolved_id(), known.id());
            let hashed: HashSet<HeaderName<'_>> = [other.clone()].into_iter().collect();
            assert!(hashed.contains(known));
            let ordered: BTreeSet<HeaderName<'_>> = [known.clone()].into_iter().collect();
            assert!(ordered.contains(other));
        }
        let custom = HeaderName::Other("X-Custom".into());
        assert_ne!(custom, HeaderName::Subject);
        assert_ne!(HeaderName::Other("Sub ject".into()), HeaderName::Subject);
        assert_eq!(custom.cmp(&HeaderName::Subject), Ordering::Greater);
        assert_eq!(custom.resolved_id(), OTHER_ID);
        let mut names = [
            HeaderName::Other("x-b".into()),
            HeaderName::Other("From".into()),
            HeaderName::Subject,
            HeaderName::Other("X-A".into()),
        ];
        names.sort();
        assert_eq!(
            names.iter().map(HeaderName::as_str).collect::<Vec<_>>(),
            ["Subject", "From", "X-A", "x-b"]
        );
    }
}

/// A header name resolved once by [`HeaderName::key`], for repeated lookups
/// with [`crate::Headers::all_key`] and [`crate::Headers::get_key`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HeaderKey<'x> {
    id: u16,
    name: &'x str,
}

impl<'x> HeaderKey<'x> {
    /// The spelling of the name the key was made from.
    pub fn as_str(&self) -> &'x str {
        self.name
    }

    pub(crate) fn id(&self) -> u16 {
        self.id
    }
}
