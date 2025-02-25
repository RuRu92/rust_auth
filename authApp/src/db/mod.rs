use std::io::empty;
use mysql::{AccessMode, Error, MySqlError, Pool, PooledConn, Result, Transaction, TxOpts};
use std::ops::FnOnce;
use std::sync::Arc;
use std::thread::sleep;
use chrono::Duration;
use mysql::AccessMode::{ReadOnly, ReadWrite};
use mysql::prelude::Queryable;
use crate::app::{APIError, APIResult};

pub struct DB {
    pub pool: Pool,
}

type SQLError = mysql::Error;

impl DB {
    pub fn init(url: &str) -> DB {
        let pool = Pool::new(url).expect("BAAAH - DB Crapped !");
        DB { pool }
    }

    pub fn in_transaction<R, F>(
        &self,
        db_access_mode: AccessMode,
        mut action: F) -> APIResult<R>
    where
        F: FnMut(&mut Transaction) -> APIResult<R, SQLError>,
    {
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
                        return self.in_transaction_with_retry(db_access_mode, action, 3, 5)
                            .map_err(|err| APIError::DBException(err));
                    };
                    tx.rollback().expect("Failed to rollback");
                }
                Err(APIError::DBException(sqL_err))
            }
        }
    }

    fn in_transaction_with_retry<R, F>(
        &self,
        db_access_mode: AccessMode,
        mut action: F,
        mut retries: usize,
        max_retries: usize,
    ) -> APIResult<R, Error>
    where
        F: FnMut(&mut Transaction) -> APIResult<R, Error>,
    {
        let mut pool = self.pool.clone();
        loop {
            let mut db_conn = pool.get_conn().expect("Error establishing DB connection");
            let mut tx: Transaction = db_conn.start_transaction(TxOpts::default().set_access_mode(Some(db_access_mode)))
                .expect("Failed to initialize transaction");

            match action(&mut tx) {
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
                        return Err(err);
                    }
                }
            }
        }
    }
}

pub struct ExecutionContext {
    pub db: Arc<DB>,
}
