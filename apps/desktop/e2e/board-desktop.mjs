import { runDesktopBoardShell } from "./board-desktop-shell.mjs";
import { runDesktopBoardGraph } from "./board-desktop-graph.mjs";
import { runDesktopBoardRun } from "./board-desktop-run.mjs";

export async function runDesktopBoard(session) {
  await runDesktopBoardShell(session);
  await runDesktopBoardGraph(session);
  await runDesktopBoardRun(session);
}
