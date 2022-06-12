#! /usr/bin/env node
import path from "path";
import { fileURLToPath } from "url";
import { readFileSync, lstatSync, readdir, readFile, readdirSync } from "fs";
import { promisify } from "util";
import fm from "front-matter";
import { lint } from "yaml-lint";
import chalk from "chalk";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const readdirPromise = promisify(readdir);
const readFilePromise = promisify(readFile);

const args = getArguments();
const defaultConfig = JSON.parse(
  readFileSync(`${__dirname}\\config\\default.json`)
);
let config = args.config
  ? { ...defaultConfig, ...JSON.parse(readFileSync(args.config)) }
  : { ...defaultConfig };
try {
  const mConfig = JSON.parse(
    readFileSync(`${process.cwd()}\\.yaml-fm-lint.json`)
  );
  config = { ...config, ...mConfig };
} catch (_) {}
const allExcludedDirs = [...config.excludeDirs, ...config.extraExcludeDirs];

(() => {
  console.time("Linting took");
  (!args.recursive && !args.r
    ? lintNonRecursively(args.path)
    : lintRecursively(args.path)
  )
    .catch(console.log)
    .finally(endOfProcess);
})();

function endOfProcess() {
  if (!process.exitCode) {
    console.log(
      `${chalk.green("All parsed files have valid front matter.")}`
    );
  }
  console.timeEnd("Linting took");
}

/**
 * Retrieves arguments from the command line
 * @returns {Object} - arguments object
 */
