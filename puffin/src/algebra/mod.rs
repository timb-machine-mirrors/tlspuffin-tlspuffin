//! The *term* module defines typed[`Term`]sof the form `fn_add(x: u8, fn_square(y: u16)) → u16`.
//! Each function like `fn_add` or `fn_square` has a shape. The variables `x` and `y` each have a
//! type. These types allow type checks during the runtime of the fuzzer.
//! These checks restrict how[`Term`]scan be mutated in the *fuzzer* module.

// Code in this directory is derived from https://github.com/joshrule/term-rewriting-rs/
// and is licensed under:
//
// The MIT License (MIT)
// Copyright (c) 2018--2021
// Maximilian Ammann <max@maxammann.org>, Joshua S. Rule <joshua.s.rule@gmail.com>
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use std::{ops::DerefMut, sync::Mutex};

use once_cell::sync::{Lazy, OnceCell};

pub use self::term::*;
use crate::algebra::signature::Signature;

pub mod atoms;
pub mod dynamic_function;
pub mod error;
pub mod macros;
pub mod signature;
pub mod term;

static CURRENT_SIGNATURE: OnceCell<&'static Signature> = OnceCell::new();

/// Returns the current signature which is used during deserialization.
pub fn current_signature() -> &'static Signature {
    CURRENT_SIGNATURE
        .get()
        .expect("current signature needs to be set")
}

pub fn set_current_signature(signature: &'static Signature) {
    CURRENT_SIGNATURE.set(signature);
}

#[cfg(test)]
#[allow(clippy::ptr_arg)]
pub mod test_signature {
    use std::{
        any::{Any, TypeId},
        fmt::{Debug, Display, Formatter},
        io::Read,
    };

    use serde::{Deserialize, Serialize};

    use crate::{
        agent::{AgentDescriptor, AgentName, TLSVersion},
        algebra::{dynamic_function::TypeShape, error::FnError, Term},
        claims::{ClaimTrait, Policy},
        define_signature,
        error::Error,
        io::MessageResult,
        put::{PutDescriptor, PutName, PutOptions},
        put_registry::{
            Factory, Message, MessageDeframer, OpaqueMessage, ProtocolBehavior, PutRegistry,
            DUMMY_PUT,
        },
        term,
        trace::{Action, InputAction, QueryMatcher, Step, Trace},
        variable_data::VariableData,
    };

    pub struct HmacKey;
    pub struct HandshakeMessage;
    pub struct Encrypted;
    pub struct ProtocolVersion;
    pub struct Random;
    pub struct ClientExtension;
    pub struct ClientExtensions;
    pub struct Group;
    pub struct SessionID;
    pub struct CipherSuites;
    pub struct CipherSuite;
    pub struct Compression;
    pub struct Compressions;

    pub fn fn_hmac256_new_key() -> Result<HmacKey, FnError> {
        Ok(HmacKey)
    }

    pub fn fn_hmac256(key: &HmacKey, msg: &Vec<u8>) -> Result<Vec<u8>, FnError> {
        Ok(Vec::new())
    }

    pub fn fn_client_hello(
        version: &ProtocolVersion,
        random: &Random,
        id: &SessionID,
        suites: &CipherSuites,
        compressions: &Compressions,
        extensions: &ClientExtensions,
    ) -> Result<HandshakeMessage, FnError> {
        Ok(HandshakeMessage)
    }

    pub fn fn_finished() -> Result<HandshakeMessage, FnError> {
        Ok(HandshakeMessage)
    }

    pub fn fn_protocol_version12() -> Result<ProtocolVersion, FnError> {
        Ok(ProtocolVersion)
    }
    pub fn fn_new_session_id() -> Result<SessionID, FnError> {
        Ok(SessionID)
    }

    pub fn fn_new_random() -> Result<Random, FnError> {
        Ok(Random)
    }

    pub fn fn_client_extensions_append(
        extensions: &ClientExtensions,
        extension: &ClientExtension,
    ) -> Result<ClientExtensions, FnError> {
        Ok(ClientExtensions)
    }

    pub fn fn_client_extensions_new() -> Result<ClientExtensions, FnError> {
        Ok(ClientExtensions)
    }
    pub fn fn_support_group_extension(group: &Group) -> Result<ClientExtension, FnError> {
        Ok(ClientExtension)
    }

