import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const tag = process.argv[2];

const nodeCmd = process.execPath;
const npmCmd = process.platform === "win32" ? "npm.cmd" : "npm";

const syncResult = spawnSync(
    nodeCmd,
    [path.join(__dirname, "set-version-from-tag.mjs"), ...(tag ? [tag] : [])],
    {
        stdio: "inherit",
    },
);

if (syncResult.status !== 0) {
    process.exit(syncResult.status ?? 1);
}

const buildResult = spawnSync(npmCmd, ["run", "tauri", "build"], {
    stdio: "inherit",
});

process.exit(buildResult.status ?? 1);
