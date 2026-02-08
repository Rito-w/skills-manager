export default {
  app: {
    tabs: {
      local: "Local Skills",
      market: "Market",
      ide: "IDE Browser"
    },
    header: {
      language: "Language",
      theme: "Theme",
      themeLight: "Light",
      themeDark: "Dark"
    }
  },
  market: {
    title: "Marketplace Search",
    searchPlaceholder: "Search skills (name / description / author)",
    search: "Search",
    searching: "Searching...",
    refresh: "Refresh",
    refreshing: "Loading",
    resultsTitle: "Results",
    loadingHint: "Loading...",
    emptyHint: "No results",
    download: "Download",
    downloading: "Downloading",
    update: "Update",
    updating: "Updating...",
    loadMore: "Load More",
    meta: "{author} · ★ {stars} · {installs} installs",
    source: "Source: {source}"
  },
  local: {
    title: "Local Skills",
    hint: "To import local skills, select the folder containing SKILL.md.",
    scanning: "Scanning local skills...",
    emptyHint: "No local skills found. Try downloading some from the Market.",
    install: "Install to IDE",
    import: "Import Local Skill",
    selectSkillDir: "Select Skill Directory",
    processing: "Processing...",
    usedBy: "Linked to {ideList}",
    unused: "Not linked"
  },
  ide: {
    title: "IDE Browser",
    switchHint: "Switch IDE to view its skills.",
    addHint: "Add custom IDE (name + relative skills path).",
    namePlaceholder: "IDE name",
    dirPlaceholder: "e.g. .myide/skills",
    addButton: "Add IDE",
    deleteButton: "Remove",
    loading: "Loading...",
    emptyHint: "No skills for this IDE",
    sourceLink: "Linked",
    sourceLocal: "Local",
    uninstall: "Uninstall"
  },
  installModal: {
    title: "Select target IDEs",
    selectAll: "Select all",
    cancel: "Cancel",
    confirm: "Install",
    needSelect: "Select at least one IDE"
  },
  uninstallModal: {
    title: "Confirm uninstall",
    hint: "This will remove the directory or symlink. This cannot be undone.",
    cancel: "Cancel",
    confirm: "Uninstall"
  },
  loading: {
    title: "Processing"
  },
  messages: {
    downloaded: "Downloaded to {path}",
    updated: "Updated to {path}",
    installed: "Installed to {ide}",
    installing: "Installing...",
    uninstalling: "Uninstalling...",
    importing: "Importing...",
    handled: "Handled {linked} targets, skipped {skipped} targets.",
    imported: "Successfully imported {success} skills, failed {failed}."
  },
  errors: {
    fillIde: "Please fill in IDE name and directory.",
    ideExists: "IDE name already exists",
    selectValidIde: "Select a valid IDE",
    selectAtLeastOne: "Select at least one IDE",
    searchFailed: "Search failed. Please try again.",
    downloadFailed: "Download failed.",
    updateFailed: "Update failed.",
    scanFailed: "Failed to scan local skills.",
    installFailed: "Installation failed.",
    uninstallFailed: "Uninstallation failed.",
    importFailed: "Import failed."
  },
  update: {
    available: "New version available: {version}",
    view: "View Release"
  },
  marketSettings: {
    title: "Market Settings",
    online: "Online",
    unavailable: "Unavailable",
    needsKey: "Needs API Key",
    apiKey: "API Key",
    apiKeyPlaceholder: "Enter API Key",
    cancel: "Cancel",
    save: "Save"
  }
};
