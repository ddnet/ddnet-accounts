use anyhow::anyhow;
use axum::async_trait;
use ddnet_account_sql::query::Query;
use sqlx::Executor;
use sqlx::Row;
use sqlx::Statement;

pub struct AddCert<'a> {
    pub cert_der: &'a [u8],
    pub valid_until: &'a sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
}

#[async_trait]
impl Query<()> for AddCert<'_> {
    async fn prepare_mysql(
        connection: &mut sqlx::mysql::MySqlConnection,
    ) -> anyhow::Result<sqlx::mysql::MySqlStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/add_cert.sql"))
            .await?)
    }
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::mysql::MySqlStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::MySql, sqlx::mysql::MySqlArguments> {
        statement.query().bind(self.cert_der).bind(self.valid_until)
    }
    fn row_data_mysql(_row: &sqlx::mysql::MySqlRow) -> anyhow::Result<()> {
        Err(anyhow!("Row data is not supported"))
    }
}

pub struct GetCerts {}

pub struct SingleCertData {
    pub cert_der: Vec<u8>,
}

#[async_trait]
impl Query<SingleCertData> for GetCerts {
    async fn prepare_mysql(
        connection: &mut sqlx::mysql::MySqlConnection,
    ) -> anyhow::Result<sqlx::mysql::MySqlStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/get_certs.sql"))
            .await?)
    }
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::mysql::MySqlStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::MySql, sqlx::mysql::MySqlArguments> {
        statement.query()
    }
    fn row_data_mysql(row: &sqlx::mysql::MySqlRow) -> anyhow::Result<SingleCertData> {
        Ok(SingleCertData {
            cert_der: row
                .try_get("cert_der")
                .map_err(|err| anyhow!("Failed get column cert_der: {err}"))?,
        })
    }
}
