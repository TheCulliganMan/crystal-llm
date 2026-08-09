//! Crystal intro LCD compositor.
//!
//! This module is deliberately separate from the Bevy shell. Its contract is
//! to consume the semantic LCD/register state driven by
//! `engine/movie/intro.asm` and produce one complete 160x144 LCD frame. It
//! owns no ECS entities, input, timing, audio, or window state.

use super::*;

/// Compose a complete native-LCD frame for the current intro state.
///
/// The cache key includes the same state TypeScript uses when rendering:
/// scene, counter/timer, scroll registers, global sprite offset, palette
/// effect and the complete OAM state.  Bevy receives only the resulting image
/// handle, so it cannot expose an intermediate clear surface.
pub(super) fn compose_frame(
    rendered_art: &mut RenderedTilesetArt,
    asset_root: &AssetRoot,
    sprite_anim_bundle: &str,
    intro: &VisibleIntroScreen,
    images: &mut Assets<Image>,
) -> Option<SpriteFrame> {
    let render_intro = exact_presentation_state(intro);
    let key = intro_scene_art_key(&render_intro);
    let frame = match load_intro_scene_frame(
        asset_root,
        sprite_anim_bundle,
        &render_intro,
        rendered_art,
        images,
    ) {
        Ok(frame) => {
            rendered_art.intro_scene_errors.remove(&key);
            frame
        }
        Err(error) => {
            rendered_art
                .intro_scene_errors
                .insert(key, error.to_string());
            return None;
        }
    };

    // Intro is one producer of the shell-wide retained LCD surface. The same
    // handle continues through title, new-game setup, and credits, eliminating
    // the formerly fragile despawn/spawn handoff between those scenes.
    present_fullscreen_frame(
        rendered_art,
        &frame,
        PresentedFullscreenFrameSource::Transient,
        images,
    )
    .ok()
}

/// Preserve every field in the semantic LCD state.  An earlier renderer
/// rounded counters, scroll registers, palettes, and OAM positions to four
/// frames; that silently dropped visible Crystal intro states.  Texture
/// allocation is already avoided by updating the persistent image above, so
/// there is no fidelity reason to alter the state before composition.
pub(super) fn exact_presentation_state(intro: &VisibleIntroScreen) -> VisibleIntroScreen {
    intro.clone()
}
