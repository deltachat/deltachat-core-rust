//! Supporting code for the QR-code invite.
//!
//! QR-codes are decoded into a more general-purpose [`Qr`] struct normally.  This makes working
//! with it rather hard, so here we have a wrapper type that specifically deals with Secure-Join
//! QR-codes so that the Secure-Join code can have more guarantees when dealing with this.

use std::convert::TryFrom;

use anyhow::{bail, Error, Result};

use crate::key::Fingerprint;
use crate::qr::Qr;

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

impl TryFrom<Qr> for QrInvite {
    type Error = Error;

    fn try_from(qr: Qr) -> Result<Self> {
        match qr {
            Qr::AskVerifyContact {
                contact_id,
                fingerprint,
                invitenumber,
                authcode,
            } => Ok(QrInvite::Contact {
                contact_id,
                fingerprint,
                invitenumber,
                authcode,
            }),
            Qr::AskVerifyGroup {
                grpname,
                grpid,
                contact_id,
                fingerprint,
                invitenumber,
                authcode,
            } => Ok(QrInvite::Group {
                contact_id,
                fingerprint,
                name: grpname,
                grpid,
                invitenumber,
                authcode,
            }),
            _ => bail!("Unsupported QR type {:?}", qr),
        }
    }
}
