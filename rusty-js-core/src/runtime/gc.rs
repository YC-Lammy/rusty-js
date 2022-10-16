#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GcFlag {
    Used,
    Old,
    NotUsed,
    /// a flag only used by the finalization registry
    Finalize,
    Garbage,
}
