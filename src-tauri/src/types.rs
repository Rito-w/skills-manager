use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSkill {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub source_url: String,
    pub description: String,
    pub author: String,
    pub installs: u64,
    pub stars: u64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSkillsResponse {
    pub skills: Vec<RemoteSkill>,
    pub total: u64,
    pub limit: u64,
    pub offset: u64,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSkillView {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub source_url: String,
    pub description: String,
    pub author: String,
    pub installs: u64,
    pub stars: u64,
    pub market_id: String,
    pub market_label: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MarketStatus {
    pub id: String,
    pub name: String,
    pub status: String, // "online" | "error"
    pub error: Option<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSkillsViewResponse {
    pub skills: Vec<RemoteSkillView>,
    pub total: u64,
    pub limit: u64,
    pub offset: u64,
    pub market_statuses: Vec<MarketStatus>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LinkTarget {
    pub name: String,
    pub path: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    pub installed_path: String,
    pub linked: Vec<String>,
    pub skipped: Vec<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequest {
    pub source_url: String,
    pub skill_name: String,
    pub install_base_dir: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DownloadResult {
    pub installed_path: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LinkRequest {
    pub skill_path: String,
    pub skill_name: String,
    pub link_targets: Vec<LinkTarget>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LocalSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub path: String,
    pub source: String,
    pub ide: Option<String>,
    pub used_by: Vec<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LocalScanRequest {
    pub project_dir: Option<String>,
    pub ide_dirs: Vec<IdeDir>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IdeSkill {
    pub id: String,
    pub name: String,
    pub path: String,
    pub ide: String,
    pub source: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub manager_skills: Vec<LocalSkill>,
    pub ide_skills: Vec<IdeSkill>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UninstallRequest {
    pub target_path: String,
    pub project_dir: Option<String>,
    pub ide_dirs: Vec<IdeDir>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IdeDir {
    pub label: String,
    pub relative_dir: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ImportRequest {
    pub source_path: String,
}
