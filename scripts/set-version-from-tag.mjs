import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const repoRoot = path.resolve(__dirname, "..");

const packageJsonPath = path.join(repoRoot, "package.json");
const cargoTomlPath = path.join(repoRoot, "src-tauri", "Cargo.toml");
const tauriConfPath = path.join(repoRoot, "src-tauri", "tauri.conf.json");

const rawTag =
    process.argv[2] ??
    process.env.RELEASE_TAG ??
    process.env.GITHUB_REF_NAME ??
    process.env.CI_COMMIT_TAG ??
    process.env.TAG_NAME;

if (!rawTag) {
    console.error(
        "No release tag found. Pass one explicitly, e.g. `node scripts/set-version-from-tag.mjs v1.2.3`, or set RELEASE_TAG/GITHUB_REF_NAME.",
    );
    process.exit(1);
}

const semverMatch = rawTag.trim().match(
    /^v?(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?)$/,
);

if (!semverMatch) {
    console.error(
        `Invalid tag format: "${rawTag}". Expected semver like v1.2.3 or 1.2.3.`,
    );
    process.exit(1);
}

const version = semverMatch[1];

async function updatePackageJson() {
    const content = await readFile(packageJsonPath, "utf8");
    const parsed = JSON.parse(content);
    parsed.version = version;
    await writeFile(packageJsonPath, `${JSON.stringify(parsed, null, 2)}\n`, "utf8");
}

async function updateCargoToml() {
    const content = await readFile(cargoTomlPath, "utf8");
    const packageSectionPattern = /(\[package\][\s\S]*?)(^\s*version\s*=\s*")[^"]+("\s*$)/m;
    let matched = false;
    let changed = false;
    const updated = content.replace(
        packageSectionPattern,
        (match, sectionPrefix, versionPrefix, versionSuffix) => {
            matched = true;
            const next = `${sectionPrefix}${versionPrefix}${version}${versionSuffix}`;
            changed = next !== match;
            return next;
        },
    );

    if (!matched) {
        throw new Error("Failed to update version in src-tauri/Cargo.toml");
    }

    if (!changed) {
        return;
    }

    await writeFile(cargoTomlPath, updated, "utf8");
}

async function updateTauriConf() {
    const content = await readFile(tauriConfPath, "utf8");
    const parsed = JSON.parse(content);
    parsed.version = version;
    await writeFile(tauriConfPath, `${JSON.stringify(parsed, null, 2)}\n`, "utf8");
}

await updateCargoToml();
await updatePackageJson();
await updateTauriConf();

console.log(`Synchronized release version to ${version}`);
