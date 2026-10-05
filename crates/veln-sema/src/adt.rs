pub(crate) mod descriptors;
pub(crate) mod registry;

mod builtin_codec_descriptors;
mod builtin_core_descriptors;
pub(crate) mod builtin_descriptors;
mod lookup_validation;
#[cfg(test)]
pub(crate) use lookup_validation::{reset_type_parameter_lookups, take_type_parameter_lookups};
mod runtime_base_variants;
mod runtime_connection_variants;
mod runtime_hpack_variants;
mod runtime_peer_limit_variants;
mod runtime_protocol_variants;
mod runtime_root_descriptors;
mod runtime_support_descriptors;
mod runtime_variant_builders;
pub(crate) mod type_operations;
pub(crate) mod unification;

#[cfg(test)]
#[path = "adt/tests.rs"]
mod tests;
