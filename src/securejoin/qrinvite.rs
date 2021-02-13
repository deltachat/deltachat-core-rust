//! Supporting code for the QR-code invite.
//!
//! QR-codes are decoded into a more general-purpose [`Lot`] struct normally, this struct is
//! so general it is not even specific to QR-codes.  This makes working with it rather hard,
//! so here we have a wrapper type that specifically deals with Secure-Join QR-codes so
//! that the Secure-Join code can have many more guarantees when dealing with this.

use std::convert::TryFrom;

use anyhow::Result;

use crate::key::{Fingerprint, FingerprintError};
use crate::lot::{Lot, LotState};

/// Represents the data from a QR-code scan.
///
/// There are methods to conveniently access fields present in both variants.
#[derive(Debug, Clone)]
pub enum QrInvite {
    Contact {
        contact_id: u32,
        fingerprint: Fingerprint,
        invitenumber: String,
        authcode: String,
    },
    Group {
        contact_id: u32,
        fingerprint: Fingerprint,
        name: String,
        grpid: String,
        invitenumber: String,
        authcode: String,
    },
}

impl QrInvite {
    /// The contact ID of the inviter.
    ///
    /// The actual QR-code contains a URL-encoded email address, but upon scanning this is
    /// translated to a contact ID.
    pub fn contact_id(&self) -> u32 {
        match self {
            Self::Contact { contact_id, .. } | Self::Group { contact_id, .. } => *contact_id,
        }
    }

    /// The fingerprint of the inviter.
    pub fn fingerprint(&self) -> &Fingerprint {
        match self {
            Self::Contact { fingerprint, .. } | Self::Group { fingerprint, .. } => fingerprint,
        }
    }

    /// The `INVITENUMBER` of the setup-contact/secure-join protocol.
    pub fn invitenumber(&self) -> &str {
        match self {
            Self::Contact { invitenumber, .. } | Self::Group { invitenumber, .. } => invitenumber,
        }
    }

    /// The `AUTH` code of the setup-contact/secure-join protocol.
    pub fn authcode(&self) -> &str {
        match self {
            Self::Contact { authcode, .. } | Self::Group { authcode, .. } => authcode,
        }
    }
}

impl TryFrom<Lot> for QrInvite {
    type Error = QrError;

    fn try_from(lot: Lot) -> Result<Self, Self::Error> {
        if lot.state != LotState::QrAskVerifyContact && lot.state != LotState::QrAskVerifyGroup {
            return Err(QrError::UnsupportedProtocol);
        }
        if lot.id == 0 {
            return Err(QrError::MissingContactId);
        }
        let fingerprint = lot.fingerprint.ok_or(QrError::MissingFingerprint)?;
        let invitenumber = lot.invitenumber.ok_or(QrError::MissingInviteNumber)?;
        let authcode = lot.auth.ok_or(QrError::MissingAuthCode)?;
        match lot.state {
            LotState::QrAskVerifyContact => Ok(QrInvite::Contact {
                contact_id: lot.id,
                fingerprint,
                invitenumber,
                authcode,
            }),
            LotState::QrAskVerifyGroup => Ok(QrInvite::Group {
                contact_id: lot.id,
                fingerprint,
                name: lot.text1.ok_or(QrError::MissingGroupName)?,
                grpid: lot.text2.ok_or(QrError::MissingGroupId)?,
                invitenumber,
                authcode,
            }),
            _ => Err(QrError::UnsupportedProtocol),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum QrError {
    #[error("Unsupported protocol in QR-code")]
    UnsupportedProtocol,
    #[error("Failed to read fingerprint")]
    InvalidFingerprint(#[from] FingerprintError),
    #[error("Missing fingerprint")]
    MissingFingerprint,
    #[error("Missing invitenumber")]
    MissingInviteNumber,
    #[error("Missing auth code")]
    MissingAuthCode,
    #[error("Missing group name")]
    MissingGroupName,
    #[error("Missing group id")]
    MissingGroupId,
    #[error("Missing contact id")]
    MissingContactId,
}
