//! # Legacy generic return values for C API.

use deltachat_derive::{FromSql, ToSql};

use crate::key::Fingerprint;
use crate::message::MessageState;
use crate::summary::{Summary, SummaryPrefix};

/// An object containing a set of values.
/// The meaning of the values is defined by the function returning the object.
/// Lot objects are created
/// eg. by chatlist.get_summary() or dc_msg_get_summary().
///
/// *Lot* is used in the meaning *heap* here.
#[derive(Default, Debug, Clone)]
pub struct Lot {
    pub(crate) text1_meaning: Meaning,
    pub(crate) text1: Option<String>,
    pub(crate) text2: Option<String>,
    pub(crate) timestamp: i64,
    pub(crate) state: LotState,
    pub(crate) id: u32,
    pub(crate) fingerprint: Option<Fingerprint>,
    pub(crate) invitenumber: Option<String>,
    pub(crate) auth: Option<String>,
}

#[repr(u8)]
#[derive(
    Debug, Display, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive, ToSql, FromSql,
)]
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
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get_text1(&self) -> Option<&str> {
        self.text1.as_deref()
    }

    pub fn get_text2(&self) -> Option<&str> {
        self.text2.as_deref()
    }

    pub fn get_text1_meaning(&self) -> Meaning {
        self.text1_meaning
    }

    pub fn get_state(&self) -> LotState {
        self.state
    }

    pub fn get_id(&self) -> u32 {
        self.id
    }

    pub fn get_timestamp(&self) -> i64 {
        self.timestamp
    }
}

#[repr(u32)]
#[derive(
    Debug, Display, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive, ToSql, FromSql,
)]
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

    /// test1=formatted fingerprint
    QrFprWithoutAddr = 230,

    /// text1=domain
    QrAccount = 250,

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
        let text2 = Some(summary.truncated_text(160).to_string());
        let (text1, text1_meaning) = match summary.prefix {
            None => (None, Meaning::None),
            Some(SummaryPrefix::Draft(text)) => (Some(text), Meaning::Text1Draft),
            Some(SummaryPrefix::Username(username)) => (Some(username), Meaning::Text1Username),
            Some(SummaryPrefix::Me(text)) => (Some(text), Meaning::Text1Self),
        };
        Self {
            text1_meaning,
            text1,
            text2,
            timestamp: summary.timestamp,
            state: summary.state.into(),
            ..Default::default()
        }
    }
}
