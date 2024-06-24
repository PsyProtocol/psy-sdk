const SampleProjectFiles = {
  "scripts/test.ts": "console.log('Hello, world!')",
  "scripts/otherdir/other.ts": "console.log('Hello, other!')",
  "ops/test/test.js": "console.log('Hello, ops!')",
}

export const sampleProject = {
  name: "demoProject",
  files: SampleProjectFiles
} as const;