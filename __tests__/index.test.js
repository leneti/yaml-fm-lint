const mockArgs = {
  path: "examples",
  fix: false,
  config: undefined,
  recursive: false,
  mandatory: true,
  quiet: false,
  oneline: false,
  colored: false,
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
  describe("example file tests: ", () => {
    beforeEach(() => {
      jest.resetModules();
      console.log = jest.fn();
    });

    afterEach(jest.clearAllMocks);

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
        .catch(reject)
      })
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
        .catch(reject)
      })
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
        .catch(reject)
      })
    });

    it("testSpaceBeforeColon.md should return 'no whitespace before colons' error", () => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testSpaceBeforeColon.md" };
      
      return new Promise((resolve, reject) => {
        main(args, mockConfig)
        .then(({ errorNumber, warningNumber }) => {
          expect(console.log).toHaveBeenCalledWith(
            expect.stringMatching(/there should be no whitespace before colons/)
          );
          expect(errorNumber).toBe(1);
          expect(warningNumber).toBe(0);
        })
        .then(resolve)
        .catch(reject)
      })
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
        .catch(reject)
      })
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
        .catch(reject)
      })
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
        .catch(reject)
      })
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
        .catch(reject)
      })
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
        .catch(reject)
      })
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
        .catch(reject)
      })
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
        .catch(reject)
      })
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
        .catch(reject)
      })
    })
  });

  describe("args tests: ", () => {
    beforeEach(() => {
      jest.resetModules();
      console.log = jest.fn();
    });

    afterEach(jest.clearAllMocks);

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
      const argv = process.argv;
      console.time = jest.fn();
      console.timeEnd = jest.fn();
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
            process.argv = argv;
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
      const { writeFile: writeFilePromise } = require("fs/promises");

      jest.mock("fs/promises", () => ({
        ...jest.requireActual("fs/promises"),
        writeFile: jest.fn(() => Promise.resolve()),
      }));

      const args = {
        ...mockArgs,
        path: "examples/testBlankLines.md",
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
      const argv = process.argv;
      console.time = jest.fn();
      console.timeEnd = jest.fn();
      process.argv = [
        "node",
        "index.js",
        "examples/testPassing.md",
        "test",
      ];
      const { run } = require("../index");

      return new Promise((resolve, reject) => {
        run()
          .then(() => {
            expect(console.time).toHaveBeenCalled();
            expect(console.log).toHaveBeenCalledWith(
              expect.stringMatching(/Invalid argument.+test/)
            );
            expect(console.timeEnd).toHaveBeenCalled();
            process.argv = argv;
          })
          .then(resolve)
          .catch(reject);
      });
    })

  });
});
