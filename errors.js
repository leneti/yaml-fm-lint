const chalk = require("chalk");
const cwd = process.cwd().replace(/\\/g, "/");

/**
 * @param {string[]} lines - the lines of the front matter
 * @param {number} col - the column number of the error
 * @param {number} row - the line number of the error
 * @returns the snippet of the line where the error occurred
 */
function getSnippet(lines, col, row) {
  return `${row - 1} | ${lines[row - 1]}\n${row} | ${
    lines[row]
  }\n${"----^".padStart(col + 3 + Math.floor(Math.log10(row)), "-")}\n${
    row + 1
  } | ${lines[row + 1]}\n`;
}

/**
 * @param {{ row: number, col: number }[]} arr
 * @param {string} filePath
 * @param {string[]} fmLines
 * @returns a string of the snippets of the lines where the errors occurred
 */
function getSnippets(arr, filePath, fmLines, args) {
  return arr.reduce((acc, curr) => {
    const path =
      args.slash === "back"
        ? `${cwd}/${filePath}`.replace(/\//g, "\\")
        : `${cwd}/${filePath}`;
    return `${acc}\n  at ${path}:${curr.row}:${curr.col}.\n\n${getSnippet(
      fmLines,
      curr.col,
      curr.row
    )}\n`;
  }, "");
}

/**
 * @param {"Error" | "Warning"} type - "Error" or "Warning"
 * @param {string} message - the message to show
 * @param {string} filePath - the file path of the file where the error occurred
 * @param {string[] | number[] | { row: number, col: number }[] | undefined} affected - affected value, line number or array of affected locations
 * @param {{ colored: boolean, oneline: boolean }} args - used to determine if the output should be colored and if the output should be shown on a single line
 */
function showOneline(type, message, filePath, affected, args) {
  const path =
    args.slash === "back"
      ? `${cwd}/${filePath}`.replace(/\//g, "\\")
      : `${cwd}/${filePath}`;
  if (affected === undefined) {
    const fileName = filePath.split("/").pop();
    console.log(
      `${
        args.colored
          ? (type === "Error" ? chalk.red : chalk.yellow)(`YAML-${type}:`)
          : `YAML-${type}:`
      } <${message}> ${path} ${fileName}${!args.oneline ? "\n" : ""}`
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
      } <${message}> ${path}:${
        typeof affected == "string" && affected.includes(":")
          ? affected
          : ` ${affected}`
      }${!args.oneline ? "\n" : ""}`
    );
}

/**
 * Log a front matter linting error or warning
 *
 * @param {{ type: "Error" | "Warning", message: string, filePath: string, fmLines?: string[], affected?: number[] | string[] | { row: number, col: number }[], args: { colored: boolean, oneline: boolean }, forceOneLine: boolean }} props information about the error or warning and how to log it
 */
function lintLog({
  type,
  message,
  filePath,
  fmLines,
  affected,
  args,
  forceOneLine = false,
}) {
  if (args.oneline || forceOneLine || !affected || !fmLines)
    return showOneline(type, message, filePath, affected, args);

  const snippets = getSnippets(affected, filePath, fmLines, args);
  console.log(
    `${
      args.colored
        ? (type === "Error" ? chalk.red : chalk.yellow)("YAMLException:")
        : "YAMLException:"
    } ${message}.\n${snippets}`
  );
}

module.exports = { lintLog };
