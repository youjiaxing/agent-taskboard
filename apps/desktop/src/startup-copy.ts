export type StartupCopy = {
  hostStartup: string;
  hostAndClient: string;
  clientOnly: string;
  hostModeHelp: string;
  hostModeActiveRuns: string;
  restartToApply: string;
  startAtLogin: string;
  startAtLoginHelp: string;
  desktopStartupBrowser: string;
  rereadLaunchEnvironment: string;
  launchEnvironmentIdle: string;
  launchEnvironmentReady: string;
  launchEnvironmentFailed: string;
};

const catalog: Record<"zh-CN" | "en", StartupCopy> = {
  "zh-CN": {
    hostStartup: "Host 启动",
    hostAndClient: "Host 与 Client",
    clientOnly: "仅作为 Client",
    hostModeHelp: "仅作为 Client 时不会启动本机 Host、Tracker、Agent 或 10529 回环页，仍可连接已配对的远程 Host。",
    hostModeActiveRuns: "还有运行中的 Run，不能切换为仅 Client。请先让它们结束或停止。",
    restartToApply: "重启应用后生效。",
    startAtLogin: "登录时自动启动",
    startAtLoginHelp: "默认关闭。开启后由 macOS 系统启动项拉起 Agent Taskboard。",
    desktopStartupBrowser: "Host 启动模式和系统启动项只能在桌面应用中修改。",
    rereadLaunchEnvironment: "重新读取启动环境",
    launchEnvironmentIdle: "尚未手动重新读取。",
    launchEnvironmentReady: "启动环境已更新；之后的 Agent 探测和 Run 会使用新环境。",
    launchEnvironmentFailed: "重新读取失败；已保留上一次可用的内存快照。",
  },
  en: {
    hostStartup: "Host startup",
    hostAndClient: "Host and Client",
    clientOnly: "Client only",
    hostModeHelp: "Client only does not start the local Host, Tracker, Agent, or port 10529 loopback page. Paired remote Hosts remain available.",
    hostModeActiveRuns: "Client only cannot be enabled while Runs are active. Let them finish or stop them first.",
    restartToApply: "Restart the app to apply this change.",
    startAtLogin: "Start at login",
    startAtLoginHelp: "Off by default. When enabled, the macOS login item starts Agent Taskboard.",
    desktopStartupBrowser: "Host startup mode and the system login item can only be changed in the desktop app.",
    rereadLaunchEnvironment: "Reread launch environment",
    launchEnvironmentIdle: "The launch environment has not been manually reread yet.",
    launchEnvironmentReady: "The launch environment was updated. Later Agent probes and Runs use the new environment.",
    launchEnvironmentFailed: "Rereading failed. The last usable in-memory snapshot was kept.",
  },
};

export function startupCopy(language: "zh-CN" | "en"): StartupCopy {
  return catalog[language];
}
