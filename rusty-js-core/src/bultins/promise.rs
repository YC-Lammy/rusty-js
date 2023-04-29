use crate::runtime::AsyncId;
use crate::value::JValue;

#[derive(Clone)]
pub enum Promise {
    Pending { id: AsyncId },
    Fulfilled(JValue),
    Rejected(JValue),
    ForeverPending,
}
