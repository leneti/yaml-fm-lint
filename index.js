#! /usr/bin/env node

/**
 * @typedef {{ noFrontMatter: true } | { customError: {row: number, col: number, message: string} } | {[message: string]: {row: number, col?: number, colStart?: number, colEnd?: number, snippet?: string}}} LintErrors
 * @typedef {{[message: string]: {row: number, col?: number, colStart?: number, colEnd?: number, snippet?: string}}} LintWarnings
 * @typedef {{filePath: string, fileErrors: number, fileWarnings: number, errors: LintErrors, warnings: LintWarnings}} LintResult
 * @typedef {{ path: string, fix: boolean, config: string, recursive: boolean, mandatory: boolean, quiet: boolean, oneline: boolean, colored: boolean }} LintArgs
 * @typedef {{ disabledAttributes: string[], excludeDirs: string[], extraExcludeDirs: string[], extensions: string[], includeDirs: string[], mandatory: boolean, requiredAttributes: string[] }} LintConfig
 * @typedef {string[] | number[] | { row: number, col: number, colStart?: number, colEnd?: number }[] | undefined} Affected
 */

const {
  readFileSync,
  lstatSync,
  readdirSync,
  writeFileSync,
  existsSync,
} = require("fs");
const chalk = require("chalk");
const { load, dump } = require("js-yaml");
const path = require("path");
const glob = require("glob");
const { lintLog } = require("./errors.js");

const cwd = process.cwd().replace(/\\/g, "/");

/**
 * @type LintArgs
 */
let args;

/**
 * @type LintConfig
 */
let config;

let errorNumber = 0;
let fixableErrors = 0;
let warningNumber = 0;
let allExcludedDirs = [];
const errorMessages = {
  missingAttributes: "missing required attributes",
  blankLines: "there must be no empty lines",
  spacesBeforeColon: "there must be no whitespace before colons",
  quotes: "there must be no quotes in the front matter",
  trailingSpaces: "there must be no trailing spaces",
  brackets: "there must be no brackets",
  curlyBraces: "there must be no curly braces",
  indentation:
    "lines cannot be indented more than 2 spaces from the previous line",
  trailingCommas: "there must be no trailing commas",
};
const warningMessages = {
  repeatingSpaces: "possibly unintended whitespace",
  warnCommas: "possibly unintended commas",
};

/**
 * Lints the front matter of all files in a directory non-recursively.
 * @param {string} path - path to file or directory
 * @returns {Promise<LintResult[]>} - null if everything is ok, otherwise error message
 */
function lintNonRecursively(path) {
  return new Promise((resolve, reject) => {
    if (lstatSync(path).isDirectory()) {
      try {
        const files = readdirSync(path, "utf8");

        const promiseArr = [];
        for (const file of files) {
          if (config.extensions.some((ext) => file.endsWith(ext))) {
            promiseArr.push(
              lintFile(`${path === "." ? "" : `${path}/`}${file}`)
            );
          }
        }

        if (!promiseArr.length) {
          console.log(`No markdown files found in ${cwd}/${path}.`);
          return resolve([]);
        }

        Promise.all(promiseArr).then(resolve).catch(reject);
      } catch (error) {
        reject(error);
      }
    } else if (config.extensions.some((ext) => path.endsWith(ext))) {
      lintFile(path)
        .then((lintRes) => resolve([lintRes]))
        .catch(reject);
    } else {
      reject(
        `${
          args.colored ? chalk.red("YAMLException:") : "YAMLException:"
        } ${path} does not have a valid extension.`
      );
    }
  });
}

