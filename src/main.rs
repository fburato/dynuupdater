mod dynu;
mod netutils;
use clap::{Parser, Subcommand};
use core::fmt;
use std::{
    env::{self, VarError},
    error::Error,
    io,
};

use crate::dynu::RecordDTO;
use crate::SelfError::MsgError;
use dynu::{ClientError, DomainDTO, DomainsDTO, DynuClient};
use netutils::{ip, public_ip_of, IP};

const API_KEY_NAME: &str = "DYNU_API_KEY";

#[derive(Debug)]
enum SelfError {
    MsgError(String),
    ClientError(ClientError),
    IOError(io::Error),
}

impl Error for SelfError {}

impl fmt::Display for SelfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClientError(req) => write!(f, "ClientError({})", req),
            Self::MsgError(msg) => write!(f, "MsgError({})", msg),
            Self::IOError(io_err) => write!(f, "IOError({})", io_err),
        }
    }
}

impl From<io::Error> for SelfError {
    fn from(value: io::Error) -> Self {
        Self::IOError(value)
    }
}

impl From<ClientError> for SelfError {
    fn from(value: ClientError) -> Self {
        Self::ClientError(value)
    }
}

impl From<VarError> for SelfError {
    fn from(value: VarError) -> Self {
        Self::MsgError(format!("{}", value))
    }
}

#[derive(Parser, Debug)]
#[command(name = "dynupdater")]
#[command(about = "Interface with dynu to manipulate entries")]
struct MainArguments {
    #[arg(
        long,
        help = "API KEY for dynu, used with priority over the DYNU_API_KEY environment variable"
    )]
    api_key: Option<String>,
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(
        about = "Update a dynu domain using the public ip of the system running the process"
    )]
    Refresh {
        #[arg(help = "Domain to update")]
        domain: String,
    },

    #[command(about = "Update or create a dynu domain TXT record with provided value")]
    #[command(name = "txt-update")]
    UpdateTxtRecord {
        #[arg(long, help = "DNS record key to update")]
        name: String,
        #[arg(long, help = "TTL for the record entry", default_value = "120")]
        ttl: u64,
        #[arg(long, help = "DNS record value to update")]
        value: String,
        #[arg(help = "Domain to update")]
        domain: String,
    },

    #[command(about = "Delete a dynu domain TXT record")]
    #[command(name = "txt-delete")]
    DeleteTxtRecord {
        #[arg(help = "Domain to update")]
        domain: String,
        #[arg(help = "DNS record key to delete")]
        name: String,
    },
}

fn get_api_key(args: &MainArguments) -> Result<String, SelfError> {
    match &args.api_key {
        Some(value) => Ok(value.clone()),
        None => env::var(API_KEY_NAME).map_err(|_| {
            SelfError::MsgError(format!(
                "provide 'api-key' argument or define environment variable {}",
                API_KEY_NAME
            ))
        }),
    }
}

fn or_empty(option: &Option<String>) -> String {
    option
        .as_ref()
        .map(|s| s.to_string())
        .unwrap_or("".to_string())
}

fn find_domain_id(dynu_client: &DynuClient, domain: &str) -> Result<DomainDTO, SelfError> {
    let body: DomainsDTO = dynu_client.get_domains()?;
    let maybe_domain = body.domains.into_iter().find(|d| d.name == domain);
    if maybe_domain.is_none() {
        return Err(SelfError::MsgError(format!(
            "domain={} cannot be found in dynu",
            domain
        )));
    }
    Ok(maybe_domain.unwrap())
}

