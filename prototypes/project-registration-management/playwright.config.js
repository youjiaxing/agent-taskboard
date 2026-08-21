const { defineConfig } = require("@playwright/test");

module.exports = defineConfig({
  testDir: ".",
  testMatch: ["e2e.spec.js"],
  workers: 1,
  use: { channel: "chrome" },
  webServer: {
    command: "PORT=4173 ./serve.sh",
    url: "http://127.0.0.1:4173",
    reuseExistingServer: false
  }
});
