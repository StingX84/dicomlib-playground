use crate::{IntoDicomErr, Result, dicom_err};
use core::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::{net::ToSocketAddrs, str::FromStr};

// cSpell:ignore addrs

/// Network to match other host addresses against
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum NetworkDefinition {
    HostName { addr: String, bits: Option<u16> },
    UnixSocket(String),
    Ip { addr: IpAddr, bits: Option<u16> },
}

/// Network mask for a prefix length, saturating: `/0` matches everything (mask
/// `0`), `bits >= 32` keeps the full address. Avoids the shift-by-width overflow
/// that `!0u32 << (32 - bits)` panics on for `/0`.
fn ipv4_mask(bits: u16) -> u32 {
    (!0u32).checked_shl(32u32.saturating_sub(bits as u32)).unwrap_or(0)
}

fn ipv6_mask(bits: u16) -> u128 {
    (!0u128).checked_shl(128u32.saturating_sub(bits as u32)).unwrap_or(0)
}

fn is_valid_domain_name(name: &str) -> bool {
    let labels: Vec<&str> = name.split('.').collect();

    for label in labels {
        if label.is_empty() || label.len() > 63 {
            return false; // Labels must be between 1 and 63 characters
        }
        if !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return false; // Labels can only contain alphanumeric characters and hyphens
        }
        if label.starts_with('-') || label.ends_with('-') {
            return false; // Labels cannot start or end with a hyphen
        }
    }
    true
}

impl NetworkDefinition {
    pub fn resolve_sync(&self) -> Result<Network> {
        match self {
            NetworkDefinition::HostName { addr, bits } => {
                let addrs = ToSocketAddrs::to_socket_addrs(&(addr.as_ref(), 0))
                    .to_dicom_err_with(|| format!("Failed to resolve hostname: {}", addr))?;
                Ok(Network {
                    definition: self.clone(),
                    resolved: if bits.unwrap_or(0) <= 32 {
                        // Any address will do
                        NetworkResolved::Ip {
                            addr: addrs.map(|s| s.ip()).collect(),
                            bits: *bits,
                        }
                    } else {
                        // Filter only IPv6
                        NetworkResolved::Ip {
                            addr: addrs.filter(|ip| ip.is_ipv6()).map(|s| s.ip()).collect(),
                            bits: *bits,
                        }
                    },
                })
            }
            NetworkDefinition::UnixSocket(file) => Ok(Network {
                definition: self.clone(),
                resolved: NetworkResolved::UnixSocket(file.clone()),
            }),
            NetworkDefinition::Ip { addr, bits } => Ok(Network {
                definition: self.clone(),
                resolved: NetworkResolved::Ip {
                    addr: vec![*addr],
                    bits: *bits,
                },
            }),
        }
    }
}

impl FromStr for NetworkDefinition {
    type Err = crate::DicomError;

    fn from_str(value: &str) -> Result<Self> {
        let addr_str: &str;
        let bits: Option<u16>;

        if value.starts_with("unix:") || value.starts_with("/") {
            let file = value.trim_start_matches("unix:");
            return Ok(NetworkDefinition::UnixSocket(file.to_string()));
        }

        if let Some((split_addr, split_bits)) = value.split_once('/') {
            addr_str = split_addr;
            bits = Some(
                split_bits
                    .parse()
                    .map_err(|e| dicom_err!(InvalidData, "Invalid network bits: {}: {}", split_bits, e))?,
            );
        } else {
            addr_str = value;
            bits = None;
        }
        match addr_str.parse::<IpAddr>() {
            Ok(IpAddr::V4(addr)) => {
                if let Some(bits) = bits
                    && bits > 32
                {
                    return Err(dicom_err!(InvalidData, "Invalid network bits for IPv4: {}", bits));
                }
                Ok(NetworkDefinition::Ip {
                    addr: IpAddr::V4(addr),
                    bits,
                })
            }
            Ok(IpAddr::V6(addr)) => {
                if let Some(bits) = bits
                    && bits > 128
                {
                    return Err(dicom_err!(InvalidData, "Invalid network bits for IPv6: {}", bits));
                }
                Ok(NetworkDefinition::Ip {
                    addr: IpAddr::V6(addr),
                    bits,
                })
            }
            _ if is_valid_domain_name(addr_str) => Ok(NetworkDefinition::HostName {
                addr: addr_str.to_string(),
                bits,
            }),
            _ => Err(dicom_err!(InvalidData, "Invalid network address: {}", addr_str)),
        }
    }
}

