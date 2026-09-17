# Issue 131：对齐基准参照物

## 决议

本轮界面对齐采用本机安装的 Codex 桌面版实际界面作为一次性冻结的像素与状态基准，不采用官方公开素材作为像素基准。官方文档或公开材料只用于核对交互含义；后续官方版本更新不改变本轮基准。

这些截图是视觉验收参照物，不是产品实现素材。

## 环境记录

- 捕获日期：2026-09-18（Asia/Shanghai）
- 平台：macOS 26.6.2（Build 25G83）
- 运行时：Codex Framework 153.0.8010.48
- 截图像素尺寸：3572 × 2162
- 基准窗口：本轮截图所示的本机 Codex 桌面窗口；验收时保持相同窗口尺寸与显示缩放

## 冻结截图

| 状态 | 文件 | 说明 | SHA-256 |
| --- | --- | --- | --- |
| 主界面·深色 | [codex-dark-main.png](codex-dark-main.png) | 无右栏、无菜单的空态主界面 | `b3be2d3ea6011b3358ae6a1eccc6511006d2d3489a6a6590181c94a75669c9e6` |
| 主界面·浅色 | [codex-light-main.png](codex-light-main.png) | 无提示卡、无右栏、无菜单的空态主界面 | `5bc6ba2972d9b306d69329137f50ed5ddd7c28bf70f56e5e71b0a8a984e399c9` |
| 专注界面·右栏展开 | [codex-dark-focus-right-panel.png](codex-dark-focus-right-panel.png) | 专注工作区打开右栏分区选择 | `51e97ac2f50a842e4c0a4d104ba9cb4474ae45c233dba2c4a42638e13e8663e0` |
| 设置页 | [codex-dark-settings.png](codex-dark-settings.png) | 设置中的外观页 | `9378284173a49f07f32e6831bc218c4a80a406f5cfee22afa3d172cd558ef708` |
| 弹窗示例 | [codex-dark-popup.png](codex-dark-popup.png) | 左下角账户菜单弹层 | `6f1937c5963f7c1809730427fb392ed20c4da25784ac6ffef132aa78905b705c` |

## 验收约束

- 像素比较以这 5 张冻结图为准；不得以未来官方版本的变化自动更新基线。
- 浅色与深色只要求主界面各一张；设置页、右栏和弹窗使用表中指定状态。
- 实现可复用的设计令牌与组件，但不得复制 Codex 的商标、产品身份、专有代码或专有素材。
