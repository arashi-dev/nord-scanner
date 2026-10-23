use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TestStatus {
    Online,
    Timeout,
    Refused,
    Unreachable,
    Error,
    Pending,
}

impl TestStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TestStatus::Online => "ONLINE",
            TestStatus::Timeout => "TIMEOUT",
            TestStatus::Refused => "REFUSED",
            TestStatus::Unreachable => "UNREACHABLE",
            TestStatus::Error => "ERROR",
            TestStatus::Pending => "PENDING",
        }
    }
}

impl std::fmt::Display for TestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for TestStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "ONLINE" => Ok(TestStatus::Online),
            "TIMEOUT" => Ok(TestStatus::Timeout),
            "REFUSED" => Ok(TestStatus::Refused),
            "UNREACHABLE" => Ok(TestStatus::Unreachable),
            "ERROR" => Ok(TestStatus::Error),
            "PENDING" => Ok(TestStatus::Pending),
            _ => Ok(TestStatus::Error),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub id: i64,
    pub name: String,
    pub hostname: String,
    pub station: String,
    pub country_name: String,
    pub country_code: String,
    pub city: String,
    pub load: i32,
    #[serde(default)]
    pub technologies: Vec<String>,
    #[serde(default)]
    pub groups: Vec<String>,
}

impl ServerInfo {
    pub fn is_obfuscated(&self) -> bool {
        self.technologies.iter().any(|t| {
            let l = t.to_lowercase();
            l.contains("xor") || l.contains("obfusc")
        }) || self.groups.iter().any(|g| g.to_lowercase().contains("obfusc"))
    }

    pub fn is_nordwhisper(&self) -> bool {
        self.technologies
            .iter()
            .any(|t| t.to_lowercase().contains("whisper"))
    }

    pub fn feature_tags(&self) -> Vec<String> {
        let mut tags = Vec::new();
        if self.is_obfuscated() {
            tags.push("Obfuscated".to_string());
        }
        if self.is_nordwhisper() {
            tags.push("NordWhisper".to_string());
        }
        if self.groups.iter().any(|g| g.to_lowercase().contains("p2p")) {
            tags.push("P2P".to_string());
        }
        if self.groups.iter().any(|g| g.to_lowercase().contains("double")) {
            tags.push("DoubleVPN".to_string());
        }
        if self.groups.iter().any(|g| g.to_lowercase().contains("onion")) {
            tags.push("Onion".to_string());
        }
        if tags.is_empty() {
            tags.push("Standard".to_string());
        }
        tags
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub server_id: i64,
    pub name: String,
    pub hostname: String,
    pub station: String,
    pub country_name: String,
    pub country_code: String,
    pub city: String,
    pub load: i32,
    pub port: u16,
    pub status: TestStatus,
    pub latency_ms: Option<f64>,
    pub tested_at: f64,
    #[serde(default)]
    pub is_cached: bool,
    #[serde(default)]
    pub features: Vec<String>,
    pub error_msg: Option<String>,
}

impl TestResult {
    #[allow(dead_code)]
    pub fn age_seconds(&self) -> f64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        (now - self.tested_at).max(0.0)
    }

    #[allow(dead_code)]
    pub fn age_formatted(&self) -> String {
        let s = self.age_seconds() as u64;
        if s < 60 {
            format!("{}s ago", s)
        } else if s < 3600 {
            format!("{}m ago", s / 60)
        } else {
            let h = s / 3600;
            let m = (s % 3600) / 60;
            format!("{}h {}m ago", h, m)
        }
    }
}
