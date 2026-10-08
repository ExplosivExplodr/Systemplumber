use crate::root::Root;

#[derive(Debug)]
pub struct BootstrapSystem {
    pub root: Root,
}

impl BootstrapSystem {
    pub fn new(root: Root) -> Self {
        Self { root }
    }

    pub fn initialize(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(self.root.etc())?;
        std::fs::create_dir_all(self.root.usr())?;
        std::fs::create_dir_all(self.root.var())?;
        std::fs::create_dir_all(self.root.boot())?;
        std::fs::create_dir_all(self.root.package_db())?;

        Ok(())
    }
}
