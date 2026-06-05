#!/usr/bin/env node
/**
 * @kontrocode/cli — the headless KontroCode agent for terminal use.
 *
 * Phase 1: a thin wrapper that prints a friendly banner and the input.
 * Real agent logic is added in Phase 2 once the Rust core is exposed
 * via FFI (or a separate `kontrocode-headless` binary).
 */

import chalk from "chalk";
import { Command } from "commander";

const program = new Command();

program
  .name("kontrocode")
  .description(
    "KontroCode — research-first, memory-aware coding agent. Headless CLI.",
  )
  .version("0.1.0", "-v, --version", "Print the version and exit.")
  .helpOption("-h, --help", "Print this help and exit.");

program
  .command("ask <prompt>")
  .description("Send a prompt to the KontroCode agent (headless).")
  .option("-p, --project <path>", "Project root", ".")
  .option("--no-research", "Skip the research step")
  .option(
    "-m, --model <id>",
    "Model to use (e.g. mock/echo, anthropic/claude-sonnet-4)",
    "mock/echo",
  )
  .action(async (prompt: string, opts: AskOptions) => {
    await ask(prompt, opts);
  });

program
  .command("info")
  .description("Print runtime info (version, project root, config path).")
  .action(() => {
    printBanner();
    console.log(`  ${chalk.gray("version")}     0.1.0 (Phase 1 scaffold)`);
    console.log(`  ${chalk.gray("node")}        ${process.version}`);
    console.log(
      `  ${chalk.gray("config")}      ${process.env["HOME"] ?? "."}/.config/kontrocode/config.toml`,
    );
    console.log();
    console.log(
      chalk.dim(
        "  Headless agent integration with the Rust core lands in Phase 2.",
      ),
    );
    console.log();
  });

program
  .command("config")
  .description("Print the resolved configuration.")
  .action(() => {
    console.log("config: Phase 2 will load ~/.config/kontrocode/config.toml");
  });

interface AskOptions {
  project: string;
  research: boolean;
  model: string;
}

async function ask(prompt: string, opts: AskOptions): Promise<void> {
  printBanner();
  console.log(`  ${chalk.gray("project")}  ${opts.project}`);
  console.log(`  ${chalk.gray("model")}    ${opts.model}`);
  console.log(
    `  ${chalk.gray("research")} ${opts.research ? chalk.green("on") : chalk.yellow("off")}`,
  );
  console.log();
  console.log(`  ${chalk.bold("›")} ${prompt}`);
  console.log();
  console.log(
    chalk.dim(
      "  Phase 1: the CLI prints the input. In Phase 2, this command",
    ),
  );
  console.log(
    chalk.dim(
      "  spawns the Rust agent and streams the response. Until then,",
    ),
  );
  console.log(
    chalk.dim("  use the desktop app to interact with the agent."),
  );
  console.log();
}

function printBanner(): void {
  const accent = chalk.hex("#3A3AFF").bold;
  console.log();
  console.log(
    `  ${accent("KontroCode")} ${chalk.gray("·")} ${chalk.gray(
      "The agent that knows before it codes.",
    )}`,
  );
  console.log();
}

program.parseAsync(process.argv).catch((err: unknown) => {
  console.error(chalk.red("Error: "), err instanceof Error ? err.message : err);
  process.exit(1);
});
