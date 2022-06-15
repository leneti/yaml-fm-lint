#! /usr/bin/env node
import path from "path";
import { fileURLToPath } from "url";
import { readFileSync, lstatSync, readdir, readFile, readdirSync } from "fs";
import { promisify } from "util";
import { lint } from "yaml-lint";
import chalk from "chalk";
import {
  checkAttributes,
  indentationError,
  spaceBeforeColonError,
  blankLinesError,
  quotesError,
  trailingSpacesError,
  bracketsError,
  curlyBracesError,
  repeatingSpacesError,
} from "./errors.js";

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
if (!!args.m || !!args.mandatory) {
  const mandatory =
    args.m === "true"
      ? true
      : args.m === "false"
      ? false
      : args.mandatory === "true"
      ? true
      : args.mandatory === "false"
      ? false
      : config.mandatory;
  config = { ...config, mandatory };
}
const allExcludedDirs = [...config.excludeDirs, ...config.extraExcludeDirs];
let errorNumber = 0,
  warningNumber = 0;

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
  if (warningNumber) {
    console.log(
      chalk.yellow(
        `⚠ ${warningNumber} warning${warningNumber > 1 ? "s" : ""} found.`
      )
    );
  }
  if (!process.exitCode) {
    console.log(chalk.green("✔ All parsed files have valid front matter."));
  } else {
    console.log(
      chalk.red(
        `✘ ${errorNumber} error${
          errorNumber > 1 ? "s" : ""
        } found. Please fix the error${
          errorNumber > 1 ? "s" : ""
        } and try again.`
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
    if (key === "path") {
      if (value.startsWith(process.cwd())) {
        value = value.replace(process.cwd(), "");
      }
      value = value.replace(/\//g, "\\");
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

function getSnippet(lines, col, i) {
  return `${i - 1 > 0 ? `${i - 1} | ${lines[i - 1]}\n` : ""}${i} | ${
    lines[i]
  }\n${"----^".padStart(col + 3 + Math.floor(Math.log10(i)), "-")}\n${
    i + 1 < lines.length ? `${i + 1} | ${lines[i + 1]}\n` : ""
  }`;
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

/**
 * Lints the file's YAML front matter.
 * @param {string} filePath - path to file
 * @returns null if everything is ok, otherwise error message
 */
function lintFile(filePath) {
  return new Promise((resolve, reject) => {
    readFilePromise(filePath, "utf8")
      .then((data) => {
        const lines = data.replace(/\r/g, "").split("\n");
        lines.unshift("");
        const fmClosingTagIndex = lines.indexOf("---", 2);

        if (!lines[1].startsWith("---") || fmClosingTagIndex === -1) {
          console.log(
            `${(config.mandatory ? chalk.red : chalk.yellow)(
              "YAMLException:"
            )} front matter not found in ${process.cwd()}\\${filePath}. Make sure front matter is at the beginning of the file.\n`
          );
          if (config.mandatory) {
            process.exitCode = 1;
            errorNumber++;
          } else {
            warningNumber++;
          }
          return resolve();
        }

        const frontMatter = lines.slice(0, fmClosingTagIndex + 1);

        lint(frontMatter.join("\n"))
          .then(() => lintLineByLine(frontMatter, filePath))
          .then(resolve)
          .catch(reject);
      })
      .catch(reject);
  });
}

/**
 * Parses given string and logs errors if any.
 * @param {string} data - data to parse
 * @param {string} filePath - path to the file
 */
function lintLineByLine(fm, filePath) {
  const attributes = [];
  const spacesBeforeColon = [];
  const blankLines = [];
  const quotes = [];
  const trailingSpaces = [];
  const brackets = [];
  const curlyBraces = [];
  const indentation = [];
  const repeatingSpaces = [];
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
        snippet: getSnippet(fm, col, i),
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
        snippet: getSnippet(fm, col, i),
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
        snippet: getSnippet(fm, col, i),
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
        snippet: getSnippet(fm, col, i),
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
        snippet: getSnippet(fm, col, i),
      });
    }

    // incorrect-indentation
    const indentationCurr = line.search(/\S/g);
    if (indentationCurr > 0) {
      let indentationPrev = fm[i - 1].search(/\S/g);
      indentationPrev = indentationPrev === -1 ? 0 : indentationPrev;
      if (indentationCurr - indentationPrev > 2) {
        errorNumber++;
        const row = i;
        const col = indentationCurr + 1;
        indentation.push({
          row,
          col,
          snippet: getSnippet(fm, col, i),
        });
      }
    }

    // no-repeating-spaces
    const repeatingSpaceRegex = /\w\s{2,}\w/g;
    while ((match = repeatingSpaceRegex.exec(line)) !== null) {
      repeatingSpaceRegex.lastIndex = match.index + 1;
      warningNumber++;
      const row = i;
      const col = match.index + match[0].search(/\s\w/g) + 2;
      repeatingSpaces.push({
        row,
        col,
        snippet: getSnippet(fm, col, i),
      });
    }
  }

  checkAttributes(attributes, config.requiredAttributes, filePath);
  if (blankLines.length > 0) blankLinesError(blankLines, filePath);
  if (trailingSpaces.length > 0) trailingSpacesError(trailingSpaces, filePath);
  if (spacesBeforeColon.length > 0)
    spaceBeforeColonError(spacesBeforeColon, filePath);
  if (quotes.length > 0) quotesError(quotes, filePath);
  if (brackets.length > 0) bracketsError(brackets, filePath);
  if (curlyBraces.length > 0) curlyBracesError(curlyBraces, filePath);
  if (indentation.length > 0) indentationError(indentation, filePath);
  if (repeatingSpaces.length > 0)
    repeatingSpacesError(repeatingSpaces, filePath);
}
