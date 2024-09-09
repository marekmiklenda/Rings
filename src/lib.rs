#![feature(try_trait_v2)]

use std::{
    convert::Infallible,
    fmt::{Debug, Display},
    ops::{ControlFlow, Deref, DerefMut, FromResidual, Try},
};

pub mod build;
pub mod error;
pub mod instruction;
pub mod vm;
pub mod io;

#[derive(Clone, Copy, Default, Debug)]
pub enum NumberSystem {
    #[default]
    Unknown,
    Decimal,
    Octal,
    Binary,
    Hexadecimal,
}

impl std::fmt::Display for NumberSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => write!(f, "Unknown"),
            Self::Decimal => write!(f, "Dec"),
            Self::Binary => write!(f, "Bin"),
            Self::Hexadecimal => write!(f, "Hex"),
            Self::Octal => write!(f, "Oct"),
        }
    }
}

pub type LocalizedResult<T, E> = Localized<Result<T, E>>;
pub struct Localized<V> {
    pub line_number: usize,
    pub char_number: usize,
    pub value: V,
}

impl<V> Localized<V> {
    pub fn new(value: V) -> Self {
        Self {
            line_number: 1,
            char_number: 1,
            value,
        }
    }

    pub fn transform<N>(&self, new: N) -> Localized<N> {
        Localized {
            char_number: self.char_number,
            line_number: self.line_number,
            value: new,
        }
    }

    pub fn cut(self) -> (Localized<()>, V) {
        let Self {
            line_number,
            char_number,
            value,
        } = self;

        (
            Localized {
                char_number,
                line_number,
                value: (),
            },
            value,
        )
    }

    pub fn inc_char(&mut self) {
        self.char_number += 1;
    }

    pub fn new_line(&mut self) {
        self.line_number += 1;
        self.char_number = 1;
    }

    pub fn map<F, N>(self, map: F) -> Localized<N>
    where
        F: FnOnce(V) -> N,
    {
        let Self {
            line_number,
            char_number,
            value,
        } = self;

        let value = (map)(value);

        Localized {
            line_number,
            char_number,
            value,
        }
    }

    pub fn unwrap(self) -> V {
        self.value
    }
}

impl<T, E> Localized<Result<T, E>> {
    pub fn into_err(self) -> Option<Localized<E>> {
        let Self {
            line_number,
            char_number,
            value,
        } = self;
        match value {
            Ok(..) => None,
            Err(e) => Some(Localized {
                line_number,
                char_number,
                value: e,
            }),
        }
    }

    pub fn into_ok(self) -> Option<Localized<T>> {
        let Self {
            line_number,
            char_number,
            value,
        } = self;
        match value {
            Err(..) => None,
            Ok(v) => Some(Localized {
                line_number,
                char_number,
                value: v,
            }),
        }
    }
}

impl<V> Default for Localized<V>
where
    V: Default,
{
    fn default() -> Self {
        Self::new(V::default())
    }
}

impl<V> Clone for Localized<V>
where
    V: Clone,
{
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            char_number: self.char_number,
            line_number: self.line_number,
        }
    }
}

impl<T, E> Try for Localized<Result<T, E>> {
    type Output = Localized<T>;
    type Residual = Localized<Result<Infallible, E>>;
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        let Self {
            line_number,
            char_number,
            value,
        } = self;

        match value {
            Ok(value) => ControlFlow::Continue(Localized {
                value,
                line_number,
                char_number,
            }),
            Err(value) => ControlFlow::Break(Localized {
                value: Err(value),
                line_number,
                char_number,
            }),
        }
    }

    fn from_output(output: Self::Output) -> Self {
        output.map(Result::from_output)
    }
}

impl<T, E> FromResidual<Localized<Result<Infallible, E>>> for Localized<Result<T, E>> {
    fn from_residual(residual: Localized<Result<Infallible, E>>) -> Self {
        residual.map(Result::from_residual)
    }
}

impl<V> Deref for Localized<V> {
    type Target = V;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<V> DerefMut for Localized<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<V> AsRef<V> for Localized<V> {
    fn as_ref(&self) -> &V {
        &self.value
    }
}

impl<V> Display for Localized<V>
where
    V: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "at {}@{}: {}",
            self.line_number, self.char_number, self.value
        )
    }
}

impl<V> Debug for Localized<V>
where
    V: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "at {}@{}: {:?}",
            self.line_number, self.char_number, self.value
        )
    }
}

pub enum MaybeLocalized<V> {
    Localized(Localized<V>),
    General(V),
}

impl<V> MaybeLocalized<V> {
    pub fn unwrap(self) -> V {
        match self {
            Self::General(v) => v,
            Self::Localized(v) => v.unwrap(),
        }
    }