impl std::fmt::Display for NetworkDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkDefinition::HostName { addr, bits: Some(bits) } => {
                write!(f, "{}/{}", addr, bits)
            }
            NetworkDefinition::HostName { addr, bits: None } => {
                write!(f, "{}", addr)
            }
            NetworkDefinition::UnixSocket(file) => write!(f, "unix:{}", file),
            NetworkDefinition::Ip { addr, bits: Some(bits) } => {
                write!(f, "{}/{}", addr, bits)
            }
            NetworkDefinition::Ip { addr, bits: None } => {
                write!(f, "{}", addr)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum NetworkResolved {
    UnixSocket(String),
    Ip { addr: Vec<IpAddr>, bits: Option<u16> },
}

#[derive(Debug, Clone)]
pub struct Network {
    pub definition: NetworkDefinition,
    pub resolved: NetworkResolved,
}

impl PartialEq for Network {
    fn eq(&self, other: &Self) -> bool {
        self.definition == other.definition
    }
}

impl Eq for Network {}

impl PartialOrd for Network {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.definition.cmp(&other.definition))
    }
}

impl Network {
    pub fn intersects(&self, other: &Network) -> bool {
        match (&self.resolved, &other.resolved) {
            (NetworkResolved::UnixSocket(f1), NetworkResolved::UnixSocket(f2)) => f1 == f2,
            (NetworkResolved::Ip { addr: a1, bits: b1 }, NetworkResolved::Ip { addr: a2, bits: b2 }) => {
                for ip1 in a1 {
                    for ip2 in a2 {
                        match (ip1, ip2) {
                            (IpAddr::V4(ip1), IpAddr::V4(ip2)) => {
                                let mask = ipv4_mask(b1.unwrap_or(32).min(b2.unwrap_or(32)));
                                if (ip1.to_bits() & mask) == (ip2.to_bits() & mask) {
                                    return true;
                                }
                            }
                            (IpAddr::V6(ip1), IpAddr::V6(ip2)) => {
                                let mask = ipv6_mask(b1.unwrap_or(128).min(b2.unwrap_or(128)));
                                if (ip1.to_bits() & mask) == (ip2.to_bits() & mask) {
                                    return true;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }
}

impl TryFrom<NetworkDefinition> for Network {
    type Error = crate::DicomError;

    fn try_from(value: NetworkDefinition) -> Result<Self> {
        value.resolve_sync()
    }
}

/// Network address
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HostDefinition {
    HostName { addr: String, port: Option<u16> },
    UnixSocket { file: String, perms: Option<u16> },
    Ip { addr: core::net::IpAddr, port: Option<u16> },
}

impl HostDefinition {
    pub fn resolve_sync(&self) -> Result<Host> {
        match self {
            HostDefinition::HostName { addr, port } => {
                let addrs = std::net::ToSocketAddrs::to_socket_addrs(&(addr.as_ref(), port.unwrap_or(0)))
                    .to_dicom_err_with(|| format!("Failed to resolve hostname: {}", addr))?;

                Ok(Host {
                    definition: self.clone(),
                    resolved: HostResolved::Ip(addrs.collect()),
                })
            }
            HostDefinition::UnixSocket { file, perms } => Ok(Host {
                definition: self.clone(),
                resolved: HostResolved::UnixSocket {
                    file: file.clone(),
                    perms: *perms,
                },
            }),
            HostDefinition::Ip { addr, port } => Ok(Host {
                definition: self.clone(),
                resolved: HostResolved::Ip(vec![SocketAddr::new(*addr, port.unwrap_or(0))]),
            }),
        }
    }

    pub fn with_default_port(self, default_port: u16) -> Self {
        match self {
            HostDefinition::HostName { addr, port } => HostDefinition::HostName {
                addr,
                port: port.or(Some(default_port)),
            },
            HostDefinition::UnixSocket { file, perms } => HostDefinition::UnixSocket { file, perms },
            HostDefinition::Ip { addr, port } => HostDefinition::Ip {
                addr,
                port: port.or(Some(default_port)),
            },
        }
    }

    pub fn set_default_port(&mut self, default_port: u16) {
        match self {
            HostDefinition::HostName { port, .. } if port.is_none() => {
                *port = Some(default_port);
            }
            HostDefinition::Ip { port, .. } if port.is_none() => {
                *port = Some(default_port);
            }
            _ => {}
        }
    }
}

impl FromStr for HostDefinition {
    type Err = crate::DicomError;

    /// Parses a [`HostDefinition`] from a string.
    ///
    /// Supported formats: "hostname", "hostname:port", "ipv4", "ipv4:port",
    /// "ipv6", "\[ipv6\]", "\[ipv6\]:port", "unix:/path/to/socket",
    /// "/path/to/socket", "/path/to/socket:perms".
    fn from_str(value: &str) -> Result<Self> {
        if value.starts_with("unix:") || value.starts_with("/") {
            let file = value.trim_start_matches("unix:");
            if let Some((file_name, file_bits)) = file.split_once(':') {
                let perms = file_bits
                    .parse::<u16>()
                    .map_err(|e| dicom_err!(InvalidData, "Invalid unix socket permissions: {}: {}", file_bits, e))?;
                return Ok(HostDefinition::UnixSocket {
                    file: file_name.to_string(),
                    perms: Some(perms),
                });
            }
            return Ok(HostDefinition::UnixSocket {
                file: file.to_string(),
                perms: None,
            });
        }

        if value.starts_with('[') {
            // Definitely an IPv6 address, find the closing bracket
            let closing_bracket = value
                .find(']')
                .ok_or_else(|| dicom_err!(InvalidData, "Invalid IPv6 address: missing closing bracket: {}", value))?;
            let addr_part = &value[1..closing_bracket];
            let ipv6 = addr_part
                .parse::<Ipv6Addr>()
                .map_err(|e| dicom_err!(InvalidData, "Invalid IPv6 address: {}: {}", addr_part, e))?;

            let rest = &value[closing_bracket + 1..];
            let port = if let Some(rest) = rest.strip_prefix(':') {
                Some(
                    rest.parse::<u16>()
                        .map_err(|e| dicom_err!(InvalidData, "Invalid port: {}: {}", rest, e))?,
                )
            } else if rest.is_empty() {
                None
            } else {
                return Err(dicom_err!(
                    InvalidData,
                    "Invalid IPv6 address format: unexpected characters after closing bracket: {}",
                    value
                ));
            };
            return Ok(HostDefinition::Ip {
                addr: IpAddr::V6(ipv6),
                port,
            });
        }

        let addr_str: &str;
        let port: Option<u16>;
        if let Some((split_addr, split_port)) = value.split_once(':') {
            addr_str = split_addr;
            port = Some(
                split_port
                    .parse::<u16>()
                    .map_err(|e| dicom_err!(InvalidData, "Invalid port: {}: {}", split_port, e))?,
            );
        } else {
            addr_str = value;
            port = None;
        }
        match addr_str.parse::<IpAddr>() {
            Ok(addr) => Ok(HostDefinition::Ip { addr, port }),
            Err(_) if is_valid_domain_name(addr_str) => Ok(HostDefinition::HostName {
                addr: addr_str.to_string(),
                port,
            }),
            Err(e) => Err(dicom_err!(InvalidData, "Invalid address: {}: {}", value, e)),
        }
    }
}

impl std::fmt::Display for HostDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HostDefinition::HostName { addr, port: None } => {
                write!(f, "{}", addr)
            }
            HostDefinition::HostName { addr, port: Some(port) } => {
                write!(f, "{}:{}", addr, port)
            }
            HostDefinition::UnixSocket { file, perms: None } => write!(f, "unix:{}", file),
            HostDefinition::UnixSocket {
                file,
                perms: Some(perms),
            } => write!(f, "unix:{}:{}", file, perms),
            HostDefinition::Ip {
                addr: IpAddr::V6(addr),
                port: None,
            } => {
                write!(f, "{}", addr)
            }
            HostDefinition::Ip {
                addr: IpAddr::V6(addr),
                port: Some(port),
            } => {
                write!(f, "[{}]:{}", addr, port)
            }
            HostDefinition::Ip {
                addr: IpAddr::V4(addr),
                port: None,
            } => {
                write!(f, "{}", addr)
            }
            HostDefinition::Ip {
                addr: IpAddr::V4(addr),
                port: Some(port),
            } => {
                write!(f, "{}:{}", addr, port)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum HostResolved {
    UnixSocket { file: String, perms: Option<u16> },
    Ip(Vec<core::net::SocketAddr>),
}

#[derive(Debug, Clone)]
pub struct Host {
    pub definition: HostDefinition,
    pub resolved: HostResolved,
}

impl Host {
    pub fn is_in_network(&self, network: &Network) -> bool {
        match (&self.resolved, &network.resolved) {
            (HostResolved::UnixSocket { file: f1, .. }, NetworkResolved::UnixSocket(f2)) => f1 == f2,
            (HostResolved::Ip(a1), NetworkResolved::Ip { addr: a2, bits: b2 }) => {
                for ip1 in a1 {
                    for ip2 in a2 {
                        match (ip1, ip2) {
                            (core::net::SocketAddr::V4(ip1), core::net::IpAddr::V4(ip2)) => {
                                let mask = ipv4_mask(b2.unwrap_or(32));
                                if (ip1.ip().to_bits() & mask) == (ip2.to_bits() & mask) {
                                    return true;
                                }
                            }
                            (core::net::SocketAddr::V6(ip1), core::net::IpAddr::V6(ip2)) => {
                                let mask = ipv6_mask(b2.unwrap_or(128));
                                if (ip1.ip().to_bits() & mask) == (ip2.to_bits() & mask) {
                                    return true;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }
}

impl PartialEq for Host {
    fn eq(&self, other: &Self) -> bool {
        self.definition == other.definition
    }
}

impl Eq for Host {}

impl PartialOrd for Host {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.definition.cmp(&other.definition))
    }
}

/// The real address of the socket (either Unix or IP).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PeerSocketAddr {
    Unix { file: String },
    Ip(core::net::SocketAddr),
}

impl PeerSocketAddr {
    pub fn is_in_network(&self, network: &Network) -> bool {
        match (self, &network.resolved) {
            (PeerSocketAddr::Unix { file: f1 }, NetworkResolved::UnixSocket(f2)) => f1 == f2,
            (PeerSocketAddr::Ip(ip1), NetworkResolved::Ip { addr: a2, bits: b2 }) => {
                for ip2 in a2 {
                    match (ip1, ip2) {
                        (core::net::SocketAddr::V4(ip1), core::net::IpAddr::V4(ip2)) => {
                            let mask = ipv4_mask(b2.unwrap_or(32));
                            if (ip1.ip().to_bits() & mask) == (ip2.to_bits() & mask) {
                                return true;
                            }
                        }
                        (core::net::SocketAddr::V6(ip1), core::net::IpAddr::V6(ip2)) => {
                            let mask = ipv6_mask(b2.unwrap_or(128));
                            if (ip1.ip().to_bits() & mask) == (ip2.to_bits() & mask) {
                                return true;
                            }
                        }
                        _ => {}
                    }
                }
                false
            }
            _ => false,
        }
    }
}

impl std::fmt::Display for PeerSocketAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PeerSocketAddr::Unix { file } => write!(f, "unix:{}", file),
            PeerSocketAddr::Ip(addr) => write!(f, "{}", addr),
        }
    }
}

/// The address used to connect to a peer or accept connections on.
#[derive(Debug, Clone)]
pub struct PeerAddress {
    /// The configured address.
    pub definition: HostDefinition,
    /// The actual socket address
    pub sock_addr: PeerSocketAddr,
}

impl std::fmt::Display for PeerAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.definition {
            HostDefinition::HostName { addr, .. } => write!(f, "{}({})", addr, self.sock_addr),
            _ => write!(f, "{}", self.sock_addr),
        }
    }
}

impl PartialEq for PeerAddress {
    fn eq(&self, other: &Self) -> bool {
        self.sock_addr == other.sock_addr
    }
}

impl PartialOrd for PeerAddress {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.sock_addr.cmp(&other.sock_addr))
    }
}

impl Eq for PeerAddress {}

//tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_prefix_network_matches_any_without_panicking() {
        // `/0` must match every address and must not overflow the mask shift.
        let net: Network = "0.0.0.0/0"
            .parse::<NetworkDefinition>()
            .unwrap()
            .resolve_sync()
            .unwrap();
        let addr = PeerSocketAddr::Ip("203.0.113.7:104".parse().unwrap());
        assert!(addr.is_in_network(&net));

        let net6: Network = "::/0".parse::<NetworkDefinition>().unwrap().resolve_sync().unwrap();
        let addr6 = PeerSocketAddr::Ip("[2001:db8::1]:104".parse().unwrap());
        assert!(addr6.is_in_network(&net6));
    }

    #[test]
    fn can_parse_network() {
        assert_eq!(
            "example.com".parse(),
            Ok(NetworkDefinition::HostName {
                addr: "example.com".into(),
                bits: None
            })
        );
    }
}
