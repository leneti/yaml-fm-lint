import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

const mockArgs = {
  path: "examples",
  fix: false,
  config: undefined,
  recursive: false,
  mandatory: true,
  quiet: false,
  oneline: false,
  colored: true,
};

const mockConfig = {
  disabledAttributes: [],
  excludeDirs: ["__mocks__", "__tests__", ".git", "coverage", "node_modules"],
  extraExcludeDirs: [],
  excludeFiles: [],
  extensions: [".md"],
  includeDirs: [],
  mandatory: true,
  requiredAttributes: ["test"],
};

describe("yaml-fm-lint", () => {
  let savedArgv;

  beforeEach(() => {
    savedArgv = process.argv;
    vi.spyOn(console, "log").mockImplementation(() => {});
    vi.spyOn(console, "time").mockImplementation(() => {});
    vi.spyOn(console, "timeEnd").mockImplementation(() => {});
  });

  afterEach(() => {
    vi.doUnmock("fs");
    vi.doUnmock("chalk");
    vi.resetModules();
    vi.restoreAllMocks();
    process.argv = savedArgv;
    process.exitCode = 0;
  });

  describe("example file tests: ", () => {
    it("testPassing.md should return no errors/warnings", async () => {
      const { main } = await import("../index.js");
      const args = { ...mockArgs, path: "examples/testPassing.md" };

      const { errorNumber, warningNumber } = await main(args, mockConfig);
      expect(errorNumber).toBe(0);
      expect(warningNumber).toBe(0);
    });

    it("testMissingAttributes.md should return 'missing attributes' error", async () => {
      const { main, errorMessages } = await import("../index.js");
      const args = { ...mockArgs, path: "examples/testMissingAttributes.md" };

      const { errorNumber, warningNumber } = await main(args, mockConfig);
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(
          new RegExp(
            `${errorMessages.missingAttributes}.+${mockConfig.requiredAttributes.join(".+")}`
          )
        )
      );
      expect(errorNumber).toBe(1);
      expect(warningNumber).toBe(0);
    });

    it("testBlankLines.md should return 'no empty lines' error", async () => {
      const { main, errorMessages } = await import("../index.js");
      const args = { ...mockArgs, path: "examples/testBlankLines.md" };

      const { errorNumber, warningNumber } = await main(args, mockConfig);
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(new RegExp(errorMessages.blankLines))
      );
      expect(errorNumber).toBe(1);
      expect(warningNumber).toBe(0);
    });

    it("testTrailingSpaces.md must return 'no trailing spaces' error", async () => {
      const { main, errorMessages } = await import("../index.js");
      const args = { ...mockArgs, path: "examples/testTrailingSpaces.md" };

      const { errorNumber, warningNumber } = await main(args, mockConfig);
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(new RegExp(errorMessages.trailingSpaces))
      );
      expect(errorNumber).toBe(1);
      expect(warningNumber).toBe(0);
    });

    it("testSpaceBeforeColon.md should return 'no whitespace before colons' error", async () => {
      const { main, errorMessages } = await import("../index.js");
      const args = { ...mockArgs, path: "examples/testSpaceBeforeColon.md" };

      const { errorNumber, warningNumber } = await main(args, mockConfig);
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(new RegExp(errorMessages.spaceBeforeColon))
      );
      expect(errorNumber).toBe(1);
      expect(warningNumber).toBe(0);
    });

    it("testQuotes.md should return 'no quotes' error", async () => {
      const { main, errorMessages } = await import("../index.js");
      const args = { ...mockArgs, path: "examples/testQuotes.md" };

      const { errorNumber, warningNumber } = await main(args, mockConfig);
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(new RegExp(errorMessages.quotes))
      );
      expect(errorNumber).toBe(2);
      expect(warningNumber).toBe(0);
    });

    it("testBrackets.md should return 'no brackets' error", async () => {
      const { main, errorMessages } = await import("../index.js");
      const args = { ...mockArgs, path: "examples/testBrackets.md" };

      const { errorNumber, warningNumber } = await main(args, mockConfig);
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(new RegExp(errorMessages.brackets))
      );
      expect(errorNumber).toBe(2);
      expect(warningNumber).toBe(0);
    });

    it("testCurlyBraces.md should return 'no curly braces' error", async () => {
      const { main, errorMessages } = await import("../index.js");
      const args = { ...mockArgs, path: "examples/testCurlyBraces.md" };

      const { errorNumber, warningNumber } = await main(args, mockConfig);
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(new RegExp(errorMessages.curlyBraces))
      );
      expect(errorNumber).toBe(2);
      expect(warningNumber).toBe(0);
    });

    it("testIndentation.md should return 'cannot be indented more than 2 spaces' error", async () => {
      const { main, errorMessages } = await import("../index.js");
      const args = { ...mockArgs, path: "examples/testIndentation.md" };

      const { errorNumber, warningNumber } = await main(args, mockConfig);
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(new RegExp(errorMessages.indentation))
      );
      expect(errorNumber).toBe(1);
      expect(warningNumber).toBe(0);
    });

    it("testTrailingCommas.md should return 'no trailing commas' error", async () => {
      const { main, errorMessages } = await import("../index.js");
      const args = { ...mockArgs, path: "examples/testTrailingCommas.md" };

      const { errorNumber, warningNumber } = await main(args, mockConfig);
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(new RegExp(errorMessages.trailingCommas))
      );
      expect(errorNumber).toBe(1);
      expect(warningNumber).toBe(0);
    });

    it("testCommas.md should return 'unintended commas' warning", async () => {
      const { main, warningMessages } = await import("../index.js");
      const args = { ...mockArgs, path: "examples/testCommas.md" };

      const { errorNumber, warningNumber } = await main(args, mockConfig);
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(new RegExp(warningMessages.warnCommas))
      );
      expect(errorNumber).toBe(0);
      expect(warningNumber).toBe(1);
    });

    it("testWhitespace.md should return 'unintended whitespace' warning", async () => {
      const { main, warningMessages } = await import("../index.js");
      const args = { ...mockArgs, path: "examples/testWhitespace.md" };

      const { errorNumber, warningNumber } = await main(args, mockConfig);
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(new RegExp(warningMessages.repeatingSpaces))
      );
      expect(errorNumber).toBe(0);
      expect(warningNumber).toBe(2);
    });

    it("testBadFormat.md should return 'bad mapping entry' error", async () => {
      const { main } = await import("../index.js");
      const args = { ...mockArgs, path: "examples/testBadFormat.md" };

      const { errorNumber, warningNumber } = await main(args, mockConfig);
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(/bad indentation of a mapping entry/)
      );
      expect(errorNumber).toBe(1);
      expect(warningNumber).toBe(0);
    });

    it("testLowercaseTags.md should only return 'missing attribute' error with no .js config", async () => {
      const { main, errorMessages } = await import("../index.js");
      const args = { ...mockArgs, path: "examples/testLowercaseTags.md" };

      const { errorNumber, warningNumber } = await main(args, mockConfig);
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(new RegExp(errorMessages.missingAttributes))
      );
      expect(errorNumber).toBe(1);
      expect(warningNumber).toBe(0);
    });

    it("testLowercaseTags.md should return 'tags must be lowercase' error with .js config", async () => {
      process.argv = ["node", "index.js", "examples/testLowercaseTags.md"];
      const { run } = await import("../index.js");
      const { writeFileSync, unlinkSync, readFileSync } = await import("fs");

      // an ESM config, because the package root is "type": "module"
      writeFileSync(".yaml-fm-lint.js", readFileSync("examples/customConfig.mjs"));

      let errorNumber;
      try {
        ({ errorNumber } = await run());
      } finally {
        unlinkSync(".yaml-fm-lint.js");
      }
      expect(console.time).toHaveBeenCalled();
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(/tags must be lowercase/)
      );
      expect(errorNumber).toBe(2);
      expect(console.timeEnd).toHaveBeenCalled();
    });
  });

  describe("args and config tests: ", () => {
    it("should replace '.' with an empty string if included in lint dir", async () => {
      const { main } = await import("../index.js");
      const args = { ...mockArgs, path: "." };

      const { errorNumber, warningNumber } = await main(args, mockConfig);
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(/front matter not found/) // main README.md does not have front matter
      );
      expect(errorNumber).toBe(1);
      expect(warningNumber).toBe(0);
    });

    it("should lint deeply nested files with recursive flag", async () => {
      const { main, errorMessages } = await import("../index.js");
      const args = { ...mockArgs, recursive: true, path: "examples/deeply" };

      const { errorNumber, warningNumber } = await main(args, mockConfig);
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(new RegExp(errorMessages.blankLines))
      );
      expect(errorNumber).toBe(1);
      expect(warningNumber).toBe(0);
    });

    it("should not find or lint any nested files without the recursive flag", async () => {
      const { main } = await import("../index.js");
      const args = { ...mockArgs, path: "examples/deeply" };

      const { errorNumber, warningNumber } = await main(args, mockConfig);
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(/No markdown files found/)
      );
      expect(errorNumber).toBe(0);
      expect(warningNumber).toBe(0);
    });

    it("should prioritise config provided as an argument", async () => {
      process.argv = [
        "node",
        "index.js",
        "examples/testPassCustomConfig.md",
        "--config=examples/customConfig.json",
      ];
      const { run } = await import("../index.js");

      await run();
      expect(console.time).toHaveBeenCalled();
      expect(console.log).toHaveBeenCalledWith(expect.stringMatching(/valid front matter/));
      expect(console.timeEnd).toHaveBeenCalled();
    });

    it("should use .js config if provided", async () => {
      process.argv = [
        "node",
        "index.js",
        "examples/testLowercaseTags.md",
        "--config=examples/customConfig.js",
      ];
      const { run } = await import("../index.js");

      const { errorNumber } = await run();
      expect(console.time).toHaveBeenCalled();
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(/tags must be lowercase/)
      );
      expect(errorNumber).toBe(2);
      expect(console.timeEnd).toHaveBeenCalled();
    });

    it("should use nested config if found", async () => {
      const { main } = await import("../index.js");
      const args = { ...mockArgs, path: "examples/nested", recursive: true };

      const { errorNumber, warningNumber } = await main(args, mockConfig);
      expect(console.log).not.toHaveBeenCalledWith();
      expect(errorNumber).toBe(0);
      expect(warningNumber).toBe(0);
    });

    it("should return no linting errors for disabled line", async () => {
      const { lintFile, warningMessages, errorMessages } = await import("../index.js");
      const path = "examples/testDisabledRule.md";
      const { readFileSync } = await import("fs");
      const fileDisabledLint = readFileSync(path, "utf8");
      const fileEnabledLint = fileDisabledLint
        .split("\n")
        .filter((line) => !/^\s*# fmlint-disable-next-line/.test(line))
        .map((line) => {
          const commentIndex = line.search(/\s+# fmlint-disable-line/);
          return commentIndex > 0 ? line.substring(0, commentIndex) : line;
        })
        .join("\n");

      const disabled = await lintFile(path, fileDisabledLint, mockArgs, mockConfig);
      expect(disabled.fileErrors).toBe(0);
      expect(disabled.fileWarnings).toBe(0);

      const { warnings, fileWarnings, errors, fileErrors } = await lintFile(
        path,
        fileEnabledLint,
        mockArgs,
        mockConfig
      );
      expect(fileWarnings).toBe(1);
      expect(fileErrors).toBe(1);
      expect(warnings[warningMessages.warnCommas].length).toBe(1);
      expect(errors[errorMessages.quotes].length).toBe(1);
    });

    it("should not lint attributes specified in config", async () => {
      const { main, errorMessages, warningMessages } = await import("../index.js");
      const args = { ...mockArgs, path: "examples/testDisabledRuleConfig.md" };
      const config = { ...mockConfig, disabledAttributes: ["disabledTags"] };

      const { errorNumber, warningNumber } = await main(args, config);
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(new RegExp(errorMessages.quotes))
      );
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(new RegExp(warningMessages.warnCommas))
      );
      expect(errorNumber).toBe(1);
      expect(warningNumber).toBe(1);
    });

    it("should not lint excluded files: non-recursive", async () => {
      const exclude = "testQuotes.md";
      const { main } = await import("../index.js");
      const args = { ...mockArgs, path: "examples/testQuotes.md" };
      const config = { ...mockConfig, excludeFiles: [exclude] };

      await main(args, config);
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(new RegExp(`excluded.*${exclude}`, "i"))
      );
    });

    it("should not lint excluded files: recursive", async () => {
      const exclude = "testQuotes.md";
      const { main } = await import("../index.js");
      const args = {
        ...mockArgs,
        path: "examples/testQuotes.md",
        recursive: true,
      };
      const config = { ...mockConfig, excludeFiles: [exclude] };

      await main(args, config);
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(new RegExp(`excluded.*${exclude}`, "i"))
      );
    });

    it("should not lint excluded files: glob", async () => {
      const { main } = await import("../index.js");
      const args = {
        ...mockArgs,
        path: "examples/glob/*.{md,mdx}",
      };
      const config = {
        ...mockConfig,
        excludeFiles: ["examples/glob/testQuotes.md", "examples\\glob\\testBlankLines.md"],
      };

      await main(args, config);
      expect(console.log).toHaveBeenCalledWith(expect.stringMatching(/excluded.*testQuotes/i));
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(/excluded.*testBlankLines/i)
      );
    });

    it("should return error when no directory found", async () => {
      const { main } = await import("../index.js");
      const args = { ...mockArgs, path: "notExamples" };

      const { errorNumber } = await main(args, mockConfig);
      expect(errorNumber).toBe(1);
    });

    it("should not lint files with extensions not in the config: non-recursive", async () => {
      const { main } = await import("../index.js");
      const args = { ...mockArgs, path: "examples/testPassing.md" };
      const config = { ...mockConfig, extensions: ["test"] };

      const { errorNumber, warningNumber } = await main(args, config);
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(/does not have a valid extension/)
      );
      expect(errorNumber).toBe(1);
      expect(warningNumber).toBe(0);
    });

    it("should not lint files with extensions not in the config: recursive", async () => {
      const { main } = await import("../index.js");
      const args = {
        ...mockArgs,
        path: "examples/testPassing.md",
        recursive: true,
      };
      const config = { ...mockConfig, extensions: [".test"] };

      const { errorNumber, warningNumber } = await main(args, config);
      expect(console.log).not.toHaveBeenCalledWith();
      expect(errorNumber).toBe(0);
      expect(warningNumber).toBe(0);
    });

    it("should not lint files in excluded directories", async () => {
      const { main } = await import("../index.js");
      const args = { ...mockArgs, path: "examples/ignored", recursive: true };
      const config = { ...mockConfig, extraExcludeDirs: ["dir"] };

      const { errorNumber, warningNumber } = await main(args, config);
      expect(console.log).not.toHaveBeenCalled();
      expect(errorNumber).toBe(0);
      expect(warningNumber).toBe(0);
    });

    it("should always lint files in included directories", async () => {
      const { main, errorMessages } = await import("../index.js");
      const args = { ...mockArgs, path: "examples/ignored", recursive: true };
      const config = {
        ...mockConfig,
        extraExcludeDirs: ["dir"],
        includeDirs: ["dir"],
      };

      const { errorNumber, warningNumber } = await main(args, config);
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(new RegExp(errorMessages.blankLines))
      );
      expect(errorNumber).toBe(1);
      expect(warningNumber).toBe(0);
    });

    it("should show a warning if no yaml is found and it is not mandatory", async () => {
      const { main } = await import("../index.js");
      const args = { ...mockArgs, path: "examples/testNoYaml.md" };
      const config = { ...mockConfig, mandatory: false };

      const { errorNumber, warningNumber } = await main(args, config);
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(/front matter not found/)
      );
      expect(errorNumber).toBe(0);
      expect(warningNumber).toBe(1);
    });

    it("should show an error if no yaml is found and it is mandatory", async () => {
      const { main } = await import("../index.js");
      const args = { ...mockArgs, path: "examples/testNoYaml.md" };
      const config = { ...mockConfig };

      const { errorNumber, warningNumber } = await main(args, config);
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(/front matter not found/)
      );
      expect(errorNumber).toBe(1);
      expect(warningNumber).toBe(0);
    });

    it("should fix errors if given the --fix flag", async () => {
      vi.doMock("fs", async (importOriginal) => ({
        ...(await importOriginal()),
        writeFileSync: vi.fn(),
      }));
      const { main } = await import("../index.js");
      const { writeFileSync } = await import("fs");

      const args = {
        ...mockArgs,
        path: "examples/testPassing.md",
        fix: true,
      };

      const { errorNumber, warningNumber } = await main(args, mockConfig);
      expect(writeFileSync).toHaveBeenCalled();
      expect(console.log).not.toHaveBeenCalled();
      expect(errorNumber).toBe(0);
      expect(warningNumber).toBe(0);
    });

    it("should give 'invalid argument' error if given an invalid argument", async () => {
      process.argv = ["node", "index.js", "examples/testPassing.md", "test"];
      const { run } = await import("../index.js");

      await run();
      expect(console.time).toHaveBeenCalled();
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(/Invalid argument.+test/)
      );
      expect(console.timeEnd).toHaveBeenCalled();
    });

    it("should display errors on a single line if given the '--oneline' flag", async () => {
      const { main } = await import("../index.js");
      const args = {
        ...mockArgs,
        path: "examples/testNoYaml.md",
        oneline: true,
      };

      const { errorNumber, warningNumber } = await main(args, mockConfig);
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(/<front matter not found>/)
      );
      expect(errorNumber).toBe(1);
      expect(warningNumber).toBe(0);
    });

    it("should display 'no brackets' error on a single line if given the '--oneline' flag", async () => {
      const { main, errorMessages } = await import("../index.js");
      const args = {
        ...mockArgs,
        path: "examples/testBrackets.md",
        oneline: true,
      };

      const { errorNumber, warningNumber } = await main(args, mockConfig);
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(new RegExp(`<${errorMessages.brackets}>`))
      );
      expect(errorNumber).toBe(2);
      expect(warningNumber).toBe(0);
    });

    it("should display 'no empty lines' error on a single line if given the '--oneline' flag", async () => {
      const { main, errorMessages } = await import("../index.js");
      const args = {
        ...mockArgs,
        path: "examples/testBlankLines.md",
        oneline: true,
      };

      const { errorNumber, warningNumber } = await main(args, mockConfig);
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(new RegExp(`<${errorMessages.blankLines}>`))
      );
      expect(errorNumber).toBe(1);
      expect(warningNumber).toBe(0);
    });

    it("should remove the cwd from path if it was included", async () => {
      const { run } = await import("../index.js");

      process.argv = [
        "node",
        "index.js",
        `${process.cwd().replace(/\\/g, "/")}/examples/testBadFormat.md`,
        "-o",
      ];

      const { args } = await run();
      expect(args.path).toBe("examples/testBadFormat.md");
    });

    it("should show a 'No path argument found' error if no path is given", async () => {
      const { run } = await import("../index.js");

      process.argv = ["node", "index.js"];

      await run();
      expect(console.log).toHaveBeenCalledWith(
        expect.stringMatching(/No path argument found/)
      );
    });

    it("should show the number of warnings found", async () => {
      const { run } = await import("../index.js");

      process.argv = ["node", "index.js", "examples/testWhitespace.md"];

      const { warningNumber, errorNumber } = await run();
      expect(warningNumber).toBe(2);
      expect(errorNumber).toBe(0);
      expect(console.log).toHaveBeenCalledWith(expect.stringMatching(/2 warnings found/));
    });

    it("should show colored output by default", async () => {
      vi.doMock("chalk", () => ({
        default: {
          green: vi.fn((msg) => `chalk.green ${msg}`),
          red: vi.fn((msg) => `chalk.red ${msg}`),
          yellow: vi.fn((msg) => `chalk.yellow ${msg}`),
        },
      }));
      const { run } = await import("../index.js");

      process.argv = ["node", "index.js", "examples/testPassing.md"];

      const { args } = await run();
      expect(args.colored).toBe(true);
      expect(console.log).toHaveBeenCalledWith(expect.stringMatching(/chalk.green/));
    });

    it("should show uncolored if the '--colored' flag is set to false", async () => {
      vi.doMock("chalk", () => ({
        default: {
          green: vi.fn((msg) => `chalk.green ${msg}`),
          red: vi.fn((msg) => `chalk.red ${msg}`),
          yellow: vi.fn((msg) => `chalk.yellow ${msg}`),
        },
      }));
      const { run } = await import("../index.js");

      process.argv = ["node", "index.js", "examples/testPassing.md", "--colored=false"];

      const { args } = await run();
      expect(args.colored).toBe(false);
      expect(console.log).toHaveBeenCalledWith(expect.not.stringMatching(/chalk.green/));
    });

    it("should not show full errors when given the '--quiet'", async () => {
      const { main } = await import("../index.js");
      const args = {
        ...mockArgs,
        path: "examples",
        quiet: true,
        recursive: true,
      };

      const { errorNumber, warningNumber } = await main(args, mockConfig);
      expect(console.log).not.toHaveBeenCalled();
      expect(errorNumber).not.toBe(0);
      expect(warningNumber).not.toBe(0);
    });

    it("includedDirPattern escapes regex metacharacters", async () => {
      const { includedDirPattern } = await import("../index.js");
      expect(() => includedDirPattern(["glob"], "c++")).not.toThrow();
      expect(includedDirPattern(["glob"], "c++").test("examples/glob/c++/a.md")).toBe(true);
      expect(includedDirPattern(["glob"], "docs.v2").test("examples/glob/docsXv2/a.md")).toBe(false);
    });

    it("includedDirPattern matches backslash paths", async () => {
      const { includedDirPattern } = await import("../index.js");
      expect(includedDirPattern(["glob"], "keep").test("examples\\glob\\keep\\a.md")).toBe(true);
      expect(includedDirPattern(["glob"], "keep").test("examples/glob/keep/a.md")).toBe(true);
    });

    it("glob lint does not crash on includeDirs with regex metacharacters", async () => {
      const { main } = await import("../index.js");
      const config = { ...mockConfig, excludeDirs: ["glob"], includeDirs: ["c++"] };
      const { errorNumber } = await main({ ...mockArgs, path: "examples/glob/*.md" }, config);
      expect(errorNumber).toBe(0);
    });
  });

  describe("lintFile() tests: ", () => {
    it("should return an error if filePath is invalid", async () => {
      const { lintFile } = await import("../index.js");
      const filePath = "no_such_file.md";
      await expect(lintFile(filePath)).rejects.toBeDefined();
    });

    it("testPassing.md should return no erorrs", async () => {
      const { lintFile } = await import("../index.js");
      const file = "examples/testPassing.md";
      const { readFileSync } = await import("fs");
      const fileContents = readFileSync(file, "utf8");
      const { fileErrors } = await lintFile(file, fileContents, mockArgs, mockConfig);
      expect(fileErrors).toBe(0);
    });

    it("testMissingAttributes.md should return a missingAttributes containing 'test'", async () => {
      const { lintFile, errorMessages } = await import("../index.js");
      const file = "examples/testMissingAttributes.md";
      const { readFileSync } = await import("fs");
      const fileContents = readFileSync(file, "utf8");
      const { fileErrors, errors } = await lintFile(file, fileContents, mockArgs, mockConfig);
      expect(fileErrors).toBe(1);
      expect(errors[errorMessages.missingAttributes]).toContain("test");
    });

    it("testBlankLines.md should return an array with the blank lines", async () => {
      const { lintFile, errorMessages } = await import("../index.js");
      const file = "examples/testBlankLines.md";
      const { readFileSync } = await import("fs");
      const fileContents = readFileSync(file, "utf8");
      const { fileErrors, errors } = await lintFile(file, fileContents, mockArgs, mockConfig);
      expect(fileErrors).toBe(1);
      expect(errors[errorMessages.blankLines]).toContain(3);
    });

    it("testWhitespace.md should return an array with the whitespace warnings", async () => {
      const { lintFile, warningMessages } = await import("../index.js");
      const file = "examples/testWhitespace.md";
      const { readFileSync } = await import("fs");
      const fileContents = readFileSync(file, "utf8");
      const { warnings, fileWarnings } = await lintFile(file, fileContents, mockArgs, mockConfig);
      expect(fileWarnings).toBe(2);
      expect(warnings[warningMessages.repeatingSpaces].length).toBe(2);
    });

    it("testNoYaml.md should return a 'noFrontMatter' error", async () => {
      const { lintFile } = await import("../index.js");
      const file = "examples/testNoYaml.md";
      const { readFileSync } = await import("fs");
      const fileContents = readFileSync(file, "utf8");
      const { fileErrors, errors } = await lintFile(file, fileContents, mockArgs, mockConfig);
      expect(fileErrors).toBe(1);
      expect(errors.noFrontMatter).toBe(true);
    });

    it("testBadFormat.md should return a 'bad mapping entry' custom error", async () => {
      const { lintFile } = await import("../index.js");
      const file = "examples/testBadFormat.md";
      const { readFileSync } = await import("fs");
      const fileContents = readFileSync(file, "utf8");
      const { fileErrors, errors } = await lintFile(file, fileContents, mockArgs, mockConfig);
      expect(fileErrors).toBe(1);
      expect(errors).toHaveProperty("customError");
    });
  });

  describe("packaging tests: ", () => {
    it("cjs consumer can require the package", async () => {
      const { createRequire } = await import("module");
      const mod = createRequire(import.meta.url)("../index.js");
      expect(typeof mod.lintFile).toBe("function");
      expect(typeof mod.run).toBe("function");
      expect(mod.errorMessages.quotes).toBe("there must be no quotes in the front matter");
    });

    it("cli.js lints a passing file with exit code 0 and a failing file with exit code 1", async () => {
      const { spawnSync } = await import("child_process");
      const lint = (file) => spawnSync(process.execPath, ["cli.js", file], { encoding: "utf8" });
      expect(lint("examples/testPassing.md").status).toBe(0);
      expect(lint("examples/testQuotes.md").status).toBe(1);
    });

    it("cli.js prints the summary only once", async () => {
      const { spawnSync } = await import("child_process");
      const r = spawnSync(process.execPath, ["cli.js", "examples/testQuotes.md", "--colored=false"], { encoding: "utf8" });
      expect(r.stdout.split("✘").length - 1).toBe(1);
    });

    it("direct node invocation (node index.js / node .) lints and sets the exit code", async () => {
      const { spawnSync } = await import("child_process");
      const lint = (...args) => spawnSync(process.execPath, args, { encoding: "utf8" }).status;
      expect(lint("index.js", "examples/testQuotes.md")).toBe(1);
      expect(lint("index.js", "examples/testPassing.md")).toBe(0);
      expect(lint(".", "examples/testQuotes.md")).toBe(1);
    });
  });
});
