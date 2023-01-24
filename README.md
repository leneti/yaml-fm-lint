<div align="center">
  <img src="assets/logo2.png" alt="Project logo" height="160" />
  <br>
  <br>
  <p>
    <b>yaml-fm-lint</b>
  </p>
  <p>
     <i>NodeJS linter for YAML front matter in markdown files</i>
  </p>
  <p>

<!-- ![Dependencies](https://img.shields.io/depfu/dependencies/github/leneti/yaml-fm-lint) -->

[![Package size](https://img.shields.io/bundlephobia/min/yaml-fm-lint?label=size)](https://bundlephobia.com/package/yaml-fm-lint)
[![NPM version](https://img.shields.io/npm/v/yaml-fm-lint?logo=npm&color=%23CB3837)](https://www.npmjs.com/package/yaml-fm-lint)
[![VS Code extension](https://img.shields.io/visual-studio-marketplace/v/leneti.yaml-fm-lint?color=success&label=VS%20Marketplace)](https://marketplace.visualstudio.com/items?itemName=leneti.yaml-fm-lint)

  </p>
</div>

---

**Content**

- [What is this?](#what-is-this)
- [Features](#features)
- [Install](#install)
- [Usage](#usage)
- [Configuration](#configuration)
  - [Disabling linting](#disabling-linting)
  - [Config files](#config-files)

## What is this

An opinionated CLI NodeJS script which extracts yaml front matter from markdown files, lints the extracted data based on a config file and, if required, fixes most issues regarding the yaml.

There is also a [VS Code extension](https://marketplace.visualstudio.com/items?itemName=leneti.yaml-fm-lint) which integrates this script within VS Code and decorates errors/warnings in real time.

## Features

- **Quick** - Only has two light-weight dependencies and lints directories asyncronously.
- **Configurable** - Lints the extracted data based on a config file.
- **Tiny** - Barely takes up space in your project.

## Install

You can either install the package as a devDependency with

```sh
npm i -D yaml-fm-lint
```

```sh
yarn add yaml-fm-lint -D
```

or use it directly with

```sh
npx yaml-fm-lint <path>
```

## Usage

Include the script in your `package.json` file:

```json
"scripts": {
  "fmlint": "yaml-fm-lint"
}
```

Then run the script:

```sh
npm run fmlint -- path/to/your/markdown/files
```

```sh
yarn run fmlint -- path/to/your/markdown/files
```

You can provide additional arguments:

- `--fix`: Automatically fix the errors
- `--config`: Path to the config file if not in root directory of project
- `-r, --recursive`: _(default: `false`)_ Recursively lint all files in the given directory
- `-m, --mandatory`: _(default: `true`)_ If set to false will show warnings instead of errors if no front matter is found
- `-q, --quiet`: _(default: `false`)_ If set to true will not show erroneous code snippets
- `-o, --oneline`: _(default: `false`)_ If set to true will condense error messages to one line, skipping snippets
- `-c, --colored`: _(default: `true`)_ If set to false will not color the output
- `-bs, --backslash`: _(default: `false`)_ When logging, use backslashes instead of forward slashes

### Example:

```sh
npm run fmlint -- docs --config="src/configs/.yaml-fm-lint.json" -r --oneline --colored=false
```

This command would recursively look for all markdown files in the `docs` directory and lint them based on the `.yaml-fm-lint.json` config file located under `src/configs/`. The output would not be colored and would not show code snippets.

You can also use glob patterns to find files.

```sh
npm run fmlint -- "**/[!README]*.{md,mdx}"
```

`node_modules` folder is ignored by default.

## Configuration

Text passed to `yaml-fm-lint` is parsed as YAML, analysed, and any issues reported.

### Disabling linting

To disable rules for a particular line within the front matter, add one of these markers to the appropriate place (comments don't affect the file's metadata):

- Disable all rules for the current line: `# fmlint-disable-line`
- Disable all rules for the next line: `# fmlint-disable-next-line`

For example:

```yaml
sidebar_label: Configuration
description: It's the configuration file # fmlint-disable-line
```

Or:

```yaml
sidebar_label: Configuration
 # fmlint-disable-next-line
description: It's the configuration file
```

### Config files

When run recursively, the script will look for the most nested config file, overriding properties from previous configurations.  
Config path specified in CLI arguments will never be overriden.

### `.yaml-fm-lint.json`

[Default config file](https://github.com/leneti/yaml-fm-lint/blob/main/config/default.json)

| Property name      | default                                                                                    | description                                                                                                                         |
| ------------------ | ------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------- |
| disabledAttributes | `[]`                                                                                       | Array of attributes to exclude from linting                                                                                         |
| excludeDirs        | [See default config](https://github.com/leneti/yaml-fm-lint/blob/main/config/default.json) | An array of directories to exclude from linting (🛑You should not overwrite this in your config unless you know what you are doing) |
| extraExcludeDirs   | `[]`                                                                                       | Additional array of directories to exclude from linting                                                                             |
| extensions         | `[".md"]`                                                                                  | Array of extensions of files to parse                                                                                               |
| includeDirs        | `[]`                                                                                       | Array of directories to include in linting                                                                                          |
| requiredAttributes | `[]`                                                                                       | Array of attributes that must be present in the yaml front matter                                                                   |
| mandatory          | `true`                                                                                     | If set to false will show warning instead of error if no front matter is found                                                      |

### `.yaml-fm-lint.js`

You will have to default export the config object.

#### Custom linters

In addition to the default config you can also add your own custom linters. These will be executed after the default linters.

The functions receive an object with the following properties:

- `filePath` - The path to the currently linted file
- `attributes` - The yaml front matter as a JavaScript object
- `fmLines` - The yaml front matter lines in a string array. Includes lines with `---` dashes
- `lintLog` - Function to call error/warning messages. Receives the following arguments:
  - `type` - "Error" or "Warning"
  - `message` - The error message
  - `affected` - This is either a `string[]` of word values, a `number[]` of erronious lines or an array of `objects` with `row` and `col` values for precise error locations (should include `colStart` and `colEnd` for decorations in the [VS Code extension](https://marketplace.visualstudio.com/items?itemName=leneti.yaml-fm-lint))

```js
/**
 * @param {{filePath: string, attributes: Object, fmLines: string[], lintLog: (type: "Error" | "Warning", message: string, affected: string[] | number[] | { row: number, col: number, colStart?: number, colEnd?: number }[] | undefined) => void}} props
 * @returns {{errors: number, warnings: number}}
 */
function lowercaseTags({ fmLines, lintLog }) {
  const tagsRegExp = /^tags.*:/g;

  const tagsLineIndex = fmLines.findIndex((line) => tagsRegExp.test(line));
  if (tagsLineIndex < 0) return { errors: 0, warnings: 0 };

  const eachTagRegExp = /^(\s*-\s+)(.+)$/;
  const locations = [];
  let errors = 0;

  for (let i = tagsLineIndex + 1; i < fmLines.length; i++) {
    const line = fmLines[i];
    if (!eachTagRegExp.test(line)) break;
    const match = line.match(eachTagRegExp);
    const tag = match[2];
    if (tag.toLowerCase() !== tag) {
      locations.push({
        row: i,
        col: match[1].length + 2,
        colStart: match[1].length,
        colEnd: match[1].length + tag.length,
      });
      errors++;
    }
  }

  lintLog("Error", "tags must be lowercase", locations);

  return { errors, warnings: 0 };
}

module.exports = {
  extraLintFns: [lowercaseTags],
  requiredAttributes: ["tags"],
};
```
