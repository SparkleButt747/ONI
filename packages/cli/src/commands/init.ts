import { Command } from "commander";
import chalk from "chalk";

export const initCommand = new Command("init")
  .description("Index the current project for context-aware assistance")
  .action(async () => {
    const { initIndex } = await import("@oni/context");
    const projectDir = process.cwd();

    console.log(chalk.dim("Indexing project..."));
    const start = Date.now();

    const result = initIndex(projectDir);

    const elapsed = Date.now() - start;
    console.log(
      chalk.green("Index complete") +
        chalk.dim(` (${elapsed}ms)`),
    );
    console.log(
      chalk.dim(
        `  files: ${result.totalFiles} (${result.filesIndexed} indexed, ${result.filesSkipped} unchanged, ${result.filesRemoved} removed)`,
      ),
    );
    console.log(chalk.dim(`  symbols: ${result.totalSymbols}`));
    console.log(chalk.dim(`  tokens: ~${result.totalTokens.toLocaleString()}`));
  });
