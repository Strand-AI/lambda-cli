#!/usr/bin/env node

import { spawnSync } from "child_process";

/**
 * Returns the executable path which is located inside `node_modules`
 * The naming convention is lambda-mcp-${os}-${arch}
 * If the platform is `win32` or `cygwin`, executable will include a `.exe` extension.
 */
function getExePath(): string {
  const arch = process.arch;
  let os = process.platform as string;
  let extension = "";

  if (["win32", "cygwin"].includes(process.platform)) {
    os = "win32";
    extension = ".exe";
  }

  try {
    return require.resolve(`@strand-ai/lambda-mcp-${os}-${arch}/bin/lambda-mcp${extension}`);
  } catch (e) {
    throw new Error(
      `Couldn't find lambda-mcp binary for ${os}-${arch}. ` +
      `Please report this issue at https://github.com/Strand-AI/lambda-cli/issues`
    );
  }
}

/**
 * Runs lambda-mcp with the given arguments
 */
function run(): void {
  const args = process.argv.slice(2);
  const result = spawnSync(getExePath(), args, { stdio: "inherit" });
  process.exit(result.status ?? 0);
}

run();
