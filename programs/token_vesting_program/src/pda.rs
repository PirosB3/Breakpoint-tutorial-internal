
pub struct Seeds(Vec<Vec<u8>>);

pub trait ToSeeds {
    fn get_seeds(&self) -> Vec<&[u8]>;
}

impl ToSeeds for Seeds {
    fn get_seeds(&self) -> Vec<&[u8]> {
        let Self(items) = &self;
        items.into_iter().map(|i| i.as_slice()).collect()
    }
}