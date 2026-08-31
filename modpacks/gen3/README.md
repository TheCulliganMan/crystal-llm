# Generation 3 Pokemon modpack

This optional modpack extends the canonical Crystal pack with every species in
National Dex order 252 through 386 and assigns the pinned Emerald primary
ability to all 386 Pokemon. It includes Emerald base stats, types, growth rates,
egg groups, compatible held items, Crystal-supported TM/HM and level-up moves,
Pokedex text, static Crystal-sized front/back art, menu icon mappings, and
canonical PCM cries.

Build it from the repository root after creating the canonical core pack:

```sh
./export
./export-gen3
```

The output is `content-packs/gen3.crystalpack`. The generated runtime does not
need a pokeemerald checkout. `scripts/generate-gen3-modpack.mjs` is only the
maintainer regeneration path for the checked-in data and assets; it requires
the exact source commit recorded in `source.lock.json`.

The Crystal engine has no Contest condition stat and does not implement
personality-branched or party-creation evolutions. Milotic, Cascoon, Shedinja,
Huntail, and Gorebyss are present as complete species but are not produced by
those unsupported Emerald evolution rules. All ordinary level, friendship,
stone, and compatible trade evolutions are retained.