/**
 * Lints the front matter of all files in a directory recursively.
 * @param {string} path - path to file or directory
 * @returns {Promise<LintResult[]>}
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
      ) {
        return resolve([]);
      }

      config = getConfig(args, path);

      try {
        const files = readdirSync(path, "utf8");

        const promiseArr = [];
        for (const file of files) {
          promiseArr.push(
            lintRecursively(`${path === "." ? "" : `${path}/`}${file}`)
          );
        }

        Promise.all(promiseArr).then(resolve).catch(reject);
      } catch (error) {
        reject(error);
      }
    } else if (config.extensions.some((ext) => path.endsWith(ext))) {
      lintFile(path)
        .then((lintRes) => resolve([lintRes]))
        .catch(reject);
    } else {
      return resolve([]);
    }
  });
}

/**
 * Lints files found with the provided glob pattern.
 * @param {string[]} files array of file paths
 * @returns {Promise<LintResult[]>}
 */
function lintGlob(files) {
  const promiseArr = files.map(file => lintFile(file));

  if (!promiseArr.length) {
    console.log(`No markdown files found with glob pattern "${args.path}".`);
    return Promise.resolve([]);
  }

  return Promise.all(promiseArr);
}

/**
 * Lints the file's YAML front matter.
 * @param {string} filePath path to file
 * @returns {Promise<LintResult>}
 */
function lintFile(filePath, text = "", a = {}, c = {}) {
  return new Promise(async (resolve, reject) => {
    try {
      let data = "";

      if (text) {
        data = text;
        args = { ...a };
        config = { ...c };
      } else {
        data = readFileSync(filePath, "utf8");
      }

      const lines = data.replace(/\r/g, "").split("\n");
      lines.unshift("");
      const fmClosingTagIndex = lines.indexOf("---", 2);

      if (!lines[1].startsWith("---") || fmClosingTagIndex === -1) {
        if (!args.quiet) {
          lintLog({
            type: config.mandatory ? "Error" : "Warning",
            message: "front matter not found",
            filePath,
            affected: "Make sure front matter is at the beginning of the file.",
            args,
            forceOneLine: true,
          });
        }

        if (config.mandatory) {
          process.exitCode = 1;
          errorNumber++;
        } else {
          warningNumber++;
        }

        return resolve({
          filePath,
          fileErrors: errorNumber,
          fileWarnings: warningNumber,
          errors: { noFrontMatter: true },
          warnings: {},
        });
      }

      let fmLines = lines.slice(0, fmClosingTagIndex + 1);

      try {
        const attributes = load(fmLines.filter((l) => l !== "---").join("\n"));
        let basic, extra;

        if (args.fix) {
          const fixedFm = dump(attributes)
            .split("\n")
            .map((line) => line.replace(/\s*,$/g, ""));
          fixedFm[fixedFm.unshift("", "---") - 1] = "---";
          const content = lines.slice(fmClosingTagIndex + 1).join("\n");
          writeFileSync(filePath, `${fixedFm.slice(1).join("\n")}\n${content}`);
          fmLines = fixedFm;
        }

        basic = lintLineByLine(fmLines, filePath);
        extra = extraLinters(attributes, fmLines, filePath);
        errorNumber += basic.fileErrors + extra.fileErrors;
        warningNumber += basic.fileWarnings + extra.fileWarnings;

        resolve({
          filePath,
          fileErrors: errorNumber,
          fileWarnings: warningNumber,
          errors: {
            ...basic.errors,
            ...extra.extraErrors,
          },
          warnings: {
            ...basic.warnings,
            ...extra.extraWarnings,
          },
        });
      } catch (error) {
        if (text) console.log("ERROR: ", error);

        errorNumber++;

        const row = error.mark ? error.mark.line + 1 : undefined;
        const col = error.mark ? error.mark.column + 1 : undefined;

        if (!args.quiet) {
          lintLog({
            type: "Error",
            message: error.reason,
            filePath,
            affected: [{ row, col }],
            args,
            fmLines,
          });
        }

        resolve({
          filePath,
          fileErrors: errorNumber,
          fileWarnings: warningNumber,
          errors: {
            customError: {
              message: error.reason,
              row,
              col,
            },
          },
          warnings: {},
        });
      }
    } catch (error) {
      reject(error);
    }
  });
}

