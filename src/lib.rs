
mod efs;
mod ondisk;
mod types;
pub use types::Result;
pub use crate::efs::Efs;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
