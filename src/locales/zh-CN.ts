export default {
  app: {
    tabs: {
      local: "已有 Skills",
      market: "Market",
      ide: "IDE 浏览"
    },
    header: {
      language: "语言",
      theme: "主题",
      themeLight: "浅色",
      themeDark: "深色"
    }
  },
  market: {
    title: "Marketplace 搜索",
    searchPlaceholder: "搜索 skills（支持名称 / 描述 / 作者）",
    search: "搜索",
    searching: "搜索中...",
    refresh: "刷新",
    refreshing: "加载中",
    resultsTitle: "结果列表",
    loadingHint: "加载中...",
    emptyHint: "暂无结果",
    download: "下载到本地",
    downloading: "下载中",
    update: "更新",
    updating: "更新中...",
    loadMore: "加载更多",
    meta: "{author} · ★ {stars} · {installs} installs",
    source: "来源：{source}"
  },
  local: {
    title: "已有 Skills",
    hint: "导入本地 Skill 需要选择包含 SKILL.md 的 Skill 文件夹。",
    scanning: "正在扫描本地 Skills...",
    emptyHint: "暂无本地 Skill，请尝试从市场下载。",
    install: "安装到编辑器",
    import: "导入本地 Skill",
    selectSkillDir: "选择 Skill 目录",
    processing: "处理中...",
    usedBy: "已关联 {ideList}",
    unused: "未关联"
  },
  ide: {
    title: "IDE 浏览",
    switchHint: "切换 IDE 查看其技能列表。",
    addHint: "添加自定义 IDE（名称 + 相对用户目录的 skills 路径）。",
    namePlaceholder: "IDE 名称",
    dirPlaceholder: "例如 .myide/skills",
    addButton: "添加 IDE",
    deleteButton: "删除",
    loading: "加载中...",
    emptyHint: "该 IDE 暂无 skills",
    sourceLink: "链接",
    sourceLocal: "本地",
    uninstall: "卸载"
  },
  installModal: {
    title: "选择安装目标 IDE",
    selectAll: "全选",
    cancel: "取消",
    confirm: "确认安装",
    needSelect: "请选择至少一个 IDE"
  },
  uninstallModal: {
    title: "确认卸载",
    hint: "将移除该 IDE 下的技能目录或软链接，无法恢复。",
    cancel: "取消",
    confirm: "确认卸载"
  },
  loading: {
    title: "处理中"
  },
  messages: {
    downloaded: "已下载至 {path}",
    updated: "已更新至 {path}",
    installed: "已安装至 {ide}",
    installing: "正在安装...",
    uninstalling: "正在卸载...",
    importing: "正在导入...",
    handled: "已处理 {linked} 个目标，跳过 {skipped} 个目标。",
    imported: "成功导入 {success} 个 Skill，失败 {failed} 个。"
  },
  errors: {
    searchFailed: "搜索失败，请重试。",
    downloadFailed: "下载失败。",
    updateFailed: "更新失败。",
    scanFailed: "扫描本地 Skill 失败。",
    installFailed: "安装失败。",
    uninstallFailed: "卸载失败。",
    importFailed: "导入失败。",
    fillIde: "请填写编辑器名称和目录。",
    ideExists: "IDE 名称已存在",
    selectValidIde: "请选择有效的 IDE",
    selectAtLeastOne: "请选择至少一个 IDE"
  },
  update: {
    available: "发现新版本: {version}",
    view: "查看更新",
    install: "立即更新"
  },
  marketSettings: {
    title: "市场管理",
    online: "在线",
    unavailable: "暂不可用",
    needsKey: "需要 API Key",
    apiKey: "API Key",
    apiKeyPlaceholder: "请输入 API Key",
    cancel: "取消",
    save: "保存"
  }
};
