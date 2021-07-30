
mod efs;
mod ondisk;
mod types;
pub use types::Result;
pub use crate::efs::Efs;
pub use crate::efs::ProcessorGeneration;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
