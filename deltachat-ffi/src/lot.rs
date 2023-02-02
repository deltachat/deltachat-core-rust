//! # Legacy generic return values for C API.

use std::borrow::Cow;

use anyhow::Error;

use crate::message::MessageState;
use crate::qr::Qr;
use crate::summary::{Summary, SummaryPrefix};

/// An object containing a set of values.
/// The meaning of the values is defined by the function returning the object.
/// Lot objects are created
/// eg. by chatlist.get_summary() or dc_msg_get_summary().
///
/// *Lot* is used in the meaning *heap* here.
// The QR code grew too large.  So be it.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum Lot {
    Summary(Summary),
    Qr(Qr),
    Error(String),
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Meaning {
    None = 0,
    Text1Draft = 1,
    Text1Username = 2,
    Text1Self = 3,
}

impl Default for Meaning {
    fn default() -> Self {
        Meaning::None
    }
}

impl Lot {
    pub fn get_text1(&self) -> Option<&str> {
        match self {
            Self::Summary(summary) => match &summary.prefix {
                None => None,
                Some(SummaryPrefix::Draft(text)) => Some(text),
                Some(SummaryPrefix::Username(username)) => Some(username),
                Some(SummaryPrefix::Me(text)) => Some(text),
            },
            Self::Qr(qr) => match qr {
                Qr::AskVerifyContact { .. } => None,
                Qr::AskVerifyGroup { grpname, .. } => Some(grpname),
                Qr::FprOk { .. } => None,
                Qr::FprMismatch { .. } => None,
                Qr::FprWithoutAddr { fingerprint, .. } => Some(fingerprint),
                Qr::Account { domain } => Some(domain),
                Qr::Backup { .. } => None,
                Qr::WebrtcInstance { domain, .. } => Some(domain),
                Qr::Addr { draft, .. } => draft.as_deref(),
                Qr::Url { url } => Some(url),
                Qr::Text { text } => Some(text),
                Qr::WithdrawVerifyContact { .. } => None,
                Qr::WithdrawVerifyGroup { grpname, .. } => Some(grpname),
                Qr::ReviveVerifyContact { .. } => None,
                Qr::ReviveVerifyGroup { grpname, .. } => Some(grpname),
                Qr::Login { address, .. } => Some(address),
            },
            Self::Error(err) => Some(err),
        }
    }

    pub fn get_text2(&self) -> Option<Cow<str>> {
        match self {
            Self::Summary(summary) => Some(summary.truncated_text(160)),
            Self::Qr(_) => None,
            Self::Error(_) => None,
        }
    }

    pub fn get_text1_meaning(&self) -> Meaning {
        match self {
            Self::Summary(summary) => match &summary.prefix {
                None => Meaning::None,
                Some(SummaryPrefix::Draft(_text)) => Meaning::Text1Draft,
                Some(SummaryPrefix::Username(_username)) => Meaning::Text1Username,
                Some(SummaryPrefix::Me(_text)) => Meaning::Text1Self,
            },
            Self::Qr(qr) => match qr {
                Qr::Addr {
                    draft: Some(_draft),
                    ..
                } => Meaning::Text1Draft,
                _ => Meaning::None,
            },
            Self::Error(_err) => Meaning::None,
        }
    }

    pub fn get_state(&self) -> LotState {
        match self {
            Self::Summary(summary) => summary.state.into(),
            Self::Qr(qr) => match qr {
                Qr::AskVerifyContact { .. } => LotState::QrAskVerifyContact,
                Qr::AskVerifyGroup { .. } => LotState::QrAskVerifyGroup,
                Qr::FprOk { .. } => LotState::QrFprOk,
                Qr::FprMismatch { .. } => LotState::QrFprMismatch,
                Qr::FprWithoutAddr { .. } => LotState::QrFprWithoutAddr,
                Qr::Account { .. } => LotState::QrAccount,
                Qr::Backup { .. } => LotState::QrBackup,
                Qr::WebrtcInstance { .. } => LotState::QrWebrtcInstance,
                Qr::Addr { .. } => LotState::QrAddr,
                Qr::Url { .. } => LotState::QrUrl,
                Qr::Text { .. } => LotState::QrText,
                Qr::WithdrawVerifyContact { .. } => LotState::QrWithdrawVerifyContact,
                Qr::WithdrawVerifyGroup { .. } => LotState::QrWithdrawVerifyGroup,
                Qr::ReviveVerifyContact { .. } => LotState::QrReviveVerifyContact,
                Qr::ReviveVerifyGroup { .. } => LotState::QrReviveVerifyGroup,
                Qr::Login { .. } => LotState::QrLogin,
            },
            Self::Error(_err) => LotState::QrError,
        }
    }

