use crate::descriptor::ReflectedLayoutSpec;
use crate::descriptor::ONION_SKIN_COMPOSITE;

pub fn onion_skin_composite_layout_spec() -> ReflectedLayoutSpec {
    ReflectedLayoutSpec::local(&ONION_SKIN_COMPOSITE)
}
