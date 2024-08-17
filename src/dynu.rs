use reqwest::{
    self,
    blocking::Response,
    header::{HeaderMap, ACCEPT, CONTENT_TYPE},
    Error as ReqError,
};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;

const DYNU_API: &str = "https://api.dynu.com";

#[derive(Debug)]
pub enum ClientError {
    MsgError(String),
    HttpError(ReqError),
}

impl Error for ClientError {}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HttpError(req) => write!(f, "HttpError({})", req),
            Self::MsgError(msg) => write!(f, "MsgError({})", msg),
        }
    }
}

impl From<ReqError> for ClientError {
    fn from(value: ReqError) -> Self {
        Self::HttpError(value)
    }
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DomainDTO {
    pub id: Option<u64>,
    pub name: String,
    pub unicode_name: String,
    pub token: Option<String>,
    pub state: String,
    pub group: String,
    pub ipv4_address: Option<String>,
    pub ipv6_address: Option<String>,
    pub ttl: u64,
    pub ipv4: bool,
    pub ipv6: bool,
    pub ipv4_wildcard_alias: bool,
    pub ipv6_wildcard_alias: bool,
    pub created_on: Option<String>,
    pub updated_on: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DomainsDTO {
    pub status_code: u32,
    pub domains: Vec<DomainDTO>,
}

pub struct DynuClient {
    _client: reqwest::blocking::Client,
    _api_key: String,
}

impl DynuClient {
    pub fn new(api_key: &str) -> DynuClient {
        let client = reqwest::blocking::Client::new();
        DynuClient {
            _client: client,
            _api_key: api_key.to_string(),
        }
    }

    pub fn get_domains(&self) -> Result<DomainsDTO, ClientError> {
        let url = format!("{}/v2/dns", DYNU_API.to_string());
        let response: Response = self._client.get(&url).headers(self._headers()).send()?;
        let status = response.status();
        if status.is_success() {
            let result: DomainsDTO = response.json()?;
            Ok(result)
        } else {
            Err(ClientError::MsgError(format!(
                "GET {}, status_code={}, body={}",
                &url,
                status.as_str(),
                response.text()?
            )))
        }
    }

    pub fn get_domain(&self, id: u64) -> Result<Option<DomainDTO>, ClientError> {
        let url = format!("{}/v2/dns/{}", DYNU_API.to_string(), id);
        let response: Response = self._client.get(&url).headers(self._headers()).send()?;
        let status = response.status();
        if status.is_success() {
            let result: DomainDTO = response.json()?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    pub fn update_domain(&self, id: u64, domain_dto: &DomainDTO) -> Result<(), ClientError> {
        let mut headers = self._headers();
        headers.append(CONTENT_TYPE, "application/json".parse().unwrap());
        let url = format!("{}/v2/dns/{}", DYNU_API, id);
        let result: Response = self
            ._client
            .post(&url)
            .headers(headers)
            .json(domain_dto)
            .send()?;
        let status = result.status();
        if status.is_success() {
            Ok(())
        } else {
            let text = result.text()?;
            Err(ClientError::MsgError(format!(
                "POST {} failed, error_code={}, body={}",
                url,
                status.as_str(),
                text
            )))
        }
    }

    fn _headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, "application/json".parse().unwrap());
        headers.insert("api-key", self._api_key.parse().unwrap());
        headers
    }
}
