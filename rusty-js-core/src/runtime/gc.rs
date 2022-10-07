#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GcFlag {
    Used,
    Old,
    NotUsed,
    Garbage,
}
