import type { AppearancePreference } from "./protocol";
import type { ShellShortcutId } from "./shortcuts";

export type StartupCopy = {
  back: string;
  more: string;
  appearance: string;
  appearanceSystem: string;
  appearanceLight: string;
  appearanceDark: string;
  appearanceWarm: string;
  hostStartup: string;
  hostAndClient: string;
  clientOnly: string;
  hostModeHelp: string;
  hostModeActiveRuns: string;
  restartToApply: string;
  startAtLogin: string;
  startAtLoginHelp: string;
  desktopStartupBrowser: string;
  mobileWorkspace: string;
  mobileNav: string;
  mobileHistory: string;
  rereadLaunchEnvironment: string;
  launchEnvironmentIdle: string;
  launchEnvironmentReady: string;
  launchEnvironmentFailed: string;
  close: string;
  activeRunCount: string;
  interruptedRunCount: string;
  removeProjectBlocked: string;
  stopRunTitle: string;
  stopRunBody: string;
  stopRunConfirm: string;
  revokeClientTitle: string;
  revokeClientBody: string;
  revokeClientConfirm: string;
  updateBlockedBody: string;
  updateRetry: string;
  updateInstallingBody: string;
  shortcuts: Record<ShellShortcutId, string>;
  shortcutsScope: string;
};

const catalog: Record<"zh-CN" | "en", StartupCopy> = {
  "zh-CN": {
    back: "返回",
    more: "更多",
    appearance: "外观",
    appearanceSystem: "跟随系统",
    appearanceLight: "素纸",
    appearanceDark: "素纸夜间",
    appearanceWarm: "暖纸",
    hostStartup: "Host 启动",
    hostAndClient: "Host 与 Client",
    clientOnly: "仅作为 Client",
    hostModeHelp: "仅作为 Client 时不会启动本机 Host、Tracker、Agent 或 10529 回环页，仍可连接已配对的远程 Host。",
    hostModeActiveRuns: "还有运行中的 Run，不能切换为仅 Client。请先让它们结束或停止。",
    restartToApply: "重启应用后生效。",
    startAtLogin: "登录时自动启动",
    startAtLoginHelp: "默认关闭。开启后由系统登录启动项拉起 Agent Taskboard。",
    desktopStartupBrowser: "Host 启动模式和系统启动项只能在桌面应用中修改。",
    mobileWorkspace: "专注工作区",
    mobileNav: "手机一级视图",
    mobileHistory: "历史",
    rereadLaunchEnvironment: "重新读取启动环境",
    launchEnvironmentIdle: "尚未手动重新读取。",
    launchEnvironmentReady: "启动环境已更新；之后的 Agent 探测和 Run 会使用新环境。",
    launchEnvironmentFailed: "重新读取失败；已保留上一次可用的内存快照。",
    close: "关闭",
    activeRunCount: "Host 报告的活跃 Run：{count}",
    interruptedRunCount: "将被中断的活跃 Run：{count}",
    removeProjectBlocked: "这个 Project 仍有活跃 Run，当前不能移除。请先让这些 Run 结束或逐一停止。",
    stopRunTitle: "停止 Run？",
    stopRunBody: "停止后，这条 Run 的 Agent 进程将结束；Issue 不会因此关闭。",
    stopRunConfirm: "停止 Run",
    revokeClientTitle: "撤销 Client？",
    revokeClientBody: "撤销 {name} 后，它将立即失去访问权限；如需再次连接，必须重新配对。",
    revokeClientConfirm: "撤销 Client",
    updateBlockedBody: "安装更新会影响当前 Host 上的活跃 Run。请先让这些 Run 结束或逐一停止，再重试安装。",
    updateRetry: "重新检查并安装",
    updateInstallingBody: "正在安装更新。此过程不能取消，请保持应用打开。",
    shortcuts: {
      help: "打开或关闭键盘帮助",
      search: "聚焦 Issue 搜索",
      "next-card": "在看板卡片间向后移动",
      "previous-card": "在看板卡片间向前移动",
      "open-card": "打开当前聚焦的卡片",
      dismiss: "关闭菜单、弹层或帮助",
    },
    shortcutsScope: "这些快捷键只在产品壳中生效；焦点在 Embedded Terminal 时，按键全部交给官方 TUI。",
  },
  en: {
    back: "Back",
    more: "More",
    appearance: "Appearance",
    appearanceSystem: "Follow system",
    appearanceLight: "Plain paper",
    appearanceDark: "Plain paper night",
    appearanceWarm: "Warm paper",
    hostStartup: "Host startup",
    hostAndClient: "Host and Client",
    clientOnly: "Client only",
    hostModeHelp: "Client only does not start the local Host, Tracker, Agent, or port 10529 loopback page. Paired remote Hosts remain available.",
    hostModeActiveRuns: "Client only cannot be enabled while Runs are active. Let them finish or stop them first.",
    restartToApply: "Restart the app to apply this change.",
    startAtLogin: "Start at login",
    startAtLoginHelp: "Off by default. When enabled, the system login item starts Agent Taskboard.",
    desktopStartupBrowser: "Host startup mode and the system login item can only be changed in the desktop app.",
    mobileWorkspace: "Focus workspace",
    mobileNav: "Mobile primary views",
    mobileHistory: "History",
    rereadLaunchEnvironment: "Reread launch environment",
    launchEnvironmentIdle: "The launch environment has not been manually reread yet.",
    launchEnvironmentReady: "The launch environment was updated. Later Agent probes and Runs use the new environment.",
    launchEnvironmentFailed: "Rereading failed. The last usable in-memory snapshot was kept.",
    close: "Close",
    activeRunCount: "Active Runs reported by the Host: {count}",
    interruptedRunCount: "Active Runs that will be interrupted: {count}",
    removeProjectBlocked: "This Project still has active Runs and cannot be removed. Let them finish or stop them first.",
    stopRunTitle: "Stop Run?",
    stopRunBody: "Stopping ends this Run's Agent process. It does not close the Issue.",
    stopRunConfirm: "Stop Run",
    revokeClientTitle: "Revoke Client?",
    revokeClientBody: "Revoking {name} removes its access immediately. It must pair again before reconnecting.",
    revokeClientConfirm: "Revoke Client",
    updateBlockedBody: "Installing the update affects active Runs on this Host. Let them finish or stop them, then retry installation.",
    updateRetry: "Check and install again",
    updateInstallingBody: "The update is being installed. This cannot be cancelled; keep the app open.",
    shortcuts: {
      help: "Open or close keyboard help",
      search: "Focus the Issue search",
      "next-card": "Move to the next board card",
      "previous-card": "Move to the previous board card",
      "open-card": "Open the focused card",
      dismiss: "Close a menu, dialog, or help",
    },
    shortcutsScope: "These shortcuts apply to the product shell only; while the Embedded Terminal has focus every key goes to the official TUI.",
  },
};

export function startupCopy(language: "zh-CN" | "en"): StartupCopy {
  return catalog[language];
}

export function appearancePreferenceLabel(
  copy: StartupCopy,
  preference: AppearancePreference,
): string {
  if (preference === "system") return copy.appearanceSystem;
  if (preference === "light") return copy.appearanceLight;
  if (preference === "dark") return copy.appearanceDark;
  return copy.appearanceWarm;
}
