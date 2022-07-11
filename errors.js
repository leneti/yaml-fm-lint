const chalk = require("chalk");
const cwd = process.cwd().replace(/\\/g, "/");

function getSnippets(arr, filePath) {
  return arr.reduce(
    (acc, curr) =>
      `${acc}\n  at ${cwd}/${filePath}:${curr.row}:${curr.col}.\n\n${curr.snippet}\n`,
    ""
  );
}

function showError(message, filePath, errors, colored) {
  const snippets = getSnippets(errors, filePath);
  console.log(
    `${
      colored ? chalk.red("YAMLException:") : "YAMLException:"
    } ${message}.\n${snippets}`
  );
}

function showWarning(message, filePath, warnings, colored) {
  const snippets = getSnippets(warnings, filePath);
  console.log(
    `${
      colored ? chalk.yellow("YAMLException:") : "YAMLException:"
    } ${message}.\n${snippets}`
  );
}

function showOneline(type, message, filePath, affected, colored) {
  if (typeof affected === "object") {
    affected.forEach((errObj) => {
      showOneline(type, message, filePath, `${errObj.row}:${errObj.col}`, colored);
    });
  } else
    console.log(
      `${
        colored
          ? (type === "Error" ? chalk.red : chalk.yellow)(`YAML-${type}:`)
          : `YAML-${type}:`
      } <${message}> ${cwd}/${filePath}: ${affected}`
    );
}

function checkAttributes(attributes, requiredAttributes, filePath, args) {
  const missingAttributes = requiredAttributes.filter(
    (attr) => !attributes.includes(attr)
  );

  if (missingAttributes.length > 0) {
    if (args.oneline) {
      missingAttributes.forEach((attr) => {
        showOneline("Error", "missing required attribute", filePath, attr, args.colored);
      });
    } else if (!args.quiet)
      console.log(
        `${chalk.red(
          "YAMLException:"
        )} missing attributes in ${cwd}/${filePath}: ${missingAttributes.join(
          ", "
        )}\n`
      );

    return 1;
  }
  return 0;
}

function indentationError(indentation, filePath, args) {
  const message = "lines cannot be indented more than 2 spaces from the previous line";

  if (args.oneline) showOneline("Error", message, filePath, indentation, args.colored);
  else showError(message, filePath, indentation, args.colored);
}

function spaceBeforeColonError(spacesBeforeColon, filePath, args) {
  const message = "there should be no whitespace before colons";

  if (args.oneline) showOneline("Error", message, filePath, spacesBeforeColon, args.colored);
  else showError(message, filePath, spacesBeforeColon, args.colored);
}

function blankLinesError(blankLines, filePath, args) {
  const message = "there should be no empty lines";
  if (args.oneline) {
    blankLines.forEach((line) => {
      showOneline("Error", message, filePath, line, args.colored);
    });
  } else {
    const blankLinesStr = blankLines.reduce(
      (acc, curr) => `${acc}\n  at ${cwd}/${filePath}:${curr}.\n`,
      ""
    );
    console.log(`${chalk.red("YAMLException:")} ${message}.\n${blankLinesStr}`);
  }
}

function quotesError(quotes, filePath, args) {
  const message = "there should be no quotes in the front matter";

  if (args.oneline) showOneline("Error", message, filePath, quotes, args.colored);
  else showError(message, filePath, quotes, args.colored);
}

function trailingSpacesError(trailingSpaces, filePath, args) {
  const message = "there should be no trailing spaces";

  if (args.oneline) showOneline("Error", message, filePath, trailingSpaces, args.colored);
  else showError(message, filePath, trailingSpaces, args.colored);
}

function bracketsError(brackets, filePath, args) {
  const message = `there should be no brackets. Please use hyphen "-" symbols followed by a space to list items on separate lines`;

  if (args.oneline) showOneline("Error", "there should be no brackets", filePath, brackets, args.colored);
  else showError(message, filePath, brackets, args.colored);
}

function curlyBracesError(curlyBraces, filePath, args) {
  const message = "there should be no curly braces. Please list key-value pairs on separate lines indented with 2 spaces";

  if (args.oneline) showOneline("Error", "there should be no curly braces", filePath, curlyBraces, args.colored);
  else showError(message, filePath, curlyBraces, args.colored);
}

function trailingCommasError(trailingCommas, filePath, args) {
  const message = "there should be no trailing commas";

  if (args.oneline) showOneline("Error", message, filePath, trailingCommas, args.colored);
  else showError(message, filePath, trailingCommas, args.colored);
}

function repeatingSpacesWarning(repeatingSpaces, filePath, args) {
  const message = "possibly unintended whitespace";

  if (args.oneline) showOneline("Warning", message, filePath, repeatingSpaces, args.colored);
  else showWarning(message, filePath, repeatingSpaces, args.colored);
}

function warnCommasWarning(warnCommas, filePath, args) {
  const message = "possibly unintended commas";

  if (args.oneline) showOneline("Warning", message, filePath, warnCommas, args.colored);
  else showWarning(message, filePath, warnCommas, args.colored);
}

module.exports = {
  checkAttributes,
  indentationError,
  spaceBeforeColonError,
  blankLinesError,
  quotesError,
  trailingSpacesError,
  bracketsError,
  curlyBracesError,
  repeatingSpacesWarning,
  warnCommasWarning,
  trailingCommasError,
};
