use crate::runtime::AsyncId;
use crate::types::JValue;

#[derive(Clone)]
pub enum Promise {
    Pending { id: AsyncId },
    Fulfilled(JValue),
    Rejected(JValue),
}