//! 1:1 port of SkyRendererMixin.

use crate::NicerSkies;
use crate::seams;

/// java: @Inject(at = @At("TAIL"), method = "buildStars") nicerSkies_buildStars.
pub fn nicer_skies_build_stars() {
    let mut instance = crate::get_instance().lock().unwrap();
    let Some(inst) = instance.as_mut() else { return };

    inst.renderer.build_buffers();

    // note: this method gets called at every resource re-load.
    // Ff we already know the seed by this time, it means we reloaded resources
    // while in-game so we can already re-generate the stars and paint the nebulas
    if crate::core::hashed_seed_manager::can_generate_sky() {
        inst.renderer.generate_sky(crate::core::hashed_seed_manager::get_seed());
    }
}

/// java: @Inject(method = "renderSunMoonAndStars", at = @At(value = "INVOKE",
/// target = "Lcom/mojang/blaze3d/vertex/PoseStack;popPose()V", ordinal = 3,
/// shift = At.Shift.BEFORE)) nicerSkies_renderNebulas.
pub fn nicer_skies_render_nebulas(pose_stack: &mut seams::PoseStack, star_angle: f32, star_brightness: f32, render_target: &seams::RenderTarget) {
    let instance = crate::get_instance().lock().unwrap();
    let Some(inst) = instance.as_ref() else { return };

    if inst.config.are_nebulas_enabled() {
        if let Some(skybox) = inst.renderer.get_skybox() {
            pose_stack.push_pose();
            pose_stack.mul_pose(seams::Axis::XP.rotation(star_angle as f64));
            // max star brightness is 0.5
            let brightness = star_brightness * 2.0;
            skybox.render(render_target, pose_stack, brightness);
            pose_stack.pop_pose();
        }
    }
}

/// java: @Inject(method = "renderStars", at = @At("HEAD"), cancellable = true)
/// nicerSkies_renderTwinklingStars.
pub fn nicer_skies_render_twinkling_stars(star_brightness: f32, pose_stack: &seams::PoseStack, render_target: &seams::RenderTarget) -> bool {
    let instance = crate::get_instance().lock().unwrap();
    let Some(inst) = instance.as_ref() else { return false };

    if inst.config.are_twinlking_stars_enabled() {
        if let Some(starbox) = inst.renderer.get_starbox() {
            starbox.render(render_target, pose_stack, star_brightness);
            return true; // java: ci.cancel()
        }
    }

    false
}

/// Registers the mixin handlers with the engine (SkyRendererMixin client entry).
pub fn register() {
    seams::register_sky_renderer_hooks(nicer_skies_build_stars, |ps, sa, sb, rt| {
        nicer_skies_render_nebulas(ps, sa, sb, rt)
    }, |sb, ps, rt| nicer_skies_render_twinkling_stars(sb, ps, rt));
}
