use crate::runtime::AsyncId;
use crate::{runtime::Runtime, types::JValue};

#[derive(Clone)]
pub enum Promise {
    Pending { id: AsyncId },
    Fulfilled(JValue),
    Rejected(JValue),
}

impl Promise {}
