//! 1:1 port of ClientLevelAccessor: duck interface the ClientLevelMixin
//! implements to expose the biome zoom (hashed) seed.

pub trait ClientLevelAccessor {
    fn nicer_skies_get_hashed_seed(&self) -> i64;
}
