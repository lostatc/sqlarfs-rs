use sqlarfs::{Connection, ErrorKind};
use xpct::{be_err, equal, expect};

fn connection() -> sqlarfs::Result<Connection> {
    Connection::open_in_memory()
}

#[test]
fn opening_file_with_absolute_path_errors() -> sqlarfs::Result<()> {
    connection()?.exec(|archive| {
        expect!(archive.open("/path/to/file"))
            .to(be_err())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::PathIsAbsolute));

        Ok(())
    })
}
