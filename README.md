[![Tests Workflow Status (main)](https://img.shields.io/github/actions/workflow/status/lostatc/sqlarfs-rs/test.yaml?branch=main&label=Tests&style=for-the-badge&logo=github)](https://github.com/lostatc/sqlarfs-rs/actions/workflows/test.yaml)
[![Codecov](https://img.shields.io/codecov/c/github/lostatc/sqlarfs-rs?logo=codecov&style=for-the-badge)](https://app.codecov.io/gh/lostatc/sqlarfs-rs)
[![Crates.io](https://img.shields.io/crates/v/sqlarfs?logo=rust&style=for-the-badge)](https://crates.io/crates/sqlarfs)
[![docs.rs](https://img.shields.io/docsrs/sqlarfs?logo=docs.rs&style=for-the-badge)](https://docs.rs/sqlarfs)

# sqlarfs

🚧 This library is under construction. It's not ready to be used just yet. 🚧

A file archive format and virtual filesystem backed by a SQLite database.

This library is a Rust implementation of the
[sqlar](https://sqlite.org/sqlar.html) format for SQLite archive files.

This library consists of:

- A Rust API
- A CLI
- A FUSE filesystem

## Rust API

To add this library to your project:

```shell
cargo add sqlarfs
```

See the [API docs](https://docs.rs/sqlarfs) for documentation and examples.

## CLI

### Installation

To install the CLI tool, [install
Rust](https://www.rust-lang.org/tools/install) and run:

```shell
cargo install sqlarfs-cli
```

The binary will be installed to `~/.cargo/bin/sqlar`.

### Examples

Archive directory and extract it to a target directory:

```shell
sqlar create ./src
sqlar extract --archive src.sqlar ~/Desktop
```

Archive two directories and extract them to the current directory:

```shell
sqlar create --archive files.sqlar ~/Documents ~/Pictures
sqlar extract --archive files.sqlar
```

Extract a specific file from an archive:

```shell
sqlar create --archive documents.sqlar ~/Documents
sqlar extract --archive documents.sqlar --source Documents/report.pdf
```

Add a file to an existing archive.

```shell
sqlar create --archive documents.sqlar ~/Documents
sqlar archive --archive documents.sqlar ~/Downloads/report.pdf Documents/report.pdf
```

List all regular files in an archive:

```shell
sqlar create --archive documents.sqlar ~/Documents
sqlar list --archive documents.sqlar --type file
```

List only the immediate children of a specific directory in an archive:

```shell
sqlar create --archive documents.sqlar ~/Documents
sqlar list --archive documents.sqlar --children Documents/Reports/
```

Remove a file from an archive:

```shell
sqlar create --archive documents.sqlar ~/Documents
sqlar remove --archive documents.sqlar Documents/report.pdf
```

The tool has a shorthand syntax for each command:

```
sqlar c -a files.sqlar ~/Documents ~/Pictures
sqlar ex -a files.sqlar
sqlar ls -a files.sqlar
sqlar rm -a files.sqlar Documents
```

## FUSE Filesystem

You can mount the archive as a FUSE filesystem using the CLI:

```shell
sqlar create --archive documents.sqlar ~/Documents
sqlar mount --archive documents.sqlar ./mnt
```

You can also mount the archive using the Rust API:

```rust
use sqlarfs::Connection;

fn main() -> sqlarfs::Result<()> {
    let mut conn = Connection::open_in_memory()?;

    conn.exec(|archive| {
        let mut dir = archive.open("path/to")?;
        dir.create_dir_all()?;

        let mut file = archive.open("path/to/file")?;
        file.create_file()?;

        // This blocks until the filesystem is unmounted.
        archive.mount("", "/home/wren/mnt")?;

        sqlarfs::Result::Ok(())
    })?;

    Ok(())
}
```