    pub fn fn_signature_algorithm_extension() -> Result<ClientExtension, FnError> {
        Ok(ClientExtension)
    }
    pub fn fn_ec_point_formats_extension() -> Result<ClientExtension, FnError> {
        Ok(ClientExtension)
    }
    pub fn fn_signed_certificate_timestamp_extension() -> Result<ClientExtension, FnError> {
        Ok(ClientExtension)
    }
    pub fn fn_renegotiation_info_extension(info: &Vec<u8>) -> Result<ClientExtension, FnError> {
        Ok(ClientExtension)
    }
    pub fn fn_signature_algorithm_cert_extension() -> Result<ClientExtension, FnError> {
        Ok(ClientExtension)
    }

    pub fn fn_empty_bytes_vec() -> Result<Vec<u8>, FnError> {
        Ok(Vec::new())
    }

    pub fn fn_named_group_secp384r1() -> Result<Group, FnError> {
        Ok(Group)
    }

    pub fn fn_client_key_exchange() -> Result<HandshakeMessage, FnError> {
        Ok(HandshakeMessage)
    }

    pub fn fn_new_cipher_suites() -> Result<CipherSuites, FnError> {
        Ok(CipherSuites)
    }

    pub fn fn_append_cipher_suite(
        suites: &CipherSuites,
        suite: &CipherSuite,
    ) -> Result<CipherSuites, FnError> {
        Ok(CipherSuites)
    }
    pub fn fn_cipher_suite12() -> Result<CipherSuite, FnError> {
        Ok(CipherSuite)
    }

    pub fn fn_compressions() -> Result<Compressions, FnError> {
        Ok(Compressions)
    }

    pub fn fn_encrypt12(finished: &HandshakeMessage, seq: &u32) -> Result<Encrypted, FnError> {
        Ok(Encrypted)
    }

    pub fn fn_seq_0() -> Result<u32, FnError> {
        Ok(0)
    }

    pub fn fn_seq_1() -> Result<u32, FnError> {
        Ok(1)
    }

    pub fn example_op_c(a: &u8) -> Result<u16, FnError> {
        Ok((a + 1) as u16)
    }

    fn create_client_hello() -> TestTerm {
        term! {
              fn_client_hello(
                fn_protocol_version12,
                fn_new_random,
                fn_new_session_id,
                (fn_append_cipher_suite(
                    (fn_new_cipher_suites()),
                    fn_cipher_suite12
                )),
                fn_compressions,
                (fn_client_extensions_append(
                    (fn_client_extensions_append(
                        (fn_client_extensions_append(
                            (fn_client_extensions_append(
                                (fn_client_extensions_append(
                                    (fn_client_extensions_append(
                                        fn_client_extensions_new,
                                        (fn_support_group_extension(fn_named_group_secp384r1))
                                    )),
                                    fn_signature_algorithm_extension
                                )),
                                fn_ec_point_formats_extension
                            )),
                            fn_signed_certificate_timestamp_extension
                        )),
                         // Enable Renegotiation
                        (fn_renegotiation_info_extension(fn_empty_bytes_vec))
                    )),
                    // Add signature cert extension
                    fn_signature_algorithm_cert_extension
                ))
            )
        }
    }

    pub fn setup_simple_trace() -> TestTrace {
        let server = AgentName::first();
        let client_hello = create_client_hello();

        Trace {
            prior_traces: vec![],
            descriptors: vec![AgentDescriptor::new_server(
                server,
                TLSVersion::V1_2,
                PutDescriptor {
                    name: DUMMY_PUT,
                    options: PutOptions::default(),
                },
            )],
            steps: vec![
                Step {
                    agent: server,
                    action: Action::Input(InputAction {
                        recipe: client_hello,
                    }),
                },
                Step {
                    agent: server,
                    action: Action::Input(InputAction {
                        recipe: term! {
                            fn_client_key_exchange
                        },
                    }),
                },
                Step {
                    agent: server,
                    action: Action::Input(InputAction {
                        recipe: term! {
                            fn_encrypt12(fn_finished, fn_seq_0)
                        },
                    }),
                },
            ],
        }
    }

    define_signature!(
        TEST_SIGNATURE,
        fn_hmac256_new_key
        fn_hmac256
        fn_client_hello
        fn_finished
        fn_protocol_version12
        fn_new_session_id
        fn_new_random
        fn_client_extensions_append
        fn_client_extensions_new
        fn_support_group_extension
        fn_signature_algorithm_extension
        fn_ec_point_formats_extension
        fn_signed_certificate_timestamp_extension
        fn_renegotiation_info_extension
        fn_signature_algorithm_cert_extension
        fn_empty_bytes_vec
        fn_named_group_secp384r1
        fn_client_key_exchange
        fn_new_cipher_suites
        fn_append_cipher_suite
        fn_cipher_suite12
        fn_compressions
        fn_encrypt12
        fn_seq_0
        fn_seq_1
    );

