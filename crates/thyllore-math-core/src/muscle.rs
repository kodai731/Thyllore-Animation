pub fn degrees_to_muscle(
    degrees: f32,
    sign: f32,
    min_degrees: f32,
    max_degrees: f32,
    rest: f32,
) -> f32 {
    let signed_degrees = sign * degrees;
    if signed_degrees >= 0.0 {
        rest + signed_degrees / max_degrees
    } else {
        rest + signed_degrees / -min_degrees
    }
}

pub fn muscle_to_degrees(
    muscle: f32,
    sign: f32,
    min_degrees: f32,
    max_degrees: f32,
    rest: f32,
) -> f32 {
    let offset = muscle - rest;
    let signed_degrees = if offset >= 0.0 {
        offset * max_degrees
    } else {
        offset * -min_degrees
    };
    sign * signed_degrees
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn muscle_round_trip_restores_degrees() {
        for sign in [1.0, -1.0] {
            for (min_degrees, max_degrees, rest) in [
                (-40.0, 40.0, 0.0),
                (-60.0, 100.0, 0.25),
                (-10.0, 80.0, -0.5),
            ] {
                for degrees in [-75.0, -40.0, -12.5, 0.0, 3.0, 40.0, 90.0] {
                    let muscle = degrees_to_muscle(degrees, sign, min_degrees, max_degrees, rest);
                    let recovered = muscle_to_degrees(muscle, sign, min_degrees, max_degrees, rest);
                    assert!(
                        (recovered - degrees).abs() < 1e-4,
                        "{degrees} -> {muscle} -> {recovered}"
                    );
                }
            }
        }
    }

    #[test]
    fn degree_round_trip_restores_muscle() {
        for sign in [1.0, -1.0] {
            for muscle in [-1.5, -1.0, -0.3, 0.0, 0.25, 0.7, 1.0, 1.8] {
                let degrees = muscle_to_degrees(muscle, sign, -60.0, 100.0, 0.25);
                let recovered = degrees_to_muscle(degrees, sign, -60.0, 100.0, 0.25);
                assert!(
                    (recovered - muscle).abs() < 1e-5,
                    "{muscle} -> {degrees} -> {recovered}"
                );
            }
        }
    }

    #[test]
    fn zero_degrees_is_rest_and_limits_are_unit_offsets() {
        assert_eq!(degrees_to_muscle(0.0, 1.0, -40.0, 60.0, 0.2), 0.2);
        assert!((degrees_to_muscle(60.0, 1.0, -40.0, 60.0, 0.0) - 1.0).abs() < 1e-6);
        assert!((degrees_to_muscle(-40.0, 1.0, -40.0, 60.0, 0.0) + 1.0).abs() < 1e-6);
        assert!((degrees_to_muscle(40.0, -1.0, -40.0, 60.0, 0.0) + 1.0).abs() < 1e-6);
    }
}
