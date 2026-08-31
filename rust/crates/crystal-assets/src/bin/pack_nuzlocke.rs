use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use crystal_assets::modpack::{build_nuzlocke_modpack, read_verified_compiled_game_pack};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let repository_root = PathBuf::from(
        args.next()
            .context("usage: pack_nuzlocke <repository-root> [base-pack] [output-pack]")?,
    );
    let base_pack = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repository_root.join("content-packs/core-modular.crystalpack"));
    let output_pack = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| repository_root.join("content-packs/nuzlocke.crystalpack"));
    if args.next().is_some() {
        anyhow::bail!("usage: pack_nuzlocke <repository-root> [base-pack] [output-pack]");
    }
    pack_nuzlocke(&base_pack, &output_pack)
}

fn pack_nuzlocke(base_pack: &Path, output_pack: &Path) -> Result<()> {
    let base = read_verified_compiled_game_pack(base_pack)
        .with_context(|| format!("load verified base pack {}", base_pack.display()))?;
    let nuzlocke = build_nuzlocke_modpack(&base)?;
    nuzlocke
        .write_preserving_storage(output_pack)
        .with_context(|| format!("write Nuzlocke pack {}", output_pack.display()))?;
    println!("exported {}", output_pack.display());
    Ok(())
}
