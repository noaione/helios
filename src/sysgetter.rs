use std::{collections::HashSet, net::IpAddr, sync::LazyLock};

use serde::Serialize;
use sysinfo::{Disks, Networks, System};
use tokio::sync::RwLock;

const MAC_VERSIONS: [(&str, &str, &str); 23] = [
    ("26", "macOS", "Tahoe"),
    ("15", "macOS", "Sequoia"),
    ("14", "macOS", "Sonoma"),
    ("13", "macOS", "Ventura"),
    ("12", "macOS", "Monterey"),
    ("11", "macOS", "Big Sur"),
    // Big Sur identifies itself as 10.16 in some situations.
    // https://en.wikipedia.org/wiki/MacOS_Big_Sur#Development_history
    ("10.16", "macOS", "Big Sur"),
    ("10.15", "macOS", "Catalina"),
    ("10.14", "macOS", "Mojave"),
    ("10.13", "macOS", "High Sierra"),
    ("10.12", "macOS", "Sierra"),
    ("10.11", "OS X", "El Capitan"),
    ("10.10", "OS X", "Yosemite"),
    ("10.9", "OS X", "Mavericks"),
    ("10.8", "OS X", "Mountain Lion"),
    ("10.7", "Mac OS X", "Lion"),
    ("10.6", "Mac OS X", "Snow Leopard"),
    ("10.5", "Mac OS X", "Leopard"),
    ("10.4", "Mac OS X", "Tiger"),
    ("10.3", "Mac OS X", "Panther"),
    ("10.2", "Mac OS X", "Jaguar"),
    ("10.1", "Mac OS X", "Puma"),
    ("10.0", "Mac OS X", "Cheetah"),
];
const MAXIMUM_HEARTBEAT: i64 = 15; // 15 seconds (a bit less than the heartbeat in the frontend)

static CACHED_HOST: LazyLock<String> = LazyLock::new(get_pc_host);
static KERNEL_LONG_VER: LazyLock<String> = LazyLock::new(System::kernel_long_version);
static OS_NAME: LazyLock<String> = LazyLock::new(|| {
    let os_name = System::name().unwrap_or_else(|| "Unknown".to_string());

    let mut actual_os_name = os_name.clone();
    let os_version = System::os_version();
    if os_name == "Darwin" {
        if let Some(version) = os_version {
            for (prefix, name, full_name) in MAC_VERSIONS.iter() {
                if version.starts_with(prefix) {
                    return format!("{name} {version} {full_name}");
                }
            }
        } else {
            actual_os_name = "macOS".to_string();
        }
    } else if let Some(version) = os_version {
        actual_os_name.push(' ');
        actual_os_name.push_str(&version);
    }

    actual_os_name
});
static HOSTNAME: LazyLock<String> =
    LazyLock::new(|| System::host_name().unwrap_or_else(|| "unknown.local".to_string()));

static LOCKED_CURRENT: LazyLock<RwLock<(SystemInfo, i64)>> = LazyLock::new(|| {
    // Initialize the first time data with system info
    let info = get_system_info_by_lines_unlocked();
    let ts = chrono::Utc::now().timestamp();

    RwLock::new((info, ts))
});

// some static information about the system
// static VIRT_HOST: &str

