/**
 * Installs the icons of MCPad from their source repositories.
 *
 * The icon artwork belongs to its authors, not to MCPad. The repository keeps
 * only the links, the pinned revision and the checksums, so the artwork itself
 * never enters the version control. The installer downloads the files, checks
 * their SHA-256 and writes them next to the code that uses them.
 *
 * Usage:
 *   node scripts/install-icons.mjs            install the icons
 *   node scripts/install-icons.mjs --check    only verify what is installed
 *   node scripts/install-icons.mjs --update   refresh the manifest
 *
 * The manifest `scripts/icons.manifest.json` is the source of truth: it names
 * the repository, the revision and the checksum of every file.
 */

import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, "..");

const MANIFEST_PATH = join(HERE, "icons.manifest.json");
const VENDOR_DIR = join(ROOT, "apps/desktop/src/icons/vendor");
const APP_SOURCE_DIR = join(ROOT, "apps/desktop/src-tauri/icons/source");
const NOTICE_PATH = join(ROOT, "THIRD_PARTY_NOTICES.md");

/** The brand colour of the application tile. */
const BRAND_TOP = "#3b82f6";
const BRAND_BOTTOM = "#1d4ed8";

/** Reads the manifest of the icons. */
function readManifest() {
  return JSON.parse(readFileSync(MANIFEST_PATH, "utf8"));
}

/** The address of a file inside the pinned revision of the source repository. */
function sourceUrl(source, path) {
  return `https://raw.githubusercontent.com/${source.repository}/${source.revision}/${path}`;
}

/** Downloads a file and returns its content. */
async function download(url) {
  const response = await fetch(url, {
    headers: { "user-agent": "mcpad-icon-installer" },
  });
  if (!response.ok) {
    throw new Error(`${url} answered ${response.status}`);
  }
  return Buffer.from(await response.arrayBuffer());
}

/** The SHA-256 of a buffer, as a hex string. */
function sha256(buffer) {
  return createHash("sha256").update(buffer).digest("hex");
}

/**
 * Extracts the inner markup of an SVG file.
 *
 * The comment before the `<svg>` element carries the tags of the icon and is
 * of no use inside the application.
 */
function innerMarkup(svg) {
  const text = svg.toString("utf8");
  const start = text.indexOf("<svg");
  if (start < 0) {
    throw new Error("the file is not an SVG");
  }
  const open = text.indexOf(">", start);
  const end = text.lastIndexOf("</svg>");
  if (open < 0 || end < 0) {
    throw new Error("the SVG is broken");
  }
  return text.slice(open + 1, end);
}

/**
 * Builds the source icon of the application.
 *
 * The tile belongs to MCPad, the document inside it comes from the icon set of
 * the source repository. Both parts are named in the notices.
 */
function buildAppIcon(glyph, glyphPath) {
  const size = 1024;
  const scale = 19.2;
  const offset = (size - 24 * scale) / 2;

  return `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 ${size} ${size}">
  <defs>
    <linearGradient id="tile" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="${BRAND_TOP}" />
      <stop offset="1" stop-color="${BRAND_BOTTOM}" />
    </linearGradient>
  </defs>
  <rect x="0" y="0" width="${size}" height="${size}" rx="224" fill="url(#tile)" />
  <g transform="translate(${offset.toFixed(2)} ${offset.toFixed(2)}) scale(${scale})" fill="#ffffff">
${glyph}
  </g>
</svg>
`;
}

/** Writes the notices about the third party artwork. */
function writeNotices(manifest, license) {
  const { source } = manifest;
  const text = `# Third party notices

MCPad itself is licensed under the Apache License 2.0 (see \`LICENSE\`). The
artwork of the icons is not part of MCPad: it belongs to its authors and is
used under the license of the source repository. The links, the pinned revision
and the checksums live in \`scripts/icons.manifest.json\`, the files themselves
are downloaded by \`npm run icons:install\`.

## ${source.name}

* Repository: ${source.repository}
* Revision: \`${source.revision}\`
* License: ${source.license}
* Copyright: ${source.copyright}
* License text: ${source.licenseUrl}

${license.trim()}
`;
  writeFileSync(NOTICE_PATH, text, "utf8");
}

/** Downloads every file of the manifest and checks it against the checksum. */
async function install(manifest, { check }) {
  const { source, icons, app } = manifest;
  const problems = [];

  mkdirSync(VENDOR_DIR, { recursive: true });
  mkdirSync(APP_SOURCE_DIR, { recursive: true });

  for (const icon of icons) {
    const url = sourceUrl(source, `${source.path}/${icon.file}`);
    const target = join(VENDOR_DIR, `${icon.name}.svg`);

    if (check) {
      if (!existsSync(target)) {
        problems.push(`${icon.name}: ${target} is missing`);
        continue;
      }
      const digest = sha256(readFileSync(target));
      if (digest !== icon.sha256) {
        problems.push(`${icon.name}: the checksum does not match`);
      }
      continue;
    }

    const content = await download(url);
    const digest = sha256(content);
    if (digest !== icon.sha256) {
      problems.push(`${icon.name}: the checksum of the download does not match`);
      continue;
    }
    writeFileSync(target, content);
    console.log(`installed ${icon.name} <- ${icon.file}`);
  }

  const appUrl = sourceUrl(source, `${app.path}/${app.file}`);
  const appTarget = join(APP_SOURCE_DIR, app.file);

  if (check) {
    if (!existsSync(appTarget)) {
      problems.push(`app icon: ${appTarget} is missing`);
    } else if (sha256(readFileSync(appTarget)) !== app.sha256) {
      problems.push("app icon: the checksum does not match");
    }
  } else {
    const content = await download(appUrl);
    if (sha256(content) !== app.sha256) {
      problems.push("app icon: the checksum of the download does not match");
    } else {
      writeFileSync(appTarget, content);
      const glyph = innerMarkup(content);
      writeFileSync(
        join(APP_SOURCE_DIR, "app-icon.svg"),
        buildAppIcon(glyph, app.file),
        "utf8",
      );
      console.log(`installed the app icon <- ${app.file}`);
    }
  }

  if (!check) {
    const license = await download(sourceUrl(source, "LICENSE"));
    writeNotices(manifest, license.toString("utf8"));
    console.log("written THIRD_PARTY_NOTICES.md");
  }

  return problems;
}

/** Downloads every file again and writes the manifest with the checksums. */
async function updateManifest(manifest) {
  const { source, icons, app } = manifest;
  const next = { ...manifest, icons: [], app: { ...app } };

  for (const icon of icons) {
    const url = sourceUrl(source, `${source.path}/${icon.file}`);
    const content = await download(url);
    next.icons.push({ ...icon, sha256: sha256(content) });
    console.log(`hashed ${icon.file}`);
  }

  const appContent = await download(sourceUrl(source, `${app.path}/${app.file}`));
  next.app.sha256 = sha256(appContent);

  writeFileSync(MANIFEST_PATH, `${JSON.stringify(next, null, 2)}\n`, "utf8");
  console.log(`written ${MANIFEST_PATH}`);
}

/** The whole run of the installer. */
async function main() {
  const args = new Set(process.argv.slice(2));
  const manifest = readManifest();

  if (args.has("--update")) {
    await updateManifest(manifest);
    return;
  }

  const check = args.has("--check");
  const problems = await install(manifest, { check });

  if (problems.length) {
    console.error("the installed icons do not match the manifest:");
    for (const problem of problems) console.error(`  ${problem}`);
    process.exitCode = 1;
    return;
  }

  console.log(check ? "the installed icons match the manifest" : "the icons are ready");
}

await main();
