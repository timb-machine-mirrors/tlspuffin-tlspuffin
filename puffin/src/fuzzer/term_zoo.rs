//! Generates a zoo of terms form a [`Signature`]. For each function symbol in the signature
//! a closed term is generated and added to the zoo.

use libafl_bolts::rands::Rand;

use crate::algebra::atoms::Function;
use crate::algebra::signature::{FunctionDefinition, Signature};
use crate::algebra::Term;
use crate::fuzzer::mutations::util::Choosable;
use crate::protocol::ProtocolTypes;

const MAX_DEPTH: u16 = 8; // how deep terms we allow max
const MAX_TRIES: u16 = 100; // How often we want to try to generate before stopping

pub struct TermZoo<PT: ProtocolTypes> {
    terms: Vec<Term<PT>>,
}

impl<PT: ProtocolTypes> TermZoo<PT> {
    pub fn generate<R: Rand>(signature: &Signature<PT>, rand: &mut R) -> Self {
        let terms = signature
            .functions
            .iter()
            .filter_map(|def| {
                let mut counter = MAX_TRIES;

                loop {
                    if counter == 0 {
                        break None;
                    }

                    counter -= 1;

                    if let Some(term) = Self::generate_term(signature, def, MAX_DEPTH, rand) {
                        break Some(term);
                    }
                }
            })
            .collect::<Vec<_>>();

        Self { terms }
    }

    fn generate_term<R: Rand>(
        signature: &Signature<PT>,
        (shape, dynamic_fn): &FunctionDefinition<PT>,
        depth: u16,
        rand: &mut R,
    ) -> Option<Term<PT>> {
        if depth == 0 {
            // Reached max depth
            return None;
        }

        let required_types = &shape.argument_types;

        let mut subterms = Vec::with_capacity(required_types.len());

        for typ in required_types {
            if let Some(possibilities) = signature.functions_by_typ.get(typ) {
                if let Some(possibility) = possibilities.choose(rand) {
                    if let Some(subterm) =
                        Self::generate_term(signature, possibility, depth - 1, rand)
                    {
                        subterms.push(subterm)
                    } else {
                        // Max depth reached
                        return None;
                    }
                } else {
                    // No possibilities available
                    return None;
                }
            } else {
                // Type not available
                return None;
            }
        }

        Some(Term::Application(
            Function::new(shape.clone(), dynamic_fn.clone()),
            subterms,
        ))
    }

    pub fn choose_filtered<P, R: Rand>(&self, filter: P, rand: &mut R) -> Option<&Term<PT>>
    where
        P: FnMut(&&Term<PT>) -> bool,
    {
        self.terms.choose_filtered(filter, rand)
    }

    pub fn terms(&self) -> &[Term<PT>] {
        &self.terms
    }
}