/**
 * @param {{[key: string]: any}} attributes YAML front matter / metadata pairs
 * @param {string[]} fmLines front matter line array
 * @param {string} filePath path to file
 * @returns {{extraErrors: {[msg: string]: Affected[]}, extraWarnings: {[msg: string]: Affected[]}, fileErrors: number, fileWarnings: number}}
 */
function extraLinters(attributes, fmLines, filePath) {
  let extraErrors = {};
  let extraWarnings = {};
  let fileErrors = 0;
  let fileWarnings = 0;

  if (!config.extraLintFns) {
    return { extraErrors, extraWarnings, fileErrors, fileWarnings };
  }

  /**
   * @param {"Error" | "Warning"} type
   * @param {string} message
   * @param {Affected} affected - array of string values, line numbers or exact locations of errors or warnings. If omitted, errors/warnings will be shown on the opening front matter tags. `colStart` and `colEnd` are used by the VS Code extension.
   */
  function extraLintLog(type, message, affected) {
    lintLog({ type, message, filePath, affected, args, fmLines });

    const updateObj = (arr, msg, aff) => ({
      ...arr,
      [msg]: [...(arr[msg] || []), ...aff],
    });

    if (type === "Error") {
      extraErrors = updateObj(extraErrors, message, affected);
    } else {
      extraWarnings = updateObj(extraWarnings, message, affected);
    }
  }

  config.extraLintFns.forEach((linter) => {
    const { errors, warnings } = linter({
      attributes,
      fmLines,
      lintLog: extraLintLog,
    });
    fileErrors += errors;
    fileWarnings += warnings;
  });

  return { extraErrors, extraWarnings, fileErrors, fileWarnings };
}

/**
 * Parses given string and logs errors if any.
 * @param {string[]} fmLines front matter lines to parse
 * @param {string} filePath path to the file
 * @returns {LintResult}
 */