    #[derive(Debug, Clone, Hash, Serialize, Deserialize, PartialEq)]
    pub struct TestQueryMatcher;

    impl Display for TestQueryMatcher {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            todo!()
        }
    }

    impl QueryMatcher for TestQueryMatcher {
        fn matches(&self, query: &Self) -> bool {
            todo!()
        }

        fn specificity(&self) -> u32 {
            todo!()
        }
    }

    pub type TestTrace = Trace<TestQueryMatcher>;
    pub type TestTerm = Term<TestQueryMatcher>;

    pub struct TestClaim;

    impl VariableData for TestClaim {
        fn boxed(&self) -> Box<dyn VariableData> {
            todo!()
        }

        fn boxed_any(&self) -> Box<dyn Any> {
            todo!()
        }

        fn type_id(&self) -> TypeId {
            todo!()
        }

        fn type_name(&self) -> &'static str {
            todo!()
        }
    }

    impl Debug for TestClaim {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            todo!()
        }
    }

    impl ClaimTrait for TestClaim {
        fn agent_name(&self) -> AgentName {
            todo!()
        }

        fn id(&self) -> TypeShape {
            todo!()
        }

        fn inner(&self) -> Box<dyn Any> {
            todo!()
        }
    }

    pub struct TestOpaqueMessage;

    impl Clone for TestOpaqueMessage {
        fn clone(&self) -> Self {
            todo!()
        }
    }

    impl Debug for TestOpaqueMessage {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            todo!()
        }
    }

    impl OpaqueMessage<TestMessage> for TestOpaqueMessage {
        fn encode(&self) -> Vec<u8> {
            todo!()
        }

        fn into_message(self) -> Result<TestMessage, Error> {
            todo!()
        }
    }

    pub struct TestMessage;

    impl Clone for TestMessage {
        fn clone(&self) -> Self {
            todo!()
        }
    }

    impl Debug for TestMessage {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            todo!()
        }
    }

    impl Message<TestOpaqueMessage> for TestMessage {
        fn create_opaque(&self) -> TestOpaqueMessage {
            todo!()
        }
    }

    pub struct TestMessageDeframer;

    impl MessageDeframer<TestMessage, TestOpaqueMessage> for TestMessageDeframer {
        fn new() -> Self {
            todo!()
        }

        fn pop_frame(&mut self) -> Option<TestOpaqueMessage> {
            todo!()
        }

        fn encode(&self) -> Vec<u8> {
            todo!()
        }

        fn read(&mut self, rd: &mut dyn Read) -> std::io::Result<usize> {
            todo!()
        }
    }

    pub struct TestProtocolBehavior;

    impl ProtocolBehavior for TestProtocolBehavior {
        type Claim = TestClaim;
        type Message = TestMessage;
        type OpaqueMessage = TestOpaqueMessage;
        type MessageDeframer = TestMessageDeframer;
        type QueryMatcher = TestQueryMatcher;

        fn policy() -> Policy<Self::Claim> {
            todo!()
        }

        fn extract_knowledge(message: &Self::Message) -> Result<Vec<Box<dyn VariableData>>, Error> {
            todo!()
        }

        fn signature() -> &'static Signature {
            todo!()
        }

        fn create_corpus() -> Vec<(Trace<Self::QueryMatcher>, &'static str)> {
            todo!()
        }

        fn new_registry() -> &'static dyn PutRegistry<Self> {
            todo!()
        }

        fn to_query_matcher(
            message_result: &MessageResult<Self::Message, Self::OpaqueMessage>,
        ) -> Self::QueryMatcher {
            todo!()
        }
    }

    pub struct TestPutRegistry;

    impl PutRegistry<TestProtocolBehavior> for TestPutRegistry {
        fn version_strings(&self) -> Vec<String> {
            todo!()
        }

        fn make_deterministic(&self) {
            todo!()
        }

        fn find_factory(
            &self,
            put_name: PutName,
        ) -> Option<Box<dyn Factory<TestProtocolBehavior>>> {
            todo!()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use itertools::Itertools;

    use super::test_signature::*;
    use crate::{
        agent::AgentName,
        algebra::{dynamic_function::TypeShape, error::FnError, signature::Signature, Term},
        term,
        trace::{Knowledge, Query, TraceContext},
    };

    #[allow(dead_code)]
    fn test_compilation() {
        // reminds me of Lisp, lol
        let client = AgentName::first();
        let _test_nested_with_variable: TestTerm = term! {
           fn_client_hello(
                (fn_client_hello(
                    fn_protocol_version12,
                    fn_new_random,
                    (fn_client_hello(fn_protocol_version12,
                        fn_new_random,
                        fn_new_random,
                        ((client,0)/ProtocolVersion)
                    ))
                )),
                fn_new_random
            )
        };

        let _set_simple_function2: TestTerm = term! {
           fn_client_hello((fn_protocol_version12()), fn_new_random, fn_new_random)
        };

        let _test_simple_function2: TestTerm = term! {
           fn_new_random(((client,0)))
        };
        let _test_simple_function1: TestTerm = term! {
           fn_protocol_version12
        };
        let _test_simple_function: TestTerm = term! {
           fn_new_random(((client,0)/ProtocolVersion))
        };
        let _test_variable: TestTerm = term! {
            (client,0)/ProtocolVersion
        };
        let _set_nested_function: TestTerm = term! {
           fn_client_extensions_append(
                (fn_client_extensions_append(
                    fn_client_extensions_new,
                    (fn_support_group_extension(fn_named_group_secp384r1))
                )),
                (fn_support_group_extension(fn_named_group_secp384r1))
            )
        };
    }

    #[test]
    fn example() {
        let hmac256_new_key = Signature::new_function(&fn_hmac256_new_key);
        let hmac256 = Signature::new_function(&fn_hmac256);
        let _client_hello = Signature::new_function(&fn_client_hello);

        let data = "hello".as_bytes().to_vec();

        //println!("TypeId of vec array {:?}", data.type_id());

        let variable = Signature::new_var(TypeShape::of::<Vec<u8>>(), AgentName::first(), None, 0);

        let generated_term = Term::Application(
            hmac256,
            vec![
                Term::Application(hmac256_new_key, vec![]),
                Term::Variable(variable),
            ],
        );

        //println!("{}", generated_term);
        let mut context = TraceContext::new(&TestPutRegistry);
        context.add_knowledge(Knowledge {
            agent_name: AgentName::first(),
            matcher: None,
            data: Box::new(data),
        });

        let _string = generated_term
            .evaluate(&context)
            .as_ref()
            .unwrap()
            .downcast_ref::<Vec<u8>>();
        //println!("{:?}", string);
    }

    #[test]
    fn playground() {
        let _var_data = fn_new_session_id();

        //println!("vec {:?}", TypeId::of::<Vec<u8>>());
        //println!("vec {:?}", TypeId::of::<Vec<u16>>());

        ////println!("{:?}", var_data.type_id());

        let func = Signature::new_function(&example_op_c);
        let dynamic_fn = func.dynamic_fn();
        let _string = dynamic_fn(&vec![Box::new(1u8)])
            .unwrap()
            .downcast_ref::<u16>()
            .unwrap();
        //println!("{:?}", string);
        let _string = Signature::new_function(&example_op_c).shape();
        //println!("{}", string);

        let constructed_term = Term::Application(
            Signature::new_function(&example_op_c),
            vec![
                Term::Application(
                    Signature::new_function(&example_op_c),
                    vec![
                        Term::Application(
                            Signature::new_function(&example_op_c),
                            vec![
                                Term::Application(Signature::new_function(&example_op_c), vec![]),
                                Term::Variable(Signature::new_var_with_type::<
                                    SessionID,
                                    TestQueryMatcher,
                                >(
                                    AgentName::first(), None, 0
                                )),
                            ],
                        ),
                        Term::Variable(
                            Signature::new_var_with_type::<SessionID, TestQueryMatcher>(
                                AgentName::first(),
                                None,
                                0,
                            ),
                        ),
                    ],
                ),
                Term::Application(
                    Signature::new_function(&example_op_c),
                    vec![
                        Term::Application(
                            Signature::new_function(&example_op_c),
                            vec![
                                Term::Variable(Signature::new_var_with_type::<SessionID, _>(
                                    AgentName::first(),
                                    None,
                                    0,
                                )),
                                Term::Application(Signature::new_function(&example_op_c), vec![]),
                            ],
                        ),
                        Term::Variable(Signature::new_var_with_type::<SessionID, _>(
                            AgentName::first(),
                            None,
                            0,
                        )),
                    ],
                ),
            ],
        );

        //println!("{}", constructed_term);
        let _graph = constructed_term.dot_subgraph(true, 0, "test");
        //println!("{}", graph);
    }
}
