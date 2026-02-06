# pi-package-test

A reference package demonstrating [pi coding agent](https://github.com/badlogic/pi-mono)'s package system features.

## What This Demonstrates

### 1. Package Structure

A pi package can include multiple resource types:

```
pi-package-test/
├── extensions/          # *.ts or *.js files
├── skills/              # Directories with SKILL.md
├── themes/              # *.json files
├── prompts/             # *.md files
├── node_modules/        # Bundled dependencies (optional)
└── package.json
```

If you follow this convention, no `pi` field is needed in package.json. Pi auto-discovers resources from these directories.

The `pi` field is only needed when:
- Resources are in non-standard locations
- You want to include resources from bundled dependencies
- You want to exclude certain files via patterns

### 2. Pi Manifest with Glob Patterns

The `pi` field in package.json declares resources using glob patterns and exclusions:

```json
{
  "pi": {
    "extensions": [
      "extensions",
      "node_modules/shitty-extensions/extensions",
      "!**/cost-tracker.ts",
      "!**/loop.ts"
    ],
    "skills": [
      "skills",
      "node_modules/shitty-extensions/skills"
    ],
    "themes": ["themes"],
    "prompts": ["prompts"]
  }
}
```

### 3. Bundling Other Pi Packages

To include resources from another package, use `bundledDependencies` to embed it in the published tarball:

```json
{
  "dependencies": {
    "shitty-extensions": "^1.0.1"
  },
  "bundledDependencies": [
    "shitty-extensions"
  ]
}
```

Without `bundledDependencies`, npm's hoisting could move the dependency elsewhere, breaking the `node_modules/...` paths in the manifest.

### 4. User-Side Filtering

Users can narrow down what the manifest provides in their settings.json:

```json
{
  "packages": [
    {
      "source": "npm:pi-package-test",
      "extensions": ["!**/oracle.ts"],
      "skills": [],
      "themes": []
    }
  ]
}
```

User filters layer on top of manifest filtering:
1. Manifest patterns are applied first (defines what package provides)
2. User patterns are applied on top (narrows down further)

So if the manifest excludes 10 extensions and user adds `"extensions": ["!**/oracle.ts"]`, all 11 are excluded.

## Contents

### Extensions
| Name | Source | Description |
|------|--------|-------------|
| confirm-destructive.ts | local | Prompts for confirmation before destructive bash commands |
| custom-footer.ts | local | Adds custom status to the footer |
| oracle.ts | bundled | Oracle extension for predictions |
| memory-mode.ts | bundled | Memory mode for persistent context |

### Skills
| Name | Source | Description |
|------|--------|-------------|
| transcribe | local | Speech-to-text transcription using Groq Whisper API |
| a-nach-b | bundled | Austrian public transit routing |

### Themes
- **funky** - A vibrant neon color theme

### Prompt Templates
- **/review** - Review code for bugs and improvements
- **/explain** - Explain code in simple terms

## Installation

```bash
pi install npm:pi-package-test
```

## License

MIT
