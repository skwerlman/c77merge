#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AcfToken {
    String(String),
    DictStart,
    DictEnd,
}
