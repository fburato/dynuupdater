use reqwest::header::InvalidHeaderValue;
use reqwest::{
    self,
    blocking::Response,
    header::{HeaderMap, ACCEPT, CONTENT_TYPE},
    Error as ReqError, StatusCode,
};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;

const DYNU_API: &str = "https://api.dynu.com";

#[derive(Debug)]
pub enum ClientError {
    MsgError(String),
    HttpError(ReqError),
    HeaderValueError(InvalidHeaderValue),
}

impl Error for ClientError {}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HttpError(req) => write!(f, "HttpError({})", req),
            Self::MsgError(msg) => write!(f, "MsgError({})", msg),
            Self::HeaderValueError(req) => write!(f, "InvalidHeaderValue({})", req),
        }
    }
}

impl From<ReqError> for ClientError {
    fn from(value: ReqError) -> Self {
        Self::HttpError(value)
    }
}

impl From<InvalidHeaderValue> for ClientError {
    fn from(value: InvalidHeaderValue) -> Self {
        Self::HeaderValueError(value)
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

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "recordType")]
pub enum RecordDTO {
    #[serde(rename = "TXT", rename_all = "camelCase")]
    TxtRecord {
        id: Option<u64>,
        domain_id: Option<u64>,
        domain_name: Option<String>,
        node_name: String,
        hostname: Option<String>,
        ttl: u64,
        state: bool,
        content: Option<String>,
        updated_on: Option<String>,
        text_data: String,
    },
    #[serde(rename = "SOA", rename_all = "camelCase")]
    SoaRecord {
        id: Option<u64>,
        domain_id: Option<u64>,
        domain_name: Option<String>,
        node_name: String,
        hostname: Option<String>,
        ttl: u64,
        state: bool,
        content: Option<String>,
        updated_on: String,
        master_name: String,
        responsible_name: String,
        refresh: u64,
        retry: u64,
        expire: u64,
        #[serde(rename = "negativeTTL")]
        negative_ttl: u64,
    },
    #[serde(rename = "A", rename_all = "camelCase")]
    ARecord {
        id: Option<u64>,
        domain_id: Option<u64>,
        domain_name: Option<String>,
        node_name: String,
        hostname: Option<String>,
        ttl: u64,
        state: bool,
        content: Option<String>,
        updated_on: Option<String>,
        group: String,
    },
}

