use std::io::empty;
use mysql::{AccessMode, Error, MySqlError, Pool, PooledConn, Result, Transaction, TxOpts};
use std::ops::FnOnce;
use std::sync::Arc;
use std::thread::sleep;
use chrono::Duration;
use mysql::AccessMode::{ReadOnly, ReadWrite};
use crate::app::{APIError, APIResult};

pub struct DB {
    pub pool: Pool,
}

type SQLError = mysql::Error;
type TransactionAction<R> = impl FnOnce(&mut Transaction) -> APIResult<R, mysql::Error>;

impl DB {
    pub fn init(url: &str) -> DB {
        let pool = Pool::new(url).expect("BAAAH - DB Crapped !");
        DB { pool }
    }

    pub fn in_transaction<R>(
        &self,
        db_access_mode: AccessMode,
        action: TransactionAction<R>) -> APIResult<R> {
        let pool = self.pool.clone();
        let mut db_conn = pool.get_conn().expect("Unable to establish connection");

        let mut tx: Transaction = db_conn
            .start_transaction(TxOpts::default().set_access_mode(Some(db_access_mode)))
            .expect("Failed to initialise transaction");

        match action(&mut tx) {
            Ok(res) => {
                if db_access_mode == ReadOnly {
                    return Ok(res);
                } else {
                    tx.commit().expect("Failed to commit");
                }
                Ok(res)
            }
            Err(sqL_err) => {
                if db_access_mode != ReadOnly {
                    // add logging
                    if let Error::MySqlError(err) = sqL_err {
                        return self.in_transaction_with_retry(&mut db_conn, db_access_mode, action, 3, 5)
                            .map_err(|err| APIError::DBException(err));
                    };
                    tx.rollback().map_err(|err| APIError::DBException(err))
                } else {
                    Err(APIError::DBException(sqL_err))
                }
            }
        }
    }

    fn in_transaction_with_retry<R>(
        &self,
        db_conn: &mut PooledConn,
        db_access_mode: AccessMode,
        mut action: impl FnOnce(&mut Transaction) -> APIResult<R, Error>,
        max_retries: usize,
    ) -> APIResult<R, Error> {
        let mut retries = 0;

        loop {
            let mut tx: Transaction = db_conn
                .start_transaction(TxOpts::default().set_access_mode(Some(db_access_mode)))
                .expect("Failed to initialize transaction");

            match &action(&mut tx) {
                Ok(res) => {
                    if db_access_mode == ReadWrite {
                        tx.commit().expect("Failed to commit");
                    }
                    return Ok(res);
                }
                Err(err) => {
                    if retries < max_retries {
                        retries += 1;
                        sleep(core::time::Duration::from_millis(200));
                    } else {
                        return Err(*err);
                    }
                }
            }
        }
    }
}

pub struct ExecutionContext {
    pub db: Arc<DB>,
}
