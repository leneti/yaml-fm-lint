<div align="center">
  <img src="assets/logo.png" alt="Project logo" height="160" />
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
![Package size](https://img.shields.io/bundlephobia/min/yaml-fm-lint?label=size)
![code style: prettier](https://img.shields.io/badge/code_style-prettier-ff69b4.svg)
![NPM version](https://img.shields.io/npm/v/yaml-fm-lint)

  </p>
</div>

---

**Content**

- [What is this?](#what-is-this)
- [Features](#features)
- [Install](#install)
- [Usage](#usage)
- [Configuration](#configuration)

## What is this

A CLI NodeJS script which extracts yaml front matter from markdown files and lints the extracted data based on a config file.

## Features

- **Quick** - Only has three light-weight dependencies and lints directories asyncronously.
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
- `--fix`: Automatically fix the errors.
- `--config`: Path to the config file if not in root directory of project.
- `-r, --recursive`: *(default: `false`)* Recursively lint all files in the given directory.
- `-m, --mandatory`: *(default: `true`)* If set to false will show warnings instead of errors if no front matter is found.
- `-q, --quiet`: *(default: `false`)* If set to true will not show erroneous code snippets.

## Configuration

### `.yaml-fm-lint.json`

[Default config file](https://github.com/leneti/yaml-fm-lint/blob/main/config/default.json)

| Property name      | default | description                                                                                                                       |
|--------------------|---------|-----------------------------------------------------------------------------------------------------------------------------------|
| excludeDirs        | [See default config](https://github.com/leneti/yaml-fm-lint/blob/main/config/default.json)   | An array of directories to exclude from linting (🛑You should not overwrite this in your config unless you know what you are doing) |
| extraExcludeDirs   | `[]`      | Additional array of directories to exclude from linting.                                                                       |
| extensions         | `[".md"]` | Array of extensions of files to parse.                                                                              |
| includeDirs        | `[]`      | Array of directories to include in linting.                                                                                    |
| requiredAttributes | `[]`      | Array of attributes that must be present in the yaml front matter.                                                             |
| mandatory          | `true`    | If set to false will show warning instead of error if no front matter is found.                                                 |