---
"title":  'readme' 
"asd": 'asd'
'description' : "it's just that"
keywords: ["lint", yaml, javascript, frontmatter] 
tags": "wrong tag"
---

A NodeJS script, which extracts yaml front matter from markdown files and lints the extracted data

## All front matter should have
- Keywords
- Tags
- sidebar_label

## Arguments
- `<path>, --path=<path>`: Path to the directory with markdown files
- `--config=<pathToConfig>`: Path to the config file if different from root of the project
- `-r, --recursive`: If true, the script will recursively search for markdown files in the given path