//! Authored outdoor-building signatures derived from Crystal's metatile maps.
//!
//! A signature claims a complete drawing before generic terrain is meshed.
//! Collision never selects a building and unknown drawings remain faithful 2D.

#[derive(Clone, Copy, Debug)]
pub(crate) struct BuildingTemplate {
    pub tileset: &'static str,
    pub rows: &'static [&'static [u16]],
    pub roof_rows: usize,
    pub ground_tile: u16,
    pub skip_top_source_rows: usize,
    pub skip_left_source_columns: usize,
}

const fn template(
    tileset: &'static str,
    rows: &'static [&'static [u16]],
    roof_rows: usize,
    ground_tile: u16,
) -> BuildingTemplate {
    BuildingTemplate {
        tileset,
        rows,
        roof_rows,
        ground_tile,
        skip_top_source_rows: 0,
        skip_left_source_columns: 0,
    }
}

const fn traditional(rows: &'static [&'static [u16]], ground_tile: u16) -> BuildingTemplate {
    BuildingTemplate {
        tileset: "johto",
        rows,
        roof_rows: 2,
        ground_tile,
        // The first two source rows in $2c/$2d are background trees. The
        // actual traditional roof begins halfway through those metatiles.
        skip_top_source_rows: 2,
        skip_left_source_columns: 0,
    }
}

const fn traditional_landmark(
    rows: &'static [&'static [u16]],
    ground_tile: u16,
) -> BuildingTemplate {
    BuildingTemplate {
        tileset: "johto",
        rows,
        // Once the two background-tree rows are skipped, the wide sacred
        // building drawing has four top-facing roof rows over a two-row
        // facade.  Treating it as two-over-four folds roof stripes upright.
        roof_rows: 4,
        ground_tile,
        skip_top_source_rows: 2,
        skip_left_source_columns: 0,
    }
}

