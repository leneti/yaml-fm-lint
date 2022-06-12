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

![NPM version](https://img.shields.io/npm/v/yaml-fm-lint)
![Package size](https://img.shields.io/bundlephobia/min/yaml-fm-lint?label=size)
![Dependencies](https://img.shields.io/depfu/dependencies/github/leneti/yaml-fm-lint)
![code style: prettier](https://img.shields.io/badge/code_style-prettier-ff69b4.svg)
![Version](https://img.shields.io/github/package-json/v/leneti/yaml-fm-lint?color=%23f88)

  </p>
</div>

---

**Content**

- [What is this?](##what%20is%20this)
- [Features](##features)
- [Install](##install)
- [Usage](##usage)
- [Configuration](##configuration)

## What is this

A CLI NodeJS script which extracts yaml front matter from markdown files and lints the extracted data based on a config file.

## Features ✨

- Speed - Only has two dependencies and lints directories asyncronously.
- Configurable - Lints the extracted data based on a config file.
- Tiny - Barely takes up space in your project.

## Install 🐙

You can either install the package as a dependency with
```sh
npm i --save yaml-fm-lint
```
```sh
yarn add yaml-fm-lint
```
or use it directly with 
```sh
npx yaml-fm-lint <path>
```

## Usage 💡

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
- `--config`: Path to the config file if not in root dir of project.
- `-r, --recursive`: Recursively lint all files in the given directory.

## Configuration 🖍

### `.yaml-fm-lint.json`

[Default config file](https://github.com/leneti/yaml-fm-lint/blob/main/config/default.json)

| Property name      | default | description                                                                                                                       |
|--------------------|---------|-----------------------------------------------------------------------------------------------------------------------------------|
| excludeDirs        | [See default config](https://github.com/leneti/yaml-fm-lint/blob/main/config/default.json)   | An array of directories to exclude from linting (🛑You should not overwrite this in your config unless you know what you are doing) |
| extraExcludeDirs   | []      | An additional array of directories to exclude from linting.                                                                       |
| includeDirs        | []      | An array of directories to include in linting.                                                                                    |
| requiredAttributes | []      | An array of attributes that must be present in the yaml front matter.                                                             |
