use super::*;
use crate::Arc;
use std::borrow::Cow;

/// Description of a DICOM association carried by [`Context`](crate::context::Context).
#[derive(Debug, Clone, Default)]
pub struct AssocDescription {
    /// Unique identifier assigned when:
    /// - Received incoming connection
    /// - Outgoing association request constructed
    /// - Virtual "pseudo" association created
    pub id: u64,
    /// Flag, indicating that "Transport Layer Security" used in communication.
    pub is_tls_used: bool,
    /// Flag, indicating, that association is incoming.
    pub is_incoming: bool,
    /// Flag, indicating, that association is virtual.
    pub is_virtual: bool,
    /// Peer AE Title.
    /// - For outgoing: known immediately
    /// - For incoming: filled after A-ASSOCIATE-RQ received
    pub peer_aet: Option<Cow<'static, str>>,
    /// Local AE Title.
    /// - For outgoing: known immediately
    /// - For incoming: filled after A-ASSOCIATE-RQ received
    pub local_aet: Option<Cow<'static, str>>,
    /// Peer socket address. Always `None` for virtual associations.
    pub peer_addr: Option<Arc<PeerAddress>>,
    /// Local socket address. Always `None` for virtual associations.
    pub local_addr: Option<Arc<PeerAddress>>,
}
