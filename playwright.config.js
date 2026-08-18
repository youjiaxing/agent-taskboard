const { defineConfig } = require("@playwright/test");

const prototypeUrl = "http://127.0.0.1:4173";

module.exports = defineConfig({
  testDir: ".",
  testMatch: ["prototypes/main-shell-hi-fi/e2e.spec.js"],
  workers: 1,
  use: {
    channel: "chrome"
  },
  webServer: {
    command: "PORT=4173 ./prototypes/main-shell-hi-fi/serve.sh",
    url: prototypeUrl,
    reuseExistingServer: false
  }
});
