use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use crystal_core::models::{FrontpicAnimProgram, PokemonSpecies, RuntimePokedexEntry};
use crystal_core::systems::evolution::EvolutionTable;
use crystal_core::systems::learnsets::SpeciesLearnsets;
use serde::Deserialize;

use crate::{
    CompiledGamePack, ModpackAudioAsset, ModpackAudioKind, ModpackAudioManifest,
    ModpackAudioSource, ModpackMetadata, ModpackPcmAudioFormat, PACK_AUDIO_COMPRESSION_GZIP,
    PACK_AUDIO_COMPRESSION_MIDI, PokemonCryMetadata,
    derive_compiled_game_pack_identity_from_manifest, fnv1a32_bytes, insert_keyed_audio_asset,
    insert_keyed_pokedex_entry, insert_keyed_pokemon_species, merge_evolution_table,
    merge_frontpic_anim_entries, merge_learnsets, merge_menu_icons, merge_pokemon_cry_entries,
    verify_compiled_game_pack_for_runtime,
};

pub const GEN3_MANIFEST_ID: &str = "gen3";
pub const GEN3_SPECIES_COUNT: usize = 135;
/// Exact primary-ability vocabulary assigned by the pinned Emerald data.
/// This validates the optional pack as data; Crystal's runtime does not
/// execute ability mechanics.
const GEN3_ASSIGNED_ABILITIES: &[&str] = &[
    "AIR_LOCK",
    "BATTLE_ARMOR",
    "BLAZE",
    "CHLOROPHYLL",
    "CLEAR_BODY",
    "COLOR_CHANGE",
    "COMPOUND_EYES",
    "CUTE_CHARM",
    "DAMP",
    "DRIZZLE",
    "DROUGHT",
    "EARLY_BIRD",
    "EFFECT_SPORE",
    "FLAME_BODY",
    "FLASH_FIRE",
    "FORECAST",
    "GUTS",
    "HUSTLE",
    "HYPER_CUTTER",
    "ILLUMINATE",
    "IMMUNITY",
    "INNER_FOCUS",
    "INSOMNIA",
    "INTIMIDATE",
    "KEEN_EYE",
    "LEVITATE",
    "LIGHTNING_ROD",
    "LIMBER",
    "LIQUID_OOZE",
    "MAGMA_ARMOR",
    "MAGNET_PULL",
    "MARVEL_SCALE",
    "MINUS",
    "NATURAL_CURE",
    "OBLIVIOUS",
    "OVERGROW",
    "OWN_TEMPO",
    "PICKUP",
    "PLUS",
    "POISON_POINT",
    "PRESSURE",
    "PURE_POWER",
    "ROCK_HEAD",
    "ROUGH_SKIN",
    "RUN_AWAY",
    "SAND_STREAM",
    "SAND_VEIL",
    "SERENE_GRACE",
    "SHADOW_TAG",
    "SHED_SKIN",
    "SHELL_ARMOR",
    "SHIELD_DUST",
    "SOUNDPROOF",
    "SPEED_BOOST",
    "STATIC",
    "STENCH",
    "STURDY",
    "SUCTION_CUPS",
    "SWARM",
    "SWIFT_SWIM",
    "SYNCHRONIZE",
    "THICK_FAT",
    "TORRENT",
    "TRACE",
    "TRUANT",
    "VITAL_SPIRIT",
    "VOLT_ABSORB",
    "WATER_ABSORB",
    "WATER_VEIL",
    "WHITE_SMOKE",
    "WONDER_GUARD",
];
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Gen3Source {
    schema_version: u16,
    metadata: ModpackMetadata,
    source: Gen3SourceIdentity,
    abilities: BTreeMap<String, String>,
    pokemon: BTreeMap<String, PokemonSpecies>,
    learnsets: SpeciesLearnsets,
    evolutions: EvolutionTable,
    menu_icons: BTreeMap<String, String>,
    pokedex_entries: BTreeMap<String, RuntimePokedexEntry>,
    pokemon_frontpic_anim: BTreeMap<String, FrontpicAnimProgram>,
    pokemon_cries: BTreeMap<String, PokemonCryMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Gen3SourceIdentity {
    repository: String,
    commit: String,
}

/// Add the checked-in Generation 3 roster and its complete presentation
/// closure to an already verified core pack.
pub fn build_gen3_modpack(
    base: &CompiledGamePack,
    modpack_root: &Path,
) -> Result<CompiledGamePack> {
    verify_compiled_game_pack_for_runtime(base)?;
    ensure!(
        base.audio_compression.as_deref() != Some(PACK_AUDIO_COMPRESSION_MIDI),
        "Generation 3 pack creation requires the full embedded-audio core pack"
    );

    let source_path = modpack_root.join("data.json");
    let source: Gen3Source = serde_json::from_slice(
        &fs::read(&source_path).with_context(|| format!("read {}", source_path.display()))?,
    )
    .with_context(|| format!("parse {}", source_path.display()))?;
    validate_source(&source)?;

    let mut pack = base.clone();
    ensure!(
        pack.data
            .pokemon
            .keys()
            .all(|species_id| source.abilities.contains_key(species_id)),
        "Generation 3 ability catalog does not cover every base species"
    );
    for (species_id, species) in &mut pack.data.pokemon {
        species.ability = source.abilities[species_id].clone();
    }
    for (species_id, species) in source.pokemon {
        insert_keyed_pokemon_species(&mut pack.data.pokemon, species_id, species)?;
    }
    merge_learnsets(&mut pack.data.learnsets, source.learnsets)?;
    merge_evolution_table(&mut pack.data.evolutions, &source.evolutions)?;
    merge_menu_icons(&mut pack.data.menu_icons, source.menu_icons)?;
    for (species_id, entry) in source.pokedex_entries {
        insert_keyed_pokedex_entry(&mut pack.data.pokedex_entries, species_id, entry)?;
    }
    merge_frontpic_anim_entries(
        &mut pack.data.pokemon_frontpic_anim,
        source.pokemon_frontpic_anim,
    )?;
    merge_pokemon_cry_entries(&mut pack.data.pokemon_cries, source.pokemon_cries)?;

    let runtime_asset_root = modpack_root.join("assets");
    let runtime_files =
        collect_runtime_files(&runtime_asset_root.join("gfx"), &runtime_asset_root)?;
    for (key, bytes) in runtime_files {
        ensure!(
            pack.runtime_files.insert(key.clone(), bytes).is_none(),
            "Generation 3 runtime asset '{key}' already exists in the base pack"
        );
    }

    let (audio_assets, raw_audio) = load_audio_assets(&runtime_asset_root.join("audio"))?;
    let extension_manifest = ModpackAudioManifest::from_assets(&audio_assets, &raw_audio)?;
    for asset in audio_assets {
        insert_keyed_audio_asset(&mut pack.data.audio, asset.id.clone(), asset)?;
    }
    for (id, entry) in extension_manifest.cries {
        ensure!(
            pack.audio_manifest
                .cries
                .insert(id.clone(), entry)
                .is_none(),
            "Generation 3 cry manifest '{id}' already exists in the base pack"
        );
    }
    for (id, bytes) in raw_audio {
        let stored = if pack.audio_compression.as_deref() == Some(PACK_AUDIO_COMPRESSION_GZIP) {
            gzip_bytes(&bytes).with_context(|| format!("compress Generation 3 cry {id}"))?
        } else {
            bytes
        };
        ensure!(
            pack.compiled_audio.insert(id.clone(), stored).is_none(),
            "Generation 3 cry payload '{id}' already exists in the base pack"
        );
    }

    let pokemon_count = pack.data.pokemon.len();
    let last_rating = pack
        .data
        .oak_ratings
        .last_mut()
        .context("base pack has no Oak rating coverage")?;
    if last_rating.caught_count_limit < pokemon_count {
        last_rating.caught_count_limit = pokemon_count;
    }

    ensure!(
        pack.data.pokemon.len() == source.abilities.len()
            && pack
                .data
                .pokemon
                .iter()
                .all(|(species_id, species)| source.abilities.get(species_id)
                    == Some(&species.ability)),
        "compiled Generation 3 pack must retain the canonical ability catalog for all 386 species"
    );

    pack.report.manifests.push(GEN3_MANIFEST_ID.to_string());
    pack.report.pokemon = pokemon_count;
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

fn validate_source(source: &Gen3Source) -> Result<()> {
    ensure!(
        source.schema_version == 1,
        "unsupported Generation 3 data schema"
    );
    ensure!(
        source.metadata.id == GEN3_MANIFEST_ID,
        "Generation 3 metadata id must be gen3"
    );
    ensure!(
        source.source.repository == "https://github.com/pret/pokeemerald"
            && source.source.commit == "c65e93f20a5275ab03b07d6f6411096a82a60ffd",
        "Generation 3 data has unexpected source provenance"
    );
    ensure!(
        source.pokemon.len() == GEN3_SPECIES_COUNT,
        "Generation 3 data must contain exactly {GEN3_SPECIES_COUNT} species"
    );
    ensure!(
        source.abilities.len() == 386,
        "Generation 3 ability catalog must contain exactly 386 species"
    );
    ensure!(
        source.abilities.values().all(|ability| ability != "NONE"),
        "Generation 3 ability catalog must declare a canonical primary ability for every species"
    );
    let assigned_abilities = source
        .abilities
        .values()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let expected_abilities = GEN3_ASSIGNED_ABILITIES
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    ensure!(
        assigned_abilities == expected_abilities,
        "Generation 3 assigned abilities must match the pinned ability catalog exactly"
    );
    ensure!(
        source.pokemon.keys().eq(source.learnsets.keys())
            && source.pokemon.keys().eq(source.evolutions.0.keys())
            && source.pokemon.keys().eq(source.menu_icons.keys())
            && source.pokemon.keys().eq(source.pokedex_entries.keys())
            && source
                .pokemon
                .keys()
                .eq(source.pokemon_frontpic_anim.keys())
            && source.pokemon.keys().eq(source.pokemon_cries.keys()),
        "Generation 3 species catalogs must have identical keys"
    );
    let mut source_ids = source
        .pokemon
        .values()
        .map(|species| species.int_id)
        .collect::<Vec<_>>();
    source_ids.sort_unstable();
    ensure!(
        source_ids == (252_u16..=386).collect::<Vec<_>>(),
        "Generation 3 species must retain National Dex ids 252 through 386"
    );
    ensure!(
        source
            .pokemon
            .iter()
            .all(|(species_id, species)| source.abilities.get(species_id) == Some(&species.ability)),
        "Generation 3 species must declare their canonical primary ability"
    );
    Ok(())
}

fn collect_runtime_files(directory: &Path, root: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut files = BTreeMap::new();
    collect_runtime_files_into(directory, root, &mut files)?;
    Ok(files)
}

fn collect_runtime_files_into(
    directory: &Path,
    root: &Path,
    files: &mut BTreeMap<String, Vec<u8>>,
) -> Result<()> {
    for entry in fs::read_dir(directory)
        .with_context(|| format!("read Generation 3 asset directory {}", directory.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_runtime_files_into(&path, root, files)?;
            continue;
        }
        let key = path
            .strip_prefix(root)?
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");
        ensure!(
            !files.contains_key(&key),
            "duplicate Generation 3 runtime asset '{key}'"
        );
        files.insert(key, fs::read(&path)?);
    }
    Ok(())
}

fn load_audio_assets(
    audio_root: &Path,
) -> Result<(Vec<ModpackAudioAsset>, BTreeMap<String, Vec<u8>>)> {
    let mut paths = fs::read_dir(audio_root)
        .with_context(|| format!("read Generation 3 audio directory {}", audio_root.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<PathBuf>>>()?;
    paths.sort();
    ensure!(
        paths.len() == GEN3_SPECIES_COUNT,
        "Generation 3 audio directory must contain exactly {GEN3_SPECIES_COUNT} cries"
    );
    let mut assets = Vec::with_capacity(paths.len());
    let mut payloads = BTreeMap::new();
    for path in paths {
        ensure!(
            path.extension().and_then(|value| value.to_str()) == Some("pcm"),
            "unexpected Generation 3 audio file {}",
            path.display()
        );
        let id = path
            .file_stem()
            .and_then(|value| value.to_str())
            .context("Generation 3 audio file has no UTF-8 stem")?
            .to_string();
        let bytes = fs::read(&path)?;
        ensure!(
            !bytes.is_empty() && bytes.len().is_multiple_of(4),
            "Generation 3 cry {id} is not canonical stereo PCM"
        );
        let payload_hash = format!("{:08x}", fnv1a32_bytes(&bytes));
        assets.push(ModpackAudioAsset {
            id: id.clone(),
            path: format!("content-packs/gen3/cries/{id}.pcm"),
            kind: ModpackAudioKind::Cry,
            source: ModpackAudioSource::Pcm,
            sfx_priority: None,
            pcm_format: Some(ModpackPcmAudioFormat {
                sample_rate_hz: 22_050,
                channels: 2,
                bits_per_sample: 16,
            }),
            pcm_frame_count: Some(bytes.len() / 4),
            payload_hash: Some(payload_hash),
            loop_start_sample: None,
            loop_end_sample: None,
            midi_program: None,
        });
        payloads.insert(id, bytes);
    }
    Ok((assets, payloads))
}

fn gzip_bytes(bytes: &[u8]) -> Result<Vec<u8>> {
    use std::io::Write;

    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    encoder.write_all(bytes)?;
    Ok(encoder.finish()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repository_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
    }

    #[test]
    fn checked_in_source_has_the_complete_national_dex_range() {
        let root = repository_root().join("modpacks/gen3");
        let source: Gen3Source =
            serde_json::from_slice(&fs::read(root.join("data.json")).expect("read Gen 3 data"))
                .expect("parse Gen 3 data");
        validate_source(&source).expect("validate Gen 3 source");
        assert_eq!(source.pokemon["TREECKO"].int_id, 252);
        assert_eq!(source.pokemon["DEOXYS"].int_id, 386);
        assert_eq!(source.pokemon.len(), GEN3_SPECIES_COUNT);
        assert_eq!(source.pokemon["MUDKIP"].ability, "TORRENT");
        assert_eq!(source.abilities["BULBASAUR"], "OVERGROW");
        assert_eq!(source.abilities["MUDKIP"], "TORRENT");
        assert_eq!(source.abilities.len(), 386);
        assert!(
            source
                .pokemon
                .values()
                .all(|species| species.ability != "NONE")
        );
    }

    #[test]
    fn checked_in_assets_cover_every_species() {
        let modpack_root = repository_root().join("modpacks/gen3");
        let source: Gen3Source = serde_json::from_slice(
            &fs::read(modpack_root.join("data.json")).expect("read Gen 3 data"),
        )
        .expect("parse Gen 3 data");
        let root = modpack_root.join("assets");
        let (_, cries) = load_audio_assets(&root.join("audio")).expect("load Gen 3 cries");
        assert_eq!(cries.len(), GEN3_SPECIES_COUNT);
        assert!(
            source
                .pokemon_cries
                .values()
                .all(|metadata| cries.contains_key(&metadata.cry))
        );
        for species_id in source.pokemon.keys() {
            let directory = root
                .join("gfx/pokemon")
                .join(species_id.to_ascii_lowercase());
            for file in [
                "front.png",
                "back.png",
                "front.2bpp",
                "front.dimensions",
                "normal.gbcpal",
                "shiny.pal",
            ] {
                assert!(
                    directory.join(file).is_file(),
                    "missing {species_id}/{file}"
                );
            }
        }
    }
}
