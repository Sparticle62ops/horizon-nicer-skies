//! 1:1 port of ClientLevelMixin. The java @Mixin handlers below register as
//! engine hooks with identical injection points and bodies.

use crate::client_level_accessor::ClientLevelAccessor;
use crate::core::hashed_seed_manager as manager; // unused here, mirror imports
use crate::NicerSkies;
use crate::seams;

/// java: @Unique private long hashedSeed.
pub struct ClientLevelMixinState {
    hashed_seed: i64,
}

impl ClientLevelMixinState {
    pub fn new() -> Self {
        ClientLevelMixinState { hashed_seed: 0 }
    }
}

impl ClientLevelAccessor for ClientLevelMixinState {
    /// java: nicerSkies_getHashedSeed().
    fn nicer_skies_get_hashed_seed(&self) -> i64 {
        self.hashed_seed
    }
}

/// java: @Inject(method = "<init>", at = @At("TAIL")) nicerSkies_saveHashedSeed.
pub fn nicer_skies_save_hashed_seed(state: &mut ClientLevelMixinState, biome_zoom_seed: i64) {
    // store the seed to be able to regenerate things in settings
    state.hashed_seed = biome_zoom_seed;

    // Note: sometimes, the seed is received after the buffers have been built by the renderer.
    // In that case it is safe to finish generating the sky.
    let instance = crate::get_instance().lock().unwrap();
    if let Some(inst) = instance.as_ref() {
        let renderer_has_built = inst.renderer.has_built_buffers();
        if renderer_has_built {
            let mut inst = instance;
            inst.as_mut().unwrap().renderer.generate_sky(biome_zoom_seed);
        }
    }
}

/// java: @Inject(method = "tick", at = @At("TAIL")) nicerSkies_tickStars.
pub fn nicer_skies_tick_stars() {
    let mut instance = crate::get_instance().lock().unwrap();
    let Some(inst) = instance.as_mut() else { return };

    if !inst.config.are_twinlking_stars_enabled() {
        return;
    }

    let mut profiler_filler = seams::ProfilerFiller::get();
    profiler_filler.push("Nicer Skies");
    inst.renderer.tick();
    profiler_filler.pop();
}

/// Registers the mixin handlers with the engine (the mixin config's client
/// entries: ClientLevelMixin, SkyRendererMixin).
pub fn register() {
    seams::register_client_level_hooks(
        |seed| {
            let mut state = seams::client_level_mixin_state();
            nicer_skies_save_hashed_seed(&mut state, seed);
        },
        nicer_skies_tick_stars,
    );
}
