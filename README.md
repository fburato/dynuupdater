# dynuupdater

`dynuupdater` is a command line utility for interfacing with Dynamic DNS provider [Dynu](https://www.dynu.com/)'s API to update DNS domain entries and records.

The command line script uses [Ipify](https://www.ipify.org/)'s API as well to determine the host public IP address.

It's purpose is primarily to serve as a background automation task to be executed regularly to refresh the DNS from a ISP provided dynamic IP.

TXT entries manipulation is also included to help the maintenance to handle DNS verification with [Let's encrypt](https://letsencrypt.org/).

## Build

The application is developed in Rust 1.80. Check the [Rust](https://www.rust-lang.org/) home page to install the compiler and cargo.

To build, run:

```bash
cargo build
# for the release build
cargo build --release
```

## Usage

`dynuupdater` authenticates against Dynu's API with an API key, which must be provided either with the environment variable `DYNU_API_KEY` or the `--api-key` command line argument.

It contains three, subcommands:

- `refresh`: resolves the public IP of the host running the application and stores it as a domain in Dynu. In order to reduce interactions with Dynu, updates are not executed if the first entry for the IP resolved for the domain matches the current public ip.
- `txt-update`: sets a TXT entry for a certain domain in Dynu.
- `txt-delete`: deletes a TXT entry for a certain domain in Dynu.

### Global help
```bash
$ dynuupdater -h
Interface with dynu to manipulate entries

Usage: dynuupdater [OPTIONS] <COMMAND>

Commands:
  refresh     Update a dynu domain using the public ip of the system running the process
  txt-update  Update or create a dynu domain TXT record with provided value
  txt-delete  Delete a dynu domain TXT record
  help        Print this message or the help of the given subcommand(s)

Options:
      --api-key <API_KEY>  API KEY for dynu, used with priority over the DYNU_API_KEY environment variable
  -h, --help               Print help
```

### `refresh` help

```bash
$ dynuupdater refresh -h
Update a dynu domain using the public ip of the system running the process

Usage: dynuupdater refresh <DOMAIN>

Arguments:
  <DOMAIN>  Domain to update

Options:
  -h, --help  Print help
```

### `txt-update` help

```bash
$ dynuupdater txt-update -h
Update or create a dynu domain TXT record with provided value

Usage: dynuupdater txt-update [OPTIONS] --name <NAME> --value <VALUE> <DOMAIN>

Arguments:
  <DOMAIN>  Domain to update

Options:
      --name <NAME>    DNS record key to update
      --ttl <TTL>      TTL for the record entry [default: 120]
      --value <VALUE>  DNS record value to update
  -h, --help           Print help
```

### `txt-delete` help

```bash
$ dynuupdater txt-delete -h
Delete a dynu domain TXT record

Usage: dynuupdater txt-delete <DOMAIN> <NAME>

Arguments:
  <DOMAIN>  Domain to update
  <NAME>    DNS record key to delete

Options:
  -h, --help  Print help
```

## Docker builds

There are two `Dockerfile`s provided as well:

* `Dockerfile`: builds an ubuntu base image with only `dynuupdater`.
* `Dockerfile.supercronic`: builds an ubuntu base image with `dynuupdater` and [`supercronic`](https://github.com/aptible/supercronic).
