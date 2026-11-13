//! 1:1 port of HashedSeedManager.

use crate::client_level_accessor::ClientLevelAccessor;
use crate::NicerSkies;

pub fn get_seed() -> i64 {
    if crate::seams::minecraft_level().is_some() {
        let level = crate::seams::minecraft_level().unwrap();
        // java: ((ClientLevelAccessor) level).nicerSkies_getHashedSeed()
        return crate::seams::client_level_hashed_seed(&level);
    }

    crate::logger().warn("Generating nebulas with fallback seed!");
    321 // handpicked decent default ;)
}

pub fn can_generate_sky() -> bool {
    crate::seams::minecraft_level().is_some()
}
