use sqlarfs::{Connection, TransactionBehavior};
use xpct::{be_false, be_ok, be_true, expect};

fn test_transaction_commits_successfully(
    conn: &mut Connection,
    behavior: TransactionBehavior,
) -> sqlarfs::Result<()> {
    let mut tx = conn.transaction_with(behavior)?;

    tx.archive_mut().open("file")?.create_file()?;

    tx.commit()?;

    conn.exec(|archive| {
        expect!(archive.open("file")?.exists())
            .to(be_ok())
            .to(be_true());

        Ok(())
    })
}

fn test_transaction_rolls_back_successfully(
    conn: &mut Connection,
    behavior: TransactionBehavior,
) -> sqlarfs::Result<()> {
    let mut tx = conn.transaction_with(behavior)?;

    tx.archive_mut().open("file")?.create_file()?;

    tx.rollback()?;

    conn.exec(|archive| {
        let file = archive.open("file")?;
        expect!(file.exists()).to(be_ok()).to(be_false());

        Ok(())
    })
}

fn test_exec_commits_successfully(
    conn: &mut Connection,
    behavior: TransactionBehavior,
) -> sqlarfs::Result<()> {
    conn.exec_with(behavior, |archive| archive.open("file")?.create_file())?;

    conn.exec(|archive| {
        expect!(archive.open("file")?.exists())
            .to(be_ok())
            .to(be_true());

        Ok(())
    })
}

//
// `Connection::transaction_with`
//

#[test]
fn transaction_with_deferred_and_commit() -> sqlarfs::Result<()> {
    let mut conn = Connection::open_in_memory()?;

    test_transaction_commits_successfully(&mut conn, TransactionBehavior::Deferred)
}

#[test]
fn transaction_with_immediate_and_commit() -> sqlarfs::Result<()> {
    let mut conn = Connection::open_in_memory()?;

    test_transaction_commits_successfully(&mut conn, TransactionBehavior::Immediate)
}

#[test]
fn transaction_with_exclusive_and_commit() -> sqlarfs::Result<()> {
    let mut conn = Connection::open_in_memory()?;

    test_transaction_commits_successfully(&mut conn, TransactionBehavior::Exclusive)
}

#[test]
fn transaction_with_deferred_and_rollback() -> sqlarfs::Result<()> {
    let mut conn = Connection::open_in_memory()?;

    test_transaction_rolls_back_successfully(&mut conn, TransactionBehavior::Deferred)
}

#[test]
fn transaction_with_immediate_and_rollback() -> sqlarfs::Result<()> {
    let mut conn = Connection::open_in_memory()?;

    test_transaction_rolls_back_successfully(&mut conn, TransactionBehavior::Immediate)
}

#[test]
fn transaction_with_exclusive_and_rollback() -> sqlarfs::Result<()> {
    let mut conn = Connection::open_in_memory()?;

    test_transaction_rolls_back_successfully(&mut conn, TransactionBehavior::Exclusive)
}

//
// `Connection::exec_with`
//

#[test]
fn exec_with_deferred_and_commit() -> sqlarfs::Result<()> {
    let mut conn = Connection::open_in_memory()?;

    test_exec_commits_successfully(&mut conn, TransactionBehavior::Deferred)
}

#[test]
fn exec_with_immediate_and_commit() -> sqlarfs::Result<()> {
    let mut conn = Connection::open_in_memory()?;

    test_exec_commits_successfully(&mut conn, TransactionBehavior::Immediate)
}

#[test]
fn exec_with_exclusive_and_commit() -> sqlarfs::Result<()> {
    let mut conn = Connection::open_in_memory()?;

    test_exec_commits_successfully(&mut conn, TransactionBehavior::Exclusive)
}