function lintLineByLine(fmLines, filePath) {
  let fileErrors = 0;
  let fileWarnings = 0;
  let match;
  let skip = false;

  const basicErrors = Object.keys(errorMessages).reduce(
    (acc, key) => ({
      ...acc,
      [errorMessages[key]]: [],
    }),
    {}
  );
  basicErrors[errorMessages.missingAttributes] = [...config.requiredAttributes];

  const basicWarnings = Object.keys(warningMessages).reduce(
    (acc, key) => ({
      ...acc,
      [warningMessages[key]]: [],
    }),
    {}
  );

  const oneLineErrors = [
    errorMessages.blankLines,
    errorMessages.missingAttributes,
  ];

  for (let i = 1; i < fmLines.length - 1; i++) {
    let line = fmLines[i];

    if (/^\s*#/.test(line)) {
      if (line.includes("fmlint-disable-next-line")) i++;
      continue;
    }

    if (/\s+#/.test(line)) {
      if (line.includes("fmlint-disable-line")) continue;
      line = line.substring(0, line.search(/\s*#/));
    }

    // no-empty-lines
    if (!args.fix && line.trim() === "") {
      fileErrors++;
      fixableErrors++;
      basicErrors[errorMessages.blankLines].push(i);
      continue;
    }

    // attributes
    if (/^"?\w+"?\s*:/.test(line)) {
      const atr = line.split(":")[0].trim();

      skip = config.disabledAttributes.includes(atr);

      const atrIndex =
        basicErrors[errorMessages.missingAttributes].indexOf(atr);
      if (atrIndex > -1) {
        basicErrors[errorMessages.missingAttributes].splice(atrIndex, 1);
      }
    }

    if (skip) continue;

    // no-whitespace-before-colon
    const wsbcRegex = /(\s+):/g;
    while (!args.fix && (match = wsbcRegex.exec(line)) !== null) {
      const wsbcLength = match[1].length + 1;
      fileErrors++;
      fixableErrors++;
      const row = i;
      const col = match.index + match[0].search(/:/) + 1;
      wsbcRegex.lastIndex = col - 1;
      basicErrors[errorMessages.spacesBeforeColon].push({
        row,
        col,
        colStart: col - wsbcLength,
        colEnd: col - 1,
      });
    }

    // no-quotes
    const quoteRegex = /['"]/g;
    while (!args.fix && (match = quoteRegex.exec(line)) !== null) {
      quoteRegex.lastIndex = match.index + 1;
      fileErrors++;
      fixableErrors++;
      const row = i;
      const col = match.index + match[0].search(quoteRegex) + 2;
      basicErrors[errorMessages.quotes].push({
        row,
        col,
      });
    }

    // no-trailing-spaces
    const trailingSpaceRegex = /(\s+)$/g;
    if (!args.fix && line.search(trailingSpaceRegex) !== -1) {
      const spaceCount = trailingSpaceRegex.exec(line)[0].length + 1;
      fileErrors++;
      fixableErrors++;
      const row = i;
      const col = line.length + 1;
      basicErrors[errorMessages.trailingSpaces].push({
        row,
        col,
        colStart: col - spaceCount,
        colEnd: col,
      });
    }

    // no-brackets
    const bracketsRegex = /[\[\]]/g;
    while (!args.fix && (match = bracketsRegex.exec(line)) !== null) {
      bracketsRegex.lastIndex = match.index + 1;
      fileErrors++;
      fixableErrors++;
      const row = i;
      const col = match.index + match[0].search(bracketsRegex) + 2;
      basicErrors[errorMessages.brackets].push({
        row,
        col,
      });
    }

    // no-curly-braces
    const curlyBraceRegex = /[\{\}]/g;
    while (!args.fix && (match = curlyBraceRegex.exec(line)) !== null) {
      curlyBraceRegex.lastIndex = match.index + 1;
      fileErrors++;
      fixableErrors++;
      const row = i;
      const col = match.index + match[0].search(curlyBraceRegex) + 2;
      basicErrors[errorMessages.curlyBraces].push({
        row,
        col,
      });
    }

    // incorrect-indentation
    const indentationCurr = line.search(/\S/g);
    if (!args.fix && indentationCurr > 0) {
      let indentationPrev = fmLines[i - 1].search(/\S/g);
      indentationPrev = indentationPrev === -1 ? 0 : indentationPrev;
      if (indentationCurr - indentationPrev > 2) {
        fileErrors++;
        fixableErrors++;
        const row = i;
        const col = indentationCurr + 1;
        basicErrors[errorMessages.indentation].push({
          row,
          col,
          colStart: 0,
          colEnd: col - 1,
        });
      }
    }

    // no-repeating-spaces
    const repeatingSpaceRegex = /\w(\s{2,})\w/g;
    while ((match = repeatingSpaceRegex.exec(line)) !== null) {
      const spaceCount = match[1].length + 1;
      repeatingSpaceRegex.lastIndex = match.index + 1;
      fileWarnings++;
      const row = i;
      const col = match.index + match[0].search(/\s\w/g) + 2;
      basicWarnings[warningMessages.repeatingSpaces].push({
        row,
        col,
        colStart: col - spaceCount,
        colEnd: col - 1,
      });
    }

    // one-space-after-colon
    const spacesAfterColon = /:([ \t]{2,})\S/g;
    while (!args.fix && (match = spacesAfterColon.exec(line)) !== null) {
      const spaceCount = match[1].length + 1;
      spacesAfterColon.lastIndex = match.index + 1;
      fileWarnings++;
      const row = i;
      const col = match.index + match[0].search(/[ \t]\S/g) + 2;
      basicWarnings[warningMessages.repeatingSpaces].push({
        row,
        col,
        colStart: col - spaceCount,
        colEnd: col - 1,
      });
    }

    // no-trailing-commas
    const trailingCommaRegex = /,\s*$/g;
    if (!args.fix && line.search(trailingCommaRegex) !== -1) {
      fileErrors++;
      fixableErrors++;
      const row = i;
      const col = line.length + 1;
      basicErrors[errorMessages.trailingCommas].push({
        row,
        col,
      });
    }

    // warn-commas-in-front-matter
    const commaInFrontMatterRegex = /,./g;
    while ((match = commaInFrontMatterRegex.exec(line)) !== null) {
      commaInFrontMatterRegex.lastIndex = match.index + 1;
      fileWarnings++;
      const row = i;
      const col = match.index + 2;
      basicWarnings[warningMessages.warnCommas].push({
        row,
        col,
      });
    }
  }

  fileErrors += basicErrors[errorMessages.missingAttributes].length;

  if (!args.quiet) {
    Object.keys(basicErrors).forEach((message) => {
      if (basicErrors[message].length > 0) {
        lintLog({
          type: "Error",
          message,
          filePath,
          fmLines,
          affected: basicErrors[message],
          args,
          forceOneLine: oneLineErrors.includes(message),
        });
      }
    });

    Object.keys(basicWarnings).forEach((message) => {
      if (basicWarnings[message].length > 0) {
        lintLog({
          type: "Warning",
          message,
          filePath,
          fmLines,
          affected: basicWarnings[message],
          args,
        });
      }
    });
  }

  return {
    filePath,
    fileErrors,
    fileWarnings,
    errors: basicErrors,
    warnings: basicWarnings,
  };
}

/**
 * Retrieves arguments from the command line
 * @returns {LintArgs} - arguments object
 */
function getArguments() {
  let pathRead = false;
  const argv = process.argv.slice(2).reduce((acc, curr) => {
    let [key, value] = curr.split("=");
    if (key.startsWith("-")) {
      key = key.replace(/^-{1,2}/, "");
    } else if (!pathRead) {
      value = key;
      key = "path";
      pathRead = true;
    } else {
      value = key;
      key = "path";
      console.log(
        `${chalk.red("Invalid argument:")} ${chalk.yellow(
          `\"${curr}\"`
        )}. Only one path argument is allowed.`
      );
      process.exitCode = 9;
    }

    if (key === "path") {
      if (value.startsWith(cwd)) {
        value = value.replace(`${cwd}/`, "");
      }
      value = value.replace(/\\/g, "/");
    }

    acc[key] = value === "false" ? false : value === undefined ? true : value;
    return acc;
  }, {});

  if (!pathRead) {
    console.log(
      `${chalk.red(
        "Invalid arguments:"
      )} No path argument found. Please specify a path.`
    );
    process.exitCode = 9;
  }

  return {
    colored:
      argv.colored !== undefined
        ? argv.colored
        : argv.c !== undefined
        ? argv.c
        : true,
    config: argv.config,
    fix: argv.fix !== undefined ? argv.fix : false,
    mandatory:
      argv.mandatory !== undefined
        ? argv.mandatory
        : argv.m !== undefined
        ? argv.m
        : true,
    oneline:
      argv.oneline !== undefined
        ? argv.oneline
        : argv.o !== undefined
        ? argv.o
        : false,
    path: argv.path,
    quiet:
      argv.quiet !== undefined
        ? argv.quiet
        : argv.q !== undefined
        ? argv.q
        : false,
    recursive:
      argv.recursive !== undefined
        ? argv.recursive
        : argv.r !== undefined
        ? argv.r
        : false,
    slash: argv.backslash || argv.bs ? "back" : "forward",
  };
}

/**
 * Finds and returns the custom linter config, or the default one.
 * @param {LintArgs} a args object including at least `mandatory` and `config` values
 * @param {string} dir path to config file (current working directory by default)
 * @returns {LintConfig}
 */
function getConfig(a, dir = cwd) {
  let conf =
    dir === cwd
      ? {
          ...JSON.parse(
            readFileSync(`${__dirname.replace(/\\/g, "/")}/config/default.json`)
          ),
        }
      : config;

  if (existsSync(`${dir}/.yaml-fm-lint.js`)) {
    conf = {
      ...conf,
      ...require(path.resolve(cwd, `${dir}/.yaml-fm-lint.js`)),
    };
  } else if (existsSync(`${dir}/.yaml-fm-lint.json`)) {
    conf = {
      ...conf,
      ...JSON.parse(readFileSync(`${dir}/.yaml-fm-lint.json`)),
    };
  }

  if (a.config) {
    conf = {
      ...conf,
      ...(a.config.endsWith(".js")
        ? require(`${cwd}/${a.config}`)
        : JSON.parse(readFileSync(a.config))),
    };
  }

  conf.mandatory = a.mandatory !== undefined ? a.mandatory : conf.mandatory;

  return conf;
}

/**
 * @param {LintArgs} a args object
 * @param {LintConfig} c config object
 * @returns {Promise<{errors?: LintErrors, errorNumber: number, warningNumber: number}>}
 */
function main(a, c) {
  return new Promise((resolve) => {
    args = { ...a };
    config = { ...c };
    allExcludedDirs = [...config.excludeDirs, ...config.extraExcludeDirs];

    let lintPromise;

    if (args.path.includes("*")) {
      lintPromise = lintGlob(
        glob.sync(args.path, { ignore: "node_modules/**/*" })
      );
    } else if (args.recursive) {
      lintPromise = lintRecursively(args.path);
    } else {
      lintPromise = lintNonRecursively(args.path);
    }

    lintPromise
      .then((errors) => resolve({ errors, errorNumber, warningNumber }))
      .catch((err) => {
        console.log(err);
        process.exitCode = 1;
        errorNumber++;
        resolve({ errorNumber, warningNumber });
      });
  });
}

/**
 * @returns {{errorNumber: number, warningNumber: number, args: LintArgs, config: LintConfig}}
 */
function run() {
  return new Promise((resolve) => {
    console.time("Linting took");

    const a = getArguments();

    if (process.exitCode) {
      console.timeEnd("Linting took");
      return resolve({ errorNumber, warningNumber, args: a, config });
    }

    const c = getConfig(a);

    main(a, c)
      .then(({ errorNumber, warningNumber }) => {
        if (warningNumber) {
          console.log(
            args.colored
              ? chalk.yellow(
                  `⚠ ${warningNumber} warning${
                    warningNumber > 1 ? "s" : ""
                  } found.`
                )
              : `⚠ ${warningNumber} warning${
                  warningNumber > 1 ? "s" : ""
                } found.`
          );
        }
        if (errorNumber) {
          process.exitCode = 1;
          console.log(
            args.colored
              ? chalk.red(
                  `✘ ${errorNumber} error${
                    errorNumber === 1 ? "" : "s"
                  } found.${
                    fixableErrors > 0
                      ? ` ${fixableErrors} error${
                          fixableErrors === 1 ? "" : "s"
                        } fixable with the \`--fix\` option.`
                      : ""
                  }`
                )
              : `✘ ${errorNumber} error${errorNumber === 1 ? "" : "s"} found.${
                  fixableErrors > 0
                    ? ` ${fixableErrors} error${
                        fixableErrors === 1 ? "" : "s"
                      } fixable with the \`--fix\` option.`
                    : ""
                }`
          );
        } else if (!warningNumber) {
          console.log(
            args.colored
              ? chalk.green("✔ All parsed files have valid front matter.")
              : "✔ All parsed files have valid front matter."
          );
        }
        console.timeEnd("Linting took");
        return { errorNumber, warningNumber, args, config };
      })
      .then(resolve);
  });
}

module.exports = {
  run,
  main,
  lintFile,
  errorMessages,
  warningMessages,
};

// Run if invoked as a CLI
if (require.main === module) run();
