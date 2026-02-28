mod commands;
mod types;
mod utils;

use commands::market::{
    download_marketplace_skill, search_marketplaces, update_marketplace_skill,
};
use commands::skills::{import_local_skill, link_local_skill, scan_overview, uninstall_skill};

pub use crate::types::{
    DownloadRequest, DownloadResult, IdeDir, IdeSkill, ImportRequest, InstallResult, LinkRequest,
    LinkTarget, LocalScanRequest, LocalSkill, MarketStatus, Overview, RemoteSkill,
    RemoteSkillView, RemoteSkillsResponse, RemoteSkillsViewResponse, UninstallRequest,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            search_marketplaces,
            download_marketplace_skill,
            update_marketplace_skill,
            link_local_skill,
            scan_overview,
            uninstall_skill,
            import_local_skill
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
