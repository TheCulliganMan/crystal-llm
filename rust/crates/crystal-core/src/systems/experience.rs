use crate::models::GrowthRate;

pub fn calculate_experience(growth_rate: GrowthRate, level: u8) -> i32 {
    let n = level as i32;
    match growth_rate {
        GrowthRate::MediumFast => n * n * n,
        GrowthRate::SlightlyFast => {
            let n2 = n * n;
            let n3 = n2 * n;
            ((3 * n3) / 4) + (10 * n2) - 30
        }
        GrowthRate::SlightlySlow => {
            let n2 = n * n;
            let n3 = n2 * n;
            ((3 * n3) / 4) + (20 * n2) - 70
        }
        GrowthRate::MediumSlow => {
            let n2 = n * n;
            let n3 = n2 * n;
            ((6 * n3) / 5) - (15 * n2) + (100 * n) - 140
        }
        GrowthRate::Fast => (4 * n * n * n) / 5,
        GrowthRate::Slow => (5 * n * n * n) / 4,
        GrowthRate::Erratic | GrowthRate::Fluctuating => {
            panic!("unsupported growth rate for Pokemon Crystal: {growth_rate:?}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_typescript_experience_curve_cases() {
        assert_eq!(calculate_experience(GrowthRate::MediumFast, 1), 1);
        assert_eq!(calculate_experience(GrowthRate::MediumFast, 50), 125000);
        assert_eq!(calculate_experience(GrowthRate::MediumFast, 100), 1000000);

        assert_eq!(calculate_experience(GrowthRate::SlightlyFast, 1), -20);
        assert_eq!(calculate_experience(GrowthRate::SlightlyFast, 50), 118720);
        assert_eq!(calculate_experience(GrowthRate::SlightlyFast, 100), 849970);

        assert_eq!(calculate_experience(GrowthRate::SlightlySlow, 1), -50);
        assert_eq!(calculate_experience(GrowthRate::SlightlySlow, 50), 143680);
        assert_eq!(calculate_experience(GrowthRate::SlightlySlow, 100), 949930);

        assert_eq!(calculate_experience(GrowthRate::MediumSlow, 1), -54);
        assert_eq!(calculate_experience(GrowthRate::MediumSlow, 50), 117360);
        assert_eq!(calculate_experience(GrowthRate::MediumSlow, 100), 1059860);

        assert_eq!(calculate_experience(GrowthRate::Fast, 1), 0);
        assert_eq!(calculate_experience(GrowthRate::Fast, 50), 100000);
        assert_eq!(calculate_experience(GrowthRate::Fast, 100), 800000);

        assert_eq!(calculate_experience(GrowthRate::Slow, 1), 1);
        assert_eq!(calculate_experience(GrowthRate::Slow, 50), 156250);
        assert_eq!(calculate_experience(GrowthRate::Slow, 100), 1250000);
    }
}