    pub fn cut(self) -> (MaybeLocalized<()>, V) {
        match self {
            Self::General(v) => (MaybeLocalized::General(()), v),
            Self::Localized(l) => {
                let (loc, val) = l.cut();
                (MaybeLocalized::Localized(loc), val)
            }
        }
    }

    pub fn transform<N>(&self, val: N) -> MaybeLocalized<N> {
        match self {
            Self::General(..) => MaybeLocalized::General(val),
            Self::Localized(loc) => MaybeLocalized::Localized(loc.transform(val)),
        }
    }

    pub fn map<F, N>(self, map: F) -> MaybeLocalized<N>
    where
        F: FnOnce(V) -> N,
    {
        match self {
            Self::General(v) => MaybeLocalized::General((map)(v)),
            Self::Localized(v) => MaybeLocalized::Localized(v.map(map)),
        }
    }
}

impl<T, E> MaybeLocalized<Result<T, E>> {
    pub fn into_err(self) -> Option<MaybeLocalized<E>> {
        match self {
            Self::General(v) => match v {
                Ok(..) => None,
                Err(e) => Some(MaybeLocalized::General(e)),
            },
            Self::Localized(v) => v.into_err().map(MaybeLocalized::from),
        }
    }

    pub fn into_ok(self) -> Option<MaybeLocalized<T>> {
        match self {
            Self::General(v) => match v {
                Err(..) => None,
                Ok(v) => Some(MaybeLocalized::General(v)),
            },
            Self::Localized(v) => v.into_ok().map(MaybeLocalized::from),
        }
    }
}

impl<V> Clone for MaybeLocalized<V>
where
    V: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::General(v) => Self::General(v.clone()),
            Self::Localized(v) => Self::Localized(v.clone()),
        }
    }
}

impl<V> From<Localized<V>> for MaybeLocalized<V> {
    fn from(value: Localized<V>) -> Self {
        Self::Localized(value)
    }
}

impl<V> From<V> for MaybeLocalized<V> {
    fn from(value: V) -> Self {
        Self::General(value)
    }
}

impl<T, E> Try for MaybeLocalized<Result<T, E>> {
    type Output = MaybeLocalized<T>;
    type Residual = MaybeLocalized<Result<Infallible, E>>;

    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            Self::General(v) => match v.branch() {
                ControlFlow::Continue(v) => ControlFlow::Continue(MaybeLocalized::General(v)),
                ControlFlow::Break(v) => ControlFlow::Break(MaybeLocalized::General(v)),
            },
            Self::Localized(v) => match v.branch() {
                ControlFlow::Continue(v) => ControlFlow::Continue(MaybeLocalized::Localized(v)),
                ControlFlow::Break(v) => ControlFlow::Break(MaybeLocalized::Localized(v)),
            },
        }
    }

    fn from_output(output: Self::Output) -> Self {
        match output {
            MaybeLocalized::General(v) => MaybeLocalized::General(Result::from_output(v)),
            MaybeLocalized::Localized(v) => MaybeLocalized::Localized(Localized::from_output(v)),
        }
    }
}

impl<T, E> FromResidual<MaybeLocalized<Result<Infallible, E>>> for MaybeLocalized<Result<T, E>> {
    fn from_residual(residual: MaybeLocalized<Result<Infallible, E>>) -> Self {
        match residual {
            MaybeLocalized::General(v) => Self::General(Result::from_residual(v)),
            MaybeLocalized::Localized(v) => Self::Localized(Localized::from_residual(v)),
        }
    }
}

impl<T, E> FromResidual<Localized<Result<Infallible, E>>> for MaybeLocalized<Result<T, E>> {
    fn from_residual(residual: Localized<Result<Infallible, E>>) -> Self {
        Self::Localized(Localized::from_residual(residual))
    }
}

impl<T, E> FromResidual<Result<Infallible, E>> for MaybeLocalized<Result<T, E>> {
    fn from_residual(residual: Result<Infallible, E>) -> Self {
        Self::General(Result::from_residual(residual))
    }
}

impl<V> Deref for MaybeLocalized<V> {
    type Target = V;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::General(v) => v,
            Self::Localized(v) => v,
        }
    }
}

impl<V> DerefMut for MaybeLocalized<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::General(v) => v,
            Self::Localized(v) => v,
        }
    }
}

impl<V> AsRef<V> for MaybeLocalized<V> {
    fn as_ref(&self) -> &V {
        match self {
            Self::General(v) => v,
            Self::Localized(v) => v,
        }
    }
}

impl<V> Display for MaybeLocalized<V>
where
    V: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::General(v) => Display::fmt(v, f),
            Self::Localized(v) => Display::fmt(v, f),
        }
    }
}

impl<V> Debug for MaybeLocalized<V>
where
    V: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::General(v) => Debug::fmt(v, f),
            Self::Localized(v) => Debug::fmt(v, f),
        }
    }
}
