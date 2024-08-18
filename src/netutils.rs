use std::io;

const IP4_API: &str = "https://api.ipify.org";
const IP6_API: &str = "https://api6.ipify.org";
pub enum IP {
    V4,
    V6,
}

pub fn ip(ip: IP) -> Option<String> {
    let address = match ip {
        IP::V4 => IP4_API,
        IP::V6 => IP6_API,
    };
    let ipv4 = reqwest::blocking::get(address).and_then(|r| r.text());
    match ipv4 {
        Ok(t) => Some(t),
        _ => None,
    }
}

pub struct Addresses {
    pub v4: Option<String>,
    pub v6: Option<String>,
}

pub fn public_ip_of(domain: &str) -> Result<Addresses, io::Error> {
    let resolved = dns_lookup::lookup_host(domain).unwrap_or_else(|_| vec!());
    if resolved.is_empty() {
        Ok(Addresses { v4: None, v6: None })
    } else {
        let mut v4: Option<String> = None;
        let mut v6: Option<String> = None;
        for ip in resolved {
            match ip {
                std::net::IpAddr::V4(v) => {
                    if v4.is_none() {
                        v4 = Some(v.to_string())
                    }
                }
                std::net::IpAddr::V6(v) => {
                    if v6.is_none() {
                        v6 = Some(v.to_string())
                    }
                }
            }
        }
        Ok(Addresses { v4, v6 })
    }
}