function getArguments() {
  const args = process.argv.slice(2);
  let pathRead = false;
  const argv = args.reduce((acc, curr) => {
    let [key, value] = curr.split("=");
    if (key.startsWith("-")) {
      key = key.replace("-", "");
    } else if (!key.startsWith("--")) {
      if (!pathRead) {
        value = key;
        key = "path";
        pathRead = true;
      } else {
        console.log(
          `${chalk.red("Invalid argument:")} ${chalk.yellow(
            `\"${curr}\"`
          )}. Only one path argument is allowed.`
        );
        process.exit(1);
      }
    } else {
      key = key.replace(/^--/, "");
    }
    acc[key] = value ?? true;
    return acc;
  }, {});

  if (!pathRead) {
    console.log(
      `${chalk.red(
        "Invalid arguments:"
      )} No path argument found. Please specify a path.`
    );
    process.exit(1);
  }

  return argv;
}

/**
 * Checks if the front matter exists.
 * @param {string} fm - front matter string
 */
function checkFrontMatterExists(fm, filePath) {
  if (!fm) {
    console.log(
      `${chalk.red(
        "YAMLException:"
      )} front matter not found in ${process.cwd()}\\${filePath}. Make sure front matter is at the beginning of the file.\n`
    );
    process.exitCode = 1;
    return false;
  }
  return true;
}

/**
 * Checks if all required attributes are present in the front matter.
 * @param {Object} attributes front matter attributes
 */
function checkAttributes(attributes, filePath) {
  const missingAttributes =
    config.requiredAttributes?.filter(
      (attribute) => !attributes.hasOwnProperty(attribute)
    ) ?? [];

  if (missingAttributes.length > 0) {
    console.log(
      `${chalk.red(
        "YAMLException:"
      )} missing attributes in ${process.cwd()}\\${filePath}: ${missingAttributes.join(
        ", "
      )}\n`
    );
    process.exitCode = 1;
  }
}

/**
 * Checks if there are any quotes in the front matter and warns against using them.
 * @param {string} fm - (front matter) string to check
 */
function checkQuotes(fm, filePath) {
  const lines = fm.split("\n");
  const redundantQuotes = [];
  const quoteRegexp =
    /(^[\"\'])|([\"\']\s*\r)|([\"\']\s*:)|(:\s+[\"\'])|([\"\']$)/g;

  for (let i = 0; i < lines.length; i++) {
    let match;
    while ((match = quoteRegexp.exec(lines[i])) !== null) {
      quoteRegexp.lastIndex = match.index + 1;
      const row = i + 2;
      const col = match.index + match[0].search(/[\"\']/) + 2;
      redundantQuotes.push({
        row,
        col,
        snippet: `${i > 0 ? `${i + 1} | ${lines[i - 1]}\n` : ""}${i + 2} | ${
          lines[i]
        }\n${"----^".padStart(col + 3, "-")}\n${
          i + 1 < lines.length ? `${i + 3} | ${lines[i + 1]}\n` : ""
        }`,
      });
    }
  }

  if (redundantQuotes.length > 0) {
    const quotes = redundantQuotes.reduce((acc, curr) => {
      return `${acc}\n  at ${process.cwd()}\\${filePath}:${curr.row}:${
        curr.col
      }.\n\n${curr.snippet}\n`;
    }, "");
    console.log(
      `${chalk.red(
        "YAMLException:"
      )} there should be no redundant quotes.\n${quotes}`
    );
    process.exitCode = 1;
  }
}

/**
 * Checks if there are any incorrectly spaced colons in the front matter and shows a warning.
 * @param {string} fm - front matter string
 * @param {string} filePath - path to file
 */
function checkNoSpacesBeforeColon(fm, filePath) {
  const lines = fm.split("\n");
  const spacesBeforeColon = [];
  const spaceBeforeColonRegexp = /\s+:/g;

  for (let i = 0; i < lines.length; i++) {
    let match;
    while ((match = spaceBeforeColonRegexp.exec(lines[i])) !== null) {
      const row = i + 2;
      const col = match.index + match[0].search(/\s+/) + 2;
      spacesBeforeColon.push({
        row,
        col,
        snippet: `${i > 0 ? `${i + 1} | ${lines[i - 1]}\n` : ""}${i + 2} | ${
          lines[i]
        }\n${"----^".padStart(col + 3, "-")}\n${
          i + 1 < lines.length ? `${i + 3} | ${lines[i + 1]}\n` : ""
        }`,
      });
    }
  }

  if (spacesBeforeColon.length > 0) {
    const spaces = spacesBeforeColon.reduce((acc, curr) => {
      return `${acc}\n  at ${process.cwd()}\\${filePath}:${curr.row}:${
        curr.col
      }.\n\n${curr.snippet}\n`;
    }, "");
    console.log(
      `${chalk.red(
        "YAMLException:"
      )} there should be no spaces before colons.\n${spaces}`
    );
    process.exitCode = 1;
  }
}

/**
 * Lints the front matter of all files in a directory non-recursively.
 * @param {string} path - path to file or directory
 * @returns {Promise<string?>} - null if everything is ok, otherwise error message
 */
function lintNonRecursively(path) {
  return new Promise((resolve, reject) => {
    if (lstatSync(path).isDirectory()) {
      const files = readdirSync(path, "utf8");

      const promiseArr = [];
      for (const file of files) {
        if (file.endsWith(".md")) {
          promiseArr.push(
            lintFile(`${path === "." ? "" : `${path}\\`}${file}`)
          );
        }
      }

      if (!promiseArr.length) {
        console.log(`No markdown files found in ${process.cwd()}\\${path}.`);
        resolve();
      }

      Promise.all(promiseArr).then(resolve).catch(reject);
    } else if (path.endsWith(".md")) {
      lintFile(path).then(resolve).catch(reject);
    } else {
      reject(`${chalk.red("YAMLException:")} ${path} is not a markdown file.`);
    }
  });
}

/**
 * Lints the front matter of all files in a directory recursively.
 * @param {string} path - path to file or directory
 */
function lintRecursively(path) {
  return new Promise((resolve, reject) => {
    if (lstatSync(path).isDirectory()) {
      if (
        allExcludedDirs.some((ignoredDirectory) =>
          path.includes(ignoredDirectory)
        ) &&
        !config.includeDirs.some((includedDirectory) =>
          path.includes(includedDirectory)
        )
      )
        resolve();

      readdirPromise(path, "utf8")
        .then((files) => {
          const promiseArr = [];
          for (const file of files) {
            promiseArr.push(
              lintRecursively(`${path === "." ? "" : `${path}\\`}${file}`)
            );
          }

          Promise.all(promiseArr).then(resolve).catch(reject);
        })
        .catch(reject);
    } else if (path.endsWith(".md")) {
      lintFile(path).then(resolve).catch(reject);
    } else resolve();
  });
}

function lintFile(filePath) {
  return new Promise((resolve, reject) => {
    readFilePromise(filePath, "utf8")
      .then((data) => {
        const content = fm(data);

        if (!checkFrontMatterExists(content.frontmatter, filePath)) resolve();

        lint(content.frontmatter)
          .then(() => {
            checkAttributes(content.attributes, filePath);
            checkQuotes(content.frontmatter, filePath);
            checkNoSpacesBeforeColon(content.frontmatter, filePath);
            resolve();
          })
          .catch(reject);
      })
      .catch(reject);
  });
}
