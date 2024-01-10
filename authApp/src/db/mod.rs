use mysql::{AccessMode, Pool, Result, Transaction, TxOpts};
use std::ops::FnOnce;
use std::sync::Arc;

pub struct DB {
    pub pool: Pool,
}

impl DB {
    pub fn init(url: &str) -> DB {
        let pool = Pool::new(url).expect("BAAAH - DB Crapped !");
        return DB { pool };
    }

    pub fn in_transaction<R>(
        &self,
        db_access_mode: AccessMode,
        action: impl FnOnce(&mut Transaction) -> Result<R>,
    ) -> R {
        let pool = self.pool.clone();
        let mut db_conn = pool.get_conn().expect("Unable to establish connection");

        let mut tx: Transaction = db_conn
            .start_transaction(TxOpts::default().set_access_mode(Some(db_access_mode)))
            .expect("Failed to initialise transaction");

        match action(&mut tx) {
            Ok(res) => {
                tx.commit().expect("Failed to commit");
                return res;
            }
            Err(x) => {
                if db_access_mode != AccessMode::ReadOnly {
                    tx.rollback().expect("Failed to rollback");
                }
                panic!("{}", x);
            }
        }
    }
}

pub struct ExecutionContext {
    pub db: Arc<DB>,
}
