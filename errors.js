const chalk = require("chalk");
const cwd = process.cwd().replace(/\\/g, "/");

function getSnippets(arr, filePath) {
  return arr.reduce(
    (acc, curr) =>
      `${acc}\n  at ${cwd}/${filePath}:${curr.row}:${curr.col}.\n\n${curr.snippet}\n`,
    ""
  );
}

function showError(message, filePath, errors, args, forceOneLine = false) {
  if (args.oneline || forceOneLine) return showOneline("Error", message, filePath, errors, args);
  const snippets = getSnippets(errors, filePath);
  console.log(
    `${
      args.colored ? chalk.red("YAMLException:") : "YAMLException:"
    } ${message}.\n${snippets}`
  );
}

function showWarning(message, filePath, warnings, args, forceOneLine = false) {
  if (args.oneline || forceOneLine) return showOneline("Warning", message, filePath, warnings, args);
  const snippets = getSnippets(warnings, filePath);
  console.log(
    `${
      args.colored ? chalk.yellow("YAMLException:") : "YAMLException:"
    } ${message}.\n${snippets}`
  );
}

/**
 * Allows users to see the error in a single line
 *
 * @param {"Error" | "Warning"} type - "Error" or "Warning"
 * @param {string} message - the message to show
 * @param {string} filePath - the file path of the file where the error occurred
 * @param {string | number | {row: number, col: number, snippet?: string}[] | undefined} affected - affected value, line number or array of affected locations
 * @param {boolean} args.colored - whether to color the output
 */
function showOneline(type, message, filePath, affected, args) {
  if (affected === undefined) {
    const fileName = filePath.split("/").pop();
    console.log(
      `${
        args.colored
          ? (type === "Error" ? chalk.red : chalk.yellow)(`YAML-${type}:`)
          : `YAML-${type}:`
      } <${message}> ${cwd}/${filePath} ${fileName}${!args.oneline ? "\n" : ""}`
    );
  } else if (typeof affected === "object") {
    affected.forEach((err) => {
      showOneline(
        type,
        message,
        filePath,
        typeof err === "object" ? `${err.row}:${err.col}` : err,
        args
      );
    });
  } else
    console.log(
      `${
        args.colored
          ? (type === "Error" ? chalk.red : chalk.yellow)(`YAML-${type}:`)
          : `YAML-${type}:`
      } <${message}> ${cwd}/${filePath}: ${affected}${!args.oneline ? "\n" : ""}`
    );
}

module.exports = {
  showOneline,
  showError,
  showWarning,
};
