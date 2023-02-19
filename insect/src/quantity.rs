use crate::arithmetic::{Power, Rational};
use crate::number::Number;
use crate::unit::Unit;

use num_traits::FromPrimitive;
use thiserror::Error;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ConversionError {
    #[error("Conversion error: unit '{0}' can not be converted to '{1}'")]
    IncompatibleUnits(Unit, Unit),
}

pub type Result<T> = std::result::Result<T, ConversionError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quantity {
    value: Number,
    unit: Unit,
}

impl Quantity {
    pub fn new(value: Number, unit: Unit) -> Self {
        Quantity { value, unit }
    }

    pub fn from_scalar(value: f64) -> Quantity {
        Quantity::new(Number::from_f64(value), Unit::scalar())
    }

    pub fn from_unit(unit: Unit) -> Quantity {
        Quantity::new(Number::from_f64(1.0), unit)
    }

    pub fn unit(&self) -> &Unit {
        &self.unit
    }

    pub fn is_zero(&self) -> bool {
        self.value.to_f64() == 0.0
    }

    fn to_base_unit_representation(&self) -> Quantity {
        let (unit, factor) = self.unit.to_base_unit_representation();
        Quantity::new(self.value * factor, unit)
    }

    pub fn convert_to(&self, target_unit: &Unit) -> Result<Quantity> {
        if &self.unit == target_unit || self.is_zero() {
            Ok(Quantity::new(self.value, target_unit.clone()))
        } else {
            let (target_base_unit_representation, factor) =
                target_unit.to_base_unit_representation();

            let quantity_base_unit_representation = self.to_base_unit_representation();
            let self_base_unit_representation = quantity_base_unit_representation.unit();

            if self_base_unit_representation == &target_base_unit_representation {
                Ok(Quantity::new(
                    *quantity_base_unit_representation.unsafe_value() / factor,
                    target_unit.clone(),
                ))
            } else {
                // TODO: can this even be triggered? replace by an assertion?
                Err(ConversionError::IncompatibleUnits(
                    self.unit.clone(),
                    target_unit.clone(),
                ))
            }
        }
    }

    pub fn full_simplify(self) -> Self {
        if let Ok(num) = self.as_scalar() {
            Self::from_scalar(num.to_f64())
        } else {
            self
        }
    }

    pub fn as_scalar(&self) -> Result<Number> {
        Ok(self.convert_to(&Unit::scalar())?.value)
    }

    pub fn unsafe_value(&self) -> &Number {
        &self.value
    }

    pub fn power(self, exp: Quantity) -> Result<Self> {
        let exponent_as_scalar = exp.as_scalar()?.to_f64();
        Ok(Quantity::new(
            Number::from_f64(self.value.to_f64().powf(exponent_as_scalar)),
            self.unit
                .power(Rational::from_f64(exponent_as_scalar).unwrap()), // TODO: error handling; can this really handle rational exponents?
        ))
    }
}

impl From<&Number> for Quantity {
    fn from(n: &Number) -> Self {
        Quantity::from_scalar(n.to_f64())
    }
}

impl std::ops::Add for &Quantity {
    type Output = Result<Quantity>;

    fn add(self, rhs: Self) -> Self::Output {
        Ok(Quantity {
            value: self.value + rhs.convert_to(&self.unit)?.value,
            unit: self.unit.clone(),
        })
    }
}

impl std::ops::Sub for &Quantity {
    type Output = Result<Quantity>;

    fn sub(self, rhs: Self) -> Self::Output {
        Ok(Quantity {
            value: self.value - rhs.convert_to(&self.unit)?.value,
            unit: self.unit.clone(),
        })
    }
}

impl std::ops::Mul for Quantity {
    type Output = Result<Quantity>;

    fn mul(self, rhs: Self) -> Self::Output {
        Ok(Quantity {
            value: self.value * rhs.value,
            unit: self.unit * rhs.unit,
        })
    }
}

impl std::ops::Div for Quantity {
    type Output = Result<Quantity>;

    fn div(self, rhs: Self) -> Self::Output {
        Ok(Quantity {
            value: self.value / rhs.value,
            unit: self.unit / rhs.unit,
        })
    }
}

impl std::ops::Neg for Quantity {
    type Output = Quantity;

    fn neg(self) -> Self::Output {
        Quantity {
            value: -self.value,
            unit: self.unit,
        }
    }
}

impl std::fmt::Display for Quantity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: we do not always want to canonicalize
        let mut unit_canonicalized = self.unit.clone();
        unit_canonicalized.canonicalize();
        write!(f, "{:.6} {}", self.value.to_f64(), unit_canonicalized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversion_trivial() {
        let meter = Unit::new_base("meter");
        let second = Unit::new_base("second");

        let length = Quantity::new(Number::from_f64(2.0), meter.clone());

        assert!(length.convert_to(&meter).is_ok());

        assert!(length.convert_to(&second).is_err());
        assert!(length.convert_to(&Unit::scalar()).is_err());
    }

    #[test]
    fn conversion_basic() {
        use approx::assert_relative_eq;

        let meter = Unit::new_base("meter");
        let foot = Unit::new_derived("foot", Number::from_f64(0.3048), meter.clone());

        let length = Quantity::new(Number::from_f64(2.0), meter.clone());

        let length_in_foot = length.convert_to(&foot).expect("conversion succeeds");
        assert_eq!(length_in_foot.unsafe_value().to_f64(), 2.0 / 0.3048);

        let length_converted_back_to_meter = length_in_foot
            .convert_to(&meter)
            .expect("conversion succeeds");
        assert_relative_eq!(
            length_converted_back_to_meter.unsafe_value().to_f64(),
            2.0,
            epsilon = 1e-6
        );
    }

    #[test]
    fn prefixes() {
        use crate::prefix::Prefix;

        use approx::assert_relative_eq;
        use num_rational::Ratio;

        let meter = Unit::new_base("meter");
        let centimeter = Unit::new_base("meter").with_prefix(Prefix::centi());

        let length = Quantity::new(Number::from_f64(2.5), meter.clone());
        {
            let length_in_centimeter = length.convert_to(&centimeter).expect("conversion succeeds");
            assert_relative_eq!(
                length_in_centimeter.unsafe_value().to_f64(),
                250.0,
                epsilon = 1e-6
            );

            let length_converted_back_to_meter = length_in_centimeter
                .convert_to(&meter)
                .expect("conversion succeeds");
            assert_relative_eq!(
                length_converted_back_to_meter.unsafe_value().to_f64(),
                2.5,
                epsilon = 1e-6
            );
        }
        {
            let volume = length
                .power(Quantity::from_scalar(3.0))
                .expect("exponent is scalar");

            println!("{}", &volume);

            let volume_in_centimeter3 = volume
                .convert_to(&centimeter.power(Ratio::from_integer(3)))
                .expect("conversion succeeds");
            assert_relative_eq!(
                volume_in_centimeter3.unsafe_value().to_f64(),
                15_625_000.0,
                epsilon = 1e-6
            );
        }
    }
}
