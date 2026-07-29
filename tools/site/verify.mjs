import { readFile, stat } from "node:fs/promises";
import { resolve } from "node:path";

const root = resolve(process.argv[2] ?? "site");
const failures = [];

function check(condition, message) {
  if (!condition) failures.push(message);
}

async function text(name) {
  try {
    return await readFile(resolve(root, name), "utf8");
  } catch (error) {
    failures.push(`${name}: ${error.message}`);
    return "";
  }
}

async function exists(name) {
  try {
    return (await stat(resolve(root, name))).isFile();
  } catch {
    return false;
  }
}

const [html, css, robots, sitemap, llms] = await Promise.all([
  text("index.html"),
  text("styles.css"),
  text("robots.txt"),
  text("sitemap.xml"),
  text("llms.txt"),
]);

check(/<html lang="en">/.test(html), "index.html must declare English");
check(!/[А-Яа-яЁё]/.test(html), "index.html must not contain Russian copy");
check(
  /rel="canonical" href="https:\/\/tigramaan\.github\.io\/esp32-flasher\/"/.test(
    html,
  ),
  "canonical URL is missing or incorrect",
);
check(
  /name="description"/.test(html) && /property="og:title"/.test(html),
  "description or Open Graph metadata is missing",
);
check(
  /"@type": "SoftwareApplication"/.test(html) &&
    /"@type": "FAQPage"/.test(html),
  "SoftwareApplication or FAQ structured data is missing",
);
check(
  html.includes(
    "https://github.com/tigramaan/esp32-flasher/releases/latest/download/ESP32-Flasher-Windows-x64.exe",
  ),
  "version-independent release download URL is missing",
);
check(
  !/\b(aggregateRating|ratingValue|downloadCount)\b/.test(html),
  "unverified rating or download claims are forbidden",
);
check(
  css.includes("--container-max") &&
    css.includes("@media (max-width: 620px)") &&
    css.includes("min-width: 320px") &&
    css.includes("min-height: 44px"),
  "responsive container/mobile/tap-target contracts are incomplete",
);
check(!css.includes("width: 100vw"), "100vw can cause page overflow");
check(
  robots.includes("sitemap.xml") &&
    sitemap.includes("https://tigramaan.github.io/esp32-flasher/"),
  "robots/sitemap canonical contract is incomplete",
);
check(
  llms.includes("## Primary use cases") &&
    llms.includes("## Important limitations"),
  "llms.txt must include capabilities and limitations",
);

for (const asset of ["mark.svg", "og-image.svg", ".nojekyll"]) {
  check(await exists(asset), `${asset} is missing`);
}

const localReferences = [
  ...html.matchAll(/(?:href|src)="\.\/([^"#?]+)"/g),
].map((match) => match[1]);
for (const asset of new Set(localReferences)) {
  check(await exists(asset), `referenced local asset is missing: ${asset}`);
}

if (failures.length) {
  console.error(`Pages verification failed (${failures.length}):`);
  for (const failure of failures) console.error(`- ${failure}`);
  process.exit(1);
}

console.log(
  `Pages verification PASS: ${root} (English, metadata, structured data, links, assets, responsive contracts)`,
);
