mod commands;
mod types;
mod utils;

use tauri::Manager;
use commands::market::{download_marketplace_skill, search_marketplaces, update_marketplace_skill};
use commands::skills::{
    adopt_ide_skill, delete_local_skills, export_local_skills, import_local_skill,
    link_local_skill, scan_overview, scan_project_ide_dirs, uninstall_skill,
};

pub use crate::types::{
    AdoptIdeSkillRequest, DeleteLocalSkillRequest, ExportSkillsRequest, IdeDir, IdeSkill,
    ImportRequest, InstallResult, LinkRequest, LinkTarget, LocalScanRequest, LocalSkill,
    MarketStatus, MarketStatusType, Overview, ProjectIdeDir, ProjectScanRequest,
    ProjectScanResult, RemoteSkill, RemoteSkillView, RemoteSkillsResponse,
    RemoteSkillsViewResponse, UninstallRequest,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
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
            import_local_skill,
            delete_local_skills,
            export_local_skills,
            adopt_ide_skill,
            scan_project_ide_dirs
        ]);

    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        // When a second instance is started, focus the existing window
        let _ = app
            .get_webview_window("main")
            .expect("no main window")
            .set_focus();
    }));

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
