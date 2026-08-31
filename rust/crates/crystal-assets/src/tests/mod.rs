fn roaming_catalog_for_tests(first_species: &str, second_species: &str) -> RoamingPokemonCatalog {
    let routes = (0_u8..16)
        .map(|index| RoamingPokemonRoute {
            map_group: 1,
            map_number: index + 1,
            connections: vec![RoamingMapLocation {
                map_group: 1,
                map_number: (index + 1) % 16 + 1,
            }],
        })
        .collect();
    RoamingPokemonCatalog {
        slot_count: 3,
        inactive_map: RoamingMapLocation {
            map_group: 0xfe,
            map_number: 0xfd,
        },
        init_writes: vec![
            RoamingPokemonInitWrite {
                slot: 0,
                species: first_species.to_string(),
                level: 40,
                map_group: 1,
                map_number: 1,
                hp: 0,
            },
            RoamingPokemonInitWrite {
                slot: 1,
                species: second_species.to_string(),
                level: 40,
                map_group: 1,
                map_number: 2,
                hp: 0,
            },
        ],
        routes,
        jump_mask: 15,
    }
}

fn map_name_sign_landmarks_for_tests(
    maps: impl IntoIterator<Item = &'static str>,
) -> PokegearLandmarksPayload {
    let definitions = [
        (0, "LANDMARK_SPECIAL"),
        (1, "LANDMARK_NEW_BARK_TOWN"),
        (2, "LANDMARK_ROUTE_29"),
        (0x11, "LANDMARK_RADIO_TOWER"),
        (0x3b, "LANDMARK_UNDERGROUND_PATH"),
        (0x44, "LANDMARK_POWER_PLANT"),
        (0x46, "LANDMARK_LAV_RADIO_TOWER"),
        (0x5a, "LANDMARK_INDIGO_PLATEAU"),
    ];
    PokegearLandmarksPayload {
        landmarks: definitions
            .into_iter()
            .map(|(id, constant)| PokegearLandmark {
                id,
                constant: constant.to_string(),
                label: constant.to_string(),
                name: constant.to_string(),
                x: 0,
                y: 0,
                region: "JOHTO".to_string(),
            })
            .collect(),
        map_to_landmark: maps
            .into_iter()
            .map(|map| (map.to_string(), "LANDMARK_ROUTE_29".to_string()))
            .collect(),
    }
}

fn bug_contest_encounters_for_tests()
-> Vec<crystal_core::systems::special_routines::BugContestEncounterEntry> {
    let mut encounters = (0..10)
        .map(
            |_| crystal_core::systems::special_routines::BugContestEncounterEntry {
                weight: 10,
                species: "RATTATA".to_string(),
                min_level: 5,
                max_level: 5,
            },
        )
        .collect::<Vec<_>>();
    encounters.push(
        crystal_core::systems::special_routines::BugContestEncounterEntry {
            weight: u8::MAX,
            species: "RATTATA".to_string(),
            min_level: 5,
            max_level: 5,
        },
    );
    encounters
}

fn magikarp_lengths_for_tests() -> Vec<crystal_core::systems::special_routines::MagikarpLengthEntry>
{
    [
        (110, 1),
        (310, 2),
        (710, 4),
        (2710, 20),
        (7710, 50),
        (17710, 100),
        (32710, 150),
        (47710, 150),
        (57710, 100),
        (62710, 50),
        (64710, 20),
        (65210, 5),
        (65410, 2),
        (65510, 1),
    ]
    .into_iter()
    .map(
        |(threshold, divisor)| crystal_core::systems::special_routines::MagikarpLengthEntry {
            threshold,
            divisor,
        },
    )
    .collect()
}

include!("pack_basics.rs");
include!("runtime_mutations.rs");
include!("verification.rs");
include!("map_modules.rs");
include!("audio_content.rs");
include!("content_tables.rs");
include!("playability.rs");
include!("items.rs");
include!("script_execution.rs");
