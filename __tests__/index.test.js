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
  excludeDirs: ["__mocks__", "__tests__", ".git", "coverage", "node_modules"],
  extraExcludeDirs: [],
  extensions: [".md"],
  includeDirs: [],
  mandatory: true,
  requiredAttributes: ["test"],
};

describe("yaml-fm-lint", () => {
  const orgProcess = { ...process };
  const orgConsole = { ...console };

  beforeEach(() => {
    console = {
      ...orgConsole,
      log: jest.fn().mockName("console.log"),
      time: jest.fn().mockName("console.time"),
      timeEnd: jest.fn().mockName("console.timeEnd"),
    };
  });

  afterEach(() => {
    jest.resetModules();
    jest.clearAllMocks();
    process = { ...orgProcess };
    console = { ...orgConsole };
  });

  describe("example file tests: ", () => {
    it("testPassing.md should return no errors/warnings", () => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testPassing.md" };

      return new Promise((resolve, reject) => {
        main(args, mockConfig)
          .then(({ errorNumber, warningNumber }) => {
            expect(errorNumber).toBe(0);
            expect(warningNumber).toBe(0);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("testMissingAttributes.md should return 'missing attributes' error", () => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testMissingAttributes.md" };

      return new Promise((resolve, reject) => {
        main(args, mockConfig)
          .then(({ errorNumber, warningNumber }) => {
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(/missing attributes.*test/)
            );
            expect(errorNumber).toBe(1);
            expect(warningNumber).toBe(0);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("testBlankLines.md should return 'no empty lines' error", () => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testBlankLines.md" };

      return new Promise((resolve, reject) => {
        main(args, mockConfig)
          .then(({ errorNumber, warningNumber }) => {
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(/there should be no empty lines/)
            );
            expect(errorNumber).toBe(1);
            expect(warningNumber).toBe(0);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("testTrailingSpaces.md should return 'no trailing spaces' error", () => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testTrailingSpaces.md" };

      return new Promise((resolve, reject) => {
        main(args, mockConfig)
          .then(({ errorNumber, warningNumber }) => {
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(/there should be no trailing spaces/)
            );
            expect(errorNumber).toBe(1);
            expect(warningNumber).toBe(0);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("testSpaceBeforeColon.md should return 'no whitespace before colons' error", () => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testSpaceBeforeColon.md" };

      return new Promise((resolve, reject) => {
        main(args, mockConfig)
          .then(({ errorNumber, warningNumber }) => {
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(
                /there should be no whitespace before colons/
              )
            );
            expect(errorNumber).toBe(1);
            expect(warningNumber).toBe(0);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("testQuotes.md should return 'no quotes' error", () => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testQuotes.md" };

      return new Promise((resolve, reject) => {
        main(args, mockConfig)
          .then(({ errorNumber, warningNumber }) => {
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(/there should be no quotes/)
            );
            expect(errorNumber).toBe(2);
            expect(warningNumber).toBe(0);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("testBrackets.md should return 'no brackets' error", () => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testBrackets.md" };

      return new Promise((resolve, reject) => {
        main(args, mockConfig)
          .then(({ errorNumber, warningNumber }) => {
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(/there should be no brackets/)
            );
            expect(errorNumber).toBe(2);
            expect(warningNumber).toBe(0);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("testCurlyBraces.md should return 'no curly braces' error", () => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testCurlyBraces.md" };

      return new Promise((resolve, reject) => {
        main(args, mockConfig)
          .then(({ errorNumber, warningNumber }) => {
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(/there should be no curly braces/)
            );
            expect(errorNumber).toBe(2);
            expect(warningNumber).toBe(0);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("testIndentation.md should return 'cannot be indented more than 2 spaces' error", () => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testIndentation.md" };

      return new Promise((resolve, reject) => {
        main(args, mockConfig)
          .then(({ errorNumber, warningNumber }) => {
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(/cannot be indented more than 2 spaces/)
            );
            expect(errorNumber).toBe(1);
            expect(warningNumber).toBe(0);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("testTrailingCommas.md should return 'no trailing commas' error", () => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testTrailingCommas.md" };

      return new Promise((resolve, reject) => {
        main(args, mockConfig)
          .then(({ errorNumber, warningNumber }) => {
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(/there should be no trailing commas/)
            );
            expect(errorNumber).toBe(1);
            expect(warningNumber).toBe(0);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("testCommas.md should return 'unintended commas' warning", () => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testCommas.md" };

      return new Promise((resolve, reject) => {
        main(args, mockConfig)
          .then(({ errorNumber, warningNumber }) => {
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(/unintended commas/)
            );
            expect(errorNumber).toBe(0);
            expect(warningNumber).toBe(1);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("testWhitespace.md should return 'unintended whitespace' warning", () => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testWhitespace.md" };

      return new Promise((resolve, reject) => {
        main(args, mockConfig)
          .then(({ errorNumber, warningNumber }) => {
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(/unintended whitespace/)
            );
            expect(errorNumber).toBe(0);
            expect(warningNumber).toBe(2);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("testBadFormat.md should return 'bad mapping entry' error", () => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testBadFormat.md" };

      return new Promise((resolve, reject) => {
        main(args, mockConfig)
          .then(({ errorNumber, warningNumber }) => {
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(/bad indentation of a mapping entry/)
            );
            expect(errorNumber).toBe(1);
            expect(warningNumber).toBe(0);
          })
          .then(resolve)
          .catch(reject);
      });
    });
  });

  describe("args tests: ", () => {
    it("should replace '.' with an empty string if included in lint dir", () => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "." };

      return new Promise((resolve, reject) => {
        main(args, mockConfig)
          .then(({ errorNumber, warningNumber }) => {
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(/front matter not found/) // main README.md does not have front matter
            );
            expect(errorNumber).toBe(1);
            expect(warningNumber).toBe(0);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("should lint deeply nested files with recursive flag", () => {
      const { main } = require("../index");
      const args = { ...mockArgs, recursive: true, path: "examples/deeply" };

      return new Promise((resolve, reject) => {
        main(args, mockConfig)
          .then(({ errorNumber, warningNumber }) => {
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(/there should be no empty lines/)
            );
            expect(errorNumber).toBe(1);
            expect(warningNumber).toBe(0);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("should not find or lint any nested files without the recursive flag", () => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/deeply" };

      return new Promise((resolve, reject) => {
        main(args, mockConfig)
          .then(({ errorNumber, warningNumber }) => {
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(/No markdown files found/)
            );
            expect(errorNumber).toBe(0);
            expect(warningNumber).toBe(0);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("should prioritise config provided as an argument", () => {
      process.argv = [
        "node",
        "index.js",
        "examples/testPassCustomConfig.md",
        "--config=examples/customConfig.json",
      ];
      const { run } = require("../index");

      return new Promise((resolve, reject) => {
        run()
          .then(() => {
            expect(console.time).toHaveBeenCalled();
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(/valid front matter/)
            );
            expect(console.timeEnd).toHaveBeenCalled();
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("should not lint files with extensions not in the config: non-recursive", () => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testPassing.md" };
      const config = { ...mockConfig, extensions: ["test"] };

      return new Promise((resolve, reject) => {
        main(args, config)
          .then(({ errorNumber, warningNumber }) => {
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(/does not have a valid extension/)
            );
            expect(errorNumber).toBe(1);
            expect(warningNumber).toBe(0);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("should not lint files with extensions not in the config: recursive", () => {
      const { main } = require("../index");
      const args = {
        ...mockArgs,
        path: "examples/testPassing.md",
        recursive: true,
      };
      const config = { ...mockConfig, extensions: ["test"] };

      return new Promise((resolve, reject) => {
        main(args, config)
          .then(({ errorNumber, warningNumber }) => {
            expect(console.log).not.toHaveBeenCalledWith();
            expect(errorNumber).toBe(0);
            expect(warningNumber).toBe(0);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("should not lint files in excluded directories", () => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/ignored", recursive: true };
      const config = { ...mockConfig, extraExcludeDirs: ["dir"] };

      return new Promise((resolve, reject) => {
        main(args, config)
          .then(({ errorNumber, warningNumber }) => {
            expect(console.log).not.toHaveBeenCalled();
            expect(errorNumber).toBe(0);
            expect(warningNumber).toBe(0);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("should always lint files in included directories", () => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/ignored", recursive: true };
      const config = {
        ...mockConfig,
        extraExcludeDirs: ["dir"],
        includeDirs: ["dir"],
      };

      return new Promise((resolve, reject) => {
        main(args, config)
          .then(({ errorNumber, warningNumber }) => {
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(/there should be no empty lines/)
            );
            expect(errorNumber).toBe(1);
            expect(warningNumber).toBe(0);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("should show a warning if no yaml is found and it is not mandatory", () => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testNoYaml.md" };
      const config = { ...mockConfig, mandatory: false };

      return new Promise((resolve, reject) => {
        main(args, config)
          .then(({ errorNumber, warningNumber }) => {
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(/front matter not found/)
            );
            expect(errorNumber).toBe(0);
            expect(warningNumber).toBe(1);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("should show an error if no yaml is found and it is mandatory", () => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testNoYaml.md" };
      const config = { ...mockConfig };

      return new Promise((resolve, reject) => {
        main(args, config)
          .then(({ errorNumber, warningNumber }) => {
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(/front matter not found/)
            );
            expect(errorNumber).toBe(1);
            expect(warningNumber).toBe(0);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("should fix errors if given the --fix flag", () => {
      const { main } = require("../index");
      const { writeFile: writeFilePromise } = require("fs").promises;

      jest.mock("fs", () => ({
        ...jest.requireActual("fs"),
        promises: {
          ...jest.requireActual("fs").promises,
          writeFile: jest.fn(() => Promise.resolve()),
        },
      }));

      const args = {
        ...mockArgs,
        path: "examples/testPassing.md",
        fix: true,
      };

      return new Promise((resolve, reject) => {
        main(args, mockConfig)
          .then(({ errorNumber, warningNumber }) => {
            expect(writeFilePromise).toHaveBeenCalled();
            expect(console.log).not.toHaveBeenCalled();
            expect(errorNumber).toBe(0);
            expect(warningNumber).toBe(0);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("should give 'invalid argument' error if given an invalid argument", () => {
      process.argv = ["node", "index.js", "examples/testPassing.md", "test"];
      const { run } = require("../index");

      return new Promise((resolve, reject) => {
        run()
          .then(() => {
            expect(console.time).toHaveBeenCalled();
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(/Invalid argument.+test/)
            );
            expect(console.timeEnd).toHaveBeenCalled();
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("should display errors on a single line if given the '--oneline' flag", () => {
      const { main } = require("../index");
      const args = {
        ...mockArgs,
        path: "examples/testNoYaml.md",
        oneline: true,
      };

      return new Promise((resolve, reject) => {
        main(args, mockConfig)
          .then(({ errorNumber, warningNumber }) => {
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(/<front matter not found>/)
            );
            expect(errorNumber).toBe(1);
            expect(warningNumber).toBe(0);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("should display 'missing attribute' error on a single line if given the '--oneline' flag", () => {
      const { main } = require("../index");
      const args = {
        ...mockArgs,
        path: "examples/testMissingAttributes.md",
        oneline: true,
      };

      return new Promise((resolve, reject) => {
        main(args, mockConfig)
          .then(({ errorNumber, warningNumber }) => {
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(/<missing required attribute>.+test/)
            );
            expect(errorNumber).toBe(1);
            expect(warningNumber).toBe(0);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("should display 'no empty lines' error on a single line if given the '--oneline' flag", () => {
      const { main } = require("../index");
      const args = {
        ...mockArgs,
        path: "examples/testBlankLines.md",
        oneline: true,
      };

      return new Promise((resolve, reject) => {
        main(args, mockConfig)
          .then(({ errorNumber, warningNumber }) => {
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(/<.+no empty lines>/)
            );
            expect(errorNumber).toBe(1);
            expect(warningNumber).toBe(0);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("should remove the cwd from path if it was included", () => {
      const { run } = require("../index");

      process.argv = [
        "node",
        "index.js",
        `${process.cwd().replace(/\\/g, "/")}/examples/testBadFormat.md`,
        "-o",
      ];

      return new Promise((resolve, reject) => {
        run()
          .then(({ args }) => {
            expect(args.path).toBe("examples/testBadFormat.md");
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("should show a 'No path argument found' error if no path is given", () => {
      const { run } = require("../index");

      process.argv = ["node", "index.js"];

      return new Promise((resolve, reject) => {
        run()
          .then(() => {
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(/No path argument found/)
            );
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("should show the number of warnings found", () => {
      const { run } = require("../index");

      process.argv = ["node", "index.js", "examples/testWhitespace.md"];

      return new Promise((resolve, reject) => {
        run()
          .then(({ warningNumber, errorNumber }) => {
            expect(warningNumber).toBe(2);
            expect(errorNumber).toBe(0);
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(/2 warnings found/)
            );
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("should show colored output by default", () => {
      const { run } = require("../index");
      jest.mock("chalk", () => ({
        ...jest.requireActual("chalk"),
        green: jest.fn((msg) => `chalk.green ${msg}`),
        red: jest.fn((msg) => `chalk.red ${msg}`),
        yellow: jest.fn((msg) => `chalk.yellow ${msg}`),
      }));

      process.argv = ["node", "index.js", "examples/testPassing.md"];

      return new Promise((resolve, reject) => {
        run()
          .then(({ args }) => {
            expect(args.colored).toBe(true);
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(/chalk.green/)
            );
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("should show uncolored if the '--colored' flag is set to false", () => {
      const { run } = require("../index");
      jest.mock("chalk", () => ({
        ...jest.requireActual("chalk"),
        green: jest.fn((msg) => `chalk.green ${msg}`),
        red: jest.fn((msg) => `chalk.red ${msg}`),
        yellow: jest.fn((msg) => `chalk.yellow ${msg}`),
      }));

      process.argv = [
        "node",
        "index.js",
        "examples/testPassing.md",
        "--colored=false",
      ];

      return new Promise((resolve, reject) => {
        run()
          .then(({ args }) => {
            expect(args.colored).toBe(false);
            expect(console.log).toHaveBeenCalledWith(
              expect.not.stringMatching(/chalk.green/)
            );
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("should not show full errors when given the '--quiet'", () => {
      const { main } = require("../index");
      const args = {
        ...mockArgs,
        path: "examples",
        quiet: true,
        recursive: true,
      };

      return new Promise((resolve, reject) => {
        main(args, mockConfig)
          .then(({ errorNumber, warningNumber }) => {
            expect(console.log).not.toHaveBeenCalled();
            expect(errorNumber).not.toBe(0);
            expect(warningNumber).not.toBe(0);
          })
          .then(resolve)
          .catch(reject);
      });
    });
  });

  describe("lintFile tests: ", () => {
    it("should return an error if filePath is invalid", () => {
      const { lintFile } = require("../index");
      const filePath = "no_such_file.md";
      return new Promise((resolve, reject) => {
        lintFile(filePath).then(reject).catch(resolve);
      });
    })

    it("testPassing.md should return no erorrs", () => {
      const { lintFile } = require("../index");
      const file = "examples/testPassing.md";
      const fileContents = require("fs").readFileSync(file, "utf8");
      return new Promise((resolve, reject) => {
        lintFile(file, fileContents, mockArgs, mockConfig)
          .then(({fileErrors}) => {
            expect(fileErrors).toBe(0);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("testMissingAttributes.md should return a missingAttributes containing 'test'", () => {
      const { lintFile } = require("../index");
      const file = "examples/testMissingAttributes.md";
      const fileContents = require("fs").readFileSync(file, "utf8");
      return new Promise((resolve, reject) => {
        lintFile(file, fileContents, mockArgs, mockConfig)
          .then(({fileErrors, errors}) => {
            expect(fileErrors).toBe(1);
            expect(errors.missingAttributes).toContain("test");
          })
          .then(resolve)
          .catch(reject);
      });
    })

    it("testBlankLines.md should return an array with the blank lines", () => {
      const { lintFile } = require("../index");
      const file = "examples/testBlankLines.md";
      const fileContents = require("fs").readFileSync(file, "utf8");
      return new Promise((resolve, reject) => {
        lintFile(file, fileContents, mockArgs, mockConfig)
          .then(({fileErrors, errors}) => {
            expect(fileErrors).toBe(1);
            expect(errors.blankLines).toContain(3);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("testWhitespace.md should return an array with the whitespace warnings", () => {
      const { lintFile } = require("../index");
      const file = "examples/testWhitespace.md";
      const fileContents = require("fs").readFileSync(file, "utf8");
      return new Promise((resolve, reject) => {
        lintFile(file, fileContents, mockArgs, mockConfig)
          .then(({fileErrors, errors}) => {
            expect(fileErrors).toBe(0);
            expect(errors.repeatingSpaces.length).toBe(2);
          })
          .then(resolve)
          .catch(reject);
      });
    });

    it("testNoYaml.md should return a 'noFrontMatter' error", () => {
      const { lintFile } = require("../index");
      const file = "examples/testNoYaml.md";
      const fileContents = require("fs").readFileSync(file, "utf8");
      return new Promise((resolve, reject) => {
        lintFile(file, fileContents, mockArgs, mockConfig)
          .then(({fileErrors, errors}) => {
            expect(fileErrors).toBe(1);
            expect(errors.noFrontMatter).toBe(true);
          })
          .then(resolve)
          .catch(reject);
      });
    })

    it("testBadFormat.md should return a 'bad mapping entry' error", () => {
      const { lintFile } = require("../index");
      const file = "examples/testBadFormat.md";
      const fileContents = require("fs").readFileSync(file, "utf8");
      return new Promise((resolve, reject) => {
        lintFile(file, fileContents, mockArgs, mockConfig)
          .then(({fileErrors, errors}) => {
            expect(fileErrors).toBe(1);
            expect(errors).toHaveProperty("customError")
          })
          .then(resolve)
          .catch(reject);
      });
    })
  });
});
