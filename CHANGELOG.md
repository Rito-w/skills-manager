# Changelog

All notable changes to this project will be documented in this file. See [standard-version](https://github.com/conventional-changelog/standard-version) for commit guidelines.

### [0.3.18](https://github.com/Rito-w/skills-manager/compare/v0.3.16...v0.3.18) (2026-03-11)


### Features

* add batch adopt for unmanaged IDE skills ([1e252ac](https://github.com/Rito-w/skills-manager/commit/1e252ac1a03ee4a30f1a704d26312fb2467a87b1))
* add batch uninstall for IDE skills ([5d924ed](https://github.com/Rito-w/skills-manager/commit/5d924ed7aef77ef9b18cc3bbaac1908873b6bc19))


### Bug Fixes

* symlink removal error on macOS (os error 20) ([8b29d06](https://github.com/Rito-w/skills-manager/commit/8b29d069c585054e7a9a9fc27f6b889ecf572b0f))

### [0.3.17](https://github.com/Rito-w/skills-manager/compare/v0.3.16...v0.3.17) (2026-03-11)

### Features

* add batch adopt for unmanaged IDE skills
* add batch uninstall for IDE skills with multi-select support

### Bug Fixes

* fix symlink removal error on macOS (os error 20)

### [0.3.16](https://github.com/Rito-w/skills-manager/compare/v0.3.15...v0.3.16) (2026-03-11)

### Features

* support absolute paths and WSL UNC paths for custom IDE configuration

### [0.3.15](https://github.com/Rito-w/skills-manager/compare/v0.3.14...v0.3.15) (2026-03-10)

### Bug Fixes

* fix macOS ENOTDIR when adopting a symlinked IDE skill by removing symlink paths safely

### [0.3.14](https://github.com/Rito-w/skills-manager/compare/v0.3.13...v0.3.14) (2026-03-09)

### Bug Fixes

* fix local install validation to allow symlinked manager dir outside home (validate against ~/.skills-manager/skills canonical path)

### [0.3.13](https://github.com/Rito-w/skills-manager/compare/v0.3.12...v0.3.13) (2026-03-09)

### Bug Fixes

* allow custom IDE install targets under user home without hardcoded base-dir allowlist
* show backend tauri string errors in UI instead of generic install failed

### [0.3.11](https://github.com/Rito-w/skills-manager/compare/v0.3.10...v0.3.11) (2026-03-06)


### Bug Fixes

* use bash for staging assets on windows ([d19069d](https://github.com/Rito-w/skills-manager/commit/d19069d31707a1a9dee9a4fae6dc67c07f3116c8))

### [0.3.10](https://github.com/Rito-w/skills-manager/compare/v0.3.9...v0.3.10) (2026-03-06)


### Bug Fixes

* aggregate updater manifest across platforms ([f466783](https://github.com/Rito-w/skills-manager/commit/f466783eaed05ba1c702be5a0daf18363f127058))
* **updater:** surface silent update installation errors to user UI ([aa275d1](https://github.com/Rito-w/skills-manager/commit/aa275d1bb7898030662c0dc6434e21645838165f))

### [0.3.9](https://github.com/Rito-w/skills-manager/compare/v0.3.8...v0.3.9) (2026-03-06)


### Bug Fixes

* install webkit 4.1 deps for linux updater builds ([35ccdb6](https://github.com/Rito-w/skills-manager/commit/35ccdb654339cebf093f0d82a546d10a9acf66dc))

### [0.3.8](https://github.com/Rito-w/skills-manager/compare/v0.3.7...v0.3.8) (2026-03-06)


### Bug Fixes

* repair release workflows for updater builds ([e9420b3](https://github.com/Rito-w/skills-manager/commit/e9420b3b7e47aeb6a0f362d104d23a63aa4a7e60))

### [0.3.7](https://github.com/Rito-w/skills-manager/compare/v0.3.6...v0.3.7) (2026-03-06)


### Bug Fixes

* allow manual updater release runs ([ffd7211](https://github.com/Rito-w/skills-manager/commit/ffd72113e18bc1b3840152dec361333b71721863))

### [0.3.6](https://github.com/Rito-w/skills-manager/compare/v0.3.5...v0.3.6) (2026-03-06)


### Features

* improve skill management and updater release flow ([e64277a](https://github.com/Rito-w/skills-manager/commit/e64277aaa15ca480c0c333b09b35b575abe34435))

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
