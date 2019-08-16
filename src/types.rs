use packed_simd::*;

use num::traits::{ Num};

use std::ops::{Add, AddAssign, Div, Mul, Rem, Sub, SubAssign};

use bigdecimal::BigDecimal;
use num::BigInt;

pub trait ManType<T: Num, U: Num>:
    Add
    + Sub
    + Mul
    + Div
    + Rem
    + SubAssign
    + AddAssign
    + Mul<Self, Output = Self>
    + Add<Self, Output = Self>
    + Sub<Self, Output = Self>
    + Div<Self, Output = Self>
    + Rem<Self, Output = Self>
    + Sized
    + Clone
    + Send
{
    type TypeMask: ManMask<U>;

    fn new(value: T) -> Self;
    fn mask() -> Self;
    fn zero() -> Self;
    fn check(&self, other: &Self) -> Self::TypeMask;
    fn len() -> usize;
}

pub trait ManMask<T: Num>
where
    Self: Sized,
{
    type TypeCount: ManCount<T> + Rem;

    fn check(&mut self) -> bool;
    fn new_count(value: T) -> Self::TypeCount;
    fn count_zero() -> Self::TypeCount;
    fn to_count(&self) -> Self::TypeCount;
    fn lane(&self, i: usize) -> bool;
}

pub trait ManCount<T: Num>:
    Sized + Rem + Mul + Rem<Self, Output = Self> + AddAssign + Mul<Self, Output = Self>
{
    type TypeMask: ManMask<T>;

    fn check(&self, other: &Self) -> Self::TypeMask;
    fn lane(&self, i: usize) -> T;
}

macro_rules! impl_vector_mantype {
    ($calc:tt, $count:tt, $vector:tt, $mask:tt) => {
        impl ManType<$calc, $count> for $vector {
            type TypeMask = $mask;

            fn new(value: $calc) -> Self {
                Self::splat(value)
            }

            fn mask() -> Self {
                let mut mask = Vec::new();
                for n in 0..Self::len() {
                    mask.push(n as $calc);
                }
                Self::from_slice_unaligned(&mask)
            }

            fn zero() -> Self {
                Self::from_slice_unaligned(&vec![0 as $calc; Self::len()])
            }

            fn check(&self, other: &Self) -> Self::TypeMask {
                self.lt(*other)
            }

            fn len() -> usize {
                Self::lanes()
            }
        }
    };
}

macro_rules! impl_vector_manmask {
    ($count:tt, $counter:tt, $mask:tt) => {
        impl ManMask<$count> for $mask {
            type TypeCount = $counter;

            fn check(&mut self) -> bool {
                self.any()
            }

            fn new_count(value: $count) -> Self::TypeCount {
                Self::TypeCount::splat(value)
            }

            fn count_zero() -> Self::TypeCount {
                Self::new_count(0)
            }

            fn to_count(&self) -> Self::TypeCount {
                -Self::TypeCount::from_cast(*self)
            }

            fn lane(&self, i: usize) -> bool {
                self.extract(i)
            }
        }
    };
}

macro_rules! impl_vector_mancount {
    ($count:tt, $counter:tt, $mask:tt) => {
        impl ManCount<$count> for $counter {
            type TypeMask = $mask;

            fn check(&self, other: &Self) -> Self::TypeMask {
                <$counter>::eq(*self, *other)
            }

            fn lane(&self, i: usize) -> $count {
                self.extract(i)
            }
        }
    };
}

macro_rules! cast_calc {
    ($e:expr, f32) => {
        $e as f32
    };
    ($e:expr, f64) => {
        $e as f64
    };
    ($e:expr, BigDecimal) => {
        BigDecimal::from($e).with_prec(4)
    };
}

macro_rules! impl_mantype {
    ($calc:tt, $count:tt) => {
        impl ManType<$calc, $count> for $calc {
            type TypeMask = bool;

            fn new(value: $calc) -> Self {
                value
            }

            fn mask() -> Self {
                cast_calc!(0, $calc)
            }

            fn zero() -> Self {
                cast_calc!(0, $calc)
            }

            fn check(&self, other: &Self) -> Self::TypeMask {
                self < other
            }

            fn len() -> usize {
                1
            }
        }
    };
}

macro_rules! cast_count {
    ($e:expr, i32) => {
        $e as i32
    };
    ($e:expr, i64) => {
        $e as i64
    };
    ($e:expr, BigInt) => {
        BigInt::from($e)
    };
}

macro_rules! impl_manmask {
    ($count:tt) => {
        impl ManMask<$count> for bool {
            type TypeCount = $count;

            fn check(&mut self) -> bool {
                *self
            }

            fn new_count(value: $count) -> Self::TypeCount {
                value
            }

            fn count_zero() -> Self::TypeCount {
                cast_count!(0, $count)
            }

            fn to_count(&self) -> Self::TypeCount {
                if *self {
                    cast_count!(1, $count)
                } else {
                    cast_count!(0, $count)
                }
            }

            fn lane(&self, _i: usize) -> bool {
                *self
            }
        }
    };
}

macro_rules! impl_mancount {
    ($count:ty) => {
        impl ManCount<$count> for $count {
            type TypeMask = bool;

            fn check(&self, other: &Self) -> Self::TypeMask {
                self == other
            }

            fn lane(&self, _i: usize) -> $count {
                self.clone()
            }
        }
    };
}

macro_rules! impl_mandel_vector {
    ($calc:ty, $count:ty, $vector:ty, $counter:ty, $mask:ty) => {
        impl_vector_mantype!($calc, $count, $vector, $mask);
        impl_vector_manmask!($count, $counter, $mask);
        impl_vector_mancount!($count, $counter, $mask);
    };
}

macro_rules! impl_mandel {
    ($calc:tt, $count:tt) => {
        impl_mantype!($calc, $count);
        impl_manmask!($count);
        impl_mancount!($count);
    };
}

impl_mandel_vector!(f32, i32, f32x2, i32x2, m32x2);
impl_mandel_vector!(f32, i32, f32x4, i32x4, m32x4);
impl_mandel_vector!(f32, i32, f32x8, i32x8, m32x8);
impl_mandel_vector!(f64, i64, f64x2, i64x2, m64x2);
impl_mandel_vector!(f64, i64, f64x4, i64x4, m64x4);
impl_mandel_vector!(f64, i64, f64x8, i64x8, m64x8);

impl_mandel!(BigDecimal, BigInt);
impl_mandel!(f64, i64);
impl_mandel!(f32, i32);