//! NetworkManager config file converter
//!
//! Converts NetworkManager .nmconnection files to netctl TOML format

use libnetctl::connection_config::*;
use std::collections::HashMap;
use std::path::PathBuf;
use clap::{Arg, Command as ClapCommand};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let matches = ClapCommand::new("nm-converter")
        .about("Convert NetworkManager config files to netctl format")
        .version("1.0.0")
        .arg(
            Arg::new("input")
                .short('i')
                .long("input")
                .value_name("FILE")
                .help("Input NetworkManager .nmconnection file")
                .required_unless_present("dir"),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("FILE")
                .help("Output netctl config file (.nctl)")
                .required_unless_present("dir"),
        )
        .arg(
            Arg::new("dir")
                .short('d')
                .long("directory")
                .value_name("DIR")
                .help("Convert all .nmconnection files in directory")
                .conflicts_with_all(["input", "output"]),
        )
        .arg(
            Arg::new("output-dir")
                .long("output-dir")
                .value_name("DIR")
                .help("Output directory for batch conversion")
                .requires("dir"),
        )
        .get_matches();

    if let Some(input_dir) = matches.get_one::<String>("dir") {
        // Batch conversion
        let output_dir = matches
            .get_one::<String>("output-dir")
            .map(|s| PathBuf::from(s))
            .unwrap_or_else(|| PathBuf::from("."));

        convert_directory(input_dir, &output_dir).await?;
    } else {
        // Single file conversion
        let input = matches.get_one::<String>("input").unwrap();
        let output = matches.get_one::<String>("output").unwrap();

        convert_file(input, output).await?;
    }

    Ok(())
}

async fn convert_file(input: &str, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("Converting {} to {}", input, output);

    let nm_config = parse_nmconnection(input).await?;
    let netctl_config = nm_to_netctl(&nm_config)?;

    netctl_config.to_file(output).await?;

    info!("Successfully converted to {}", output);
    Ok(())
}

async fn convert_directory(
    input_dir: &str,
    output_dir: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Converting all .nmconnection files in {}", input_dir);

    tokio::fs::create_dir_all(output_dir).await?;

    let mut entries = tokio::fs::read_dir(input_dir).await?;
    let mut count = 0;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        if let Some(ext) = path.extension() {
            if ext == "nmconnection" {
                let stem = path.file_stem().unwrap().to_string_lossy();
                let output_path = output_dir.join(format!("{}.nctl", stem));

                match convert_file(path.to_str().unwrap(), output_path.to_str().unwrap()).await {
                    Ok(_) => count += 1,
                    Err(e) => warn!("Failed to convert {}: {}", path.display(), e),
                }
            }
        }
    }

    info!("Converted {} files", count);
    Ok(())
}

/// Parse NetworkManager .nmconnection file (INI format)
async fn parse_nmconnection(path: &str) -> Result<HashMap<String, HashMap<String, String>>, Box<dyn std::error::Error>> {
    let content = tokio::fs::read_to_string(path).await?;
    let mut sections: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current_section = String::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        // Section header
        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].to_string();
            sections.insert(current_section.clone(), HashMap::new());
            continue;
        }

        // Key-value pair
        if let Some(pos) = line.find('=') {
            let key = line[..pos].trim().to_string();
            let value = line[pos + 1..].trim().to_string();

            if !current_section.is_empty() {
                if let Some(section) = sections.get_mut(&current_section) {
                    section.insert(key, value);
                }
            }
        }
    }

    Ok(sections)
}

/// Convert NetworkManager config to netctl config
fn nm_to_netctl(
    nm_config: &HashMap<String, HashMap<String, String>>,
) -> Result<NetctlConnectionConfig, Box<dyn std::error::Error>> {
    let connection_section = nm_config
        .get("connection")
        .ok_or("Missing [connection] section")?;

    let conn_type = connection_section
        .get("type")
        .ok_or("Missing connection type")?;

    let uuid = connection_section
        .get("uuid")
        .ok_or("Missing UUID")?
        .clone();

    let name = connection_section
        .get("id")
        .ok_or("Missing connection ID")?
        .clone();

    let autoconnect = connection_section
        .get("autoconnect")
        .map(|v| v == "true")
        .unwrap_or(false);

    let interface_name = connection_section.get("interface-name").cloned();

    // Build connection section
    let connection = ConnectionSection {
        name,
        uuid,
        conn_type: conn_type.clone(),
        autoconnect,
        interface_name,
        plugin: None,  // Will be set based on type
    };

    // Parse type-specific sections
    let wifi = if conn_type == "wifi" || conn_type == "802-11-wireless" {
        Some(parse_wifi_section(nm_config)?)
    } else {
        None
    };

    let wifi_security = if nm_config.contains_key("wifi-security") || nm_config.contains_key("802-11-wireless-security") {
        Some(parse_wifi_security_section(nm_config)?)
    } else {
        None
    };

    let vpn = if conn_type == "vpn" {
        Some(parse_vpn_section(nm_config)?)
    } else {
        None
    };

    let ethernet = if conn_type == "ethernet" || conn_type == "802-3-ethernet" {
        Some(parse_ethernet_section(nm_config)?)
    } else {
        None
    };

    let ipv4 = nm_config.get("ipv4").map(|s| parse_ip_section(s)).transpose()?;
    let ipv6 = nm_config.get("ipv6").map(|s| parse_ip_section(s)).transpose()?;

    Ok(NetctlConnectionConfig {
        connection,
        wifi,
        wifi_security,
        vpn,
        ethernet,
        ipv4,
        ipv6,
    })
}

