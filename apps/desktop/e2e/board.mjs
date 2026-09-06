import { createBoardSession } from "./board-harness.mjs";
import { runDesktopBoard } from "./board-desktop.mjs";
import { runMobileBoard } from "./board-mobile.mjs";

const session = await createBoardSession();
await session.configurePage();
try {
  await runDesktopBoard(session);
  await session.switchMobile();
  await runMobileBoard(session);
} finally {
  await session.close();
}
console.log("board e2e ok");