impl RecordDTO {
    fn txt_record(node_name: &str, text_data: &str, ttl: u64) -> RecordDTO {
        RecordDTO::TxtRecord {
            id: None,
            domain_id: None,
            domain_name: None,
            node_name: node_name.to_string(),
            hostname: None,
            ttl,
            state: true,
            content: None,
            updated_on: None,
            text_data: text_data.to_string(),
        }
    }
    fn id(&self) -> Option<u64> {
        match self {
            RecordDTO::ARecord { id, .. } => id.clone(),
            RecordDTO::SoaRecord { id, .. } => id.clone(),
            RecordDTO::TxtRecord { id, .. } => id.clone(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RecordsDTO {
    pub status_code: u32,
    pub dns_records: Vec<RecordDTO>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ResponseWithId {
    status_code: u32,
    id: u64,
}

pub struct DynuClient {
    _client: reqwest::blocking::Client,
    _api_key: String,
}

fn http_error<T>(
    response: Response,
    url: &str,
    method: &str,
    status_code: &StatusCode,
) -> Result<T, ClientError> {
    let error_body = response.text()?;
    Err(ClientError::MsgError(format!(
        "{} {}, status_code={}, body={}",
        method,
        url,
        status_code.as_str(),
        error_body
    )))
}

impl DynuClient {
    pub fn new(api_key: &str) -> Result<DynuClient, ClientError> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, "application/json".parse()?);
        headers.insert("api-key", api_key.parse()?);
        let client = reqwest::blocking::Client::builder()
            .default_headers(headers)
            .build()?;
        Ok(DynuClient {
            _client: client,
            _api_key: api_key.to_string(),
        })
    }

    pub fn get_domains(&self) -> Result<DomainsDTO, ClientError> {
        let url = format!("{}/v2/dns", DYNU_API.to_string());
        let response: Response = self._client.get(&url).send()?;
        let status = response.status();
        if !status.is_success() {
            return http_error(response, &url, "GET", &status);
        }
        let result: DomainsDTO = response.json()?;
        Ok(result)
    }

    pub fn get_domain(&self, id: u64) -> Result<Option<DomainDTO>, ClientError> {
        let url = format!("{}/v2/dns/{}", DYNU_API.to_string(), id);
        let response: Response = self._client.get(&url).send()?;
        let status = response.status();
        if !status.is_success() {
            return Ok(None);
        }
        let result: DomainDTO = response.json()?;
        Ok(Some(result))
    }

    pub fn update_domain(&self, domain_dto: &DomainDTO) -> Result<(), ClientError> {
        let url = format!("{}/v2/dns/{}", DYNU_API, domain_dto.id.unwrap());
        let result: Response = self
            ._client
            .post(&url)
            .headers(self.json_content_header()?)
            .json(domain_dto)
            .send()?;
        let status = result.status();
        if !status.is_success() {
            return http_error(result, &url, "POST", &status);
        }
        Ok(())
    }

    fn json_content_header(&self) -> Result<HeaderMap, ClientError> {
        let mut headers = HeaderMap::new();
        headers.append(CONTENT_TYPE, "application/json".parse()?);
        Ok(headers)
    }

    pub fn get_records(&self, domain_id: u64) -> Result<RecordsDTO, ClientError> {
        let url = format!("{}/v2/dns/{}/record", DYNU_API, domain_id);
        let result: Response = self._client.get(&url).send()?;
        let status = result.status();
        if !status.is_success() {
            return http_error(result, &url, "GET", &status);
        }
        let response: RecordsDTO = result.json()?;
        Ok(response)
    }

    pub fn get_record(
        &self,
        domain_id: u64,
        record_id: u64,
    ) -> Result<Option<RecordDTO>, ClientError> {
        let url = format!("{}/v2/dns/{}/record/{}", DYNU_API, domain_id, record_id);
        let result: Response = self._client.get(&url).send()?;
        let status = result.status();
        if !status.is_success() {
            return Ok(None);
        }
        let response: RecordDTO = result.json()?;
        Ok(Some(response))
    }

    pub fn delete_record(&self, domain_id: u64, record_id: u64) -> Result<(), ClientError> {
        let url = format!("{}/v2/dns/{}/record/{}", DYNU_API, domain_id, record_id);
        let result: Response = self._client.delete(&url).send()?;
        let status = result.status();
        if !status.is_success() {
            return http_error(result, &url, "DELETE", &status);
        }
        Ok(())
    }

    pub fn create_record(
        &self,
        domain_id: u64,
        record_dto: &RecordDTO,
    ) -> Result<u64, ClientError> {
        let url = format!("{}/v2/dns/{}/record", DYNU_API, domain_id);
        let result: Response = self
            ._client
            .post(&url)
            .headers(self.json_content_header()?)
            .json(record_dto)
            .send()?;
        let status = result.status();
        if !status.is_success() {
            return http_error(result, &url, "POST", &status);
        }
        let response: ResponseWithId = result.json()?;
        Ok(response.id)
    }

    pub fn update_record(&self, domain_id: u64, record_dto: &RecordDTO) -> Result<(), ClientError> {
        let url = format!(
            "{}/v2/dns/{}/record/{}",
            DYNU_API,
            domain_id,
            record_dto.id().unwrap()
        );
        let result: Response = self
            ._client
            .post(&url)
            .headers(self.json_content_header()?)
            .json(record_dto)
            .send()?;
        let status = result.status();
        if !status.is_success() {
            return http_error(result, &url, "POST", &status);
        }
        Ok(())
    }
}

/*
 * Run tests with
 * cargo test dynu::tests -- --ignored
 * with the environment variable DYNU_API_KEY set on the shell
 */
#[cfg(test)]
mod tests {
    use super::*;
    const DOMAIN_ID: u64 = 10053136;
    const RECORD_ID: u64 = 10926510;

    use std::env;

    fn make_client() -> DynuClient {
        let api_key = env::var("DYNU_API_KEY").unwrap();
        DynuClient::new(&api_key).unwrap()
    }

    mod get {
        use super::*;
        #[test]
        #[ignore]
        fn get_records_should_deserialise() {
            let client = make_client();
            let result = client.get_records(DOMAIN_ID).unwrap();

            assert_eq!(result.status_code, 200);
            assert!(result.dns_records.len() > 1);
        }

        #[test]
        #[ignore]
        fn get_record_should_deserialise() {
            let client = make_client();
            let result = client.get_record(DOMAIN_ID, RECORD_ID).unwrap();

            assert!(result.is_some());
        }
    }

    mod mutating {
        use super::*;
        #[test]
        #[ignore]
        fn create_record_should_work() {
            let client = make_client();
            let before_run = client.get_records(DOMAIN_ID).unwrap();

            let txt_record = RecordDTO::txt_record("test", "test-value", 120);
            let result = client.create_record(DOMAIN_ID, &txt_record).unwrap();

            let after_creation = client.get_records(DOMAIN_ID).unwrap();
            assert_eq!(before_run.dns_records.len() + 1, after_creation.dns_records.len());

            client.delete_record(DOMAIN_ID, result).unwrap();

            let after_deletion = client.get_records(DOMAIN_ID).unwrap();
            assert_eq!(before_run.dns_records.len(), after_deletion.dns_records.len())
        }
    }
}
