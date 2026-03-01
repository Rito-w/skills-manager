# Changelog

All notable changes to this project will be documented in this file. See [standard-version](https://github.com/conventional-changelog/standard-version) for commit guidelines.

### [0.3.3](https://github.com/Rito-w/skills-manager/compare/v0.3.2...v0.3.3) (2026-03-01)


### Features

* 启动时自动检测更新 ([a66e070](https://github.com/Rito-w/skills-manager/commit/a66e070574c246a85b1e1330cc0758d08b74ad9e))

### [0.3.2](https://github.com/Rito-w/skills-manager/compare/v0.3.1...v0.3.2) (2026-03-01)


### Features

* 添加 API Key 可见性切换以及完善自定义 IDE 路径的校验逻辑 ([3d81bf0](https://github.com/Rito-w/skills-manager/commit/3d81bf02efeaed9618a6cdec5ec083210cdd2510))
* 添加设置页面，支持版本检查和更新 ([b0f52f7](https://github.com/Rito-w/skills-manager/commit/b0f52f770e218e7889a7e9c7e386d169aed1c346))


### Bug Fixes

* add camelCase serde attribute to response structs ([df38df7](https://github.com/Rito-w/skills-manager/commit/df38df702120d5b148824f990f4dcacd6a2ea805))
* add symlink attack protection in link_local_skill ([d933a63](https://github.com/Rito-w/skills-manager/commit/d933a63dc5b830af896da16704905fa1d80c347a))
* check all active statuses in addToDownloadQueue ([2751866](https://github.com/Rito-w/skills-manager/commit/2751866f02c3212c1a29c8a73909bfafc5d8edec))
* cleanup timer in error branch to prevent memory leak ([c3c1105](https://github.com/Rito-w/skills-manager/commit/c3c1105006851944195947bb38e439311ee6d05e))
* use defer pattern to ensure temp dir cleanup in download_skill_to_dir ([2e3b7ca](https://github.com/Rito-w/skills-manager/commit/2e3b7caff9dc16a8777fcc91bded3a6c4d8bc848))
* 修复 Windows 路径防注入拦截和 setTimeout 内存泄漏 ([e096caa](https://github.com/Rito-w/skills-manager/commit/e096caa12dd9db1de4b30440077e4e26ee7fd7b7))
* 修复安全漏洞和代码质量问题 ([ae73fc9](https://github.com/Rito-w/skills-manager/commit/ae73fc9ec598aa01446860dc11166acd3f98eda8))

## [0.2.1] - 2026-02-06
- Release workflow updated for tag-based builds.
- Added Kiro IDE support.
- UI improvements and i18n/theme toggles.
