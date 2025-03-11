use crate::base::{math, scalar::Scalar};
use arrow::datatypes::i256;

const MIN_SUPPORTED_I256: i256 = i256::from_parts(
    326_411_208_032_252_286_695_448_638_536_326_387_210,
    -10_633_823_966_279_326_983_230_456_482_242_756_609,
);
const MAX_SUPPORTED_I256: i256 = i256::from_parts(
    13_871_158_888_686_176_767_925_968_895_441_824_246,
    10_633_823_966_279_326_983_230_456_482_242_756_608,
);

/// Converts a type implementing [Scalar] into an arrow i256
pub fn convert_scalar_to_i256<S: Scalar>(val: &S) -> i256 {
    let is_negative = val > &S::MAX_SIGNED;
    let abs_scalar = if is_negative { -*val } else { *val };
    let limbs: [u64; 4] = abs_scalar.into();

    let low = u128::from(limbs[0]) | (u128::from(limbs[1]) << 64);
    let high = i128::from(limbs[2]) | (i128::from(limbs[3]) << 64);

    let abs_i256 = i256::from_parts(low, high);
    if is_negative {
        i256::wrapping_neg(abs_i256)
    } else {
        abs_i256
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
/// Converts an arrow i256 into limbed representation and then
/// into a type implementing [Scalar]
#[must_use]
pub fn convert_i256_to_scalar<S: Scalar>(value: &i256) -> Option<S> {
    // Check if value is within the bounds
    if value < &MIN_SUPPORTED_I256 || value > &MAX_SUPPORTED_I256 {
        None
    } else {
        // Prepare the absolute value for conversion
        let abs_value = if value.is_negative() { -*value } else { *value };
        let (low, high) = abs_value.to_parts();
        let limbs = [
            low as u64,
            (low >> 64) as u64,
            high as u64,
            (high >> 64) as u64,
        ];

        // Convert limbs to Scalar and adjust for sign
        let scalar: S = limbs.into();
        Some(if value.is_negative() { -scalar } else { scalar })
    }
}

#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
impl From<i256> for math::i256::I256 {
    fn from(value: i256) -> Self {
        let (low, high) = value.to_parts();
        Self::new([
            low as u64,
            (low >> 64) as u64,
            high as u64,
            (high >> 64) as u64,
        ])
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::base::scalar::{test_scalar::TestScalar, Scalar};
    use num_traits::Zero;
    use rand::RngCore;

    /// Generate a random i256 within a supported range. Values generated by this function will
    /// fit into the i256 but will not exceed 252 bits of width.
    fn random_i256<R: RngCore + ?Sized>(rng: &mut R) -> i256 {
        use rand::Rng;
        let max_signed_as_parts: (u128, i128) =
            convert_scalar_to_i256(&TestScalar::MAX_SIGNED).to_parts();

        // Generate a random high part
        let high: i128 = rng.gen_range(-max_signed_as_parts.1..=max_signed_as_parts.1);

        // Generate a random low part, adjusted based on the high part
        let low: u128 = if high < max_signed_as_parts.1 {
            rng.gen()
        } else {
            rng.gen_range(0..=max_signed_as_parts.0)
        };

        i256::from_parts(low, high)
    }

    impl TryFrom<i256> for TestScalar {
        type Error = ();

        // Must fit inside 252 bits and so requires fallible
        fn try_from(value: i256) -> Result<Self, ()> {
            convert_i256_to_scalar(&value).ok_or(())
        }
    }

    impl From<TestScalar> for i256 {
        fn from(value: TestScalar) -> Self {
            convert_scalar_to_i256(&value)
        }
    }

    #[test]
    fn test_testscalar_to_i256_conversion() {
        let positive_scalar = TestScalar::from(12345);
        let expected_i256 = i256::from(12345);
        assert_eq!(i256::from(positive_scalar), expected_i256);

        let negative_scalar = TestScalar::from(-12345);
        let expected_i256 = i256::from(-12345);
        assert_eq!(i256::from(negative_scalar), expected_i256);

        let max_scalar = TestScalar::MAX_SIGNED;
        let expected_max = i256::from(TestScalar::MAX_SIGNED);
        assert_eq!(i256::from(max_scalar), expected_max);

        let min_scalar = TestScalar::from(0);
        let expected_min = i256::from(TestScalar::from(0));
        assert_eq!(i256::from(min_scalar), expected_min);
    }

    #[test]
    fn test_testscalar_i256_overflow_and_underflow() {
        // 2^256 overflows
        assert!(TestScalar::try_from(i256::MAX).is_err());

        // MAX_SIGNED + 1 overflows
        assert!(TestScalar::try_from(MAX_SUPPORTED_I256 + i256::from(1)).is_err());

        // -2^255 underflows
        assert!(i256::MIN < -(i256::from(TestScalar::MAX_SIGNED)));
        assert!(TestScalar::try_from(i256::MIN).is_err());

        // -MAX-SIGNED - 1 underflows
        assert!(TestScalar::try_from(MIN_SUPPORTED_I256 - i256::from(1)).is_err());
    }

    #[test]
    fn test_i256_testscalar_negative() {
        // Test conversion from i256(-1) to TestScalar
        let neg_one_i256_testscalar = TestScalar::try_from(i256::from(-1));
        assert!(neg_one_i256_testscalar.is_ok());
        let neg_one_testscalar = TestScalar::from(-1);
        assert_eq!(neg_one_i256_testscalar.unwrap(), neg_one_testscalar);
    }

    #[test]
    fn test_i256_testscalar_zero() {
        // Test conversion from i256(0) to TestScalar
        let zero_i256_testscalar = TestScalar::try_from(i256::from(0));
        assert!(zero_i256_testscalar.is_ok());
        let zero_testscalar = TestScalar::zero();
        assert_eq!(zero_i256_testscalar.unwrap(), zero_testscalar);
    }

    #[test]
    fn test_i256_testscalar_positive() {
        // Test conversion from i256(42) to TestScalar
        let forty_two_i256_testscalar = TestScalar::try_from(i256::from(42));
        let forty_two_testscalar = TestScalar::from(42);
        assert_eq!(forty_two_i256_testscalar.unwrap(), forty_two_testscalar);
    }

    #[test]
    fn test_i256_testscalar_max_signed() {
        let max_signed = MAX_SUPPORTED_I256;
        // max signed value
        let max_signed_scalar = TestScalar::MAX_SIGNED;
        // Test conversion from i256 to TestScalar
        let i256_testscalar = TestScalar::try_from(max_signed);
        assert!(i256_testscalar.is_ok());
        assert_eq!(i256_testscalar.unwrap(), max_signed_scalar);
    }

    #[test]
    fn test_i256_testscalar_min_signed() {
        let min_signed = MIN_SUPPORTED_I256;
        let i256_testscalar = TestScalar::try_from(min_signed);
        // -MAX_SIGNED is ok
        assert!(i256_testscalar.is_ok());
        assert_eq!(
            i256_testscalar.unwrap(),
            TestScalar::MAX_SIGNED + TestScalar::from(1)
        );
    }

    #[test]
    fn test_i256_testscalar_random() {
        let mut rng = rand::thread_rng();
        for _ in 0..1000 {
            let i256_value = random_i256(&mut rng);
            let curve25519_scalar = TestScalar::try_from(i256_value).expect("Conversion failed");
            let back_to_i256 = i256::from(curve25519_scalar);
            assert_eq!(i256_value, back_to_i256, "Round-trip conversion failed");
        }
    }

    #[expect(clippy::cast_sign_loss)]
    #[test]
    fn test_arrow_i256_to_posql_i256_conversion() {
        // Test zero
        assert_eq!(
            math::i256::I256::from(i256::ZERO),
            math::i256::I256::new([0, 0, 0, 0])
        );

        // Test positive values
        assert_eq!(
            math::i256::I256::from(i256::from(1)),
            math::i256::I256::new([1, 0, 0, 0])
        );
        assert_eq!(
            math::i256::I256::from(i256::from(2)),
            math::i256::I256::new([2, 0, 0, 0])
        );

        // Test negative values
        assert_eq!(
            math::i256::I256::from(i256::from(-1)),
            math::i256::I256::new([u64::MAX; 4])
        );
        assert_eq!(
            math::i256::I256::from(i256::from(-2)),
            math::i256::I256::new([u64::MAX - 1, u64::MAX, u64::MAX, u64::MAX])
        );

        // Test some boundary values
        assert_eq!(
            math::i256::I256::from(i256::MAX),
            math::i256::I256::new([u64::MAX, u64::MAX, u64::MAX, i64::MAX as u64])
        );
        assert_eq!(
            math::i256::I256::from(i256::MIN),
            math::i256::I256::new([0, 0, 0, i64::MIN as u64])
        );

        // Test other values
        assert_eq!(
            math::i256::I256::from(i256::from_parts(40, 20)),
            math::i256::I256::new([40, 0, 20, 0])
        );
        assert_eq!(
            math::i256::I256::from(i256::from_parts(20, -20)),
            math::i256::I256::new([20, 0, u64::MAX - 19, u64::MAX])
        );
    }
}