fn refresh(dynu_client: DynuClient, domain: &str) -> Result<(), SelfError> {
    let ipv4 = ip(IP::V4);
    let ipv6 = ip(IP::V6);
    eprintln!(
        "detected ipv4='{}', ipv6='{}'",
        or_empty(&ipv4),
        or_empty(&ipv6)
    );
    let resolved = public_ip_of(domain)?;
    eprintln!(
        "domain={}, resolved ipv4={}, resolved ipv6={}",
        domain,
        or_empty(&resolved.v4),
        or_empty(&resolved.v6)
    );
    if resolved.v4 == ipv4 && resolved.v6 == ipv6 {
        eprintln!("ips resolved(v4={}, v6={}) are identical to the current ones(v4={}, v6={}), not updating domain={}",
                  or_empty(&resolved.v4), or_empty(&resolved.v6), or_empty(&ipv4), or_empty(&ipv6), domain);
        return Ok(());
    }
    eprintln!("ips resolved(v4={}, v6={}) are different from the registered ones(v4={}, v6={}), updating the record for domain={}",
              or_empty(&resolved.v4), or_empty(&resolved.v6), or_empty(&ipv4), or_empty(&ipv6), domain);
    let mut domain_dto = find_domain_id(&dynu_client, domain)?;
    eprintln!("{:?}", &domain_dto);
    domain_dto.ipv4 = ipv4.is_some();
    domain_dto.ipv6 = ipv6.is_some();
    domain_dto.ipv4_address = ipv4;
    domain_dto.ipv6_address = ipv6;
    dynu_client.update_domain(&domain_dto)?;
    let result = dynu_client.get_domain(domain_dto.id.unwrap())?;
    eprintln!("updated domain={:?}", &result);
    Ok(())
}

fn txt_update(
    dynu_client: DynuClient,
    domain: &str,
    name: &str,
    value: &str,
    ttl: u64,
) -> Result<(), SelfError> {
    let domain = find_domain_id(&dynu_client, domain)?;
    let domain_id = domain.id.unwrap();
    let records = dynu_client.get_records(domain_id)?;
    let maybe_existing_record = records.dns_records.iter().find(|r| match r {
        RecordDTO::TxtRecord { node_name, .. } => node_name == name,
        _ => false,
    });
    if maybe_existing_record.is_none() {
        eprintln!("{} record does not exist, defining a new one now", name);
        let txt_record = RecordDTO::txt_record(name, value, ttl, None);
        let id = dynu_client.create_record(domain_id, &txt_record)?;
        eprintln!("created record with id={}", id);
    } else {
        let record_id = maybe_existing_record.unwrap().id().unwrap();
        eprintln!(
            "{} record already exists with id={}, updating it",
            name, record_id
        );
        let txt_record = RecordDTO::txt_record(name, value, ttl, Some(record_id));
        dynu_client.update_record(domain_id, &txt_record)?;
        eprintln!("{} record updated", record_id)
    }
    Ok(())
}

fn txt_delete(dynu_client: DynuClient, domain_name: &str, name: &str) -> Result<(), SelfError> {
    let domain = find_domain_id(&dynu_client, domain_name)?;
    let domain_id = domain.id.unwrap();
    let records = dynu_client.get_records(domain_id)?;
    let maybe_existing_record = records.dns_records.iter().find(|r| match r {
        RecordDTO::TxtRecord { node_name, .. } => node_name == name,
        _ => false,
    });
    if maybe_existing_record.is_none() {
        return Err(MsgError(format!(
            "{} in domain {} does not exist",
            name, domain_name
        )));
    }
    let existing_record = maybe_existing_record.unwrap();
    dynu_client.delete_record(domain_id, existing_record.id().unwrap())?;
    eprintln!("{} in domain {} deleted", name, domain_name);
    Ok(dynu_client.delete_record(domain_id, existing_record.id().unwrap())?)
}

fn main() -> Result<(), SelfError> {
    let arguments = MainArguments::parse();
    let api_key = get_api_key(&arguments)?;
    let dynu_client = DynuClient::new(&api_key)?;
    match arguments.cmd {
        Commands::Refresh { domain } => refresh(dynu_client, &domain),
        Commands::UpdateTxtRecord {
            ttl,
            name,
            value,
            domain,
        } => txt_update(dynu_client, &domain, &name, &value, ttl),
        Commands::DeleteTxtRecord { domain, name } => txt_delete(dynu_client, &domain, &name),
    }
}
