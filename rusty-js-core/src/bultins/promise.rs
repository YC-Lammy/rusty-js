
use crate::{types::JValue, runtime::Runtime};
use crate::runtime::AsyncId;

#[derive(Clone)]
pub enum Promise{
    Pending{
        id:AsyncId,
    },
    Fulfilled(JValue),
    Rejected(JValue)
}

impl Promise{

}