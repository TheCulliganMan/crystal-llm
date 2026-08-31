use anyhow::{Result, ensure};

use crate::{
    CompiledGamePack, NUZLOCKE_MANIFEST_ID, NuzlockeRules,
    derive_compiled_game_pack_identity_from_manifest, verify_compiled_game_pack_for_runtime,
};

/// Add the standard Nuzlocke challenge rules to an already verified game pack.
pub fn build_nuzlocke_modpack(base: &CompiledGamePack) -> Result<CompiledGamePack> {
    verify_compiled_game_pack_for_runtime(base)?;
    ensure!(
        !base
            .report
            .manifests
            .iter()
            .any(|manifest| manifest == NUZLOCKE_MANIFEST_ID),
        "compiled pack already includes the Nuzlocke modpack"
    );
    ensure!(
        !base.data.nuzlocke_rules.enabled(),
        "compiled base pack already enables challenge rules"
    );

    let mut pack = base.clone();
    pack.data.nuzlocke_rules = NuzlockeRules::standard();
    pack.report.manifests.push(NUZLOCKE_MANIFEST_ID.to_string());
    pack.identity = derive_compiled_game_pack_identity_from_manifest(
        pack.format_version,
        &pack.data,
        &pack.audio_manifest,
        &pack.runtime_files,
        &pack.report,
    )?;
    verify_compiled_game_pack_for_runtime(&pack)?;
    Ok(pack)
}