#[derive(Debug, Clone, Serialize)]
pub struct LineInfo {
    key: String,
    value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemInfo {
    host: String,
    lines: Vec<LineInfo>,
}

impl From<(String, String)> for LineInfo {
    fn from(tuple: (String, String)) -> Self {
        LineInfo {
            key: tuple.0,
            value: tuple.1,
        }
    }
}

impl SystemInfo {
    pub fn as_html_info(&self) -> String {
        let mut html = String::new();
        // host
        html.push_str(r#"<p class="host-header">noaione<span class="host-at">@</span>"#);
        html.push_str(&self.host);
        html.push_str("</p>\n");

        // create separator
        // noaione@ = 8 len
        let host_length = self.host.len() + 8;
        html.push_str(r#"<p class="detail-line">"#);
        html.push_str(&"-".repeat(host_length));
        html.push_str("</p>\n");

        for line in &self.lines {
            html.push_str(r#"<p class="detail-line"><span class="detail-line-root">"#);
            html.push_str(&line.key);
            html.push_str("</span>: ");
            html.push_str(&line.value);
            html.push_str("</p>\n");
        }

        html
    }

    pub async fn update_self(&mut self) {
        let sys_info = get_system_info_by_lines_unlocked();
        self.host = sys_info.host;
        self.lines = sys_info.lines;
    }
}

/// Not a future, but a function that retrieves system information.
pub fn get_system_info_by_lines_unlocked() -> SystemInfo {
    let mut sys = System::new_all();
    sys.refresh_all();

    let mut merged_lines: Vec<(String, String)> = vec![];

    // Get OS name
    let actual_os_name = OS_NAME.clone();
    merged_lines.push(("OS".to_string(), actual_os_name));

    // Get hostname
    let pc_host = CACHED_HOST.clone();
    if !pc_host.is_empty() {
        merged_lines.push(("Host".to_string(), pc_host));
    }

    // Get kernel
    let kernel_version = KERNEL_LONG_VER.clone();
    merged_lines.push(("Kernel".to_string(), kernel_version));

    // Get system uptime (in seconds, convert to human readable)
    let uptime_seconds = System::uptime();
    let uptime_str = format_uptime(uptime_seconds);

    merged_lines.push(("Uptime".to_string(), uptime_str));

    // Get CPU information
    let cpus = sys.cpus();
    let cpu_count = cpus.len();
    if !cpus.is_empty() {
        let cpu = &cpus[0];
        let cpu_brand = cpu.brand().to_string();
        let cpu_freq = calculate_cpu_freq(cpu.frequency());
        merged_lines.push((
            "CPU".to_string(),
            format!("{cpu_brand} ({cpu_count}) @ {cpu_freq}"),
        ))
    }

    let total_memory = sys.total_memory();
    let used_memory = sys.used_memory();
    let memory_usage = if total_memory > 0 {
        (used_memory as f64 / total_memory as f64) * 100.0
    } else {
        0.0
    };

    merged_lines.push((
        "Memory".to_string(),
        format!(
            "{} / {} ({:.1}%)",
            format_bytes(used_memory),
            format_bytes(total_memory),
            memory_usage
        ),
    ));

    let total_swap = sys.total_swap();
    let used_swap = sys.used_swap();
    let swap_usage = if total_swap > 0 {
        (used_swap as f64 / total_swap as f64) * 100.0
    } else {
        0.0
    };

    if total_swap > 0 {
        merged_lines.push((
            "Swap".to_string(),
            format!(
                "{} / {} ({:.1}%)",
                format_bytes(used_swap),
                format_bytes(total_swap),
                swap_usage
            ),
        ));
    } else {
        merged_lines.push(("Swap".to_string(), "Disabled".to_string()));
    }

    let disks = Disks::new_with_refreshed_list();
    let mut mounted: HashSet<String> = HashSet::new();
    let mut disk_lines: Vec<(String, String)> = vec![];
    for disk in &disks {
        let total_space = disk.total_space();
        let available_space = disk.available_space();
        let used_space = total_space - available_space;
        let usage_percent = if total_space > 0 {
            (used_space as f64 / total_space as f64) * 100.0
        } else {
            0.0
        };

        let disk_name = disk.name().to_string_lossy().to_string();
        if mounted.contains(&disk_name) {
            continue; // Skip already processed disks
        }
        mounted.insert(disk_name);

        let file_system = disk.file_system().to_string_lossy();
        if file_system.is_empty() {
            continue; // Skip disks without a file system
        }
        match file_system.as_ref() {
            "tmpfs" | "devtmpfs" | "overlay" | "squashfs" => continue, // Skip temporary or special filesystems
            _ => {}
        }

        disk_lines.push((
            format!(
                "{} / {} ({:.1}%) - {file_system}",
                format_bytes(used_space),
                format_bytes(total_space),
                usage_percent
            ),
            disk.mount_point().to_string_lossy().to_string(),
        ));
    }

    let disk_total = disk_lines.len();
    if disk_total > 0 {
        for (line, mnt_point) in &disk_lines {
            let disk_key = if disk_total > 1 {
                format!("Disk ({})", mnt_point)
            } else {
                "Disk".to_string()
            };

            merged_lines.push((disk_key, line.to_string()));
        }
    }

    let networks = Networks::new_with_refreshed_list();
    let mut valid_ipv4 = 0;
    let mut valid_ipv6 = 0;
    for (_, network) in &networks {
        let ip_address = network.ip_networks();
        for ip in ip_address {
            match ip.addr {
                IpAddr::V4(addr) => {
                    if !addr.is_broadcast()
                        && !addr.is_documentation()
                        && !addr.is_link_local()
                        && !addr.is_loopback()
                        && !addr.is_multicast()
                        && !addr.is_unspecified()
                        && !addr.is_private()
                    {
                        valid_ipv4 += 1;
                    }
                }
                IpAddr::V6(addr) => {
                    if !addr.is_loopback()
                        && !addr.is_multicast()
                        && !addr.is_unicast_link_local()
                        && !addr.is_unique_local()
                        && !addr.is_unspecified()
                    {
                        valid_ipv6 += 1;
                    }
                }
            }
        }
    }

    if valid_ipv4 > 0 || valid_ipv6 > 0 {
        let mut string_data = vec![];
        if valid_ipv4 > 0 {
            string_data.push(format!("{valid_ipv4}x IPv4"));
        }
        if valid_ipv6 > 0 {
            string_data.push(format!("{valid_ipv6}x IPv6"));
        }

        merged_lines.push(("Network".to_string(), string_data.join(", ")));
    }

    let all_lines = merged_lines
        .into_iter()
        .map(LineInfo::from)
        .collect::<Vec<_>>();

    SystemInfo {
        host: HOSTNAME.clone(),
        lines: all_lines,
    }
}

pub async fn get_system_info_by_lines_with_lock() -> SystemInfo {
    let now_ts = chrono::Utc::now().timestamp();
    let read_guard = LOCKED_CURRENT.read().await;
    if now_ts - read_guard.1 < MAXIMUM_HEARTBEAT {
        return read_guard.0.clone();
    }

    // This function is called from the main thread, so we can use the unlocked version
    let sys_info = get_system_info_by_lines_unlocked();

    // Update the cached data
    let mut write_guard = LOCKED_CURRENT.write().await;
    write_guard.0 = sys_info.clone();
    write_guard.1 = now_ts;

    sys_info
}

// Helper function to format uptime
fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if days > 0 {
        format!(
            "{} day{}, {} hour{}, {} minute{}",
            days,
            if days == 1 { "" } else { "s" },
            hours,
            if hours == 1 { "" } else { "s" },
            minutes,
            if minutes == 1 { "" } else { "s" }
        )
    } else if hours > 0 {
        format!(
            "{} hour{}, {} minute{}",
            hours,
            if hours == 1 { "" } else { "s" },
            minutes,
            if minutes == 1 { "" } else { "s" }
        )
    } else if minutes > 0 {
        format!(
            "{} minute{}, {} second{}",
            minutes,
            if minutes == 1 { "" } else { "s" },
            secs,
            if secs == 1 { "" } else { "s" }
        )
    } else {
        format!("{} second{}", secs, if secs == 1 { "" } else { "s" })
    }
}

// Helper function to format bytes
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

fn calculate_cpu_freq(freq: u64) -> String {
    // it's in mhz
    if freq >= 1_000 {
        // use 2 decimal places for GHz
        let freq = freq as f64 / 1_000.0;
        if freq.fract() == 0.0 {
            format!("{} GHz", freq as u64)
        } else {
            format!("{freq:.2} GHz")
        }
    } else {
        format!("{freq} MHz")
    }
}

fn get_pc_host() -> String {
    let host_family = read_dmi(
        "/sys/devices/virtual/dmi/id/product_family",
        "/sys/class/dmi/id/product_family",
    );
    let host_name = read_dmi(
        "/sys/devices/virtual/dmi/id/product_name",
        "/sys/class/dmi/id/product_name",
    )
    .or_else(get_host_product_name);
    let host_version = read_dmi(
        "/sys/devices/virtual/dmi/id/product_version",
        "/sys/class/dmi/id/product_version",
    );

    let mut merged_str = String::new();

    if let Some(family) = host_family {
        if !family.trim().is_empty() {
            merged_str.push_str(family.trim());
            merged_str.push(' ');
        }
    }
    if let Some(name) = host_name {
        let trim = name.trim();
        if !trim.is_empty() {
            if trim.starts_with("Standard PC") {
                merged_str.push_str("KVM/QEMU ");
            }
            merged_str.push_str(name.trim());
            merged_str.push(' ');
        }
    }

    if let Some(version) = host_version {
        if !version.trim().is_empty() {
            merged_str.push('(');
            merged_str.push_str(version.trim());
            merged_str.push_str(") ");
        }
    }

    // trim the result
    merged_str.trim().to_string()
}

fn get_host_product_name() -> Option<String> {
    if let Ok(value) = std::fs::read_to_string("/sys/firmware/devicetree/base/model") {
        return Some(value.trim().to_string());
    }

    if let Ok(value) = std::fs::read_to_string("/sys/firmware/devicetree/base/banner-name") {
        return Some(value.trim().to_string());
    }

    if let Ok(value) = std::fs::read_to_string("/tmp/sysinfo/model") {
        return Some(value.trim().to_string());
    }

    None
}

fn read_dmi(path: &str, class_name: &str) -> Option<String> {
    if let Ok(value) = std::fs::read_to_string(path) {
        return Some(value.trim().to_string());
    }
    if let Ok(value) = std::fs::read_to_string(class_name) {
        return Some(value.trim().to_string());
    }
    None
}
