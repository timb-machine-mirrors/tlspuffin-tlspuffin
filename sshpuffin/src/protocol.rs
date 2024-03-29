use puffin::{
    algebra::{signature::Signature, AnyMatcher},
    protocol::ProtocolBehavior,
    put_registry::PutRegistry,
    trace::Trace,
};

use crate::{
    claim::SshClaim,
    ssh::{
        message::{RawSshMessage, SshMessage},
        SSH_SIGNATURE,
    },
    violation::SshSecurityViolationPolicy,
    SSH_PUT_REGISTRY,
};

#[derive(Clone, Debug, PartialEq)]
pub struct SshProtocolBehavior {}

impl ProtocolBehavior for SshProtocolBehavior {
    type Claim = SshClaim;
    type SecurityViolationPolicy = SshSecurityViolationPolicy;
    type ProtocolMessage = SshMessage;
    type OpaqueProtocolMessage = RawSshMessage;
    type Matcher = AnyMatcher;

    fn signature() -> &'static Signature {
        &SSH_SIGNATURE
    }

    fn registry() -> &'static PutRegistry<Self>
    where
        Self: Sized,
    {
        &SSH_PUT_REGISTRY
    }

    fn create_corpus() -> Vec<(Trace<Self::Matcher>, &'static str)> {
        vec![] // TODO
    }
}
