#! /usr/bin/env node
import path from "path";
import { fileURLToPath } from "url";
import { readFileSync, lstatSync, readdir, readFile, readdirSync } from "fs";
import { promisify } from "util";
import { lint } from "yaml-lint";
import chalk from "chalk";
import fm from "./front-matter/index.js";

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
let errorNumber = 0;

function getSnippet2(lines, col, i) {
  return `${i - 1 > 0 ? `${i - 1} | ${lines[i - 1]}\n` : ""}${i} | ${
    lines[i]
  }\n${"----^".padStart(col + 3 + Math.floor(Math.log10(i)), "-")}\n${
    i + 1 < lines.length ? `${i + 1} | ${lines[i + 1]}\n` : ""
  }`;
}

/**
 * Checks whether there are any missing attributes in the front-matter.
 * @param {string[]} attributes - list of attributes to check for
 * @param {string} filePath - path to file to check
 */
function checkAttributes2(attributes, filePath) {
  const requiredAttributes = config.requiredAttributes;
  const missingAttributes = requiredAttributes.filter(
    (attr) => !attributes.includes(attr)
  );

  if (missingAttributes.length > 0) {
    console.log(
      `${chalk.red(
        "YAMLException:"
      )} missing attributes in ${process.cwd()}\\${filePath}: ${missingAttributes.join(
        ", "
      )}\n`
    );
    process.exitCode = 1;
    errorNumber++;
  }
}

/**
 * Parses given string and returns an object with all front matter attributes and possible special characters surrounding them.
 * @param {string} data - data to parse
 * @param {string} filePath - path to the file
 */
