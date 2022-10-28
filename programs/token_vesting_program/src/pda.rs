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

#[macro_export]
macro_rules! grant_custody_seeds {
    ($ctx:expr) => {
        &[&[
            b"grant_custody",
            $ctx.accounts.employer.key().as_ref(),
            $ctx.accounts.employee.key().as_ref(),
            &[$ctx.accounts.grant_account.grant_custody_bump],
        ]]
    };
}
