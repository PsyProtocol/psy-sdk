#[derive(Clone, Debug, PartialEq)]
pub enum StorageNode<F> {
    Read(F),
    Write(F, F),
}
