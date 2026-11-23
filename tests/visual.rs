#![allow(missing_docs, reason = "tests don't need docs")]
#![recursion_limit = "256"]

mod setup;

#[macro_use]
mod visual_acceptance_testing;

test_case!(rendered, base_alignment);
test_case!(rendered, thirds_alignment:custom_alignment:3);
test_case!(rendered, quarters_alignment:custom_alignment:4);
test_case!(rendered, fifths_alignment:custom_alignment:5);
test_case!(rendered, manual_positioning);
test_case!(rendered, sizes);

#[cfg(feature = "atlas_sprites")]
mod sprites {
    test_case!(sprites, base_alignment);
    test_case!(sprites, thirds_alignment:custom_alignment:3);
    test_case!(sprites, quarters_alignment:custom_alignment:4);
    test_case!(sprites, fifths_alignment:custom_alignment:5);
    test_case!(sprites, manual_positioning);
    test_case!(sprites, sizes);
    test_case!(sprites, spacing);
}
