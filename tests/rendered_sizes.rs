#![allow(missing_docs, reason = "tests don't need docs")]
#![recursion_limit = "256"]

mod setup;

#[macro_use]
mod visual_acceptance_testing;

test_case!(rendered, sizes);
