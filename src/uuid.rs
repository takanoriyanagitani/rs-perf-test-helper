use core::fmt;

use tonic::Status;

use crate::rpc::perf::helper;
use helper::proto::common::v1::Uuid as Cuid;

#[derive(Clone, Copy)]
pub struct Uuid {
    raw: u128,
}

#[cfg(feature = "uv4")]
impl Uuid {
    pub fn new_v4() -> Self {
        let u: uuid::Uuid = uuid::Uuid::new_v4();
        let raw: u128 = u.as_u128();
        Self { raw }
    }

    fn new(hi: u64, lo: u64) -> Self {
        let h: u128 = hi.into();
        let l: u128 = lo.into();
        let raw: u128 = (h << 64) | l;
        Self { raw }
    }

    fn split(&self) -> (u64, u64) {
        let hi: u128 = self.raw >> 64;
        let lo: u128 = self.raw & 0xffff_ffff_ffff_ffff;
        (hi as u64, lo as u64)
    }
}

impl fmt::Display for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{:032x}", self.raw)
    }
}

pub trait UuidLike {
    fn hi(&self) -> u64;
    fn lo(&self) -> u64;
}

impl<U> From<U> for Uuid
where
    U: UuidLike,
{
    fn from(u: U) -> Self {
        let hi: u64 = u.hi();
        let lo: u64 = u.lo();
        Self::new(hi, lo)
    }
}

impl UuidLike for Cuid {
    fn hi(&self) -> u64 {
        self.hi
    }
    fn lo(&self) -> u64 {
        self.lo
    }
}

impl<U> TryFrom<Option<&U>> for Uuid
where
    U: UuidLike,
{
    type Error = Status;
    fn try_from(ou: Option<&U>) -> Result<Self, Self::Error> {
        let u: &U = ou.ok_or_else(|| Status::invalid_argument("uuid missing"))?;
        Ok(Self::new(u.hi(), u.lo()))
    }
}

impl From<Uuid> for Cuid {
    fn from(d: Uuid) -> Self {
        let (hi, lo) = d.split();
        Self { hi, lo }
    }
}
