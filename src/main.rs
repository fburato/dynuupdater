mod dynu;
mod netutils;
use clap::Parser;
use core::fmt;
use std::{
    env::{self, VarError},
    error::Error,
    io,
};

use dynu::{ClientError, DomainsDTO, DynuClient};
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
#[command(about = "Update a dynu domain using the public ip")]
struct MainArguments {
    #[arg(
        long,
        help = "API KEY for dynu, used with priority over the DYNU_API_KEY environment variable"
    )]
    api_key: Option<String>,
    domain: String,
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

fn main() -> Result<(), SelfError> {
    let arguments = MainArguments::parse();
    let api_key = get_api_key(&arguments)?;
    let dynu_client = DynuClient::new(&api_key)?;
    let ipv4 = ip(IP::V4);
    let ipv6 = ip(IP::V6);
    eprintln!(
        "detected ipv4='{}', ipv6='{}'",
        or_empty(&ipv4),
        or_empty(&ipv6)
    );
    let resolved = public_ip_of(&arguments.domain)?;
    eprintln!(
        "domain={}, resolved ipv4={}, resolved ipv6={}",
        &arguments.domain,
        or_empty(&resolved.v4),
        or_empty(&resolved.v6)
    );
    if resolved.v4 == ipv4 && resolved.v6 == ipv6 {
        eprintln!("ips resolved(v4={}, v6={}) are identical to the current ones(v4={}, v6={}), not updating domain={}",
            or_empty(&resolved.v4), or_empty(&resolved.v6), or_empty(&ipv4), or_empty(&ipv6), arguments.domain);
        return Ok(());
    }
    eprintln!("ips resolved(v4={}, v6={}) are different from the registered ones(v4={}, v6={}), updating the record for domain={}",
            or_empty(&resolved.v4), or_empty(&resolved.v6), or_empty(&ipv4), or_empty(&ipv6), arguments.domain);
    let body: DomainsDTO = dynu_client.get_domains()?;
    let domain_dto = body
        .domains
        .into_iter()
        .find(|domain| domain.name == arguments.domain);
    if domain_dto.is_none() {
        return Err(SelfError::MsgError(format!(
            "domain={} cannot be found in dynu",
            arguments.domain
        )));
    }
    let mut domain_dto = domain_dto.unwrap();
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
