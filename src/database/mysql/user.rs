use crate::database::mysql::Transaction;
use crate::entity::DatabaseTransaction;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
impl DatabaseTransaction for Transaction<'_> {
    async fn remove_user(&self, id: u64) -> Result<()> {
        log::debug!("remove_user: id = {id}");
        Ok(())
    }
}