/// Exact whole-building drawings shared by all maps using these tilesets.
/// Ordering is most-specific first because placement claiming is deterministic.
pub(crate) const BUILDING_TEMPLATES: &[BuildingTemplate] = &[
    template(
        "johto",
        &[&[0x18, 0x1f, 0x19], &[0x1c, 0x77, 0x1e]],
        4,
        0x06,
    ),
    template(
        "johto",
        &[&[0x18, 0x1f, 0x19], &[0x1c, 0x1d, 0x1e]],
        4,
        0x06,
    ),
    template("johto", &[&[0x18, 0x19], &[0x16, 0x1e]], 4, 0x06),
    template("johto", &[&[0x18, 0x19], &[0x1a, 0x17]], 4, 0x06),
    template("johto", &[&[0x18, 0x19], &[0x1a, 0x1b]], 4, 0x06),
    template("johto", &[&[0x18, 0x19], &[0x1a, 0x11]], 4, 0x06),
    template("johto", &[&[0x18, 0x19], &[0x10, 0x11]], 4, 0x06),
    template("johto", &[&[0x14, 0x15]], 2, 0x06),
    traditional(&[&[0x2c, 0x2d], &[0x26, 0x2f]], 0x06),
    traditional(&[&[0x2c, 0x2d], &[0x2e, 0x2f]], 0x06),
    traditional_landmark(&[&[0x2c, 0x2a, 0x2d], &[0x26, 0x27, 0x2f]], 0x06),
    template("johto", &[&[0x20, 0x21], &[0x37, 0x3b]], 4, 0x06),
    template("johto", &[&[0x74, 0x75]], 2, 0x06),
    template(
        "johto",
        &[&[0x24, 0x25], &[0x24, 0x25], &[0x24, 0x25]],
        2,
        0x06,
    ),
    template("johto", &[&[0x08, 0x09], &[0x1c, 0x1e]], 4, 0x06),
    template(
        "johto",
        &[
            &[0x08, 0x09],
            &[0x7e, 0x7f],
            &[0x13, 0x0f],
            &[0x13, 0x0f],
            &[0x13, 0x0f],
            &[0x13, 0x0f],
            &[0x1a, 0x11],
        ],
        8,
        0x06,
    ),
    template(
        "johto_modern",
        &[&[0x18, 0x1f, 0x19], &[0x1c, 0x1d, 0x1e]],
        4,
        0x06,
    ),
    template(
        "johto_modern",
        &[&[0x18, 0x1f, 0x19], &[0x1a, 0x0f, 0x11]],
        4,
        0x06,
    ),
    template(
        "johto_modern",
        &[&[0x18, 0x1f, 0x19], &[0x1a, 0x2c, 0x11]],
        4,
        0x06,
    ),
    template(
        "johto_modern",
        &[&[0x18, 0x1f, 0x19], &[0x10, 0x17, 0x11]],
        4,
        0x06,
    ),
    // Goldenrod Department Store: the upper metatile mixes a shallow two-row
    // cap with the first window course.  Only that cap lies top-facing; the
    // remaining two source rows join the repeated storeys and entrance on the
    // upright facade.  Treating the whole metatile as roof folds an entire
    // floor onto the top and makes the landmark visibly too short.
    template(
        "johto_modern",
        &[
            &[0x18, 0x1f, 0x19],
            &[0x27, 0x23, 0x28],
            &[0x27, 0x23, 0x28],
            &[0x10, 0x17, 0x33],
        ],
        2,
        0x06,
    ),
    template(
        "johto_modern",
        &[&[0x18, 0x1f, 0x19], &[0x27, 0x23, 0x28]],
        4,
        0x06,
    ),
    // Goldenrod Radio Tower is a single narrow landmark drawing. Its upper
    // metatile row supplies the shallow cap; all remaining native rows are
    // unique antenna/window/facade bands and must stand on the door seam.
    template(
        "johto_modern",
        &[&[0x25, 0x26], &[0x29, 0x2a], &[0x2d, 0x2e]],
        2,
        0x06,
    ),
    template("johto_modern", &[&[0x18, 0x19], &[0x16, 0x1e]], 4, 0x06),
    template("johto_modern", &[&[0x18, 0x19], &[0x1a, 0x33]], 4, 0x06),
    template("johto_modern", &[&[0x18, 0x19], &[0x1a, 0x1b]], 4, 0x06),
    template("johto_modern", &[&[0x18, 0x19], &[0x10, 0x11]], 4, 0x06),
    template("johto_modern", &[&[0x18, 0x19], &[0x1c, 0x1e]], 4, 0x06),
    // Goldenrod's repeated square shops/houses are four independent 4x4
    // drawings. They are commonly adjacent in the block map, but $12/$13
    // and $14/$15 are not left/right halves of one wide structure.
    template("johto_modern", &[&[0x12]], 2, 0x06),
    template("johto_modern", &[&[0x13]], 2, 0x06),
    template("johto_modern", &[&[0x14]], 2, 0x06),
    template("johto_modern", &[&[0x15]], 2, 0x06),
    template("johto_modern", &[&[0x67]], 2, 0x06),
    template(
        "forest",
        &[
            &[0x1d, 0x1e, 0x1f],
            &[0x21, 0x22, 0x23],
            &[0x24, 0x26, 0x27],
        ],
        4,
        0x05,
    ),
    template(
        "forest",
        &[
            &[0x1d, 0x1e, 0x1f],
            &[0x21, 0x22, 0x23],
            &[0x25, 0x26, 0x27],
        ],
        4,
        0x05,
    ),
    // Battle Tower Outside is one complete tall landmark. Only its shallow
    // top cap is roof; the repeated glass/window courses are vertical tower
    // storeys and the final course contains the entrance.
    template(
        "battle_tower_outside",
        &[
            &[0x08, 0x09, 0x0b, 0x0e, 0x0f],
            &[0x08, 0x09, 0x0b, 0x0e, 0x0f],
            &[0x08, 0x09, 0x0b, 0x0e, 0x0f],
            &[0x10, 0x11, 0x12, 0x13, 0x14],
        ],
        2,
        0x06,
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_template_is_rectangular_and_has_a_real_facade() {
        for template in BUILDING_TEMPLATES {
            let width = template.rows[0].len();
            assert!(width > 0);
            assert!(template.rows.iter().all(|row| row.len() == width));
            let source_rows = template.rows.len() * 4 - template.skip_top_source_rows;
            let source_columns = width * 4 - template.skip_left_source_columns;
            assert!(source_columns > 0);
            assert!(template.roof_rows > 0);
            assert!(template.roof_rows < source_rows);
        }
    }

    #[test]
    fn goldenrod_square_buildings_are_independent_four_by_four_drawings() {
        for metatile in [0x12, 0x13, 0x14, 0x15, 0x67] {
            let template = BUILDING_TEMPLATES
                .iter()
                .find(|template| {
                    template.tileset == "johto_modern"
                        && template.rows.len() == 1
                        && template.rows[0] == &[metatile]
                })
                .expect("every Goldenrod square building must be independently authored");
            assert_eq!(template.roof_rows, 2);
            assert_eq!(template.ground_tile, 0x06);
        }
    }

    #[test]
    fn goldenrod_department_store_keeps_its_first_window_course_upright() {
        let store = BUILDING_TEMPLATES
            .iter()
            .find(|template| {
                template.tileset == "johto_modern"
                    && template.rows
                        == &[
                            &[0x18, 0x1f, 0x19][..],
                            &[0x27, 0x23, 0x28][..],
                            &[0x27, 0x23, 0x28][..],
                            &[0x10, 0x17, 0x33][..],
                        ]
            })
            .expect("Goldenrod Department Store template");

        assert_eq!(store.roof_rows, 2, "only the shallow cap is top-facing");
        assert_eq!(
            store.rows.len() * 4 - store.roof_rows,
            14,
            "all three window storeys and the entrance must remain upright"
        );
    }

    #[test]
    fn battle_tower_claims_its_complete_roof_and_entrance_drawing() {
        let tower = BUILDING_TEMPLATES
            .iter()
            .find(|template| template.tileset == "battle_tower_outside")
            .expect("Battle Tower landmark template");
        assert_eq!(tower.rows.len(), 4);
        assert!(tower.rows.iter().all(|row| row.len() == 5));
        assert_eq!(tower.roof_rows, 2);
        assert_eq!(tower.ground_tile, 0x06);
    }
}
