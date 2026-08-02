include!("runtime_tests/runtime_basics.rs");
include!("runtime_tests/overworld.rs");
include!("runtime_tests/battle_turns.rs");
include!("runtime_tests/special_routines.rs");
include!("runtime_tests/battle_items.rs");
include!("runtime_tests/save_validation.rs");

fn crystal_gift_inputs(seed: u32) -> (Dv, u32) {
    let mut rng = Random::new_crystal(seed);
    let dvs = Dv::from_non_hp(
        rng.randrange(16) as u8,
        rng.randrange(16) as u8,
        rng.randrange(16) as u8,
        rng.randrange(16) as u8,
    );
    (dvs, rng.seed())
}
