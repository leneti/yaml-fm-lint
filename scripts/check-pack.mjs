import { execFileSync } from "child_process";

const expected = [
  "LICENSE",
  "README.md",
  "cli.js",
  "config/default.json",
  "errors.js",
  "index.js",
  "package.json",
];

const npm = process.platform === "win32" ? "npm.cmd" : "npm";
const out = execFileSync(npm, ["pack", "--dry-run", "--json"], {
  encoding: "utf8",
  stdio: ["ignore", "pipe", "ignore"],
  shell: process.platform === "win32",
});
const actual = JSON.parse(out)[0].files.map((f) => f.path).sort();

if (JSON.stringify(actual) !== JSON.stringify(expected)) {
  console.error("Packed file list mismatch.");
  console.error("Expected:", expected);
  console.error("Actual:  ", actual);
  process.exit(1);
}
console.log("Pack contents OK:", actual.join(", "));
