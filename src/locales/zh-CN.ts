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
    meta: "{author} · ★ {stars} · {installs} installs"
  },
  local: {
    title: "已有 Skills",
    hint: "这里展示 Skills Manager 本地仓库中的 skills。",
    actionsHint: "下载到本地仓库后会自动出现。",
    scanning: "扫描中...",
    emptyHint: "暂未扫描到本地 skills",
    install: "安装",
    processing: "处理中",
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
    installing: "安装中...",
    uninstalling: "卸载中...",
    downloaded: "已下载到 {path}",
    updated: "已更新 {path}",
    handled: "已处理 {linked}，跳过 {skipped}"
  },
  errors: {
    fillIde: "请填写 IDE 名称和目录",
    ideExists: "IDE 名称已存在",
    selectValidIde: "请选择有效的 IDE",
    selectAtLeastOne: "请选择至少一个 IDE",
    searchFailed: "搜索失败",
    downloadFailed: "下载失败",
    updateFailed: "更新失败",
    scanFailed: "扫描失败",
    installFailed: "安装失败",
    uninstallFailed: "卸载失败"
  },
  update: {
    available: "发现新版本: {version}",
    view: "查看更新"
  }
};
