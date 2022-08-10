use puffin::{
    algebra::{signature::Signature, Matcher},
    error::Error,
    protocol::{MessageDeframer, OpaqueProtocolMessage, ProtocolBehavior, ProtocolMessage},
    put::{PutDescriptor, PutName},
    put_registry::{Factory, PutRegistry},
    stream::MessageResult,
    trace::Trace,
    variable_data::VariableData,
};

use crate::protocol::SshProtocolBehavior;

pub const LIBSSH_PUT: PutName = PutName(['L', 'I', 'B', 'S', 'S', 'H', '_', '_', '_', '_']);

pub const SSH_PUT_REGISTRY: PutRegistry<SshProtocolBehavior> = PutRegistry {
    factories: &[crate::libssh::new_libssh_factory],
    default: DEFAULT_PUT_FACTORY,
};

pub const DEFAULT_PUT_FACTORY: fn() -> Box<dyn Factory<SshProtocolBehavior>> =
    { crate::libssh::new_libssh_factory };
