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
    meta: "{author} · ★ {stars} · {installs} installs"
  },
  local: {
    title: "Local Skills",
    hint: "Skills in the local Skills Manager repository.",
    actionsHint: "Downloaded skills will appear here.",
    scanning: "Scanning...",
    emptyHint: "No local skills found",
    install: "Install",
    processing: "Processing",
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
    installing: "Installing...",
    uninstalling: "Uninstalling...",
    downloaded: "Downloaded to {path}",
    updated: "Updated {path}",
    handled: "Linked {linked}, skipped {skipped}"
  },
  errors: {
    fillIde: "Please fill IDE name and directory",
    ideExists: "IDE name already exists",
    selectValidIde: "Select a valid IDE",
    selectAtLeastOne: "Select at least one IDE",
    searchFailed: "Search failed",
    downloadFailed: "Download failed",
    updateFailed: "Update failed",
    scanFailed: "Scan failed",
    installFailed: "Install failed",
    uninstallFailed: "Uninstall failed"
  }
};