function parseSpecialChars(data, filePath) {
  const lines = data.replace(/\r/g, "").split("\n");
  lines.unshift("");
  const fmClosingTagIndex = lines.indexOf("---", 2);

  if (!lines[1].startsWith("---") || fmClosingTagIndex === -1) {
    console.log(
      `${chalk.red(
        "YAMLException:"
      )} front matter not found in ${process.cwd()}\\${filePath}. Make sure front matter is at the beginning of the file.\n`
    );
    process.exitCode = 1;
    errorNumber++;
    return;
  }

  const fm = lines.slice(0, fmClosingTagIndex + 1);
  const attributes = [];
  const spacesBeforeColon = [];
  const blankLines = [];
  const quotes = [];
  const trailingSpaces = [];
  const brackets = [];
  const curlyBraces = [];
  let match;

  for (let i = 1; i < fm.length - 1; i++) {
    const line = fm[i];

    if (line.match(/^"?\w+"?\s*:/g)) {
      attributes.push(line.split(":")[0].trim());
    }

    // no-empty-lines
    if (line.trim() === "") {
      errorNumber++;
      blankLines.push(i);
      continue;
    }

    // no-whitespace-before-colon
    const wsbcRegex = /\s+:/g;
    while ((match = wsbcRegex.exec(line)) !== null) {
      errorNumber++;
      const row = i;
      const col = match.index + match[0].search(/:/) + 1;
      wsbcRegex.lastIndex = col - 1;
      spacesBeforeColon.push({
        row,
        col,
        snippet: getSnippet2(fm, col, i),
      });
    }

    // no-quotes
    const quoteRegex = /['"]/g;
    while ((match = quoteRegex.exec(line)) !== null) {
      quoteRegex.lastIndex = match.index + 1;
      errorNumber++;
      const row = i;
      const col = match.index + match[0].search(quoteRegex) + 2;
      quotes.push({
        row,
        col,
        snippet: getSnippet2(fm, col, i),
      });
    }

    // no-trailing-spaces
    const trailingSpaceRegex = /\s+$/g;
    if (line.search(trailingSpaceRegex) !== -1) {
      errorNumber++;
      const row = i;
      const col = line.length + 1;
      trailingSpaces.push({
        row,
        col,
        snippet: getSnippet2(fm, col, i),
      });
    }

    // no-brackets
    const bracketsRegex = /[\[\]]/g;
    while ((match = bracketsRegex.exec(line)) !== null) {
      bracketsRegex.lastIndex = match.index + 1;
      errorNumber++;
      const row = i;
      const col = match.index + match[0].search(bracketsRegex) + 2;
      brackets.push({
        row,
        col,
        snippet: getSnippet2(fm, col, i),
      });
    }

    // no-curly-braces
    const curlyBraceRegex = /[\{\}]/g;
    while ((match = curlyBraceRegex.exec(line)) !== null) {
      curlyBraceRegex.lastIndex = match.index + 1;
      errorNumber++;
      const row = i;
      const col = match.index + match[0].search(curlyBraceRegex) + 2;
      curlyBraces.push({
        row,
        col,
        snippet: getSnippet2(fm, col, i),
      });
    }
  }

  checkAttributes2(attributes, filePath);
  if (blankLines.length > 0) blankLinesError(blankLines, filePath);
  if (trailingSpaces.length > 0) trailingSpacesError(trailingSpaces, filePath);
  if (spacesBeforeColon.length > 0) spaceBeforeColonError(spacesBeforeColon, filePath);
  if (quotes.length > 0) quotesError(quotes, filePath);
  if (brackets.length > 0) bracketsError(brackets, filePath);
  if (curlyBraces.length > 0) curlyBracesError(curlyBraces, filePath);
}

function spaceBeforeColonError(spacesBeforeColon, filePath) {
  const spaces = spacesBeforeColon.reduce(
    (acc, curr) =>
      `${acc}\n  at ${process.cwd()}\\${filePath}:${curr.row}:${curr.col}.\n\n${
        curr.snippet
      }\n`,
    ""
  );
  console.log(
    `${chalk.red(
      "YAMLException:"
    )} there should be no whitespace before colons.\n${spaces}`
  );
  process.exitCode = 1;
}

function blankLinesError(blankLines, filePath) {
  const blankLinesStr = blankLines.reduce(
    (acc, curr) => `${acc}\n  at ${process.cwd()}\\${filePath}:${curr}.\n`,
    ""
  );
  console.log(
    `${chalk.red(
      "YAMLException:"
    )} there should be no empty lines.\n${blankLinesStr}`
  );
  process.exitCode = 1;
}

function quotesError(quotes, filePath) {
  const quotesStr = quotes.reduce(
    (acc, curr) =>
      `${acc}\n  at ${process.cwd()}\\${filePath}:${curr.row}:${curr.col}.\n\n${
        curr.snippet
      }\n`,
    ""
  );
  console.log(
    `${chalk.red("YAMLException:")} there should be no quotes.\n${quotesStr}`
  );
  process.exitCode = 1;
}

function trailingSpacesError(trailingSpaces, filePath) {
  const trailingSpacesStr = trailingSpaces.reduce(
    (acc, curr) =>
      `${acc}\n  at ${process.cwd()}\\${filePath}:${curr.row}:${curr.col}.\n\n${
        curr.snippet
      }\n`,
    ""
  );
  console.log(
    `${chalk.red(
      "YAMLException:"
    )} there should be no trailing spaces.\n${trailingSpacesStr}`
  );
  process.exitCode = 1;
}

function bracketsError(brackets, filePath) {
  const bracketsStr = brackets.reduce(
    (acc, curr) =>
      `${acc}\n  at ${process.cwd()}\\${filePath}:${curr.row}:${curr.col}.\n\n${
        curr.snippet
      }\n`,
    ""
  );
  console.log(
    `${chalk.red(
      "YAMLException:"
    )} there should be no brackets.\n${bracketsStr}`
  );
  process.exitCode = 1;
}

function curlyBracesError(curlyBraces, filePath) {
  const curlyBracesStr = curlyBraces.reduce(
    (acc, curr) =>
      `${acc}\n  at ${process.cwd()}\\${filePath}:${curr.row}:${curr.col}.\n\n${
        curr.snippet
      }\n`,
    ""
  );
  console.log(
    `${chalk.red(
      "YAMLException:"
    )} there should be no curly braces.\n${curlyBracesStr}`
  );
  process.exitCode = 1;
}

(() => {
  console.time("Linting took");
  (!args.recursive && !args.r
    ? lintNonRecursively(args.path)
    : lintRecursively(args.path)
  )
    .catch((err) => {
      console.log(err);
      process.exitCode = 1;
      errorNumber++;
    })
    .finally(endOfProcess);
})();

function endOfProcess() {
  if (!process.exitCode) {
    console.log(chalk.green("✔ All parsed files have valid front matter."));
  } else {
    console.log(
      chalk.red(
        `✘ ${errorNumber} error(s) found. Please fix the errors and try again.`
      )
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
    if (key === "path" && value.startsWith(process.cwd())) {
      value = value.replace(process.cwd(), "");
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
        return resolve();
      }

      Promise.all(promiseArr).then(resolve).catch(reject);
    } else if (config.extensions.some((ext) => path.endsWith(ext))) {
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
          path.endsWith(ignoredDirectory)
        ) &&
        !config.includeDirs.some((includedDirectory) =>
          path.endsWith(includedDirectory)
        )
      )
        return resolve();

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
    } else if (config.extensions.some((ext) => path.endsWith(ext))) {
      lintFile(path).then(resolve).catch(reject);
    } else return resolve();
  });
}

function lintFile(filePath) {
  return new Promise((resolve, reject) => {
    readFilePromise(filePath, "utf8")
      .then((data) => {
        const content = fm(data);
        if (!content.frontmatter) return resolve();

        lint(content.frontmatter)
          .then(() => {
            parseSpecialChars(data, filePath);
            return resolve();
          })
          .catch(reject);
      })
      .catch(reject);
  });
}