fn parse_wifi_section(
    nm_config: &HashMap<String, HashMap<String, String>>,
) -> Result<WifiSection, Box<dyn std::error::Error>> {
    let wifi_section = nm_config
        .get("wifi")
        .or_else(|| nm_config.get("802-11-wireless"))
        .ok_or("Missing wifi section")?;

    let ssid = wifi_section.get("ssid").ok_or("Missing SSID")?.clone();
    let mode = wifi_section.get("mode").cloned().unwrap_or_else(|| "infrastructure".to_string());
    let bssid = wifi_section.get("bssid").cloned();
    let channel = wifi_section.get("channel").and_then(|c| c.parse().ok());

    Ok(WifiSection {
        ssid,
        mode,
        bssid,
        channel,
    })
}

fn parse_wifi_security_section(
    nm_config: &HashMap<String, HashMap<String, String>>,
) -> Result<WifiSecuritySection, Box<dyn std::error::Error>> {
    let sec_section = nm_config
        .get("wifi-security")
        .or_else(|| nm_config.get("802-11-wireless-security"))
        .ok_or("Missing wifi-security section")?;

    let key_mgmt = sec_section
        .get("key-mgmt")
        .ok_or("Missing key-mgmt")?
        .clone();

    let psk = sec_section.get("psk").cloned();
    let password = sec_section.get("password").cloned();

    Ok(WifiSecuritySection {
        key_mgmt,
        psk,
        password,
    })
}

fn parse_vpn_section(
    nm_config: &HashMap<String, HashMap<String, String>>,
) -> Result<VpnSection, Box<dyn std::error::Error>> {
    let vpn_section = nm_config.get("vpn").ok_or("Missing vpn section")?;

    let remote = vpn_section.get("remote").cloned();
    let port = vpn_section.get("port").and_then(|p| p.parse().ok());
    let proto = vpn_section.get("proto").cloned();
    let ca = vpn_section.get("ca").cloned();
    let cert = vpn_section.get("cert").cloned();
    let key = vpn_section.get("key").cloned();
    let config_file = vpn_section.get("config").cloned();
    let connection_type = vpn_section
        .get("connection-type")
        .or_else(|| vpn_section.get("dev"))
        .cloned()
        .unwrap_or_else(|| "openvpn".to_string());

    Ok(VpnSection {
        connection_type,
        wireguard: None,
        openvpn: None,
        remote,
        port,
        proto,
        ca,
        cert,
        key,
        config_file,
    })
}

fn parse_ethernet_section(
    nm_config: &HashMap<String, HashMap<String, String>>,
) -> Result<EthernetSection, Box<dyn std::error::Error>> {
    let eth_section = nm_config
        .get("ethernet")
        .or_else(|| nm_config.get("802-3-ethernet"));

    let mac_address = eth_section.and_then(|s| s.get("mac-address")).cloned();
    let mtu = eth_section.and_then(|s| s.get("mtu")).and_then(|m| m.parse().ok());

    Ok(EthernetSection { mac_address, mtu })
}

fn parse_ip_section(
    section: &HashMap<String, String>,
) -> Result<IpConfigSection, Box<dyn std::error::Error>> {
    let method = section
        .get("method")
        .ok_or("Missing IP method")?
        .clone();

    // Parse address1, address2, etc.
    let address = section.get("address1").or_else(|| section.get("address")).cloned();

    let gateway = if let Some(addr) = address.as_ref() {
        // NetworkManager format: "192.168.1.100/24,192.168.1.1"
        if let Some(pos) = addr.find(',') {
            Some(addr[pos + 1..].to_string())
        } else {
            section.get("gateway").cloned()
        }
    } else {
        section.get("gateway").cloned()
    };

    // Parse DNS servers
    let dns = section
        .get("dns")
        .map(|d| d.split(';').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect());

    Ok(IpConfigSection {
        method,
        address,
        gateway,
        dns,
        routes: None,
    })
}
