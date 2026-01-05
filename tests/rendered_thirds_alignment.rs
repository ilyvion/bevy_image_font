#![allow(missing_docs, reason = "tests don't need docs")]

mod setup;

#[macro_use]
mod visual_acceptance_testing;

test_case!(rendered, thirds_alignment:custom_alignment:3);
