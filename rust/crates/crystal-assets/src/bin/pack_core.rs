use std::path::PathBuf;

use anyhow::{Context, Result};
use crystal_assets::AssetRoot;
use crystal_assets::modpack::{
    COMPILED_GAME_PACK_EXTENSION, ModpackCompileOptions, ModpackCompiler, ModpackManifest,
    ModpackMetadata,
};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let repository_root = args
        .next()
        .context("usage: pack_core <repository-root> <output-relative-path>")?;
    let output_relative_path = args
        .next()
        .context("usage: pack_core <repository-root> <output-relative-path>")?;
    if args.next().is_some() {
        anyhow::bail!("usage: pack_core <repository-root> <output-relative-path>");
    }
    if !output_relative_path.ends_with(&format!(".{COMPILED_GAME_PACK_EXTENSION}")) {
        anyhow::bail!("core runtime pack output must use .{COMPILED_GAME_PACK_EXTENSION}");
    }

    let asset_root = AssetRoot::new(PathBuf::from(&repository_root));
    let compiler = ModpackCompiler::new(&asset_root);
    let core_manifest = ModpackManifest {
        metadata: ModpackMetadata {
            id: "core-modular".to_string(),
            name: "Pokemon Crystal".to_string(),
            version: "1.0.0".to_string(),
            author: None,
            description: None,
        },
        priority: -100,
        ..ModpackManifest::default()
    };
    let compiled = compiler
        .compile(&[core_manifest], ModpackCompileOptions::default())
        .context("compile core modpack from exported content pack")?;
    compiled
        .write_game_pack(
            asset_root
                .runtime_assets()
                .join("data")
                .join(output_relative_path),
        )
        .context("write core runtime game pack")?;
    Ok(())
}