    pub fn get_id(&self) -> u32 {
        match self {
            Self::Summary(_) => Default::default(),
            Self::Qr(qr) => match qr {
                Qr::AskVerifyContact { contact_id, .. } => contact_id.to_u32(),
                Qr::AskVerifyGroup { .. } => Default::default(),
                Qr::FprOk { contact_id } => contact_id.to_u32(),
                Qr::FprMismatch { contact_id } => contact_id.unwrap_or_default().to_u32(),
                Qr::FprWithoutAddr { .. } => Default::default(),
                Qr::Account { .. } => Default::default(),
                Qr::Backup { .. } => Default::default(),
                Qr::WebrtcInstance { .. } => Default::default(),
                Qr::Addr { contact_id, .. } => contact_id.to_u32(),
                Qr::Url { .. } => Default::default(),
                Qr::Text { .. } => Default::default(),
                Qr::WithdrawVerifyContact { contact_id, .. } => contact_id.to_u32(),
                Qr::WithdrawVerifyGroup { .. } => Default::default(),
                Qr::ReviveVerifyContact { contact_id, .. } => contact_id.to_u32(),
                Qr::ReviveVerifyGroup { .. } => Default::default(),
                Qr::Login { .. } => Default::default(),
            },
            Self::Error(_) => Default::default(),
        }
    }

    pub fn get_timestamp(&self) -> i64 {
        match self {
            Self::Summary(summary) => summary.timestamp,
            Self::Qr(_) => Default::default(),
            Self::Error(_) => Default::default(),
        }
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LotState {
    // Default
    Undefined = 0,

    // Qr States
    /// id=contact
    QrAskVerifyContact = 200,

    /// text1=groupname
    QrAskVerifyGroup = 202,

    /// id=contact
    QrFprOk = 210,

    /// id=contact
    QrFprMismatch = 220,

    /// text1=formatted fingerprint
    QrFprWithoutAddr = 230,

    /// text1=domain
    QrAccount = 250,

    QrBackup = 251,

    /// text1=domain, text2=instance pattern
    QrWebrtcInstance = 260,

    /// id=contact
    QrAddr = 320,

    /// text1=text
    QrText = 330,

    /// text1=URL
    QrUrl = 332,

    /// text1=error string
    QrError = 400,

    QrWithdrawVerifyContact = 500,

    /// text1=groupname
    QrWithdrawVerifyGroup = 502,

    QrReviveVerifyContact = 510,

    /// text1=groupname
    QrReviveVerifyGroup = 512,

    /// text1=email_address
    QrLogin = 520,

    // Message States
    MsgInFresh = 10,
    MsgInNoticed = 13,
    MsgInSeen = 16,
    MsgOutPreparing = 18,
    MsgOutDraft = 19,
    MsgOutPending = 20,
    MsgOutFailed = 24,
    MsgOutDelivered = 26,
    MsgOutMdnRcvd = 28,
}

impl Default for LotState {
    fn default() -> Self {
        LotState::Undefined
    }
}

impl From<MessageState> for LotState {
    fn from(s: MessageState) -> Self {
        use MessageState::*;
        match s {
            Undefined => LotState::Undefined,
            InFresh => LotState::MsgInFresh,
            InNoticed => LotState::MsgInNoticed,
            InSeen => LotState::MsgInSeen,
            OutPreparing => LotState::MsgOutPreparing,
            OutDraft => LotState::MsgOutDraft,
            OutPending => LotState::MsgOutPending,
            OutFailed => LotState::MsgOutFailed,
            OutDelivered => LotState::MsgOutDelivered,
            OutMdnRcvd => LotState::MsgOutMdnRcvd,
        }
    }
}

impl From<Summary> for Lot {
    fn from(summary: Summary) -> Self {
        Lot::Summary(summary)
    }
}

impl From<Qr> for Lot {
    fn from(qr: Qr) -> Self {
        Lot::Qr(qr)
    }
}

// Make it easy to convert errors into the final `Lot`.
impl From<Error> for Lot {
    fn from(error: Error) -> Self {
        Lot::Error(error.to_string())
    }
}
